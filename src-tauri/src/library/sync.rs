//! Sync: bring the Rekordbox Library into the Local DB. One-way.

use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::Connection;
use serde::Serialize;

use super::error::SyncError;
use super::{local_db, rekordbox};

/// Sync phases, in the order they run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Playlists,
    Tracks,
    Cues,
}

/// Sent to the UI as the `sync-progress` event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Progress {
    pub phase: Phase,
    pub done: usize,
    pub total: usize,
}

impl Progress {
    pub fn new(phase: Phase, done: usize, total: usize) -> Self {
        Self { phase, done, total }
    }
}

/// Counts of what a Sync copied.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct SyncSummary {
    pub playlists: usize,
    pub tracks: usize,
    pub cues: usize,
}

/// The Local DB plus a guard so only one Sync runs at a time.
pub struct Library {
    pub db: Mutex<Connection>,
    syncing: AtomicBool,
}

impl Library {
    pub fn new(conn: Connection) -> Self {
        Self {
            db: Mutex::new(conn),
            syncing: AtomicBool::new(false),
        }
    }

    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        // A panic while holding the lock can't leave a half-written Sync behind
        // (the transaction rolls back), so a poisoned lock is safe to reuse.
        self.db.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Run a Sync from the `master.db` at `master_db`. Rekordbox is read before
    /// the Local DB is locked, so reads of the Local DB keep working meanwhile.
    pub fn sync(
        &self,
        master_db: &Path,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<SyncSummary, SyncError> {
        if self.syncing.swap(true, Ordering::AcqRel) {
            return Err(SyncError::AlreadyRunning);
        }
        let result = (|| {
            let snapshot = rekordbox::read_snapshot(master_db, progress)?;
            local_db::replace_library(&mut self.conn(), &snapshot, progress)?;
            Ok(SyncSummary {
                playlists: snapshot.playlists.len(),
                tracks: snapshot.tracks.len(),
                cues: snapshot.cues.len(),
            })
        })();
        self.syncing.store(false, Ordering::Release);
        result
    }
}
