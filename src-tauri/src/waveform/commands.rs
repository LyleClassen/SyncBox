//! Tauri commands and events for Waveforms.

use serde::Serialize;
use tauri::State;

use super::peaks::{self, VERSION};
use crate::library::Library;
use crate::library::error::LocalDbError;
use crate::library::local_db::{self, CueRow};

/// Event sent with `{ trackId }` once a Track's Peaks are stored.
pub const PEAKS_READY_EVENT: &str = "waveform-peaks-ready";
/// Event sent with `{ trackId, message }` if computing a Track's Peaks failed.
pub const PEAKS_FAILED_EVENT: &str = "waveform-peaks-failed";

/// What a Track's Waveform is drawn from.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WaveformData {
    /// [`peaks::PEAK_COUNT`] signed values in [-1, 1]; `None` until computed.
    pub peaks: Option<Vec<f32>>,
    /// The decoded length in seconds; `None` until Peaks are computed.
    pub duration: Option<f64>,
    /// Always `null` until the Beat Grid exists (#7).
    pub beat_grid: Option<()>,
    pub cues: Vec<CueRow>,
}

/// A Track's stored Peaks, duration and Cues. Never decodes: Peaks are
/// computed when the Track is first loaded.
#[tauri::command]
pub fn get_waveform_data(
    library: State<Library>,
    track_id: String,
) -> Result<WaveformData, LocalDbError> {
    let conn = library.conn();
    let stored = local_db::read_peaks(&conn, &track_id, VERSION)?;
    let cues = local_db::track_cues(&conn, &track_id)?;
    let (peaks, duration) = match stored {
        Some((blob, duration)) => (Some(peaks::decode(&blob)), Some(duration)),
        None => (None, None),
    };
    Ok(WaveformData {
        peaks,
        duration,
        beat_grid: None,
        cues,
    })
}
