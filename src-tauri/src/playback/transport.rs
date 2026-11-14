//! The transport: which Track is loaded, whether it's playing, and the rules
//! for loading, seeking and finishing. Pure state over a [`Deck`], so it can be
//! tested without an audio device.

use std::time::Duration;

use serde::Serialize;

/// Something that plays one loaded Track.
pub trait Deck {
    fn play(&mut self);
    fn pause(&mut self);
    /// Seeks, reopening the Track first if it already played to the end.
    fn seek(&mut self, to: Duration) -> Result<(), String>;
    fn position(&self) -> Duration;
    /// True once the Track has played to the end.
    fn is_finished(&self) -> bool;
    /// Output gain, already mapped from the volume slider.
    fn set_gain(&mut self, gain: f32);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    /// Nothing loaded.
    Idle,
    Loading,
    Paused,
    Playing,
    /// Played to the end. Play starts again from the beginning.
    Ended,
}

/// Sent to the UI as the `playback-state` event.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub track_id: Option<String>,
    pub status: Status,
    /// Seconds.
    pub position: f64,
    /// Seconds; 0 until a Track has loaded.
    pub duration: f64,
}

/// Identifies one load request. Only the latest request may finish.
pub type LoadTicket = u64;

struct Loaded<D> {
    track_id: String,
    deck: D,
    duration: Duration,
}

pub struct Transport<D> {
    /// The latest load request. Older ones are stale when they finish.
    ticket: LoadTicket,
    loading: Option<String>,
    loaded: Option<Loaded<D>>,
    status: Status,
    volume: f32,
}

/// Maps the 0–1 volume slider to output gain. Squaring it makes the slider
/// feel even: a linear gain does almost nothing in its top half.
pub fn gain_for(volume: f32) -> f32 {
    let v = volume.clamp(0.0, 1.0);
    v * v
}

impl<D: Deck> Default for Transport<D> {
    fn default() -> Self {
        Self {
            ticket: 0,
            loading: None,
            loaded: None,
            status: Status::Idle,
            volume: 0.8,
        }
    }
}

impl<D: Deck> Transport<D> {
    /// Starts loading a Track. The current one stops straight away, so the old
    /// Track never keeps playing while the new one opens.
    pub fn begin_load(&mut self, track_id: &str) -> LoadTicket {
        self.ticket += 1;
        self.loaded = None;
        self.loading = Some(track_id.to_owned());
        self.status = Status::Loading;
        self.ticket
    }

    pub fn is_current(&self, ticket: LoadTicket) -> bool {
        ticket == self.ticket
    }

    /// Finishes a load. A stale ticket (another load started since) is
    /// ignored and its deck dropped, which stops and frees its decoder.
    /// Returns whether the deck was taken.
    pub fn finish_load(&mut self, ticket: LoadTicket, mut deck: D, duration: Duration) -> bool {
        if !self.is_current(ticket) {
            return false;
        }
        let Some(track_id) = self.loading.take() else {
            return false;
        };
        deck.set_gain(gain_for(self.volume));
        deck.pause();
        self.loaded = Some(Loaded {
            track_id,
            deck,
            duration,
        });
        self.status = Status::Paused;
        true
    }

    /// A load that failed. Leaves nothing loaded, unless it was stale.
    pub fn fail_load(&mut self, ticket: LoadTicket) {
        if self.is_current(ticket) {
            self.loading = None;
            self.status = Status::Idle;
        }
    }

    pub fn play(&mut self) {
        let Some(loaded) = &mut self.loaded else {
            return;
        };
        if self.status == Status::Ended {
            // A failed restart leaves it ended; there's nothing else to do.
            if loaded.deck.seek(Duration::ZERO).is_err() {
                return;
            }
        }
        loaded.deck.play();
        self.status = Status::Playing;
    }

    pub fn pause(&mut self) {
        if let Some(loaded) = &mut self.loaded
            && self.status == Status::Playing
        {
            loaded.deck.pause();
            self.status = Status::Paused;
        }
    }

