//! Opening documents in the reader, and saving annotations.
//!
//! PDF.js saves annotations by appending an update to the end of the PDF. Billy keeps only
//! those appended bytes, in the roaming profile (`%APPDATA%\Billy\annotations`), with the
//! length of the original they extend and a check of its last bytes. The annotated version is
//! the original's first part followed by those bytes, joined when it is read. This keeps
//! annotations small (usually a few kilobytes, even for books of hundreds of megabytes).
//!
//! When a save is not a simple append, Billy keeps a full annotated copy instead, in the local
//! profile (`%LOCALAPPDATA%\Billy\annotations`), so large copies never roam.
//!
//! The original is only written by "Save to original file", through
//! [`fs_guard::write_original`].

use crate::db::now_ms;
use crate::error::{Error, Result};
use crate::fs_guard;
use crate::scan::fingerprint::fingerprint;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use xxhash_rust::xxh3::xxh3_64;

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

/// Where Billy keeps annotations.
#[derive(Debug, Clone)]
pub struct Dirs {
    /// Billy's roaming data folder, %APPDATA%\Billy: small annotation files.
    pub roaming: PathBuf,
    /// Billy's local data folder, %LOCALAPPDATA%\Billy: full copies, when needed.
    pub local: PathBuf,
}

impl Dirs {
    pub fn delta_path(&self, file_id: i64) -> PathBuf {
        self.roaming.join("annotations").join(format!("{file_id}.pdfdelta"))
    }

    pub fn copy_path(&self, file_id: i64) -> PathBuf {
        self.local.join("annotations").join(format!("{file_id}.pdf"))
    }

    /// True for files in Billy's own folders, the only ones Billy deletes.
    pub fn owns(&self, path: &Path) -> bool {
        path.starts_with(&self.roaming) || path.starts_with(&self.local)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenInfo {
    pub file_id: i64,
    pub name: String,
    pub path: String,
    pub original_size: i64,
    /// Size of the annotated version, when it can be shown.
    pub annotated_size: Option<i64>,
    /// The original changed after it was annotated (the annotations can still be shown).
    pub original_changed: bool,
    /// There are saved annotations, but the original was replaced, so they can't be shown.
    pub annotations_broken: bool,
    pub last_page: Option<i64>,
}

/// Bytes of the original that are hashed to check it is still the version that was annotated.
const CHECK_LEN: u64 = 4096;

/// A saved annotation record.
struct Stored {
    path: PathBuf,
    /// Set for appended annotations; `None` for a full copy.
    base_len: Option<u64>,
    base_check: Option<String>,
}

fn stored(conn: &Connection, file_id: i64) -> Result<Option<Stored>> {
    Ok(conn
        .query_row(
            "SELECT copy_path, base_len, base_check FROM annotations WHERE file_id = ?1",
            [file_id],
            |r| {
                Ok(Stored {
                    path: PathBuf::from(r.get::<_, String>(0)?),
                    base_len: r.get::<_, Option<i64>>(1)?.map(|n| n as u64),
                    base_check: r.get(2)?,
                })
            },
        )
        .optional()?)
}

fn original_path(conn: &Connection, file_id: i64) -> Result<PathBuf> {
    conn.query_row("SELECT path FROM files WHERE id = ?1", [file_id], |r| r.get::<_, String>(0))
        .optional()?
        .map(PathBuf::from)
        .ok_or_else(|| Error::msg("Billy no longer knows that document."))
}

/// Hash of the original's last bytes before `base_len`.
fn prefix_check(original: &Path, base_len: u64) -> io::Result<String> {
    let mut file = fs_guard::open_read(original)?;
    if file.metadata()?.len() < base_len {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "the original is shorter now"));
    }
    let n = base_len.min(CHECK_LEN);
    let mut buf = vec![0u8; n as usize];
    file.seek(SeekFrom::Start(base_len - n))?;
    file.read_exact(&mut buf)?;
    Ok(format!("{:016x}", xxh3_64(&buf)))
}

/// Whether saved appended annotations still fit the original on disk.
fn delta_fits(original: &Path, stored: &Stored) -> bool {
    match (stored.base_len, &stored.base_check) {
        (Some(base_len), Some(check)) => prefix_check(original, base_len).is_ok_and(|c| &c == check),
        _ => false,
    }
}

/// Something that can be read from anywhere.
pub trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

