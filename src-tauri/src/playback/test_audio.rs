//! Small in-memory audio files for tests.

use std::path::PathBuf;

pub const RATE: u32 = 8000;

/// One second of 16-bit mono silence as AIFF (big-endian, an 80-bit
/// extended float for the sample rate).
pub fn aiff() -> Vec<u8> {
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

/// One second of 16-bit mono silence as WAV.
pub fn wav() -> Vec<u8> {
    wav_samples(1, RATE, &vec![0; RATE as usize])
}

/// A 16-bit PCM WAV of interleaved `samples`.
pub fn wav_samples(channels: u16, rate: u32, samples: &[i16]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let block_align = channels * 2;
    let mut b = Vec::new();
    b.extend(b"RIFF");
    b.extend((36 + data_len).to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16u32.to_le_bytes());
    b.extend(1u16.to_le_bytes()); // PCM
    b.extend(channels.to_le_bytes());
    b.extend(rate.to_le_bytes());
    b.extend((rate * block_align as u32).to_le_bytes());
    b.extend(block_align.to_le_bytes());
    b.extend(16u16.to_le_bytes());
    b.extend(b"data");
    b.extend(data_len.to_le_bytes());
    for s in samples {
        b.extend(s.to_le_bytes());
    }
    b
}

/// Writes `bytes` to a file named `name` in a per-process temp folder.
pub fn write_temp(name: &str, bytes: &[u8]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("syncbox-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}
