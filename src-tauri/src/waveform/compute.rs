//! Decoding a Track's file into Peaks, with the same decoder as playback.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use rodio::Source;

use super::peaks::{PeakReducer, Peaks, PeaksError};
use crate::playback::error::PlaybackError;
use crate::playback::rodio_deck;

/// Check for cancellation every this many samples.
const CANCEL_CHECK_SAMPLES: usize = 4096;

/// Decodes the whole file at `path` and reduces it to [`Peaks`]. Stops with
/// [`PeaksError::Cancelled`] soon after `cancel` is set.
pub fn compute_peaks(path: &Path, cancel: &AtomicBool) -> Result<Peaks, PeaksError> {
    let decoder = rodio_deck::open(path).map_err(|e| match e {
        PlaybackError::Open(m) => PeaksError::Open(m),
        other => PeaksError::Decode(other.to_string()),
    })?;
    let channels = usize::from(decoder.channels().get());
    let sample_rate = decoder.sample_rate().get();
    let mut reducer = PeakReducer::new(channels);
    for (i, sample) in decoder.enumerate() {
        if i % CANCEL_CHECK_SAMPLES == 0 && cancel.load(Ordering::Relaxed) {
            return Err(PeaksError::Cancelled);
        }
        reducer.push(sample);
    }
    reducer.finish(sample_rate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playback::test_audio::{RATE, wav, wav_samples, write_temp};
    use crate::waveform::peaks::PEAK_COUNT;

    #[test]
    fn computes_peaks_of_a_tone() {
        // One second of a stereo square wave at half scale, loud on the right.
        let samples: Vec<i16> = (0..RATE)
            .flat_map(|i| {
                let v = if (i / 20) % 2 == 0 { 16384 } else { -16384 };
                [v / 4, v]
            })
            .collect();
        let path = write_temp("peaks-tone.wav", &wav_samples(2, RATE, &samples));
        let peaks = compute_peaks(&path, &AtomicBool::new(false)).unwrap();
        assert_eq!(peaks.peaks.len(), PEAK_COUNT);
        assert!((peaks.duration_sec - 1.0).abs() < 0.001, "{peaks:?}");
        let max = peaks.peaks.iter().fold(0f32, |m, p| m.max(p.abs()));
        assert!((max - 0.5).abs() < 0.01, "max {max}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn cancelled_before_start() {
        let path = write_temp("peaks-cancel.wav", &wav());
        let err = compute_peaks(&path, &AtomicBool::new(true)).unwrap_err();
        assert!(matches!(err, PeaksError::Cancelled));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_file_is_an_open_error() {
        let err = compute_peaks(Path::new("Z:/not/here.mp3"), &AtomicBool::new(false)).unwrap_err();
        assert!(matches!(err, PeaksError::Open(_)));
    }
}
