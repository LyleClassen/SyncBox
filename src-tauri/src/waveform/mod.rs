//! The data behind a Track's Waveform: Peaks computed in Rust from the same
//! decoder as playback (ADR-0002) and kept in the Local DB's `sb_peaks`, and
//! the Beat Grid read from the Track's ANLZ file and kept in `sb_beat_grid`.

pub mod beat_grid;
pub mod commands;
pub mod compute;
pub mod peaks;
pub mod worker;

pub use worker::PeaksWorker;