/// The first `first_len` bytes of one file followed by all of another, read as one.
struct Joined {
    first: File,
    first_len: u64,
    second: File,
    len: u64,
    pos: u64,
}

impl Read for Joined {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.len {
            return Ok(0);
        }
        let n = if self.pos < self.first_len {
            let max = ((self.first_len - self.pos) as usize).min(buf.len());
            self.first.seek(SeekFrom::Start(self.pos))?;
            self.first.read(&mut buf[..max])?
        } else {
            self.second.seek(SeekFrom::Start(self.pos - self.first_len))?;
            self.second.read(buf)?
        };
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for Joined {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let pos = match to {
            SeekFrom::Start(n) => n as i128,
            SeekFrom::End(n) => self.len as i128 + n as i128,
            SeekFrom::Current(n) => self.pos as i128 + n as i128,
        };
        if pos < 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "seek before start"));
        }
        self.pos = pos as u64;
        Ok(self.pos)
    }
}

/// Opens a version of a document for reading, with its length.
fn open_version(conn: &Connection, file_id: i64, source: Source) -> Result<(Box<dyn ReadSeek>, u64)> {
    let original = original_path(conn, file_id)?;
    match source {
        Source::Original => {
            let file = fs_guard::open_read(&original)?;
            let len = file.metadata()?.len();
            Ok((Box::new(file), len))
        }
        Source::Annotated => {
            let stored = stored(conn, file_id)?.ok_or_else(|| Error::msg("This document has no annotations."))?;
            match stored.base_len {
                None => {
                    let file = fs_guard::open_read(&stored.path)?;
                    let len = file.metadata()?.len();
                    Ok((Box::new(file), len))
                }
                Some(base_len) => {
                    if !delta_fits(&original, &stored) {
                        return Err(Error::msg(
                            "The original was replaced, so the saved annotations can no longer be shown.",
                        ));
                    }
                    let second = fs_guard::open_read(&stored.path)?;
                    let len = base_len + second.metadata()?.len();
                    let first = fs_guard::open_read(&original)?;
                    Ok((Box::new(Joined { first, first_len: base_len, second, len, pos: 0 }), len))
                }
            }
        }
    }
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
    let original = PathBuf::from(&path);
    let original_size = fs_guard::metadata(&original)
        .map_err(|e| Error::msg(format!("Billy can't read {path}: {e}")))?
        .len() as i64;

    let source_fp: Option<String> = conn
        .query_row("SELECT source_fingerprint FROM annotations WHERE file_id = ?1", [file_id], |r| r.get(0))
        .optional()?;
    let (mut annotated_size, mut annotations_broken) = (None, false);
    if let Some(stored) = stored(conn, file_id)? {
        let stored_len = std::fs::metadata(&stored.path).map(|m| m.len());
        match (stored.base_len, stored_len) {
            (None, Ok(len)) => annotated_size = Some(len as i64),
            (Some(base_len), Ok(len)) if delta_fits(&original, &stored) => {
                annotated_size = Some((base_len + len) as i64)
            }
            _ => annotations_broken = true,
        }
    }
    let original_changed = annotated_size.is_some() && source_fp.is_some_and(|s| s != fp);
    let last_page = conn
        .query_row("SELECT last_page FROM file_state WHERE file_id = ?1", [file_id], |r| r.get(0))
        .optional()?
        .flatten();
    let name = original.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    Ok(OpenInfo { file_id, name, path, original_size, annotated_size, original_changed, annotations_broken, last_page })
}

/// Reads `length` bytes from `start` (fewer at the end of the file). PDF.js asks for the parts
/// it needs, so large books open without reading the whole file.
pub fn read_range(conn: &Connection, file_id: i64, source: Source, start: u64, length: u64) -> Result<Vec<u8>> {
    let (mut reader, _) = open_version(conn, file_id, source)?;
    reader.seek(SeekFrom::Start(start))?;
    let mut buf = Vec::with_capacity(length.min(64 * 1024 * 1024) as usize);
    reader.take(length).read_to_end(&mut buf)?;
    Ok(buf)
}

/// A saved document, as the reader sends it.
#[derive(Debug, Clone)]
pub enum Payload {
    /// The whole document.
    Full(Vec<u8>),
    /// PDF.js saves annotations as an incremental update: the version that was open, followed
    /// by new bytes. Only the new bytes are sent, with `probe`, the last bytes of the open
    /// version, so Billy can check that what is on disk is that version.
    Append { source: Source, base_len: u64, probe: Vec<u8>, tail: Vec<u8> },
}

