//! The audio engine: one thread that owns the output device and the
//! [`Transport`]. Everything that touches playback runs there, in order, so
//! commands never race each other. It also ticks about 30 times a second to
//! report the position and notice when a Track ends.

use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use rodio::{DeviceSinkBuilder, MixerDeviceSink};

use super::error::PlaybackError;
use super::rodio_deck::{RodioDeck, TrackDecoder};
use super::transport::{LoadTicket, PlayerState, Status, Transport};

/// How often the position is reported while playing.
const TICK: Duration = Duration::from_millis(33);

/// What the engine tells the UI.
#[derive(Debug, Clone)]
pub enum EngineEvent {
    /// The loaded Track, status or duration changed.
    State(PlayerState),
    /// The position moved: while playing, or after a seek.
    Position { track_id: String, position: f64 },
}

/// A Track's file opened for decoding, ready to hand to the engine.
pub struct Opened {
    pub decoder: TrackDecoder,
    pub path: PathBuf,
    /// The decoder's length, else the length Rekordbox stored.
    pub duration: Duration,
}

/// State owned by the engine thread.
pub struct Core {
    pub transport: Transport<RodioDeck>,
    /// Opened on the first load, so the app starts without an audio device.
    sink: Option<MixerDeviceSink>,
}

impl Core {
    /// Finishes the load `ticket` started, returning whether its Track is now
    /// loaded. A stale load is dropped and gives `Ok(false)`, even if it
    /// failed: a newer load has replaced it.
    pub fn finish_load(
        &mut self,
        ticket: LoadTicket,
        opened: Result<Opened, PlaybackError>,
    ) -> Result<bool, PlaybackError> {
        if !self.transport.is_current(ticket) {
            return Ok(false);
        }
        let result = opened.and_then(|opened| {
            let sink = self.sink()?;
            let deck = RodioDeck::new(sink.mixer(), opened.decoder, opened.path);
            Ok(self.transport.finish_load(ticket, deck, opened.duration))
        });
        if result.is_err() {
            self.transport.fail_load(ticket);
        }
        result
    }

    fn sink(&mut self) -> Result<&MixerDeviceSink, PlaybackError> {
        if self.sink.is_none() {
            let mut sink = DeviceSinkBuilder::open_default_sink()
                .map_err(|e| PlaybackError::Output(e.to_string()))?;
            sink.log_on_drop(false);
            self.sink = Some(sink);
        }
        Ok(self.sink.as_ref().expect("opened above"))
    }
}

type Job = Box<dyn FnOnce(&mut Core) + Send>;

/// A handle to the engine thread. Kept in Tauri state.
pub struct Engine {
    jobs: mpsc::Sender<Job>,
}

impl Engine {
    /// Starts the engine thread. `emit` runs on that thread.
    pub fn start(mut emit: impl FnMut(EngineEvent) + Send + 'static) -> Self {
        let (jobs, rx) = mpsc::channel::<Job>();
        thread::Builder::new()
            .name("syncbox-audio".into())
            .spawn(move || run(rx, &mut emit))
            .expect("could not start the audio thread");
        Self { jobs }
    }

    /// Runs `f` on the engine thread and waits for its result. Blocks, so
    /// call it from a blocking task, not the main thread.
    pub fn call<R: Send + 'static>(
        &self,
        f: impl FnOnce(&mut Core) -> R + Send + 'static,
    ) -> Result<R, PlaybackError> {
        let (reply, result) = mpsc::sync_channel(1);
        self.jobs
            .send(Box::new(move |core| {
                let _ = reply.send(f(core));
            }))
            .map_err(|_| PlaybackError::EngineStopped)?;
        result.recv().map_err(|_| PlaybackError::EngineStopped)
    }
}

fn run(rx: mpsc::Receiver<Job>, emit: &mut dyn FnMut(EngineEvent)) {
    let mut core = Core {
        transport: Transport::default(),
        sink: None,
    };
    let mut last = core.transport.state();
    let mut next_tick = Instant::now() + TICK;
    loop {
        let ticked = match rx.recv_timeout(next_tick.saturating_duration_since(Instant::now())) {
            Ok(job) => {
                job(&mut core);
                false
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                core.transport.tick();
                next_tick = Instant::now() + TICK;
                true
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        };

        let state = core.transport.state();
        if (&state.track_id, state.status, state.duration)
            != (&last.track_id, last.status, last.duration)
        {
            emit(EngineEvent::State(state.clone()));
        }
        // Report the position on every tick while playing, and after any
        // command (a seek moves it while paused).
        let moved = !ticked || state.status == Status::Playing;
        if moved
            && state.status != Status::Loading
            && let Some(track_id) = &state.track_id
        {
            emit(EngineEvent::Position {
                track_id: track_id.clone(),
                position: state.position,
            });
        }
        last = state;
    }
}
