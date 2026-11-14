//! Tauri commands for playback. Each runs on a blocking task and hands its
//! work to the engine thread.

use std::time::Duration;

use tauri::{AppHandle, Manager};

use super::engine::{Engine, Opened};
use super::error::PlaybackError;
use super::rodio_deck;
use super::transport::PlayerState;
use crate::library::Library;
use crate::library::local_db;
use crate::library::track_file::FileCheck;
use crate::waveform::PeaksWorker;

/// Event sent with a [`PlayerState`] when the loaded Track, status or
/// duration changes.
pub const STATE_EVENT: &str = "playback-state";
/// Event sent with `{ trackId, position }` about 30 times a second while
/// playing, and after each command.
pub const POSITION_EVENT: &str = "playback-position";

async fn on_engine<R: Send + 'static>(
    app: AppHandle,
    f: impl FnOnce(&mut super::engine::Core) -> R + Send + 'static,
) -> Result<R, PlaybackError> {
    tauri::async_runtime::spawn_blocking(move || app.state::<Engine>().call(f))
        .await
        .unwrap_or(Err(PlaybackError::EngineStopped))
}

/// Finds and opens a Track's file. Checks the file every time (never
/// cached), since drives come and go.
fn open_track(app: &AppHandle, track_id: &str) -> Result<Opened, PlaybackError> {
    let (raw, stored_len) = local_db::track_source(&app.state::<Library>().conn(), track_id)?
        .ok_or(PlaybackError::UnknownTrack)?;
    let path = FileCheck::on_disk()
        .resolve(raw.as_deref())
        .map_err(PlaybackError::Unplayable)?;
    let decoder = rodio_deck::open(&path)?;
    let duration = rodio_deck::duration_of(&decoder)
        .or(stored_len.map(Duration::from_secs))
        .unwrap_or_default();
    Ok(Opened {
        decoder,
        path,
        duration,
    })
}

/// Loads a Track, paused at the start. The current Track stops at once.
/// Returns `true` once it's loaded. If another load starts before this one
/// finishes, this one is dropped and returns `false` (even if it failed):
/// only the latest load counts.
///
/// A Track loaded for the first time has its Peaks computed in the
/// background afterwards; the load doesn't wait for them.
#[tauri::command]
pub async fn playback_load(app: AppHandle, track_id: String) -> Result<bool, PlaybackError> {
    let id = track_id.clone();
    let ticket = on_engine(app.clone(), move |core| core.transport.begin_load(&id)).await?;
    let handle = app.clone();
    let id = track_id.clone();
    let opened = tauri::async_runtime::spawn_blocking(move || open_track(&handle, &id))
        .await
        .unwrap_or(Err(PlaybackError::EngineStopped));
    let path = opened.as_ref().ok().map(|o| o.path.clone());
    let loaded = on_engine(app.clone(), move |core| core.finish_load(ticket, opened)).await??;
    if loaded && let Some(path) = path {
        app.state::<PeaksWorker>()
            .ensure(app.clone(), track_id, path);
    }
    Ok(loaded)
}

#[tauri::command]
pub async fn playback_play(app: AppHandle) -> Result<(), PlaybackError> {
    on_engine(app, |core| core.transport.play()).await
}

#[tauri::command]
pub async fn playback_pause(app: AppHandle) -> Result<(), PlaybackError> {
    on_engine(app, |core| core.transport.pause()).await
}

#[tauri::command]
pub async fn playback_toggle(app: AppHandle) -> Result<(), PlaybackError> {
    on_engine(app, |core| core.transport.toggle()).await
}

/// Seeks to `seconds` into the loaded Track, clamped to its length.
#[tauri::command]
pub async fn playback_seek(app: AppHandle, seconds: f64) -> Result<(), PlaybackError> {
    on_engine(app, move |core| core.transport.seek(seconds))
        .await?
        .map_err(PlaybackError::Seek)
}

/// Sets the volume, 0–1. Kept for Tracks loaded later.
#[tauri::command]
pub async fn playback_set_volume(app: AppHandle, volume: f32) -> Result<(), PlaybackError> {
    on_engine(app, move |core| core.transport.set_volume(volume)).await
}

/// The current state, for a window that has just opened.
#[tauri::command]
pub async fn playback_state(app: AppHandle) -> Result<PlayerState, PlaybackError> {
    on_engine(app, |core| core.transport.state()).await
}
