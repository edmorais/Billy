//! A quick content fingerprint used to recognize a file after it moves or is renamed.
//!
//! Hashing whole books would be slow, so the fingerprint combines the file size with an
//! xxHash3 hash of the first and last 64 KB. Two different PDFs with the same size and the
//! same first and last 64 KB are very unlikely.

use crate::fs_guard;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use xxhash_rust::xxh3::Xxh3;

const CHUNK: u64 = 64 * 1024;

pub fn fingerprint(path: &Path) -> io::Result<String> {
    let mut file = fs_guard::open_read(path)?;
    let size = file.metadata()?.len();

    let mut hasher = Xxh3::new();
    hasher.update(&size.to_le_bytes());

    let mut buf = vec![0u8; CHUNK as usize];
    let n = read_up_to(&mut file, &mut buf)?;
    hasher.update(&buf[..n]);

    if size > CHUNK {
        // Start the tail after the head so small files are not hashed twice.
        let tail_start = size.saturating_sub(CHUNK).max(CHUNK);
        file.seek(SeekFrom::Start(tail_start))?;
        let n = read_up_to(&mut file, &mut buf)?;
        hasher.update(&buf[..n]);
    }

    Ok(format!("{size:x}-{:016x}", hasher.digest()))
}

/// Reads until `buf` is full or the file ends.
fn read_up_to(file: &mut impl Read, buf: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match file.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(filled)
}
