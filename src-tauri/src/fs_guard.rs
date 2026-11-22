//! Read-only access to library folders.
//!
//! Billy must never change anything inside a folder that belongs to a library. Every access
//! to library folders goes through this module, and nothing here can write, create, rename or
//! delete. Other modules must not call `std::fs` on library paths directly.
//!
//! The one exception is [`write_original`], used only by "Save to original file" in the reader
//! after the user confirms it.

use std::fs::{File, Metadata, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use walkdir::{DirEntry, WalkDir};

/// Opens a file for reading only. Other programs can still read, write, rename or delete the
/// file while Billy has it open, so Billy never locks your files.
pub fn open_read(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ: u32 = 0x1;
        const FILE_SHARE_WRITE: u32 = 0x2;
        const FILE_SHARE_DELETE: u32 = 0x4;
        options.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE);
    }
    options.open(path)
}

pub fn metadata(path: &Path) -> io::Result<Metadata> {
    std::fs::metadata(path)
}

pub fn is_dir(path: &Path) -> bool {
    path.is_dir()
}

/// Resolves a folder path to its canonical form, without the `\\?\` prefix.
pub fn canonical_dir(path: &Path) -> io::Result<PathBuf> {
    let canonical = dunce::canonicalize(path)?;
    if !canonical.is_dir() {
        return Err(io::Error::new(io::ErrorKind::NotADirectory, "not a folder"));
    }
    Ok(canonical)
}

/// Walks a folder tree, skipping hidden and system items, names that start with `.`, and
/// Office lock files (`~$…`). Symbolic links and junctions are not followed. Subfolders for
/// which `skip_dir` returns true are left out, with everything inside them.
pub fn walk(
    root: &Path,
    mut skip_dir: impl FnMut(&Path) -> bool,
) -> impl Iterator<Item = walkdir::Result<DirEntry>> {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(move |entry| {
            entry.depth() == 0
                || !(is_ignored(entry) || (entry.file_type().is_dir() && skip_dir(entry.path())))
        })
}

fn is_ignored(entry: &DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    if name.starts_with('.') || name.starts_with("~$") {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;
        if let Ok(meta) = entry.metadata() {
            if meta.file_attributes() & (FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM) != 0 {
                return true;
            }
        }
    }
    false
}

/// THE ONLY FUNCTION THAT WRITES TO A LIBRARY FOLDER. Replaces a PDF with new contents, for
/// "Save to original file" in the reader, after the user has confirmed it.
///
/// The new contents, produced by `write`, go to a temporary `~$billy-…` file next to the
/// original first (the scanner ignores `~$` names). Only if that file is complete and starts
/// like a PDF does it replace the original, in one step. So a failed or wrong write never
/// leaves the original damaged.
pub fn write_original(path: &Path, write: impl FnOnce(&mut io::BufWriter<File>) -> io::Result<()>) -> io::Result<()> {
    if !path.is_file() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "the original file is gone"));
    }
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let tmp = path.with_file_name(format!("~$billy-{name}.tmp"));
    let result = (|| {
        let mut out = io::BufWriter::new(File::create(&tmp)?);
        write(&mut out)?;
        io::Write::flush(&mut out)?;
        drop(out);
        let mut head = [0u8; 5];
        io::Read::read_exact(&mut File::open(&tmp)?, &mut head)?;
        if &head != b"%PDF-" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "refusing to write something that is not a PDF"));
        }
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

pub fn is_pdf(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
}

/// Modification time in milliseconds since the Unix epoch.
pub fn mtime_ms(meta: &Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
