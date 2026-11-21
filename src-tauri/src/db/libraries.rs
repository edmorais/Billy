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
           (SELECT COUNT(*) FROM entries e WHERE e.library_id = l.id),
           (SELECT COUNT(*) FROM roots r WHERE r.library_id = l.id)
    FROM libraries l";

fn library_from_row(row: &Row) -> rusqlite::Result<Library> {
    Ok(Library {
        id: row.get(0)?,
        name: row.get(1)?,
        created_at: row.get(2)?,
        document_count: row.get(3)?,
        root_count: row.get(4)?,
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

/// Windows paths are case-insensitive, so compare them in lower case.
fn lowercase_path(path: &str) -> PathBuf {
    PathBuf::from(path.to_lowercase())
}
