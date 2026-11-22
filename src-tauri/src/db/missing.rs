//! Files that disappeared from disk. Billy keeps their records, favorites, read status and
//! annotations until the user confirms a clean-up, so nothing is lost if a file comes back.

use crate::error::Result;
use rusqlite::{Connection, params};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingFile {
    pub file_id: i64,
    pub path: String,
    pub missing_since: i64,
    pub favorite: bool,
    pub read: bool,
    pub annotated: bool,
}

/// Missing files in a library, whose root folder is available. Files on an offline root are
/// not missing; their drive is just not there.
pub fn list_missing(conn: &Connection, library_id: i64) -> Result<Vec<MissingFile>> {
    let mut stmt = conn.prepare(
        "SELECT x.id, x.path, x.missing_since,
                EXISTS (SELECT 1 FROM favorites f WHERE f.library_id = e.library_id AND f.file_id = x.id),
                COALESCE((SELECT rs.read FROM read_state rs WHERE rs.library_id = e.library_id AND rs.file_id = x.id), 0),
                EXISTS (SELECT 1 FROM annotations a WHERE a.file_id = x.id)
         FROM entries e
         JOIN files x ON x.id = e.file_id
         JOIN roots r ON r.id = e.root_id
         WHERE e.library_id = ?1 AND x.missing_since IS NOT NULL AND r.status = 'ok'
         ORDER BY x.path COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([library_id], |r| {
        Ok(MissingFile {
            file_id: r.get(0)?,
            path: r.get(1)?,
            missing_since: r.get(2)?,
            favorite: r.get(3)?,
            read: r.get(4)?,
            annotated: r.get(5)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Forgets missing files in a library: their entries, favorites and read status there. A file
/// that no other library holds is forgotten completely, with its annotations. Returns the
/// annotated copies that the caller should delete from Billy's data folder.
pub fn clean_up(conn: &mut Connection, library_id: i64, file_ids: &[i64]) -> Result<Vec<PathBuf>> {
    let tx = crate::db::write_tx(conn)?;
    let mut copies = Vec::new();
    for &file_id in file_ids {
        let missing: bool = tx.query_row(
            "SELECT missing_since IS NOT NULL FROM files WHERE id = ?1",
            [file_id],
            |r| r.get(0),
        ).unwrap_or(false);
        if !missing {
            continue; // It came back in the meantime.
        }
        for table in ["entries", "favorites", "read_state"] {
            tx.execute(
                &format!("DELETE FROM {table} WHERE library_id = ?1 AND file_id = ?2"),
                params![library_id, file_id],
            )?;
        }
        let elsewhere: bool = tx.query_row(
            "SELECT EXISTS (SELECT 1 FROM entries WHERE file_id = ?1)",
            [file_id],
            |r| r.get(0),
        )?;
        if !elsewhere {
            if let Ok(copy) = tx.query_row("SELECT copy_path FROM annotations WHERE file_id = ?1", [file_id], |r| {
                r.get::<_, String>(0)
            }) {
                copies.push(PathBuf::from(copy));
            }
            // Cascades to file_state, annotations and any favorites left in other libraries.
            tx.execute("DELETE FROM files WHERE id = ?1", [file_id])?;
        }
    }
    tx.commit()?;
    Ok(copies)
}
