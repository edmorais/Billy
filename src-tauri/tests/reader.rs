//! Tests for the reader: reading ranges, stored annotations, and saving to the original.

use billy_lib::db::{self, browse, libraries};
use billy_lib::reader::{self, Dirs, Payload, Source};
use billy_lib::scan::scan_root;
use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn pdf_bytes(label: &str, size: usize) -> Vec<u8> {
    let mut bytes = format!("%PDF-1.7\n% {label}\n").into_bytes();
    for i in 0..size {
        bytes.push(b'a' + (i % 26) as u8);
    }
    bytes.extend_from_slice(b"\n%%EOF\n");
    bytes
}

struct Setup {
    _dirs: (TempDir, TempDir, TempDir),
    conn: Connection,
    dirs: Dirs,
    book: PathBuf,
    library: i64,
    root: i64,
    file: i64,
}

fn setup() -> Setup {
    let tree = TempDir::new().unwrap();
    let roaming = TempDir::new().unwrap();
    let local = TempDir::new().unwrap();
    let books = billy_lib::fs_guard::canonical_dir(tree.path()).unwrap().join("Books");
    fs::create_dir_all(&books).unwrap();
    let book = books.join("book.pdf");
    fs::write(&book, pdf_bytes("original", 300_000)).unwrap();
    let dirs = Dirs { roaming: roaming.path().to_path_buf(), local: local.path().to_path_buf() };
    let mut conn = db::open(&dirs.roaming.join("billy.db")).unwrap();
    let library = libraries::create_library(&conn, "L").unwrap().id;
    let root = libraries::insert_root(&conn, library, &books.to_string_lossy()).unwrap().id;
    scan_root(&mut conn, root, &mut |_| {}).unwrap();
    let file = browse::list_documents(&conn, library, None, true, false).unwrap()[0].file_id;
    Setup { _dirs: (tree, roaming, local), conn, dirs, book, library, root, file }
}

fn annotated(s: &Setup) -> bool {
    browse::list_documents(&s.conn, s.library, None, true, false).unwrap()[0].annotated
}

/// What PDF.js sends when it appends `tail` to the open version of length `base_len`.
fn append(version: &[u8], source: Source, tail: &[u8]) -> Payload {
    Payload::Append {
        source,
        base_len: version.len() as u64,
        probe: version[version.len() - 4096..].to_vec(),
        tail: tail.to_vec(),
    }
}

fn read_all(s: &Setup, source: Source) -> Vec<u8> {
    reader::read_range(&s.conn, s.file, source, 0, u64::MAX).unwrap()
}

#[test]
fn reads_ranges_of_the_original() {
    let s = setup();
    let whole = fs::read(&s.book).unwrap();
    let info = reader::open_info(&s.conn, s.file).unwrap();
    assert_eq!(info.original_size as usize, whole.len());
    assert_eq!(info.annotated_size, None);

    assert_eq!(reader::read_range(&s.conn, s.file, Source::Original, 0, 100).unwrap(), whole[..100]);
    assert_eq!(reader::read_range(&s.conn, s.file, Source::Original, 1000, 5000).unwrap(), whole[1000..6000]);
    let tail = reader::read_range(&s.conn, s.file, Source::Original, whole.len() as u64 - 10, 1000).unwrap();
    assert_eq!(tail, whole[whole.len() - 10..]);
    assert!(reader::read_range(&s.conn, s.file, Source::Annotated, 0, 10).is_err());
}

#[test]
fn annotations_are_stored_as_the_appended_bytes_only() {
    let s = setup();
    let original = fs::read(&s.book).unwrap();
    let book_mtime = fs::metadata(&s.book).unwrap().modified().unwrap();
    let note = b"\n% a note, as PDF.js appends it\n%%EOF\n";

    reader::save_annotated(&s.conn, &s.dirs, s.file, &append(&original, Source::Original, note), true).unwrap();

    // Only the note is stored, in the roaming folder; no full copy anywhere.
    assert_eq!(fs::read(s.dirs.delta_path(s.file)).unwrap(), note);
    assert!(!s.dirs.copy_path(s.file).exists());
    assert_eq!(fs::read(&s.book).unwrap(), original, "the original is untouched");
    assert_eq!(fs::metadata(&s.book).unwrap().modified().unwrap(), book_mtime);
    assert!(annotated(&s));

    // The annotated version is the original followed by the note.
    let mut expected = original.clone();
    expected.extend_from_slice(note);
    let info = reader::open_info(&s.conn, s.file).unwrap();
    assert_eq!(info.annotated_size, Some(expected.len() as i64));
    assert!(!info.original_changed && !info.annotations_broken);
    assert_eq!(read_all(&s, Source::Annotated), expected);
    let middle = original.len() as u64 - 10;
    assert_eq!(reader::read_range(&s.conn, s.file, Source::Annotated, middle, 20).unwrap(), expected[middle as usize..middle as usize + 20]);

    reader::discard_annotated(&s.conn, &s.dirs, s.file).unwrap();
    assert!(!s.dirs.delta_path(s.file).exists());
    assert!(!annotated(&s));
}

