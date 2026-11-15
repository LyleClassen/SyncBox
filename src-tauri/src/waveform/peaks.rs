//! Reducing decoded audio to Peaks: 8,000 signed mono values in [-1, 1], to
//! three decimal places. The same shape as wavesurfer's
//! `exportPeaks({ channels: 1, maxLength: 8000, precision: 1000 })`.

/// How many Peaks a Track has.
pub const PEAK_COUNT: usize = 8000;
/// Peaks are rounded to `1 / PRECISION`.
pub const PRECISION: f32 = 1000.0;
/// The algorithm version stored with each Track's Peaks. Bump it when the
/// algorithm changes, and stored Peaks are recomputed on next load.
pub const VERSION: i64 = 1;

/// Frames are first folded into blocks of this many, so a long mix doesn't
/// have to be held in memory before the bucket size is known.
const BLOCK_FRAMES: u64 = 256;

#[derive(Debug, Clone, PartialEq)]
pub struct Peaks {
    pub peaks: Vec<f32>,
    /// The decoded length in seconds.
    pub duration_sec: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum PeaksError {
    #[error("Could not open the audio file: {0}")]
    Open(String),
    #[error("Could not decode the audio file: {0}")]
    Decode(String),
    #[error("The audio file has no audio.")]
    Empty,
    #[error("Computing Peaks was cancelled.")]
    Cancelled,
    #[error("Could not write the Local DB: {0}")]
    LocalDb(#[from] rusqlite::Error),
}

/// Whichever of `a` and `b` is further from zero, keeping its sign.
fn louder(a: f32, b: f32) -> f32 {
    if b.abs() > a.abs() { b } else { a }
}

/// Reduces a stream of interleaved samples to [`Peaks`].
pub struct PeakReducer {
    channels: usize,
    /// Channel index of the next sample within its frame.
    channel: usize,
    frame_peak: f32,
    frames: u64,
    block_peak: f32,
    block_frames: u64,
    /// The signed max-abs of each finished block.
    blocks: Vec<f32>,
}

impl PeakReducer {
    pub fn new(channels: usize) -> Self {
        Self {
            channels: channels.max(1),
            channel: 0,
            frame_peak: 0.0,
            frames: 0,
            block_peak: 0.0,
            block_frames: 0,
            blocks: Vec::new(),
        }
    }

    /// Adds one interleaved sample.
    pub fn push(&mut self, sample: f32) {
        self.frame_peak = louder(self.frame_peak, sample);
        self.channel += 1;
        if self.channel == self.channels {
            self.end_frame();
        }
    }

    pub fn extend(&mut self, samples: impl IntoIterator<Item = f32>) {
        for s in samples {
            self.push(s);
        }
    }

    fn end_frame(&mut self) {
        self.block_peak = louder(self.block_peak, self.frame_peak);
        self.frame_peak = 0.0;
        self.channel = 0;
        self.frames += 1;
        self.block_frames += 1;
        if self.block_frames == BLOCK_FRAMES {
            self.end_block();
        }
    }

    fn end_block(&mut self) {
        self.blocks.push(self.block_peak);
        self.block_peak = 0.0;
        self.block_frames = 0;
    }

    /// Frames seen so far.
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// Splits the audio into [`PEAK_COUNT`] buckets with wavesurfer's bounds
    /// (`floor(i·n/count)` to `ceil((i+1)·n/count)`) and keeps the signed
    /// max-abs of each. Bucket edges fall on block edges, so they are off by
    /// under [`BLOCK_FRAMES`] frames, which can't be seen at this resolution.
    pub fn finish(mut self, sample_rate: u32) -> Result<Peaks, PeaksError> {
        if self.channel > 0 {
            // A final partial frame still counts.
            self.end_frame();
        }
        if self.block_frames > 0 {
            self.end_block();
        }
        if self.frames == 0 || sample_rate == 0 {
            return Err(PeaksError::Empty);
        }
        let n = self.blocks.len() as u64;
        let count = PEAK_COUNT as u64;
        let peaks = (0..count)
            .map(|i| {
                let start = (i * n / count) as usize;
                let end = ((i + 1) * n).div_ceil(count) as usize;
                let peak = self.blocks[start..end]
                    .iter()
                    .fold(0.0, |acc, &b| louder(acc, b));
                (peak * PRECISION).round() / PRECISION
            })
            .collect();
        Ok(Peaks {
            peaks,
            duration_sec: self.frames as f64 / f64::from(sample_rate),
        })
    }
}

/// Peaks as stored in the Local DB: each value × [`PRECISION`] as a
/// little-endian `i16`.
pub fn encode(peaks: &[f32]) -> Vec<u8> {
    peaks
        .iter()
        .flat_map(|&v| {
            let milli = (v * PRECISION)
                .round()
                .clamp(f32::from(i16::MIN), f32::from(i16::MAX));
            (milli as i16).to_le_bytes()
        })
        .collect()
}

/// The reverse of [`encode`].
pub fn decode(bytes: &[u8]) -> Vec<f32> {
    bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&b| f32::from(i16::from_le_bytes(b)) / PRECISION)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reduce(channels: usize, samples: impl IntoIterator<Item = f32>, rate: u32) -> Peaks {
        let mut r = PeakReducer::new(channels);
        r.extend(samples);
        r.finish(rate).unwrap()
    }

    #[test]
    fn always_gives_peak_count_values() {
        let one = reduce(1, [0.5], 44100);
        assert_eq!(one.peaks.len(), PEAK_COUNT);
        assert!(one.peaks.iter().all(|&p| p == 0.5));

        let many = reduce(1, (0..10_000_000).map(|i| (i % 7) as f32 / 10.0), 44100);
        assert_eq!(many.peaks.len(), PEAK_COUNT);
        assert!(many.peaks.iter().all(|&p| p == 0.6));
    }

    #[test]
    fn keeps_the_sign_of_the_loudest_sample() {
        let samples =
            (0..(PEAK_COUNT as u64 * BLOCK_FRAMES)).map(|i| if i % 256 == 3 { -0.9 } else { 0.4 });
        let p = reduce(1, samples, 44100);
        assert!(p.peaks.iter().all(|&v| v == -0.9));
    }

    #[test]
    fn rounds_to_three_decimals() {
        let p = reduce(1, [0.123_456], 44100);
        assert_eq!(p.peaks[0], 0.123);
        let p = reduce(1, [-0.987_65], 44100);
        assert_eq!(p.peaks[0], -0.988);
    }

    #[test]
    fn louder_channel_wins() {
        // Left quiet, right loud and negative.
        let p = reduce(2, [0.1, -0.7, 0.2, 0.3], 44100);
        assert_eq!(p.peaks[0], -0.7);
        assert!((p.duration_sec - 2.0 / 44100.0).abs() < 1e-12);
    }

    #[test]
    fn duration_is_frames_over_rate() {
        let p = reduce(2, vec![0.0; 2 * 44100 * 3], 44100);
        assert!((p.duration_sec - 3.0).abs() < 1e-12);
    }

    #[test]
    fn empty_input_is_an_error() {
        let r = PeakReducer::new(2);
        assert!(matches!(r.finish(44100), Err(PeaksError::Empty)));
    }

    #[test]
    fn encode_round_trips() {
        let peaks = [0.0, 1.0, -1.0, 0.123, -0.456, 0.999];
        let bytes = encode(&peaks);
        assert_eq!(bytes.len(), peaks.len() * 2);
        assert_eq!(decode(&bytes), peaks);
    }
}
