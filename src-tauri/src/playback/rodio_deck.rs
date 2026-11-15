//! A [`Deck`] that decodes with symphonia (through rodio) and plays on the
//! default output device.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rodio::mixer::Mixer;
use rodio::{Decoder, Player, Source};

use super::error::PlaybackError;
use super::transport::Deck;

pub type TrackDecoder = Decoder<BufReader<File>>;

/// Opens a Track's file for decoding. Safe to call off the audio thread: the
/// decoder is `Send`, and opening one can be slow on a network drive.
pub fn open(path: &Path) -> Result<TrackDecoder, PlaybackError> {
    let file = File::open(path).map_err(|e| PlaybackError::Open(e.to_string()))?;
    let len = file
        .metadata()
        .map_err(|e| PlaybackError::Open(e.to_string()))?
        .len();
    let mut builder = Decoder::builder()
        .with_data(BufReader::new(file))
        .with_byte_len(len)
        .with_seekable(true)
        .with_gapless(true);
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        builder = builder.with_hint(&ext.to_ascii_lowercase());
    }
    builder
        .build()
        .map_err(|e| PlaybackError::Decode(e.to_string()))
}

/// The decoder's length, if the format reports one.
pub fn duration_of(decoder: &TrackDecoder) -> Option<Duration> {
    decoder.total_duration()
}

pub struct RodioDeck {
    player: Player,
    path: PathBuf,
}

impl RodioDeck {
    /// Connects a paused player to `mixer` with `decoder` queued on it.
    pub fn new(mixer: &Mixer, decoder: TrackDecoder, path: PathBuf) -> Self {
        let player = Player::connect_new(mixer);
        player.pause();
        player.append(decoder);
        Self { player, path }
    }
}

impl Deck for RodioDeck {
    fn play(&mut self) {
        self.player.play();
    }

    fn pause(&mut self) {
        self.player.pause();
    }

    fn seek(&mut self, to: Duration) -> Result<(), String> {
        // A finished Track has left the player's queue; queue it again.
        if self.player.empty() {
            let decoder = open(&self.path).map_err(|e| e.to_string())?;
            self.player.append(decoder);
        }
        self.player.try_seek(to).map_err(|e| e.to_string())
    }

    fn position(&self) -> Duration {
        self.player.get_pos()
    }

    fn is_finished(&self) -> bool {
        self.player.empty()
    }

    fn set_gain(&mut self, gain: f32) {
        self.player.set_volume(gain);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playback::test_audio::{RATE, aiff, wav, write_temp};

    fn decodes(name: &str, bytes: Vec<u8>) {
        let path = write_temp(name, &bytes);
        let decoder = open(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
        let duration = duration_of(&decoder).expect("duration");
        assert!(
            (duration.as_secs_f64() - 1.0).abs() < 0.01,
            "{name}: {duration:?}"
        );
        assert_eq!(decoder.count(), RATE as usize, "{name}: sample count");
        let _ = std::fs::remove_file(&path);
    }

    // WebView2 can't play AIFF, which is why playback is in Rust at all.
    #[test]
    fn decodes_aiff() {
        decodes("tone.aiff", aiff());
    }

    #[test]
    fn decodes_wav() {
        decodes("tone.wav", wav());
    }

    #[test]
    fn missing_file_is_an_open_error() {
        let err = open(Path::new("Z:/definitely/not/here.mp3")).err().unwrap();
        assert_eq!(err.kind(), "open");
    }
}