#[test]
fn saving_again_from_the_annotated_version_extends_the_stored_bytes() {
    let s = setup();
    let original = fs::read(&s.book).unwrap();
    reader::save_annotated(&s.conn, &s.dirs, s.file, &append(&original, Source::Original, b"% first\n"), true).unwrap();
    let opened = read_all(&s, Source::Annotated);

    reader::save_annotated(&s.conn, &s.dirs, s.file, &append(&opened, Source::Annotated, b"% second\n"), false).unwrap();

    assert_eq!(fs::read(s.dirs.delta_path(s.file)).unwrap(), b"% first\n% second\n");
    let mut expected = opened.clone();
    expected.extend_from_slice(b"% second\n");
    assert_eq!(read_all(&s, Source::Annotated), expected);
}

#[test]
fn a_whole_document_that_extends_the_original_is_stored_as_its_extra_bytes() {
    let s = setup();
    let mut whole = fs::read(&s.book).unwrap();
    whole.extend_from_slice(b"% appended\n");

    reader::save_annotated(&s.conn, &s.dirs, s.file, &Payload::Full(whole.clone()), true).unwrap();

    assert_eq!(fs::read(s.dirs.delta_path(s.file)).unwrap(), b"% appended\n");
    assert_eq!(read_all(&s, Source::Annotated), whole);
}

#[test]
fn a_rewritten_document_is_kept_as_a_local_copy() {
    let s = setup();
    let rewritten = pdf_bytes("rewritten by the reader", 1_000);

    reader::save_annotated(&s.conn, &s.dirs, s.file, &Payload::Full(rewritten.clone()), true).unwrap();

    assert_eq!(fs::read(s.dirs.copy_path(s.file)).unwrap(), rewritten);
    assert!(s.dirs.copy_path(s.file).starts_with(&s.dirs.local), "full copies never roam");
    assert!(!s.dirs.delta_path(s.file).exists());
    assert_eq!(read_all(&s, Source::Annotated), rewritten);

    // A later appended save switches back to the small form.
    let original = fs::read(&s.book).unwrap();
    reader::save_annotated(&s.conn, &s.dirs, s.file, &append(&original, Source::Original, b"% note\n"), true).unwrap();
    assert!(!s.dirs.copy_path(s.file).exists());
    assert!(s.dirs.delta_path(s.file).exists());
}

#[test]
fn annotations_survive_an_original_that_was_appended_to() {
    let mut s = setup();
    let original = fs::read(&s.book).unwrap();
    reader::save_annotated(&s.conn, &s.dirs, s.file, &append(&original, Source::Original, b"% note\n"), true).unwrap();

    // Another program saves its own changes by appending to the file.
    let mut edited = original.clone();
    edited.extend_from_slice(b"% another program's update\n");
    fs::write(&s.book, &edited).unwrap();
    scan_root(&mut s.conn, s.root, &mut |_| {}).unwrap();

    let info = reader::open_info(&s.conn, s.file).unwrap();
    assert!(info.original_changed);
    assert!(!info.annotations_broken);
    let mut expected = original.clone();
    expected.extend_from_slice(b"% note\n");
    assert_eq!(read_all(&s, Source::Annotated), expected, "the annotated version is still the old one plus the note");
}