/// Checks that a version of the document is what the reader had open: it is long enough,
/// starts like a PDF, and its bytes just before `base_len` are `probe`.
fn verify(reader: &mut dyn ReadSeek, len: u64, base_len: u64, probe: &[u8]) -> Result<()> {
    let probe_len = probe.len() as u64;
    if len < base_len || probe_len > base_len {
        return Err(Error::msg("The file on disk is not the version that was open. Nothing was saved."));
    }
    let mut on_disk = vec![0u8; probe.len()];
    reader.seek(SeekFrom::Start(base_len - probe_len))?;
    reader.read_exact(&mut on_disk)?;
    if on_disk != probe {
        return Err(Error::msg("The file on disk changed while it was open. Nothing was saved."));
    }
    let mut head = [0u8; 5];
    reader.seek(SeekFrom::Start(0))?;
    reader.read_exact(&mut head)?;
    if &head != b"%PDF-" {
        return Err(Error::msg("The open document is not a PDF."));
    }
    reader.seek(SeekFrom::Start(0))?;
    Ok(())
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
                let (mut reader, len) = open_version(conn, file_id, *source)?;
                verify(reader.as_mut(), len, *base_len, probe)?;
                io::copy(&mut reader.as_mut().take(*base_len), out)?;
                out.write_all(tail)?;
            }
        }
        Ok(())
    }
}

