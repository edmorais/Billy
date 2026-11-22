//! Queries for libraries and their root folders.

use crate::db::now_ms;
use crate::error::{Error, Result};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub id: i64,
    pub name: String,
    pub created_at: i64,
    pub document_count: i64,
    pub root_count: i64,
    /// View mode, sort order and other browsing choices, stored as JSON for the UI.
    pub view_settings: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub id: i64,
    pub library_id: i64,
    pub path: String,
    pub auto_sync: bool,
    pub status: String,
    pub added_at: i64,
    pub last_synced_at: Option<i64>,
    pub file_count: i64,
}

const LIBRARY_SELECT: &str = "
    SELECT l.id, l.name, l.created_at,
           (SELECT COUNT(*) FROM entries e JOIN files x ON x.id = e.file_id
            WHERE e.library_id = l.id AND x.missing_since IS NULL),
           (SELECT COUNT(*) FROM roots r WHERE r.library_id = l.id),
           l.view_settings
    FROM libraries l";

fn library_from_row(row: &Row) -> rusqlite::Result<Library> {
    let settings: String = row.get(5)?;
    Ok(Library {
        id: row.get(0)?,
        name: row.get(1)?,
        created_at: row.get(2)?,
        document_count: row.get(3)?,
        root_count: row.get(4)?,
        view_settings: serde_json::from_str(&settings).unwrap_or_else(|_| serde_json::json!({})),
    })
}

const ROOT_SELECT: &str = "
    SELECT id, library_id, path, auto_sync, status, added_at, last_synced_at, file_count
    FROM roots";

fn root_from_row(row: &Row) -> rusqlite::Result<Root> {
    Ok(Root {
        id: row.get(0)?,
        library_id: row.get(1)?,
        path: row.get(2)?,
        auto_sync: row.get(3)?,
        status: row.get(4)?,
        added_at: row.get(5)?,
        last_synced_at: row.get(6)?,
        file_count: row.get(7)?,
    })
}

fn clean_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::msg("A library needs a name."));
    }
    Ok(name.to_string())
}

