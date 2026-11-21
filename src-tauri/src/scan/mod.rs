//! Scanning library folders for PDFs.
//!
//! The scanner only reads library folders, through [`crate::fs_guard`]. Everything it
//! learns is written to Billy's database.

pub mod fingerprint;

use crate::db::{libraries, now_ms};
use crate::error::Result;
use crate::fs_guard;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Files are committed and progress is reported after every batch of this many PDFs.
const BATCH: u64 = 200;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub root_id: i64,
    pub files_found: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub root_id: i64,
    /// PDFs found under the root.
    pub files: u64,
    /// PDFs that were added to the library by this scan.
    pub new_files: u64,
    /// The root folder could not be found, for example because its drive is unplugged.
    pub offline: bool,
    /// Items that could not be read. The scan skips them and carries on.
    pub errors: Vec<String>,
}

/// Scans one root folder and adds every PDF in it to the root's library.
///
/// This adds and updates records only. Files that have disappeared are handled by sync.
pub fn scan_root(
    conn: &mut Connection,
    root_id: i64,
    progress: &mut dyn FnMut(&ScanProgress),
) -> Result<ScanReport> {
    let root = libraries::get_root(conn, root_id)?;
    let root_path = PathBuf::from(&root.path);
    let mut report = ScanReport { root_id, ..Default::default() };

    if !fs_guard::is_dir(&root_path) {
        conn.execute("UPDATE roots SET status = 'offline' WHERE id = ?1", [root_id])?;
        report.offline = true;
        return Ok(report);
    }

    let now = now_ms();
    let mut folder_ids: HashMap<PathBuf, i64> = HashMap::new();
    let mut tx = conn.transaction()?;
    let mut in_batch = 0;

    for item in fs_guard::walk(&root_path) {
        let item = match item {
            Ok(item) => item,
            Err(err) => {
                report.errors.push(err.to_string());
                continue;
            }
        };
        let path = item.path();
        let rel = path.strip_prefix(&root_path).unwrap_or(path).to_path_buf();

        if item.file_type().is_dir() {
            let parent_id = rel.parent().and_then(|p| folder_ids.get(p)).copied();
            let name = if rel.as_os_str().is_empty() {
                root_path.file_name().unwrap_or(root_path.as_os_str())
            } else {
                item.file_name()
            };
            let id = upsert_folder(&tx, root_id, parent_id, &rel, &name.to_string_lossy())?;
            folder_ids.insert(rel, id);
            continue;
        }

        if !item.file_type().is_file() || !fs_guard::is_pdf(path) {
            continue;
        }
        let Some(&folder_id) = rel.parent().and_then(|p| folder_ids.get(p)) else {
            continue;
        };

        let file_id = match import_file(&tx, path) {
            Ok(id) => id,
            Err(err) => {
                report.errors.push(format!("{}: {err}", path.display()));
                continue;
            }
        };
        let added = tx.execute(
            "INSERT OR IGNORE INTO entries (library_id, root_id, file_id, folder_id, date_added)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![root.library_id, root_id, file_id, folder_id, now],
        )?;
        report.files += 1;
        report.new_files += added as u64;

        in_batch += 1;
        if in_batch >= BATCH {
            tx.commit()?;
            tx = conn.transaction()?;
            in_batch = 0;
            progress(&ScanProgress { root_id, files_found: report.files });
        }
    }

    tx.execute(
        "UPDATE roots SET status = 'ok', file_count = ?2, last_synced_at = ?3 WHERE id = ?1",
        params![root_id, report.files as i64, now_ms()],
    )?;
    tx.commit()?;
    progress(&ScanProgress { root_id, files_found: report.files });
    Ok(report)
}

fn upsert_folder(
    tx: &Transaction,
    root_id: i64,
    parent_id: Option<i64>,
    rel: &Path,
    name: &str,
) -> Result<i64> {
    let id = tx.query_row(
        "INSERT INTO folders (root_id, parent_id, rel_path, name) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (root_id, rel_path) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name
         RETURNING id",
        params![root_id, parent_id, rel.to_string_lossy(), name],
        |r| r.get(0),
    )?;
    Ok(id)
}

/// Adds or updates the record for one PDF and returns its id. A file is fingerprinted only
/// when it is new or its size or modification time changed.
fn import_file(tx: &Transaction, path: &Path) -> Result<i64> {
    let meta = fs_guard::metadata(path)?;
    let size = meta.len() as i64;
    let mtime = fs_guard::mtime_ms(&meta);
    let path_str = path.to_string_lossy();

    let existing: Option<(i64, i64, i64)> = tx
        .query_row(
            "SELECT id, size, mtime FROM files WHERE path = ?1",
            [&path_str],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;

    match existing {
        Some((id, old_size, old_mtime)) if old_size == size && old_mtime == mtime => {
            tx.execute("UPDATE files SET missing_since = NULL WHERE id = ?1", [id])?;
            Ok(id)
        }
        Some((id, _, _)) => {
            let fp = fingerprint::fingerprint(path)?;
            tx.execute(
                "UPDATE files SET size = ?2, mtime = ?3, missing_since = NULL,
                     thumb_status = CASE WHEN fingerprint = ?4 THEN thumb_status ELSE 'pending' END,
                     fingerprint = ?4
                 WHERE id = ?1",
                params![id, size, mtime, fp],
            )?;
            Ok(id)
        }
        None => {
            let fp = fingerprint::fingerprint(path)?;
            tx.execute(
                "INSERT INTO files (path, size, mtime, fingerprint) VALUES (?1, ?2, ?3, ?4)",
                params![path_str, size, mtime, fp],
            )?;
            Ok(tx.last_insert_rowid())
        }
    }
}
