//! Scanning and syncing library folders.
//!
//! The scanner only reads library folders, through [`crate::fs_guard`]. Everything it
//! learns is written to Billy's database. A scan is also a sync: it adds new PDFs, updates
//! changed ones, follows moved and renamed files, and marks files that disappeared as missing.

pub mod fingerprint;

use crate::db::{libraries::is_inside, now_ms, write_tx};
use crate::db::libraries;
use crate::error::Result;
use crate::fs_guard;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Files are committed and progress is reported after every batch of this many PDFs, or
/// sooner when a batch takes longer than `BATCH_TIME`. The scan holds the write lock while a
/// batch is open, so the thumbnail worker and the app wait for at most that long.
const BATCH: u64 = 200;
const BATCH_TIME: std::time::Duration = std::time::Duration::from_secs(1);

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
    /// PDFs added to the library by this scan.
    pub new_files: u64,
    /// PDFs found at a new path that Billy recognized by their content.
    pub moved: u64,
    /// PDFs that disappeared from disk and are now marked as missing.
    pub missing: u64,
    /// PDFs that left this folder for a place outside the library, or are now ignored.
    pub removed: u64,
    /// The root folder could not be found, for example because its drive is unplugged.
    pub offline: bool,
    /// Items that could not be read. The scan skips them and carries on.
    pub errors: Vec<String>,
}

/// A root folder of any library.
struct RootInfo {
    path: String,
    depth: usize,
}

