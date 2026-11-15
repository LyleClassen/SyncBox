//! Computes a Track's Peaks in the background on its first load, so playback
//! never waits for them.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager};

use super::commands::{PEAKS_FAILED_EVENT, PEAKS_READY_EVENT};
use super::compute::compute_peaks;
use super::peaks::{self, PeaksError, VERSION};
use crate::library::Library;
use crate::library::local_db;

struct Job {
    track_id: String,
    cancel: Arc<AtomicBool>,
}

/// Runs at most one Peaks job at a time. Kept in Tauri state.
#[derive(Default)]
pub struct PeaksWorker {
    running: Mutex<Option<Job>>,
}

impl PeaksWorker {
    /// Makes sure `track_id` gets Peaks. Returns at once: if the Track has
    /// none stored, they're computed on a background thread, which sends
    /// [`PEAKS_READY_EVENT`] or [`PEAKS_FAILED_EVENT`] when done. A job for
    /// another Track is cancelled, so switching Tracks quickly doesn't pile up
    /// full-file decodes.
    pub fn ensure(&self, app: AppHandle, track_id: String, path: PathBuf) {
        let stored = local_db::has_peaks(&app.state::<Library>().conn(), &track_id, VERSION);
        match stored {
            Ok(true) => return,
            Ok(false) => {}
            Err(e) => {
                log::warn!("Could not check stored Peaks for {track_id}: {e}");
                return;
            }
        }

        let mut running = self.running.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(job) = running.as_ref() {
            if job.track_id == track_id {
                return;
            }
            job.cancel.store(true, Ordering::Relaxed);
        }
        let cancel = Arc::new(AtomicBool::new(false));
        *running = Some(Job {
            track_id: track_id.clone(),
            cancel: cancel.clone(),
        });
        drop(running);

        let spawned = std::thread::Builder::new()
            .name("syncbox-peaks".into())
            .spawn({
                let app = app.clone();
                let track_id = track_id.clone();
                let cancel = cancel.clone();
                move || run(&app, &track_id, &path, &cancel)
            });
        if let Err(e) = spawned {
            log::warn!("Could not start computing Peaks for {track_id}: {e}");
            self.clear(&cancel);
        }
    }

    /// Forgets the running job if it's still the one with `cancel`.
    fn clear(&self, cancel: &Arc<AtomicBool>) {
        let mut running = self.running.lock().unwrap_or_else(|e| e.into_inner());
        if running
            .as_ref()
            .is_some_and(|job| Arc::ptr_eq(&job.cancel, cancel))
        {
            *running = None;
        }
    }
}

fn run(app: &AppHandle, track_id: &str, path: &std::path::Path, cancel: &Arc<AtomicBool>) {
    // Decode without holding the Local DB lock; lock only to write.
    let result = compute_peaks(path, cancel).and_then(|p| {
        let blob = peaks::encode(&p.peaks);
        let library = app.state::<Library>();
        let conn = library.conn();
        Ok(local_db::write_peaks(
            &conn,
            track_id,
            &blob,
            p.duration_sec,
            VERSION,
        )?)
    });
    app.state::<PeaksWorker>().clear(cancel);
    let _ = match result {
        Ok(true) => app.emit(
            PEAKS_READY_EVENT,
            serde_json::json!({ "trackId": track_id }),
        ),
        // The Track was removed by a Sync meanwhile; nobody is waiting.
        Ok(false) | Err(PeaksError::Cancelled) => Ok(()),
        Err(e) => {
            log::warn!("Could not compute Peaks for {track_id}: {e}");
            app.emit(
                PEAKS_FAILED_EVENT,
                serde_json::json!({ "trackId": track_id, "message": e.to_string() }),
            )
        }
    };
}
