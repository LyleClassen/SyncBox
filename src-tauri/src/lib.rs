pub mod library;

use tauri::Manager;

use library::{Library, commands, local_db};

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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::sync_from_rekordbox,
            commands::sync_if_empty,
            commands::get_playlist_tree,
            commands::get_playlist_tracks,
            commands::get_all_tracks,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