pub fn list_libraries(conn: &Connection) -> Result<Vec<Library>> {
    let mut stmt = conn.prepare(&format!("{LIBRARY_SELECT} ORDER BY l.name COLLATE NOCASE"))?;
    let rows = stmt.query_map([], library_from_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn get_library(conn: &Connection, id: i64) -> Result<Library> {
    conn.query_row(&format!("{LIBRARY_SELECT} WHERE l.id = ?1"), [id], library_from_row)
        .optional()?
        .ok_or_else(|| Error::msg("That library no longer exists."))
}

pub fn create_library(conn: &Connection, name: &str) -> Result<Library> {
    let name = clean_name(name)?;
    conn.execute(
        "INSERT INTO libraries (name, created_at) VALUES (?1, ?2)",
        params![name, now_ms()],
    )?;
    get_library(conn, conn.last_insert_rowid())
}

pub fn rename_library(conn: &Connection, id: i64, name: &str) -> Result<Library> {
    let name = clean_name(name)?;
    conn.execute("UPDATE libraries SET name = ?2 WHERE id = ?1", params![id, name])?;
    get_library(conn, id)
}

pub fn save_view_settings(conn: &Connection, id: i64, settings: &serde_json::Value) -> Result<()> {
    conn.execute(
        "UPDATE libraries SET view_settings = ?2 WHERE id = ?1",
        params![id, settings.to_string()],
    )?;
    Ok(())
}

/// Deletes a library and all of Billy's records for it. Files on disk are not touched.
pub fn delete_library(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM libraries WHERE id = ?1", [id])?;
    Ok(())
}

pub fn list_roots(conn: &Connection, library_id: i64) -> Result<Vec<Root>> {
    let mut stmt =
        conn.prepare(&format!("{ROOT_SELECT} WHERE library_id = ?1 ORDER BY path COLLATE NOCASE"))?;
    let rows = stmt.query_map([library_id], root_from_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn get_root(conn: &Connection, id: i64) -> Result<Root> {
    conn.query_row(&format!("{ROOT_SELECT} WHERE id = ?1"), [id], root_from_row)
        .optional()?
        .ok_or_else(|| Error::msg("That folder is no longer in the library."))
}

/// Adds a root folder record. `path` must already be canonical. Scanning is separate.
pub fn insert_root(conn: &Connection, library_id: i64, path: &str) -> Result<Root> {
    get_library(conn, library_id)?;
    let exists: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM roots WHERE library_id = ?1 AND path = ?2)",
        params![library_id, path],
        |r| r.get(0),
    )?;
    if exists {
        return Err(Error::msg("This folder is already in the library."));
    }
    conn.execute(
        "INSERT INTO roots (library_id, path, added_at) VALUES (?1, ?2, ?3)",
        params![library_id, path, now_ms()],
    )?;
    get_root(conn, conn.last_insert_rowid())
}

pub fn set_auto_sync(conn: &Connection, root_id: i64, on: bool) -> Result<Root> {
    conn.execute("UPDATE roots SET auto_sync = ?2 WHERE id = ?1", params![root_id, on])?;
    get_root(conn, root_id)
}

/// Every root folder, in every library, that has Auto-sync turned on.
pub fn auto_sync_roots(conn: &Connection) -> Result<Vec<Root>> {
    let mut stmt = conn.prepare(&format!("{ROOT_SELECT} WHERE auto_sync = 1"))?;
    let rows = stmt.query_map([], root_from_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Result of removing a root folder from a library.
pub struct RemovedRoot {
    pub library_id: i64,
    /// Other roots in the same library that contain, or sit inside, the removed folder.
    /// They must be scanned again, because they may have shared documents with it.
    pub overlapping_roots: Vec<i64>,
}

/// Removes a root and its entries from its library. Favorites and read status are kept;
/// call [`purge_orphan_state`] afterwards to delete them.
pub fn remove_root(conn: &Connection, root_id: i64) -> Result<RemovedRoot> {
    let root = get_root(conn, root_id)?;
    conn.execute("DELETE FROM roots WHERE id = ?1", [root_id])?;
    let removed = lowercase_path(&root.path);
    let overlapping_roots = list_roots(conn, root.library_id)?
        .into_iter()
        .filter(|other| {
            let other_path = lowercase_path(&other.path);
            removed.starts_with(&other_path) || other_path.starts_with(&removed)
        })
        .map(|other| other.id)
        .collect();
    Ok(RemovedRoot { library_id: root.library_id, overlapping_roots })
}

/// Deletes favorites and read status in a library for files that are no longer in it.
pub fn purge_orphan_state(conn: &Connection, library_id: i64) -> Result<()> {
    for table in ["favorites", "read_state"] {
        conn.execute(
            &format!(
                "DELETE FROM {table} WHERE library_id = ?1
                 AND file_id NOT IN (SELECT file_id FROM entries WHERE library_id = ?1)"
            ),
            [library_id],
        )?;
    }
    Ok(())
}

/// Forgets files that no library holds and that nothing else refers to: no favorite, read
/// status or saved annotations in any library. Their last-read page goes with them. Files with
/// such marks are kept, so the marks come back if their folder is added again. Returns the
/// fingerprints of the forgotten files, whose covers may be deleted.
pub fn forget_unused_files(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "DELETE FROM files
         WHERE NOT EXISTS (SELECT 1 FROM entries e WHERE e.file_id = files.id)
           AND NOT EXISTS (SELECT 1 FROM favorites f WHERE f.file_id = files.id)
           AND NOT EXISTS (SELECT 1 FROM read_state r WHERE r.file_id = files.id)
           AND NOT EXISTS (SELECT 1 FROM annotations a WHERE a.file_id = files.id)
         RETURNING fingerprint",
    )?;
    let fingerprints = stmt.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
    Ok(fingerprints)
}

/// Subfolders that a library leaves out when it scans, as absolute paths.
pub fn list_ignored(conn: &Connection, library_id: i64) -> Result<Vec<String>> {
    let mut stmt =
        conn.prepare("SELECT path FROM ignored_folders WHERE library_id = ?1 ORDER BY path COLLATE NOCASE")?;
    let rows = stmt.query_map([library_id], |r| r.get(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// The ignored folders that sit inside `root_path`, for scanning that root.
pub fn ignored_under(conn: &Connection, library_id: i64, root_path: &str) -> Result<Vec<String>> {
    Ok(list_ignored(conn, library_id)?.into_iter().filter(|p| is_inside(p, root_path)).collect())
}

/// Makes a library leave out a subfolder of one of its folders. Returns the roots that hold
/// it, which must be scanned again.
pub fn ignore_folder(conn: &Connection, library_id: i64, path: &str) -> Result<Vec<i64>> {
    let roots = list_roots(conn, library_id)?;
    if roots.iter().any(|r| lowercase_path(&r.path) == lowercase_path(path)) {
        return Err(Error::msg(
            "That is one of the library's own folders. To leave it out, remove it from the library.",
        ));
    }
    let holders: Vec<i64> = roots.iter().filter(|r| is_inside(path, &r.path)).map(|r| r.id).collect();
    if holders.is_empty() {
        return Err(Error::msg("Only subfolders of this library's folders can be ignored."));
    }
    conn.execute(
        "INSERT OR IGNORE INTO ignored_folders (library_id, path, added_at) VALUES (?1, ?2, ?3)",
        params![library_id, path, now_ms()],
    )?;
    Ok(holders)
}

/// Stops ignoring a subfolder. Returns the roots that hold it, which must be scanned again.
pub fn unignore_folder(conn: &Connection, library_id: i64, path: &str) -> Result<Vec<i64>> {
    conn.execute(
        "DELETE FROM ignored_folders WHERE library_id = ?1 AND path = ?2",
        params![library_id, path],
    )?;
    Ok(list_roots(conn, library_id)?.into_iter().filter(|r| is_inside(path, &r.path)).map(|r| r.id).collect())
}

/// True when `path` is `folder` or inside it.
pub fn is_same_or_inside(path: &str, folder: &str) -> bool {
    lowercase_path(path).starts_with(lowercase_path(folder))
}

/// Windows paths are case-insensitive, so compare them in lower case.
fn lowercase_path(path: &str) -> PathBuf {
    PathBuf::from(path.to_lowercase())
}

/// True when `child` is a folder strictly inside `parent`.
pub fn is_inside(child: &str, parent: &str) -> bool {
    let child = lowercase_path(child);
    let parent = lowercase_path(parent);
    child != parent && child.starts_with(&parent)
}
