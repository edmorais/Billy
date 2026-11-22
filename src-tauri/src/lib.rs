pub mod commands;
pub mod db;
pub mod error;
pub mod fs_guard;
pub mod reader;
pub mod scan;
pub mod state;
pub mod thumbs;
pub mod watch;

use serde::Serialize;
use state::AppState;
use tauri::{Emitter, Manager};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ThumbsUpdated<'a> {
    updates: &'a [thumbs::ThumbUpdate],
    remaining: i64,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        // Serves cached thumbnails to the UI at http://thumb.localhost/<fingerprint>.
        .register_asynchronous_uri_scheme_protocol("thumb", |ctx, request, responder| {
            let thumbs_dir = ctx.app_handle().state::<AppState>().thumbs_dir.clone();
            let path = request.uri().path().to_string();
            std::thread::spawn(move || responder.respond(thumbs::serve(&thumbs_dir, &path)));
        })
        .setup(|app| {
            // Billy's data lives in %APPDATA%\Billy and its cache in %LOCALAPPDATA%\Billy,
            // never in a library folder.
            // BILLY_DATA_DIR puts both in one folder instead, to try a build without touching
            // your own libraries.
            let (db_path, thumbs_dir) = match std::env::var_os("BILLY_DATA_DIR") {
                Some(dir) => {
                    let dir = std::path::PathBuf::from(dir);
                    (dir.join("billy.db"), dir.join("thumbs"))
                }
                None => (
                    app.path().data_dir()?.join("Billy").join("billy.db"),
                    app.path().local_data_dir()?.join("Billy").join("thumbs"),
                ),
            };
            let conn = db::open(&db_path)?;
            let sync_handle = app.handle().clone();
            let watcher = watch::SyncWatcher::new(move |root_id| commands::spawn_scan(sync_handle.clone(), root_id));
            let state = AppState::new(conn, db_path.clone(), thumbs_dir.clone(), watcher);
            let queue = state.thumbs.clone();
            app.manage(state);
            commands::start_auto_sync(app.handle().clone());
            commands::spawn_cleanup(app.handle().clone(), true);

            let handle = app.handle().clone();
            let error_handle = app.handle().clone();
            thumbs::spawn_worker(
                queue,
                db_path,
                thumbs_dir,
                app.path().resource_dir()?,
                move |updates, remaining| {
                    let _ = handle.emit("thumbs-updated", ThumbsUpdated { updates, remaining });
                },
                move |message| {
                    let _ = error_handle.emit("thumbs-error", message);
                },
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_version,
            commands::get_settings,
            commands::set_setting,
            commands::list_libraries,
            commands::create_library,
            commands::rename_library,
            commands::delete_library,
            commands::save_view_settings,
            commands::folder_tree,
            commands::list_documents,
            commands::set_favorite,
            commands::set_read,
            commands::open_with_default_app,
            commands::show_in_folder,
            commands::open_folder,
            commands::open_document,
            commands::read_pdf,
            commands::set_last_page,
            commands::save_annotations,
            commands::save_to_original,
            commands::export_copy,
            commands::export_saved_copy,
            commands::discard_annotations,
            commands::sync_root,
            commands::sync_library,
            commands::set_auto_sync,
            commands::list_missing,
            commands::clean_up_missing,
            commands::list_roots,
            commands::scanning_roots,
            commands::add_root,
            commands::remove_root,
            commands::list_ignored,
            commands::repair_covers,
            commands::rebuild_covers,
            commands::ignore_folder,
            commands::unignore_folder,
            commands::prioritize_thumbs,
            commands::thumbs_remaining,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Billy");
}
