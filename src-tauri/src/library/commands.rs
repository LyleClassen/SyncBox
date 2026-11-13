//! Tauri commands for Sync and reading the Local DB.

use std::collections::HashMap;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use super::error::{LocalDbError, SyncError};
use super::local_db::{self, PlaylistNode, TrackRow};
use super::rekordbox;
use super::sync::{Library, SyncSummary};
use super::track_file::{FileCheck, UnplayableReason};

/// Event sent with a [`super::sync::Progress`] payload while a Sync runs.
pub const SYNC_PROGRESS_EVENT: &str = "sync-progress";

#[derive(Debug, Serialize)]
pub struct SyncOutcome {
    pub summary: SyncSummary,
    pub tree: Vec<PlaylistNode>,
}

fn run_sync(app: &AppHandle) -> Result<SyncOutcome, SyncError> {
    let library = app.state::<Library>();
    let master_db = rekordbox::default_master_db_path()?;
    let summary = library.sync(&master_db, &mut |p| {
        let _ = app.emit(SYNC_PROGRESS_EVENT, p);
    })?;
    let tree = local_db::playlist_tree(&library.conn())?;
    Ok(SyncOutcome { summary, tree })
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, SyncError> + Send + 'static,
) -> Result<T, SyncError> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .unwrap_or(Err(SyncError::Unreadable))
}

/// Sync the Rekordbox Library into the Local DB.
#[tauri::command]
pub async fn sync_from_rekordbox(app: AppHandle) -> Result<SyncOutcome, SyncError> {
    blocking(move || run_sync(&app)).await
}

/// Sync only if the Local DB is empty. The UI calls this once on launch.
/// Returns `None` when the Local DB already has a library.
#[tauri::command]
pub async fn sync_if_empty(app: AppHandle) -> Result<Option<SyncOutcome>, SyncError> {
    blocking(move || {
        let empty = local_db::is_empty(&app.state::<Library>().conn())?;
        if empty {
            run_sync(&app).map(Some)
        } else {
            Ok(None)
        }
    })
    .await
}

#[tauri::command]
pub fn get_playlist_tree(library: State<Library>) -> Result<Vec<PlaylistNode>, LocalDbError> {
    Ok(local_db::playlist_tree(&library.conn())?)
}

#[tauri::command]
pub fn get_playlist_tracks(
    library: State<Library>,
    playlist_id: String,
) -> Result<Vec<TrackRow>, LocalDbError> {
    Ok(local_db::playlist_tracks(&library.conn(), &playlist_id)?)
}

#[tauri::command]
pub fn get_all_tracks(library: State<Library>) -> Result<Vec<TrackRow>, LocalDbError> {
    Ok(local_db::all_tracks(&library.conn())?)
}

/// Check these Tracks' files right now and return the Unplayable Reason for
/// each one that can't be loaded. Tracks missing from the result are playable.
///
/// Separate from the track list reads because the first check after a reboot
/// can take seconds while the OS reads directory entries from disk. Runs off
/// the main thread, and releases the Local DB lock before touching the
/// filesystem. Nothing is cached: every call checks again.
#[tauri::command]
pub async fn check_track_files(
    app: AppHandle,
    track_ids: Vec<String>,
) -> Result<HashMap<String, UnplayableReason>, LocalDbError> {
    tauri::async_runtime::spawn_blocking(move || {
        let paths = local_db::track_file_paths(&app.state::<Library>().conn(), track_ids)?;
        let mut check = FileCheck::on_disk();
        Ok(paths
            .into_iter()
            .filter_map(|(id, raw)| check.reason(raw.as_deref()).map(|r| (id, r)))
            .collect())
    })
    .await
    .unwrap_or(Err(LocalDbError::Interrupted))
}
