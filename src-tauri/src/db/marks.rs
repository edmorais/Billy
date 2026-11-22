//! Favorites and read status. Both belong to a library: the same file can be a favorite or
//! read in one library and not in another.

use crate::db::now_ms;
use crate::error::{Error, Result};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::PathBuf;

/// Pins or unpins documents in a library. Files that are not in the library are ignored.
pub fn set_favorite(conn: &mut Connection, library_id: i64, file_ids: &[i64], favorite: bool) -> Result<()> {
    let tx = crate::db::write_tx(conn)?;
    let now = now_ms();
    for &file_id in file_ids {
        if favorite {
            tx.execute(
                "INSERT OR IGNORE INTO favorites (library_id, file_id, pinned_at)
                 SELECT ?1, ?2, ?3 WHERE EXISTS (SELECT 1 FROM entries WHERE library_id = ?1 AND file_id = ?2)",
                params![library_id, file_id, now],
            )?;
        } else {
            tx.execute(
                "DELETE FROM favorites WHERE library_id = ?1 AND file_id = ?2",
                params![library_id, file_id],
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// Marks documents in a library as read or unread. Files that are not in the library are ignored.
pub fn set_read(conn: &mut Connection, library_id: i64, file_ids: &[i64], read: bool) -> Result<()> {
    let tx = crate::db::write_tx(conn)?;
    let now = now_ms();
    for &file_id in file_ids {
        tx.execute(
            "INSERT INTO read_state (library_id, file_id, read, changed_at)
             SELECT ?1, ?2, ?3, ?4 WHERE EXISTS (SELECT 1 FROM entries WHERE library_id = ?1 AND file_id = ?2)
             ON CONFLICT (library_id, file_id) DO UPDATE SET read = excluded.read, changed_at = excluded.changed_at",
            params![library_id, file_id, read, now],
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// Path of a document on disk.
pub fn file_path(conn: &Connection, file_id: i64) -> Result<PathBuf> {
    conn.query_row("SELECT path FROM files WHERE id = ?1", [file_id], |r| r.get::<_, String>(0))
        .optional()?
        .map(PathBuf::from)
        .ok_or_else(|| Error::msg("Billy no longer knows that document."))
}

/// Path of a library folder on disk.
pub fn folder_path(conn: &Connection, folder_id: i64) -> Result<PathBuf> {
    conn.query_row(
        "SELECT r.path, f.rel_path FROM folders f JOIN roots r ON r.id = f.root_id WHERE f.id = ?1",
        [folder_id],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
    )
    .optional()?
    .map(|(root, rel)| if rel.is_empty() { PathBuf::from(root) } else { PathBuf::from(root).join(rel) })
    .ok_or_else(|| Error::msg("Billy no longer knows that folder."))
}
