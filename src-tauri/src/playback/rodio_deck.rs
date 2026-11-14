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

    const RATE: u32 = 8000;

    /// One second of 16-bit mono silence as AIFF (big-endian, an 80-bit
    /// extended float for the sample rate).
    fn aiff() -> Vec<u8> {
        let frames = RATE;
        let data_len = frames * 2;
        let mut b = Vec::new();
        b.extend(b"FORM");
        b.extend((4 + 26 + 16 + data_len).to_be_bytes());
        b.extend(b"AIFF");
        b.extend(b"COMM");
        b.extend(18u32.to_be_bytes());
        b.extend(1u16.to_be_bytes()); // channels
        b.extend(frames.to_be_bytes());
        b.extend(16u16.to_be_bytes()); // bits per sample
        // 8000 = 1.953125 × 2^12 → exponent 16383 + 12, mantissa 0xFA00…
        b.extend((16383u16 + 12).to_be_bytes());
        b.extend(0xFA00_0000_0000_0000u64.to_be_bytes());
        b.extend(b"SSND");
        b.extend((8 + data_len).to_be_bytes());
        b.extend([0u8; 8]); // offset, block size
        b.extend(vec![0u8; data_len as usize]);
        b
    }

    fn wav() -> Vec<u8> {
        let data_len = RATE * 2;
        let mut b = Vec::new();
        b.extend(b"RIFF");
        b.extend((36 + data_len).to_le_bytes());
        b.extend(b"WAVEfmt ");
        b.extend(16u32.to_le_bytes());
        b.extend(1u16.to_le_bytes()); // PCM
        b.extend(1u16.to_le_bytes()); // channels
        b.extend(RATE.to_le_bytes());
        b.extend((RATE * 2).to_le_bytes());
        b.extend(2u16.to_le_bytes());
        b.extend(16u16.to_le_bytes());
        b.extend(b"data");
        b.extend(data_len.to_le_bytes());
        b.extend(vec![0u8; data_len as usize]);
        b
    }

    fn decodes(name: &str, bytes: Vec<u8>) {
        let dir = std::env::temp_dir().join(format!("syncbox-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
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
