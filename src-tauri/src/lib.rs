pub mod library;
pub mod playback;
pub mod waveform;

use tauri::Manager;

use library::{Library, commands, local_db};
use playback::commands as play;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let conn = local_db::open(&data_dir.join("library.db"))?;
            app.manage(Library::new(conn));
            app.manage(playback::start(app.handle().clone()));
            app.manage(waveform::PeaksWorker::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::sync_from_rekordbox,
            commands::sync_if_empty,
            commands::get_playlist_tree,
            commands::get_playlist_tracks,
            commands::get_all_tracks,
            commands::check_track_files,
            play::playback_load,
            play::playback_play,
            play::playback_pause,
            play::playback_toggle,
            play::playback_seek,
            play::playback_set_volume,
            play::playback_state,
            waveform::commands::get_waveform_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
