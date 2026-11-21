pub mod commands;
pub mod db;
pub mod error;
pub mod fs_guard;
pub mod scan;
pub mod state;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Billy's data lives in %APPDATA%\Billy, never in a library folder.
            let db_path = app.path().data_dir()?.join("Billy").join("billy.db");
            let conn = db::open(&db_path)?;
            app.manage(AppState::new(conn, db_path));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_version,
            commands::list_libraries,
            commands::create_library,
            commands::rename_library,
            commands::delete_library,
            commands::list_roots,
            commands::scanning_roots,
            commands::add_root,
            commands::remove_root,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Billy");
}