fn all_roots(conn: &Connection) -> Result<HashMap<i64, RootInfo>> {
    let mut stmt = conn.prepare("SELECT id, path FROM roots")?;
    let rows = stmt.query_map([], |r| {
        let path: String = r.get(1)?;
        Ok((r.get::<_, i64>(0)?, RootInfo { depth: Path::new(&path).components().count(), path }))
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn exists(path: &str) -> bool {
    fs_guard::metadata(Path::new(path)).is_ok()
}

/// Scans one root folder and brings its library up to date.
pub fn scan_root(
    conn: &mut Connection,
    root_id: i64,
    progress: &mut dyn FnMut(&ScanProgress),
) -> Result<ScanReport> {
    let root = libraries::get_root(conn, root_id)?;
    let root_path = PathBuf::from(&root.path);
    let mut report = ScanReport { root_id, ..Default::default() };

    // An unplugged drive or a renamed folder: change nothing, so no file is marked missing.
    if !fs_guard::is_dir(&root_path) {
        conn.execute("UPDATE roots SET status = 'offline' WHERE id = ?1", [root_id])?;
        report.offline = true;
        return Ok(report);
    }

    // Tell the UI right away that this folder is syncing.
    progress(&ScanProgress { root_id, files_found: 0 });
    let now = now_ms();
    let roots = all_roots(conn)?;
    let my_depth = roots.get(&root_id).map(|r| r.depth).unwrap_or(0);
    let mut folder_ids: HashMap<PathBuf, i64> = HashMap::new();
    let mut seen: HashSet<i64> = HashSet::new();
    let ignored = libraries::ignored_under(conn, root.library_id, &root.path)?;
    let mut tx = write_tx(conn)?;
    let mut in_batch = 0;
    let mut batch_started = std::time::Instant::now();

    let skip_dir = |dir: &Path| {
        let dir = dir.to_string_lossy();
        ignored.iter().any(|folder| libraries::is_same_or_inside(&dir, folder))
    };
    for item in fs_guard::walk(&root_path, skip_dir) {
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

        let (file_id, moved) = match import_file(&tx, path, &roots) {
            Ok(result) => result,
            Err(err) => {
                report.errors.push(format!("{}: {err}", path.display()));
                continue;
            }
        };
        seen.insert(file_id);
        let path_str = path.to_string_lossy();

        if moved {
            report.moved += 1;
            // Other libraries whose folders no longer hold the file lose it. Their scans would
            // find out too, but this keeps them right in the meantime.
            let mut stmt = tx.prepare("SELECT library_id, root_id FROM entries WHERE file_id = ?1")?;
            let stale: Vec<(i64, i64)> = stmt
                .query_map([file_id], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?
                .into_iter()
                .filter(|(library, owner)| {
                    *library != root.library_id
                        && roots.get(owner).is_none_or(|r| !is_inside(&path_str, &r.path))
                })
                .collect();
            drop(stmt);
            for (library, _) in stale {
                tx.execute("DELETE FROM entries WHERE library_id = ?1 AND file_id = ?2", params![library, file_id])?;
            }
        }

        let owner: Option<i64> = tx
            .query_row(
                "SELECT root_id FROM entries WHERE library_id = ?1 AND file_id = ?2",
                params![root.library_id, file_id],
                |r| r.get(0),
            )
            .optional()?;
        match owner {
            None => {
                tx.execute(
                    "INSERT INTO entries (library_id, root_id, file_id, folder_id, date_added)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![root.library_id, root_id, file_id, folder_id, now],
                )?;
                if !moved {
                    report.new_files += 1;
                }
            }
            Some(owner) if owner == root_id => {
                // The file may have moved to another folder of this root.
                tx.execute(
                    "UPDATE entries SET folder_id = ?3 WHERE library_id = ?1 AND file_id = ?2 AND folder_id != ?3",
                    params![root.library_id, file_id, folder_id],
                )?;
            }
            Some(owner) => {
                // When roots overlap, the outermost root that holds the file owns it.
                let take_over = match roots.get(&owner) {
                    None => true,
                    Some(r) => !is_inside(&path_str, &r.path) || r.depth > my_depth,
                };
                if take_over {
                    tx.execute(
                        "UPDATE entries SET root_id = ?3, folder_id = ?4 WHERE library_id = ?1 AND file_id = ?2",
                        params![root.library_id, file_id, root_id, folder_id],
                    )?;
                }
            }
        }
        report.files += 1;

        in_batch += 1;
        if in_batch >= BATCH || batch_started.elapsed() >= BATCH_TIME {
            tx.commit()?;
            tx = write_tx(conn)?;
            in_batch = 0;
            batch_started = std::time::Instant::now();
            progress(&ScanProgress { root_id, files_found: report.files });
        }
    }

    // Documents of this root that the walk did not find.
    let unseen: Vec<(i64, String, bool)> = {
        let mut stmt = tx.prepare(
            "SELECT e.file_id, x.path, x.missing_since IS NOT NULL
             FROM entries e JOIN files x ON x.id = e.file_id WHERE e.root_id = ?1",
        )?;
        stmt.query_map([root_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .filter(|(id, _, _)| !seen.contains(id))
            .collect()
    };
    for (file_id, path, already_missing) in unseen {
        if !exists(&path) {
            // Gone from disk. Keep everything, so it comes back if the file does.
            if !already_missing {
                tx.execute("UPDATE files SET missing_since = ?2 WHERE id = ?1", params![file_id, now])?;
                report.missing += 1;
            }
        } else if !is_inside(&path, &root.path) || report.errors.is_empty() || skip_dir(Path::new(&path)) {
            // Still on disk, but moved out of this folder, now hidden, or in an ignored folder.
            // When the walk had errors, Billy may simply have failed to read it, so it is
            // otherwise left alone.
            tx.execute(
                "DELETE FROM entries WHERE library_id = ?1 AND file_id = ?2",
                params![root.library_id, file_id],
            )?;
            report.removed += 1;
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

/// Adds or updates the record for one PDF. Returns its id, and whether it is a known file
/// that moved here. A file is fingerprinted only when it is new or its size or modification
/// time changed.
fn import_file(tx: &Transaction, path: &Path, roots: &HashMap<i64, RootInfo>) -> Result<(i64, bool)> {
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
            tx.execute("UPDATE files SET missing_since = NULL WHERE id = ?1 AND missing_since IS NOT NULL", [id])?;
            Ok((id, false))
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
            Ok((id, false))
        }
        None => {
            let fp = fingerprint::fingerprint(path)?;
            if let Some(id) = moved_from(tx, &fp, roots)? {
                tx.execute(
                    "UPDATE files SET path = ?2, size = ?3, mtime = ?4, missing_since = NULL WHERE id = ?1",
                    params![id, path_str, size, mtime],
                )?;
                return Ok((id, true));
            }
            tx.execute(
                "INSERT INTO files (path, size, mtime, fingerprint) VALUES (?1, ?2, ?3, ?4)",
                params![path_str, size, mtime, fp],
            )?;
            Ok((tx.last_insert_rowid(), false))
        }
    }
}

/// Finds a known file with this fingerprint whose old path no longer exists: the file has
/// moved. Files on an unavailable root folder (an unplugged drive) are never taken, because
/// they have not gone anywhere.
fn moved_from(tx: &Transaction, fingerprint: &str, roots: &HashMap<i64, RootInfo>) -> Result<Option<i64>> {
    let mut stmt = tx.prepare(
        "SELECT id, path FROM files WHERE fingerprint = ?1 ORDER BY missing_since IS NULL, id",
    )?;
    let candidates: Vec<(i64, String)> = stmt
        .query_map([fingerprint], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    for (id, old_path) in candidates {
        if exists(&old_path) {
            continue;
        }
        let on_unavailable_root = roots
            .values()
            .any(|r| is_inside(&old_path, &r.path) && !fs_guard::is_dir(Path::new(&r.path)));
        if !on_unavailable_root {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

/// Checks every root folder and marks it offline or available. Cheap; run at startup.
pub fn refresh_root_status(conn: &Connection) -> Result<()> {
    let roots = all_roots(conn)?;
    for (id, root) in roots {
        let status = if fs_guard::is_dir(Path::new(&root.path)) { "ok" } else { "offline" };
        conn.execute("UPDATE roots SET status = ?2 WHERE id = ?1 AND status != ?2", params![id, status])?;
    }
    Ok(())
}
