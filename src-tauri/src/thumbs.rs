//! Thumbnails and metadata, made with PDFium.
//!
//! A background worker renders the first page of each PDF to a JPEG in Billy's cache folder
//! (`%LOCALAPPDATA%\Billy\thumbs`) and reads the page count, title and author. Files are read
//! through [`crate::fs_guard`] only. Cache files are named by the file's fingerprint, so they
//! stay valid when a file is moved or renamed.

use crate::error::{Error, Result};
use crate::fs_guard;
use image::codecs::jpeg::JpegEncoder;
use pdfium_render::prelude::*;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Width of a thumbnail in pixels. Large enough for the large grid on a high-DPI screen.
pub const THUMB_WIDTH: i32 = 400;
const JPEG_QUALITY: u8 = 82;

static PDFIUM: OnceLock<Pdfium> = OnceLock::new();
static PDFIUM_INIT: Mutex<()> = Mutex::new(());

/// Loads `pdfium.dll` from `dir` the first time; later calls return the same instance.
pub fn pdfium(dir: &Path) -> Result<&'static Pdfium> {
    let _guard = PDFIUM_INIT.lock().unwrap();
    if let Some(pdfium) = PDFIUM.get() {
        return Ok(pdfium);
    }
    let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(dir))
        .map_err(|e| Error::msg(format!("Could not load PDFium from {}: {e}", dir.display())))?;
    Ok(PDFIUM.get_or_init(|| Pdfium::new(bindings)))
}

/// What Billy learns from a PDF besides its cover.
#[derive(Debug, Clone, PartialEq)]
pub struct PdfInfo {
    pub pages: i64,
    pub title: Option<String>,
    pub author: Option<String>,
}

/// Renders page 1 of `pdf` as a JPEG at `out` and returns the document's page count, title
/// and author. The JPEG is written to a temporary name first, so a half-written file never
/// appears in the cache.
pub fn render(pdfium: &Pdfium, pdf: &Path, out: &Path) -> Result<PdfInfo> {
    // PDFium makes many small reads; a buffer turns them into few large ones.
    let file = std::io::BufReader::with_capacity(256 * 1024, fs_guard::open_read(pdf)?);
    let pdf_err = |e: PdfiumError| Error::msg(format!("{}: {e}", pdf.display()));
    let document = pdfium.load_pdf_from_reader(file, None).map_err(pdf_err)?;

    let tag = |kind| {
        document
            .metadata()
            .get(kind)
            .map(|t| t.value().trim().to_string())
            .filter(|v| !v.is_empty())
    };
    let info = PdfInfo {
        pages: document.pages().len() as i64,
        title: tag(PdfDocumentMetadataTagType::Title),
        author: tag(PdfDocumentMetadataTagType::Author),
    };

    let page = document.pages().first().map_err(pdf_err)?;
    let config = PdfRenderConfig::new()
        .set_target_width(THUMB_WIDTH)
        .set_maximum_height(THUMB_WIDTH * 2);
    let image = page.render_with_config(&config).map_err(pdf_err)?.as_image().map_err(pdf_err)?.into_rgb8();

    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, JPEG_QUALITY)
        .encode_image(&image)
        .map_err(|e| Error::msg(format!("Could not encode thumbnail: {e}")))?;
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = out.with_extension("tmp");
    std::fs::write(&tmp, &jpeg)?;
    std::fs::rename(&tmp, out)?;
    Ok(info)
}

/// Where the thumbnail for a fingerprint lives in the cache.
pub fn thumb_path(thumbs_dir: &Path, fingerprint: &str) -> PathBuf {
    thumbs_dir.join(format!("{fingerprint}.jpg"))
}

/// A change to one document, sent to the UI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbUpdate {
    pub file_id: i64,
    pub status: String,
    pub pages: Option<i64>,
    pub title: Option<String>,
    pub author: Option<String>,
}

/// Returns the next file that needs a thumbnail: one that is pending, not missing, and in at
/// least one library.
pub fn next_pending(conn: &Connection) -> Result<Option<i64>> {
    Ok(conn
        .query_row(
            "SELECT f.id FROM files f
             WHERE f.thumb_status = 'pending' AND f.missing_since IS NULL
               AND EXISTS (SELECT 1 FROM entries e WHERE e.file_id = f.id)
             ORDER BY f.id LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()?)
}

pub fn remaining(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM files f
         WHERE f.thumb_status = 'pending' AND f.missing_since IS NULL
           AND EXISTS (SELECT 1 FROM entries e WHERE e.file_id = f.id)",
        [],
        |r| r.get(0),
    )?)
}

