//! Opening documents in the reader, and saving annotations.
//!
//! By default annotations are saved to an annotated copy in Billy's data folder
//! (`%APPDATA%\Billy\annotations`), and Billy opens that copy next time. The original is only
//! written by "Save to original file", through [`fs_guard::write_original`].

use crate::db::now_ms;
use crate::error::{Error, Result};
use crate::fs_guard;
use crate::scan::fingerprint::fingerprint;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Which version of a document to read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Source {
    Original,
    Annotated,
}

impl Source {
    pub fn parse(text: &str) -> Result<Self> {
        match text {
            "original" => Ok(Source::Original),
            "annotated" => Ok(Source::Annotated),
            _ => Err(Error::msg(format!("unknown source {text}"))),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenInfo {
    pub file_id: i64,
    pub name: String,
    pub path: String,
    pub original_size: i64,
    /// Size of the annotated copy, when there is one.
    pub annotated_size: Option<i64>,
    /// The original changed after the annotated copy was made.
    pub original_changed: bool,
    pub last_page: Option<i64>,
}

/// Where the annotated copy of a file lives.
pub fn copy_path(data_dir: &Path, file_id: i64) -> PathBuf {
    data_dir.join("annotations").join(format!("{file_id}.pdf"))
}

/// What the reader needs to know before opening a document.
pub fn open_info(conn: &Connection, file_id: i64) -> Result<OpenInfo> {
    let (path, fp, missing): (String, String, bool) = conn
        .query_row(
            "SELECT path, fingerprint, missing_since IS NOT NULL FROM files WHERE id = ?1",
            [file_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?
        .ok_or_else(|| Error::msg("Billy no longer knows that document."))?;
    if missing {
        return Err(Error::msg("This file is no longer on disk."));
    }
    let original_size = fs_guard::metadata(Path::new(&path))
        .map_err(|e| Error::msg(format!("Billy can't read {path}: {e}")))?
        .len() as i64;

    let annotation: Option<(String, String)> = conn
        .query_row(
            "SELECT copy_path, source_fingerprint FROM annotations WHERE file_id = ?1",
            [file_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let (annotated_size, original_changed) = match annotation {
        Some((copy, source_fp)) => match std::fs::metadata(&copy) {
            Ok(meta) => (Some(meta.len() as i64), source_fp != fp),
            Err(_) => (None, false),
        },
        None => (None, false),
    };
    let last_page = conn
        .query_row("SELECT last_page FROM file_state WHERE file_id = ?1", [file_id], |r| r.get(0))
        .optional()?
        .flatten();
    let name = Path::new(&path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    Ok(OpenInfo { file_id, name, path, original_size, annotated_size, original_changed, last_page })
}

fn source_path(conn: &Connection, file_id: i64, source: Source) -> Result<PathBuf> {
    let sql = match source {
        Source::Original => "SELECT path FROM files WHERE id = ?1",
        Source::Annotated => "SELECT copy_path FROM annotations WHERE file_id = ?1",
    };
    conn.query_row(sql, [file_id], |r| r.get::<_, String>(0))
        .optional()?
        .map(PathBuf::from)
        .ok_or_else(|| Error::msg("That version of the document does not exist."))
}

/// Reads `length` bytes from `start` (fewer at the end of the file). PDF.js asks for the parts
/// it needs, so large books open without reading the whole file.
pub fn read_range(conn: &Connection, file_id: i64, source: Source, start: u64, length: u64) -> Result<Vec<u8>> {
    let path = source_path(conn, file_id, source)?;
    let mut file = fs_guard::open_read(&path)?;
    file.seek(SeekFrom::Start(start))?;
    let mut buf = Vec::with_capacity(length.min(64 * 1024 * 1024) as usize);
    file.take(length).read_to_end(&mut buf)?;
    Ok(buf)
}

/// A saved document, as the reader sends it.
#[derive(Debug, Clone)]
pub enum Payload {
    /// The whole document.
    Full(Vec<u8>),
    /// PDF.js saves annotations as an incremental update: the version that was open, followed
    /// by new bytes. Only the new bytes are sent, with `probe`, the last bytes of the open
    /// version, so Billy can check that the file on disk is that version. Billy copies the rest
    /// from disk. This keeps saving cheap for very large books.
    Append { source: Source, base_len: u64, probe: Vec<u8>, tail: Vec<u8> },
}

impl Payload {
    /// Writes the complete document to `out`.
    pub fn write_to(&self, conn: &Connection, file_id: i64, out: &mut impl Write) -> Result<()> {
        match self {
            Payload::Full(bytes) => {
                if !bytes.starts_with(b"%PDF-") {
                    return Err(Error::msg("The reader did not produce a PDF."));
                }
                out.write_all(bytes)?;
            }
            Payload::Append { source, base_len, probe, tail } => {
                let base = source_path(conn, file_id, *source)?;
                let mut file = fs_guard::open_read(&base)?;
                let len = file.metadata()?.len();
                let probe_len = probe.len() as u64;
                if len < *base_len || probe_len > *base_len {
                    return Err(Error::msg("The file on disk is not the version that was open. Nothing was saved."));
                }
                let mut on_disk = vec![0u8; probe.len()];
                file.seek(SeekFrom::Start(base_len - probe_len))?;
                file.read_exact(&mut on_disk)?;
                if on_disk != *probe {
                    return Err(Error::msg("The file on disk changed while it was open. Nothing was saved."));
                }
                file.seek(SeekFrom::Start(0))?;
                let mut head = [0u8; 5];
                file.read_exact(&mut head)?;
                if &head != b"%PDF-" {
                    return Err(Error::msg("The open document is not a PDF."));
                }
                file.seek(SeekFrom::Start(0))?;
                std::io::copy(&mut (&mut file).take(*base_len), out)?;
                out.write_all(tail)?;
            }
        }
        Ok(())
    }
}

/// Writes a payload to `path` through a temporary file, so a failed save never leaves a
/// half-written document. For Billy's own folders only; originals go through fs_guard.
fn write_file(conn: &Connection, file_id: i64, payload: &Payload, path: &Path) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    let result = (|| -> Result<()> {
        let mut out = std::io::BufWriter::new(std::fs::File::create(&tmp)?);
        payload.write_to(conn, file_id, &mut out)?;
        out.flush()?;
        drop(out);
        std::fs::rename(&tmp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Writes a payload to a place the user picked in the save dialog.
pub fn export(conn: &Connection, file_id: i64, payload: &Payload, path: &Path) -> Result<()> {
    write_file(conn, file_id, payload, path)
}

/// Saves the document as its annotated copy. `from_original` says whether the reader had the
/// original open (so the copy now matches the current original) or the earlier annotated
/// copy (which keeps its record of what it was based on).
pub fn save_annotated(conn: &Connection, data_dir: &Path, file_id: i64, payload: &Payload, from_original: bool) -> Result<()> {
    let current_fp: String = conn.query_row("SELECT fingerprint FROM files WHERE id = ?1", [file_id], |r| r.get(0))?;
    let copy = copy_path(data_dir, file_id);
    write_file(conn, file_id, payload, &copy)?;
    conn.execute(
        "INSERT INTO annotations (file_id, copy_path, source_fingerprint, updated_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (file_id) DO UPDATE SET
             copy_path = excluded.copy_path,
             updated_at = excluded.updated_at,
             source_fingerprint = CASE WHEN ?5 THEN excluded.source_fingerprint ELSE annotations.source_fingerprint END",
        params![file_id, copy.to_string_lossy(), current_fp, now_ms(), from_original],
    )?;
    Ok(())
}

/// Deletes the annotated copy and its record. Only touches Billy's data folder.
pub fn discard_annotated(conn: &Connection, data_dir: &Path, file_id: i64) -> Result<()> {
    let copy: Option<String> = conn
        .query_row("SELECT copy_path FROM annotations WHERE file_id = ?1", [file_id], |r| r.get(0))
        .optional()?;
    conn.execute("DELETE FROM annotations WHERE file_id = ?1", [file_id])?;
    if let Some(copy) = copy.map(PathBuf::from) {
        if copy.starts_with(data_dir) {
            let _ = std::fs::remove_file(copy);
        }
    }
    Ok(())
}

/// Writes annotated bytes into the original file, after the user confirmed it. The annotated
/// copy is no longer needed and is removed. The file's record is updated, so a sync sees no
/// change and the cover is made again.
pub fn save_to_original(conn: &Connection, data_dir: &Path, file_id: i64, payload: &Payload) -> Result<()> {
    let path = source_path(conn, file_id, Source::Original)?;
    let mut failure: Option<Error> = None;
    fs_guard::write_original(&path, |out| {
        payload.write_to(conn, file_id, out).map_err(|e| {
            let message = e.to_string();
            failure = Some(e);
            std::io::Error::other(message)
        })
    })
    .map_err(|e| failure.take().unwrap_or_else(|| Error::msg(format!("Could not save {}: {e}", path.display()))))?;
    let meta = fs_guard::metadata(&path)?;
    let fp = fingerprint(&path)?;
    conn.execute(
        "UPDATE files SET size = ?2, mtime = ?3, missing_since = NULL,
             thumb_status = CASE WHEN fingerprint = ?4 THEN thumb_status ELSE 'pending' END,
             fingerprint = ?4
         WHERE id = ?1",
        params![file_id, meta.len() as i64, fs_guard::mtime_ms(&meta), fp],
    )?;
    discard_annotated(conn, data_dir, file_id)
}

pub fn set_last_page(conn: &Connection, file_id: i64, page: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO file_state (file_id, last_page, last_opened_at) VALUES (?1, ?2, ?3)
         ON CONFLICT (file_id) DO UPDATE SET last_page = excluded.last_page, last_opened_at = excluded.last_opened_at",
        params![file_id, page, now_ms()],
    )?;
    Ok(())
}