/// Writes `write`'s output to `path` through a temporary file, so a failed save never leaves a
/// half-written file. For Billy's own folders and exports; originals go through fs_guard.
fn write_atomic(path: &Path, write: impl FnOnce(&mut io::BufWriter<File>) -> Result<()>) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    let result = (|| -> Result<()> {
        let mut out = io::BufWriter::new(File::create(&tmp)?);
        write(&mut out)?;
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
    write_atomic(path, |out| payload.write_to(conn, file_id, out))
}

/// True when `bytes` are exactly the first bytes of the file at `path`.
fn file_starts_with(path: &Path, bytes: &[u8]) -> io::Result<bool> {
    let mut file = fs_guard::open_read(path)?;
    if file.metadata()?.len() < bytes.len() as u64 {
        return Ok(false);
    }
    let mut buf = vec![0u8; 1024 * 1024];
    let mut done = 0;
    while done < bytes.len() {
        let n = (bytes.len() - done).min(buf.len());
        file.read_exact(&mut buf[..n])?;
        if buf[..n] != bytes[done..done + n] {
            return Ok(false);
        }
        done += n;
    }
    Ok(true)
}

/// How a save will be stored.
enum NewStore {
    /// The first `base_len` bytes of the original, followed by `bytes`.
    Delta { base_len: u64, bytes: Vec<u8> },
    /// A full copy, written from the payload.
    Copy,
}

/// Saves the document's annotations. `from_original` says whether the reader had the original
/// open (so the annotations now match the current original) or the earlier annotated version
/// (which keeps its record of what it was based on).
pub fn save_annotated(conn: &Connection, dirs: &Dirs, file_id: i64, payload: &Payload, from_original: bool) -> Result<()> {
    let original = original_path(conn, file_id)?;
    let current = stored(conn, file_id)?;
    let current_delta_base = current.as_ref().and_then(|s| s.base_len);

    let plan = match payload {
        Payload::Append { source, base_len, probe, tail } => {
            let (mut reader, len) = open_version(conn, file_id, *source)?;
            verify(reader.as_mut(), len, *base_len, probe)?;
            match (source, current_delta_base) {
                (Source::Original, _) => NewStore::Delta { base_len: *base_len, bytes: tail.clone() },
                (Source::Annotated, Some(old_base)) if *base_len >= old_base => {
                    let mut bytes = std::fs::read(&current.as_ref().unwrap().path)?;
                    let keep = (*base_len - old_base) as usize;
                    if keep > bytes.len() {
                        return Err(Error::msg("The saved annotations changed while the document was open."));
                    }
                    bytes.truncate(keep);
                    bytes.extend_from_slice(tail);
                    NewStore::Delta { base_len: old_base, bytes }
                }
                _ => NewStore::Copy,
            }
        }
        Payload::Full(bytes) => {
            if !bytes.starts_with(b"%PDF-") {
                return Err(Error::msg("The reader did not produce a PDF."));
            }
            let original_len = fs_guard::metadata(&original)?.len();
            let mut plan = NewStore::Copy;
            for base_len in current_delta_base.into_iter().chain([original_len]) {
                let n = base_len as usize;
                if n < bytes.len() && file_starts_with(&original, &bytes[..n])? {
                    plan = NewStore::Delta { base_len, bytes: bytes[n..].to_vec() };
                    break;
                }
            }
            plan
        }
    };

    let current_fp: String = conn.query_row("SELECT fingerprint FROM files WHERE id = ?1", [file_id], |r| r.get(0))?;
    let (path, base_len, base_check) = match plan {
        NewStore::Delta { base_len, bytes } => {
            let path = dirs.delta_path(file_id);
            write_atomic(&path, |out| Ok(out.write_all(&bytes)?))?;
            (path, Some(base_len as i64), Some(prefix_check(&original, base_len)?))
        }
        NewStore::Copy => {
            let path = dirs.copy_path(file_id);
            write_atomic(&path, |out| payload.write_to(conn, file_id, out))?;
            (path, None, None)
        }
    };
    // Only one kind of record is kept; remove the other kind's file.
    for other in [dirs.delta_path(file_id), dirs.copy_path(file_id)] {
        if other != path {
            let _ = std::fs::remove_file(other);
        }
    }
    conn.execute(
        "INSERT INTO annotations (file_id, copy_path, source_fingerprint, updated_at, base_len, base_check)
         VALUES (?1, ?2, ?3, ?4, ?6, ?7)
         ON CONFLICT (file_id) DO UPDATE SET
             copy_path = excluded.copy_path,
             updated_at = excluded.updated_at,
             base_len = excluded.base_len,
             base_check = excluded.base_check,
             source_fingerprint = CASE WHEN ?5 THEN excluded.source_fingerprint ELSE annotations.source_fingerprint END",
        params![file_id, path.to_string_lossy(), current_fp, now_ms(), from_original, base_len, base_check],
    )?;
    Ok(())
}

/// Deletes the saved annotations and their record. Only touches Billy's own folders.
pub fn discard_annotated(conn: &Connection, dirs: &Dirs, file_id: i64) -> Result<()> {
    let path = stored(conn, file_id)?.map(|s| s.path);
    conn.execute("DELETE FROM annotations WHERE file_id = ?1", [file_id])?;
    for file in path.into_iter().chain([dirs.delta_path(file_id), dirs.copy_path(file_id)]) {
        if dirs.owns(&file) {
            let _ = std::fs::remove_file(file);
        }
    }
    Ok(())
}

/// Writes the annotated document into the original file, after the user confirmed it. The
/// saved annotations are then part of the original and are removed. The file's record is
/// updated, so a sync sees no change and the cover is made again.
pub fn save_to_original(conn: &Connection, dirs: &Dirs, file_id: i64, payload: &Payload) -> Result<()> {
    let path = original_path(conn, file_id)?;
    let mut failure: Option<Error> = None;
    fs_guard::write_original(&path, |out| {
        payload.write_to(conn, file_id, out).map_err(|e| {
            let message = e.to_string();
            failure = Some(e);
            io::Error::other(message)
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
    discard_annotated(conn, dirs, file_id)
}

pub fn set_last_page(conn: &Connection, file_id: i64, page: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO file_state (file_id, last_page) VALUES (?1, ?2)
         ON CONFLICT (file_id) DO UPDATE SET last_page = excluded.last_page",
        params![file_id, page],
    )?;
    Ok(())
}

/// Notes that a document was opened now, for the history.
pub fn record_opened(conn: &Connection, file_id: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO file_state (file_id, last_opened_at) VALUES (?1, ?2)
         ON CONFLICT (file_id) DO UPDATE SET last_opened_at = excluded.last_opened_at",
        params![file_id, now_ms()],
    )?;
    Ok(())
}

/// Forgets when every document was opened. Last pages are kept.
pub fn clear_history(conn: &Connection) -> Result<()> {
    conn.execute("UPDATE file_state SET last_opened_at = NULL WHERE last_opened_at IS NOT NULL", [])?;
    Ok(())
}