/// Makes the thumbnail and reads the metadata for one file, and records the result. Returns
/// `None` when the file no longer needs it (already done, or no longer known).
pub fn process(conn: &Connection, pdfium: &Pdfium, thumbs_dir: &Path, file_id: i64) -> Result<Option<ThumbUpdate>> {
    let row: Option<(String, String, String)> = conn
        .query_row(
            "SELECT path, fingerprint, thumb_status FROM files WHERE id = ?1",
            [file_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((path, fingerprint, status)) = row else { return Ok(None) };
    if status != "pending" {
        return Ok(None);
    }

    let update = match render(pdfium, Path::new(&path), &thumb_path(thumbs_dir, &fingerprint)) {
        Ok(info) => {
            conn.execute(
                "UPDATE files SET thumb_status = 'ok', page_count = ?2, title = ?3, author = ?4 WHERE id = ?1",
                params![file_id, info.pages, info.title, info.author],
            )?;
            ThumbUpdate { file_id, status: "ok".into(), pages: Some(info.pages), title: info.title, author: info.author }
        }
        Err(_) => {
            // Encrypted, damaged or unreadable. Billy shows a placeholder cover.
            conn.execute("UPDATE files SET thumb_status = 'error' WHERE id = ?1", [file_id])?;
            ThumbUpdate { file_id, status: "error".into(), pages: None, title: None, author: None }
        }
    };
    Ok(Some(update))
}

/// Deletes the covers in `thumbs_dir` that no file record uses any more, for example after a
/// library was deleted. Only files named like a cover (`<fingerprint>.jpg`) are touched.
/// Returns how many were deleted.
pub fn remove_unused_covers(conn: &Connection, thumbs_dir: &Path) -> Result<usize> {
    // List the folder before reading the records. A cover is written after its record, so a
    // cover in this listing whose record is still there afterwards is kept.
    let names: Vec<String> = match std::fs::read_dir(thumbs_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|name| name.strip_suffix(".jpg").is_some_and(is_fingerprint))
            .collect(),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(err) => return Err(err.into()),
    };
    let mut stmt = conn.prepare("SELECT fingerprint FROM files")?;
    let known: std::collections::HashSet<String> =
        stmt.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
    let mut removed = 0;
    for name in names {
        if !known.contains(&name[..name.len() - 4]) && std::fs::remove_file(thumbs_dir.join(&name)).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

/// A fingerprint as Billy writes them: hex digits and dashes.
fn is_fingerprint(name: &str) -> bool {
    !name.is_empty() && name.len() <= 64 && name.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
}

/// Work waiting for the thumbnail worker.
#[derive(Default)]
pub struct ThumbQueue {
    state: Mutex<QueueState>,
    signal: Condvar,
}

#[derive(Default)]
struct QueueState {
    /// Files on screen, most urgent last.
    urgent: Vec<i64>,
    woken: bool,
}

impl ThumbQueue {
    /// Puts these files first, in the given order. Replaces earlier requests, which are
    /// probably no longer on screen.
    pub fn prioritize(&self, file_ids: &[i64]) {
        let mut state = self.state.lock().unwrap();
        // A stack: the first requested file ends up last, so it is popped first.
        state.urgent = file_ids.iter().take(500).rev().copied().collect();
        state.woken = true;
        self.signal.notify_one();
    }

    /// Tells the worker that new pending files may exist, for example after a scan.
    pub fn wake(&self) {
        self.state.lock().unwrap().woken = true;
        self.signal.notify_one();
    }

    fn pop_urgent(&self) -> Option<i64> {
        self.state.lock().unwrap().urgent.pop()
    }

    fn wait(&self, timeout: Duration) {
        let mut state = self.state.lock().unwrap();
        if !state.woken {
            state = self.signal.wait_timeout(state, timeout).unwrap().0;
        }
        state.woken = false;
    }
}

/// Runs the thumbnail worker on its own thread. `emit` receives batches of updates and the
/// number of files still waiting.
pub fn spawn_worker(
    queue: std::sync::Arc<ThumbQueue>,
    db_path: PathBuf,
    thumbs_dir: PathBuf,
    pdfium_dir: PathBuf,
    emit: impl Fn(&[ThumbUpdate], i64) + Send + 'static,
    on_error: impl Fn(String) + Send + 'static,
) {
    std::thread::spawn(move || {
        let setup = (|| -> Result<(Connection, &'static Pdfium)> {
            Ok((crate::db::open(&db_path)?, pdfium(&pdfium_dir)?))
        })();
        let (conn, pdfium) = match setup {
            Ok(v) => v,
            Err(e) => return on_error(e.to_string()),
        };

        let mut batch: Vec<ThumbUpdate> = Vec::new();
        let mut last_emit = Instant::now();
        let flush = |batch: &mut Vec<ThumbUpdate>, last_emit: &mut Instant| {
            emit(batch, remaining(&conn).unwrap_or(0));
            batch.clear();
            *last_emit = Instant::now();
        };

        loop {
            let next = match queue.pop_urgent() {
                Some(id) => Some(id),
                None => next_pending(&conn).unwrap_or(None),
            };
            match next {
                Some(file_id) => {
                    if let Ok(Some(update)) = process(&conn, pdfium, &thumbs_dir, file_id) {
                        batch.push(update);
                    }
                    if batch.len() >= 24 || (!batch.is_empty() && last_emit.elapsed() > Duration::from_millis(250)) {
                        flush(&mut batch, &mut last_emit);
                    }
                }
                None => {
                    if !batch.is_empty() {
                        flush(&mut batch, &mut last_emit);
                    }
                    queue.wait(Duration::from_secs(60));
                }
            }
        }
    });
}

/// Answers a `thumb://` request. The path is a fingerprint; anything else is refused, so the
/// protocol can only ever serve files from the thumbnail cache.
pub fn serve(thumbs_dir: &Path, uri_path: &str) -> tauri::http::Response<Vec<u8>> {
    use tauri::http::{Response, StatusCode, header};
    let name = uri_path.trim_start_matches('/');
    let valid = is_fingerprint(name);
    let found = if valid { std::fs::read(thumb_path(thumbs_dir, name)).ok() } else { None };
    match found {
        Some(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "image/jpeg")
            .header(header::CACHE_CONTROL, "max-age=31536000, immutable")
            .body(bytes)
            .unwrap(),
        None => Response::builder().status(StatusCode::NOT_FOUND).body(Vec::new()).unwrap(),
    }
}