#[test]
fn a_replaced_original_makes_the_annotations_unshowable_but_keeps_them() {
    let mut s = setup();
    let original = fs::read(&s.book).unwrap();
    reader::save_annotated(&s.conn, &s.dirs, s.file, &append(&original, Source::Original, b"% note\n"), true).unwrap();

    fs::write(&s.book, pdf_bytes("a different edition", 290_000)).unwrap();
    scan_root(&mut s.conn, s.root, &mut |_| {}).unwrap();

    let info = reader::open_info(&s.conn, s.file).unwrap();
    assert!(info.annotations_broken);
    assert_eq!(info.annotated_size, None);
    assert!(reader::read_range(&s.conn, s.file, Source::Annotated, 0, 10).is_err());
    assert!(annotated(&s), "the annotations are kept until discarded");
    reader::discard_annotated(&s.conn, &s.dirs, s.file).unwrap();
    assert!(!annotated(&s));
}

#[test]
fn save_to_original_appends_the_annotations_and_drops_the_stored_ones() {
    let mut s = setup();
    let original = fs::read(&s.book).unwrap();
    reader::save_annotated(&s.conn, &s.dirs, s.file, &append(&original, Source::Original, b"% note\n"), true).unwrap();
    let opened = read_all(&s, Source::Annotated);

    reader::save_to_original(&s.conn, &s.dirs, s.file, &append(&opened, Source::Annotated, b"% more\n")).unwrap();

    let mut expected = opened.clone();
    expected.extend_from_slice(b"% more\n");
    assert_eq!(fs::read(&s.book).unwrap(), expected);
    assert!(!annotated(&s));
    assert!(!s.dirs.delta_path(s.file).exists());
    let folder = s.book.parent().unwrap();
    let names: Vec<_> = fs::read_dir(folder).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert_eq!(names, vec![std::ffi::OsString::from("book.pdf")], "no temporary file is left behind");

    // A sync sees the same document, unchanged.
    let report = scan_root(&mut s.conn, s.root, &mut |_| {}).unwrap();
    assert_eq!((report.new_files, report.moved, report.missing), (0, 0, 0));
    let docs = browse::list_documents(&s.conn, s.library, None, true, false).unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].file_id, s.file);
}

#[test]
fn refuses_bad_saves_and_leaves_everything_as_it_was() {
    let s = setup();
    let original = fs::read(&s.book).unwrap();
    let mut wrong_probe = append(&original, Source::Original, b"% tail\n");
    if let Payload::Append { probe, .. } = &mut wrong_probe {
        probe[0] ^= 0xff;
    }

    assert!(reader::save_to_original(&s.conn, &s.dirs, s.file, &Payload::Full(b"<html>oops</html>".to_vec())).is_err());
    assert!(reader::save_annotated(&s.conn, &s.dirs, s.file, &Payload::Full(b"not a pdf".to_vec()), true).is_err());
    assert!(reader::save_annotated(&s.conn, &s.dirs, s.file, &wrong_probe, true).is_err());
    assert!(reader::save_to_original(&s.conn, &s.dirs, s.file, &wrong_probe).is_err());

    assert_eq!(fs::read(&s.book).unwrap(), original);
    assert!(!annotated(&s));
    assert!(!s.dirs.delta_path(s.file).exists() && !s.dirs.copy_path(s.file).exists());
    assert_eq!(fs::read_dir(s.book.parent().unwrap()).unwrap().count(), 1, "no temporary file is left behind");
}

#[test]
fn exports_the_annotated_version() {
    let s = setup();
    let original = fs::read(&s.book).unwrap();
    reader::save_annotated(&s.conn, &s.dirs, s.file, &append(&original, Source::Original, b"% note\n"), true).unwrap();
    let size = reader::open_info(&s.conn, s.file).unwrap().annotated_size.unwrap() as u64;
    let out = s.dirs.local.join("exported.pdf");

    let whole = Payload::Append { source: Source::Annotated, base_len: size, probe: Vec::new(), tail: Vec::new() };
    reader::export(&s.conn, s.file, &whole, &out).unwrap();

    assert_eq!(fs::read(&out).unwrap(), read_all(&s, Source::Annotated));
}

#[test]
fn remembers_the_last_page() {
    let s = setup();
    assert_eq!(reader::open_info(&s.conn, s.file).unwrap().last_page, None);
    reader::set_last_page(&s.conn, s.file, 42).unwrap();
    assert_eq!(reader::open_info(&s.conn, s.file).unwrap().last_page, Some(42));
}

#[test]
fn missing_files_cannot_be_opened() {
    let mut s = setup();
    fs::remove_file(&s.book).unwrap();
    scan_root(&mut s.conn, s.root, &mut |_| {}).unwrap();
    assert!(reader::open_info(&s.conn, s.file).is_err());
}
