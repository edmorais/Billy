//! Tauri commands called by the UI.

use crate::db::browse::{self, Document, FolderNode};
use crate::db::libraries::{Library, Root};
use crate::db::{self, libraries, marks};
use crate::error::{Error, Result};
use crate::fs_guard;
use crate::scan::{self, ScanReport};
use crate::state::AppState;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, State};

/// Returns the app version from Cargo.toml.
#[tauri::command]
pub fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[tauri::command]
pub fn list_libraries(state: State<AppState>) -> Result<Vec<Library>> {
    libraries::list_libraries(&state.db())
}

#[tauri::command]
pub fn create_library(state: State<AppState>, name: String) -> Result<Library> {
    libraries::create_library(&state.db(), &name)
}

#[tauri::command]
pub fn rename_library(state: State<AppState>, id: i64, name: String) -> Result<Library> {
    libraries::rename_library(&state.db(), id, &name)
}

#[tauri::command]
pub fn delete_library(state: State<AppState>, id: i64) -> Result<()> {
    libraries::delete_library(&state.db(), id)
}

#[tauri::command]
pub fn save_view_settings(state: State<AppState>, id: i64, settings: serde_json::Value) -> Result<()> {
    libraries::save_view_settings(&state.db(), id, &settings)
}

#[tauri::command]
pub fn folder_tree(state: State<AppState>, library_id: i64) -> Result<Vec<FolderNode>> {
    browse::folder_tree(&state.db(), library_id)
}

#[tauri::command]
pub fn list_documents(
    state: State<AppState>,
    library_id: i64,
    folder_id: Option<i64>,
    recursive: bool,
    favorites_only: bool,
) -> Result<Vec<Document>> {
    browse::list_documents(&state.db(), library_id, folder_id, recursive, favorites_only)
}

#[tauri::command]
pub fn set_favorite(state: State<AppState>, library_id: i64, file_ids: Vec<i64>, favorite: bool) -> Result<()> {
    marks::set_favorite(&mut state.db(), library_id, &file_ids, favorite)
}

#[tauri::command]
pub fn set_read(state: State<AppState>, library_id: i64, file_ids: Vec<i64>, read: bool) -> Result<()> {
    marks::set_read(&mut state.db(), library_id, &file_ids, read)
}

/// Opens a document in the default PDF app. The UI passes an id, never a path, so it can only
/// open documents that Billy knows.
#[tauri::command]
pub fn open_with_default_app(state: State<AppState>, file_id: i64) -> Result<()> {
    let path = marks::file_path(&state.db(), file_id)?;
    tauri_plugin_opener::open_path(&path, None::<&str>)
        .map_err(|e| Error::msg(format!("Windows could not open {}: {e}", path.display())))
}

/// Opens File Explorer with the document selected.
#[tauri::command]
pub fn show_in_folder(state: State<AppState>, file_id: i64) -> Result<()> {
    let path = marks::file_path(&state.db(), file_id)?;
    tauri_plugin_opener::reveal_item_in_dir(&path)
        .map_err(|e| Error::msg(format!("Could not show {}: {e}", path.display())))
}

/// Opens a library folder in File Explorer.
#[tauri::command]
pub fn open_folder(state: State<AppState>, folder_id: i64) -> Result<()> {
    let path = marks::folder_path(&state.db(), folder_id)?;
    tauri_plugin_opener::open_path(&path, None::<&str>)
        .map_err(|e| Error::msg(format!("Could not open {}: {e}", path.display())))
}

#[tauri::command]
pub fn list_roots(state: State<AppState>, library_id: i64) -> Result<Vec<Root>> {
    libraries::list_roots(&state.db(), library_id)
}

/// Makes these documents' thumbnails next, for example because they are on screen.
#[tauri::command]
pub fn prioritize_thumbs(state: State<AppState>, file_ids: Vec<i64>) {
    state.thumbs.prioritize(&file_ids);
}

/// Number of documents still waiting for a thumbnail.
#[tauri::command]
pub fn thumbs_remaining(state: State<AppState>) -> Result<i64> {
    crate::thumbs::remaining(&state.db())
}

/// Returns the ids of roots that are being scanned right now.
#[tauri::command]
pub fn scanning_roots(state: State<AppState>) -> Vec<i64> {
    state.scanning.lock().unwrap().iter().copied().collect()
}

/// Adds a folder to a library and starts its first scan in the background.
#[tauri::command]
pub fn add_root(app: AppHandle, state: State<AppState>, library_id: i64, path: String) -> Result<Root> {
    let canonical = fs_guard::canonical_dir(&PathBuf::from(&path))
        .map_err(|e| Error::msg(format!("Billy can't open that folder: {e}")))?;
    let root = libraries::insert_root(&state.db(), library_id, &canonical.to_string_lossy())?;
    spawn_scan(app, root.id);
    Ok(root)
}

/// Removes a folder from a library. With `purge`, also deletes favorites and read status for
/// documents that are no longer in the library.
#[tauri::command]
pub async fn remove_root(app: AppHandle, root_id: i64, purge: bool) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || -> Result<()> {
        let state = app.state::<AppState>();
        let removed = libraries::remove_root(&state.db(), root_id)?;
        // Overlapping roots may have shared documents with the removed one, so scan them
        // again before deciding which documents left the library.
        let mut conn = db::open(&state.db_path)?;
        for other in removed.overlapping_roots {
            scan::scan_root(&mut conn, other, &mut |_| {})?;
        }
        if purge {
            libraries::purge_orphan_state(&conn, removed.library_id)?;
        }
        Ok(())
    })
    .await
    .map_err(|e| Error::msg(e.to_string()))?
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanFinished {
    root_id: i64,
    report: Option<ScanReport>,
    error: Option<String>,
}

/// Scans a root on a background thread. Emits `scan-progress` while it runs and
/// `scan-finished` at the end. Does nothing if that root is already being scanned.
pub fn spawn_scan(app: AppHandle, root_id: i64) {
    let state = app.state::<AppState>();
    if !state.scanning.lock().unwrap().insert(root_id) {
        return;
    }
    let db_path = state.db_path.clone();
    std::thread::spawn(move || {
        let result = db::open(&db_path).and_then(|mut conn| {
            scan::scan_root(&mut conn, root_id, &mut |p| {
                let _ = app.emit("scan-progress", p);
                // Start on thumbnails for the files found so far.
                app.state::<AppState>().thumbs.wake();
            })
        });
        let state = app.state::<AppState>();
        state.scanning.lock().unwrap().remove(&root_id);
        state.thumbs.wake();
        let payload = match result {
            Ok(report) => ScanFinished { root_id, report: Some(report), error: None },
            Err(err) => ScanFinished { root_id, report: None, error: Some(err.to_string()) },
        };
        let _ = app.emit("scan-finished", payload);
    });
}
