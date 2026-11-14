//! Tauri commands and events for Waveforms.

use serde::Serialize;
use tauri::{AppHandle, Manager};

use super::beat_grid::{self, BeatGrid};
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
    /// Rekordbox's Beat Grid from the ANLZ file; `None` if there is none.
    pub beat_grid: Option<BeatGrid>,
    pub cues: Vec<CueRow>,
}

fn waveform_data(library: &Library, track_id: &str) -> Result<WaveformData, LocalDbError> {
    let (stored, cues) = {
        let conn = library.conn();
        (
            local_db::read_peaks(&conn, track_id, VERSION)?,
            local_db::track_cues(&conn, track_id)?,
        )
    };
    let (peaks, duration) = match stored {
        Some((blob, duration)) => (Some(peaks::decode(&blob)), Some(duration)),
        None => (None, None),
    };
    Ok(WaveformData {
        peaks,
        duration,
        beat_grid: beat_grid::ensure_beat_grid(library, track_id)?,
        cues,
    })
}

/// A Track's stored Peaks, duration, Beat Grid and Cues. Never decodes:
/// Peaks are computed when the Track is first loaded. The Beat Grid is read
/// from the ANLZ file on the first call and again only if that file changes,
/// so this runs off the main thread.
#[tauri::command]
pub async fn get_waveform_data(
    app: AppHandle,
    track_id: String,
) -> Result<WaveformData, LocalDbError> {
    tauri::async_runtime::spawn_blocking(move || waveform_data(&app.state::<Library>(), &track_id))
        .await
        .unwrap_or(Err(LocalDbError::Interrupted))
}
