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
}

/// Lists the documents of a library. With `folder_id`, only that folder, and with `recursive`
/// also all its subfolders. Without `folder_id`, the whole library. Missing files are left out.
pub fn list_documents(
    conn: &Connection,
    library_id: i64,
    folder_id: Option<i64>,
    recursive: bool,
) -> Result<Vec<Document>> {
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
        "SELECT x.id, x.path, e.folder_id, x.size, x.mtime, e.date_added, x.page_count,
                x.title, x.author, x.fingerprint,
                COALESCE(rs.read, 0), fav.file_id IS NOT NULL, x.thumb_status
         FROM entries e
         JOIN files x ON x.id = e.file_id
         LEFT JOIN read_state rs ON rs.library_id = e.library_id AND rs.file_id = e.file_id
         LEFT JOIN favorites fav ON fav.library_id = e.library_id AND fav.file_id = e.file_id
         WHERE e.library_id = ?1 AND x.missing_since IS NULL {scope}"
    );
    let mut stmt = conn.prepare(&sql)?;
    let map = |r: &rusqlite::Row| {
        let path: String = r.get(1)?;
        let name = Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        Ok(Document {
            file_id: r.get(0)?,
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
        })
    };
    let rows = match folder_id {
        None => stmt.query_map(params![library_id], map)?.collect::<rusqlite::Result<Vec<_>>>()?,
        Some(folder) => stmt
            .query_map(params![library_id, folder], map)?
            .collect::<rusqlite::Result<Vec<_>>>()?,
    };
    Ok(rows)
}