    pub fn toggle(&mut self) {
        match self.status {
            Status::Playing => self.pause(),
            _ => self.play(),
        }
    }

    /// Seeks within the loaded Track, clamped to its length. Seeking after the
    /// end leaves it paused at the new position.
    pub fn seek(&mut self, seconds: f64) -> Result<(), String> {
        let Some(loaded) = &mut self.loaded else {
            return Ok(());
        };
        let mut to = Duration::from_secs_f64(seconds.max(0.0));
        if !loaded.duration.is_zero() {
            to = to.min(loaded.duration);
        }
        loaded.deck.seek(to)?;
        if self.status == Status::Ended {
            loaded.deck.pause();
            self.status = Status::Paused;
        }
        Ok(())
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(loaded) = &mut self.loaded {
            loaded.deck.set_gain(gain_for(self.volume));
        }
    }

    /// Notices a Track that has played to the end. Call regularly.
    pub fn tick(&mut self) {
        if let Some(loaded) = &self.loaded
            && self.status == Status::Playing
            && loaded.deck.is_finished()
        {
            self.status = Status::Ended;
        }
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn track_id(&self) -> Option<&str> {
        self.loaded
            .as_ref()
            .map(|l| l.track_id.as_str())
            .or(self.loading.as_deref())
    }

    /// Seconds into the loaded Track.
    pub fn position(&self) -> f64 {
        match &self.loaded {
            None => 0.0,
            Some(l) if self.status == Status::Ended => l.duration.as_secs_f64(),
            Some(l) => l.deck.position().as_secs_f64(),
        }
    }

    pub fn state(&self) -> PlayerState {
        PlayerState {
            track_id: self.track_id().map(str::to_owned),
            status: self.status,
            position: self.position(),
            duration: self
                .loaded
                .as_ref()
                .map_or(0.0, |l| l.duration.as_secs_f64()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;

    #[derive(Default, Debug)]
    struct Log {
        playing: bool,
        position: Duration,
        finished: bool,
        gain: f32,
        dropped: bool,
    }

    /// A deck that records what was asked of it.
    struct FakeDeck(Rc<RefCell<Log>>);

    impl Deck for FakeDeck {
        fn play(&mut self) {
            self.0.borrow_mut().playing = true;
        }
        fn pause(&mut self) {
            self.0.borrow_mut().playing = false;
        }
        fn seek(&mut self, to: Duration) -> Result<(), String> {
            let mut log = self.0.borrow_mut();
            log.position = to;
            log.finished = false;
            Ok(())
        }
        fn position(&self) -> Duration {
            self.0.borrow().position
        }
        fn is_finished(&self) -> bool {
            self.0.borrow().finished
        }
        fn set_gain(&mut self, gain: f32) {
            self.0.borrow_mut().gain = gain;
        }
    }

    impl Drop for FakeDeck {
        fn drop(&mut self) {
            self.0.borrow_mut().dropped = true;
        }
    }

    fn deck() -> (FakeDeck, Rc<RefCell<Log>>) {
        let log = Rc::new(RefCell::new(Log::default()));
        (FakeDeck(log.clone()), log)
    }

    const MINUTE: Duration = Duration::from_secs(60);

    fn loaded(id: &str) -> (Transport<FakeDeck>, Rc<RefCell<Log>>) {
        let mut t = Transport::default();
        let (d, log) = deck();
        let ticket = t.begin_load(id);
        assert!(t.finish_load(ticket, d, MINUTE));
        (t, log)
    }

    #[test]
    fn a_loaded_track_starts_paused_at_the_current_volume() {
        let mut t = Transport::default();
        t.set_volume(0.5);
        let (d, log) = deck();
        let ticket = t.begin_load("a");
        assert_eq!(t.status(), Status::Loading);
        t.finish_load(ticket, d, MINUTE);
        let state = t.state();
        assert_eq!(state.track_id.as_deref(), Some("a"));
        assert_eq!(state.status, Status::Paused);
        assert_eq!(state.duration, 60.0);
        assert!(!log.borrow().playing);
        assert_eq!(log.borrow().gain, 0.25);
    }

    #[test]
    fn a_stale_load_is_discarded_and_its_deck_dropped() {
        let mut t = Transport::default();
        let first = t.begin_load("a");
        let second = t.begin_load("b");
        let (stale, stale_log) = deck();
        assert!(!t.finish_load(first, stale, MINUTE));
        assert!(stale_log.borrow().dropped);
        assert_eq!(t.status(), Status::Loading);
        assert_eq!(t.track_id(), Some("b"));

        let (d, _) = deck();
        assert!(t.finish_load(second, d, MINUTE));
        assert_eq!(t.track_id(), Some("b"));
    }

    #[test]
    fn a_stale_failure_does_not_cancel_the_newer_load() {
        let mut t: Transport<FakeDeck> = Transport::default();
        let first = t.begin_load("a");
        t.begin_load("b");
        t.fail_load(first);
        assert_eq!(t.status(), Status::Loading);
        assert_eq!(t.track_id(), Some("b"));
    }

    #[test]
    fn loading_another_track_stops_the_current_one() {
        let (mut t, log) = loaded("a");
        t.play();
        t.begin_load("b");
        assert!(log.borrow().dropped);
        assert_eq!(t.status(), Status::Loading);
    }

    #[test]
    fn a_failed_load_leaves_nothing_loaded() {
        let (mut t, _) = loaded("a");
        let ticket = t.begin_load("b");
        t.fail_load(ticket);
        assert_eq!(t.state().track_id, None);
        assert_eq!(t.status(), Status::Idle);
    }

    #[test]
    fn toggle_plays_and_pauses() {
        let (mut t, log) = loaded("a");
        t.toggle();
        assert_eq!(t.status(), Status::Playing);
        assert!(log.borrow().playing);
        t.toggle();
        assert_eq!(t.status(), Status::Paused);
        assert!(!log.borrow().playing);
    }

    #[test]
    fn transport_commands_do_nothing_with_no_track() {
        let mut t: Transport<FakeDeck> = Transport::default();
        t.play();
        t.toggle();
        assert_eq!(t.seek(10.0), Ok(()));
        assert_eq!(t.status(), Status::Idle);
    }

    #[test]
    fn finishing_ends_and_play_restarts_from_the_beginning() {
        let (mut t, log) = loaded("a");
        t.play();
        log.borrow_mut().position = Duration::from_secs(59);
        log.borrow_mut().finished = true;
        t.tick();
        assert_eq!(t.status(), Status::Ended);
        assert_eq!(t.position(), 60.0);

        t.play();
        assert_eq!(t.status(), Status::Playing);
        assert_eq!(log.borrow().position, Duration::ZERO);
        assert!(!log.borrow().finished);
    }

    #[test]
    fn seeking_after_the_end_pauses_there() {
        let (mut t, log) = loaded("a");
        t.play();
        log.borrow_mut().finished = true;
        t.tick();
        t.seek(30.0).unwrap();
        assert_eq!(t.status(), Status::Paused);
        assert_eq!(t.position(), 30.0);
    }

    #[test]
    fn seek_is_clamped_to_the_track() {
        let (mut t, log) = loaded("a");
        t.seek(-5.0).unwrap();
        assert_eq!(log.borrow().position, Duration::ZERO);
        t.seek(500.0).unwrap();
        assert_eq!(log.borrow().position, MINUTE);
    }

    #[test]
    fn volume_changes_reach_the_loaded_deck() {
        let (mut t, log) = loaded("a");
        t.set_volume(1.0);
        assert_eq!(log.borrow().gain, 1.0);
        t.set_volume(7.0);
        assert_eq!(log.borrow().gain, 1.0);
        t.set_volume(0.0);
        assert_eq!(log.borrow().gain, 0.0);
    }

    #[test]
    fn state_serializes_for_the_ui() {
        let (t, _) = loaded("a");
        assert_eq!(
            serde_json::to_value(t.state()).unwrap(),
            serde_json::json!({
                "trackId": "a",
                "status": "paused",
                "position": 0.0,
                "duration": 60.0
            })
        );
    }
}
