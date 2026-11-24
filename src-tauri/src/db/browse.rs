//! Queries for browsing a library: its folder tree and its documents.

use crate::db::libraries::{self, is_inside};
use crate::error::Result;
use rusqlite::{Connection, params};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderNode {
    pub id: i64,
    pub root_id: i64,
    /// `None` for a root folder.
    pub parent_id: Option<i64>,
    pub name: String,
    pub rel_path: String,
    pub root_path: String,
    pub offline: bool,
    /// Documents directly in this folder.
    pub count: i64,
    /// Documents in this folder and all its subfolders.
    pub total: i64,
}

/// Returns the library's folders as a flat list; the UI builds the tree from `parent_id`.
///
/// Roots that sit inside another root of the same library are left out, because their
/// documents already appear under the outer root. Subfolders without any documents are left
/// out too. Root folders are always included, even when empty.
pub fn folder_tree(conn: &Connection, library_id: i64) -> Result<Vec<FolderNode>> {
    let roots = libraries::list_roots(conn, library_id)?;
    let nested: HashSet<i64> = roots
        .iter()
        .filter(|r| roots.iter().any(|o| is_inside(&r.path, &o.path)))
        .map(|r| r.id)
        .collect();

    let mut stmt = conn.prepare(
        "SELECT f.id, f.root_id, f.parent_id, f.name, f.rel_path, r.path, r.status,
                (SELECT COUNT(*) FROM entries e JOIN files x ON x.id = e.file_id
                 WHERE e.folder_id = f.id AND e.library_id = ?1 AND x.missing_since IS NULL)
         FROM folders f JOIN roots r ON r.id = f.root_id
         WHERE r.library_id = ?1",
    )?;
    let mut nodes: Vec<FolderNode> = stmt
        .query_map([library_id], |r| {
            let status: String = r.get(6)?;
            let count: i64 = r.get(7)?;
            Ok(FolderNode {
                id: r.get(0)?,
                root_id: r.get(1)?,
                parent_id: r.get(2)?,
                name: r.get(3)?,
                rel_path: r.get(4)?,
                root_path: r.get(5)?,
                offline: status == "offline",
                count,
                total: count,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .filter(|n| !nested.contains(&n.root_id))
        .collect();

    // Add each folder's total to its parent, deepest folders first.
    nodes.sort_by_key(|n| std::cmp::Reverse(depth(&n.rel_path)));
    let index: HashMap<i64, usize> = nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
    for i in 0..nodes.len() {
        if let Some(parent) = nodes[i].parent_id.and_then(|p| index.get(&p)).copied() {
            nodes[parent].total += nodes[i].total;
        }
    }

    nodes.retain(|n| n.parent_id.is_none() || n.total > 0);
    nodes.sort_by_key(|n| depth(&n.rel_path));
    Ok(nodes)
}

fn depth(rel_path: &str) -> usize {
    Path::new(rel_path).components().count()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub file_id: i64,
    /// The library this entry belongs to. Favorites and read status are kept per library.
    pub library_id: i64,
    pub name: String,
    pub path: String,
    pub folder_id: i64,
    pub size: i64,
    pub modified: i64,
    pub added: i64,
    pub pages: Option<i64>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub fingerprint: String,
    /// pending | ok | error
    pub thumb_status: String,
    pub read: bool,
    pub favorite: bool,
    /// The file is gone from disk; Billy keeps its record until a clean-up.
    pub missing: bool,
    /// The file's root folder is unavailable, for example an unplugged drive.
    pub offline: bool,
    /// Billy has an annotated copy of this file.
    pub annotated: bool,
}

/// Lists the documents of a library. With `folder_id`, only that folder, and with `recursive`
/// also all its subfolders. Without `folder_id`, the whole library. Missing files are left out.
/// With `favorites_only`, only the library's favorites.
pub fn list_documents(
    conn: &Connection,
    library_id: i64,
    folder_id: Option<i64>,
    recursive: bool,
    favorites_only: bool,
) -> Result<Vec<Document>> {
    list_documents_with(conn, library_id, folder_id, recursive, favorites_only, false)
}

/// Like [`list_documents`], and with `include_missing` also lists missing files.
pub fn list_documents_with(
    conn: &Connection,
    library_id: i64,
    folder_id: Option<i64>,
    recursive: bool,
    favorites_only: bool,
    include_missing: bool,
) -> Result<Vec<Document>> {
    let favorites = if favorites_only { "AND fav.file_id IS NOT NULL" } else { "" };
    let missing = if include_missing { "" } else { "AND x.missing_since IS NULL" };
    let scope = match (folder_id, recursive) {
        (None, _) => "",
        (Some(_), false) => "AND e.folder_id = ?2",
        (Some(_), true) => {
            "AND e.folder_id IN (
                WITH RECURSIVE sub(id) AS (
                    SELECT ?2
                    UNION ALL
                    SELECT f.id FROM folders f JOIN sub ON f.parent_id = sub.id
                )
                SELECT id FROM sub)"
        }
    };
    let sql = format!(
        "SELECT {DOCUMENT_COLUMNS} FROM {DOCUMENT_TABLES}
         WHERE e.library_id = ?1 {missing} {scope} {favorites}"
    );
    let mut stmt = conn.prepare(&sql)?;
    let map = document_from_row;
    let rows = match folder_id {
        None => stmt.query_map(params![library_id], map)?.collect::<rusqlite::Result<Vec<_>>>()?,
        Some(folder) => stmt
            .query_map(params![library_id, folder], map)?
            .collect::<rusqlite::Result<Vec<_>>>()?,
    };
    Ok(rows)
}

/// The columns [`document_from_row`] reads, for the tables in [`DOCUMENT_TABLES`].
const DOCUMENT_COLUMNS: &str = "x.id, x.path, e.folder_id, x.size, x.mtime, e.date_added, x.page_count,
    x.title, x.author, x.fingerprint,
    COALESCE(rs.read, 0), fav.file_id IS NOT NULL, x.thumb_status,
    x.missing_since IS NOT NULL, r.status = 'offline',
    EXISTS (SELECT 1 FROM annotations a WHERE a.file_id = x.id), e.library_id";

/// A library's entries with their files, roots, read status and favorites.
const DOCUMENT_TABLES: &str = "entries e
    JOIN files x ON x.id = e.file_id
    JOIN roots r ON r.id = e.root_id
    LEFT JOIN read_state rs ON rs.library_id = e.library_id AND rs.file_id = e.file_id
    LEFT JOIN favorites fav ON fav.library_id = e.library_id AND fav.file_id = e.file_id";

fn document_from_row(r: &rusqlite::Row) -> rusqlite::Result<Document> {
    let path: String = r.get(1)?;
    let name = Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.clone());
    Ok(Document {
        file_id: r.get(0)?,
        library_id: r.get(16)?,
        name,
        path,
        folder_id: r.get(2)?,
        size: r.get(3)?,
        modified: r.get(4)?,
        added: r.get(5)?,
        pages: r.get(6)?,
        title: r.get(7)?,
        author: r.get(8)?,
        fingerprint: r.get(9)?,
        thumb_status: r.get(12)?,
        read: r.get(10)?,
        favorite: r.get(11)?,
        missing: r.get(13)?,
        offline: r.get(14)?,
        annotated: r.get(15)?,
    })
}

/// The documents opened most recently, newest first, at most `limit` of them. With
/// `library_id`, only that library's documents; without it, every library's, each file once:
/// as an entry of `prefer` (the library that is open) if it is there, otherwise of the first
/// library that holds it. Missing files are left out.
pub fn list_history(
    conn: &Connection,
    library_id: Option<i64>,
    limit: i64,
    prefer: Option<i64>,
) -> Result<Vec<Document>> {
    let which = match library_id {
        Some(_) => "e.library_id = ?1",
        None => {
            "e.library_id = (SELECT e2.library_id FROM entries e2 WHERE e2.file_id = x.id
                             ORDER BY e2.library_id = ?3 DESC, e2.library_id LIMIT 1)"
        }
    };
    let sql = format!(
        "SELECT {DOCUMENT_COLUMNS} FROM {DOCUMENT_TABLES}
         JOIN file_state st ON st.file_id = x.id
         WHERE {which} AND st.last_opened_at IS NOT NULL AND x.missing_since IS NULL
         ORDER BY st.last_opened_at DESC, x.id DESC
         LIMIT ?2"
    );
    let mut stmt = conn.prepare(&sql)?;
    let limit = limit.clamp(1, 100);
    let rows = match library_id {
        Some(id) => stmt.query_map(params![id, limit], document_from_row)?,
        None => stmt.query_map(params![0, limit, prefer.unwrap_or(-1)], document_from_row)?,
    };
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}
