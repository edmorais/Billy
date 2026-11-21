//! Tests for the reader: reading ranges, annotated copies, and saving to the original.

use billy_lib::db::{self, browse, libraries};
use billy_lib::reader::{self, Payload, Source};
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
    _dirs: (TempDir, TempDir),
    conn: Connection,
    data: PathBuf,
    book: PathBuf,
    library: i64,
    root: i64,
    file: i64,
}

fn setup() -> Setup {
    let tree = TempDir::new().unwrap();
    let data_dir = TempDir::new().unwrap();
    let books = billy_lib::fs_guard::canonical_dir(tree.path()).unwrap().join("Books");
    fs::create_dir_all(&books).unwrap();
    let book = books.join("book.pdf");
    fs::write(&book, pdf_bytes("original", 300_000)).unwrap();
    let data = data_dir.path().to_path_buf();
    let mut conn = db::open(&data.join("billy.db")).unwrap();
    let library = libraries::create_library(&conn, "L").unwrap().id;
    let root = libraries::insert_root(&conn, library, &books.to_string_lossy()).unwrap().id;
    scan_root(&mut conn, root, &mut |_| {}).unwrap();
    let file = browse::list_documents(&conn, library, None, true, false).unwrap()[0].file_id;
    Setup { _dirs: (tree, data_dir), conn, data, book, library, root, file }
}

fn annotated(s: &Setup) -> bool {
    browse::list_documents(&s.conn, s.library, None, true, false).unwrap()[0].annotated
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
    // Past the end gives what is there.
    let tail = reader::read_range(&s.conn, s.file, Source::Original, whole.len() as u64 - 10, 1000).unwrap();
    assert_eq!(tail, whole[whole.len() - 10..]);
    assert!(reader::read_range(&s.conn, s.file, Source::Annotated, 0, 10).is_err());
}

#[test]
fn annotations_go_to_a_copy_in_billys_folder() {
    let s = setup();
    let before = fs::read(&s.book).unwrap();
    let book_mtime = fs::metadata(&s.book).unwrap().modified().unwrap();
    let copy = pdf_bytes("annotated", 1_000);

    reader::save_annotated(&s.conn, &s.data, s.file, &Payload::Full(copy.to_vec()), true).unwrap();

    assert_eq!(fs::read(&s.book).unwrap(), before, "the original is untouched");
    assert_eq!(fs::metadata(&s.book).unwrap().modified().unwrap(), book_mtime);
    let copy_file = reader::copy_path(&s.data, s.file);
    assert!(copy_file.starts_with(&s.data));
    assert_eq!(fs::read(&copy_file).unwrap(), copy);
    assert!(annotated(&s));
    let info = reader::open_info(&s.conn, s.file).unwrap();
    assert_eq!(info.annotated_size, Some(copy.len() as i64));
    assert!(!info.original_changed);
    assert_eq!(reader::read_range(&s.conn, s.file, Source::Annotated, 0, 20).unwrap(), copy[..20]);

    reader::discard_annotated(&s.conn, &s.data, s.file).unwrap();
    assert!(!copy_file.exists());
    assert!(!annotated(&s));
}

#[test]
fn notices_when_the_original_changes_after_annotating() {
    let mut s = setup();
    reader::save_annotated(&s.conn, &s.data, s.file, &Payload::Full(pdf_bytes("annotated", 1_000)), true).unwrap();

    // Someone edits the original in another program.
    fs::write(&s.book, pdf_bytes("edited elsewhere", 310_000)).unwrap();
    scan_root(&mut s.conn, s.root, &mut |_| {}).unwrap();
    assert!(reader::open_info(&s.conn, s.file).unwrap().original_changed);

    // Saving the old annotated copy again keeps the warning.
    reader::save_annotated(&s.conn, &s.data, s.file, &Payload::Full(pdf_bytes("annotated 2", 1_000)), false).unwrap();
    assert!(reader::open_info(&s.conn, s.file).unwrap().original_changed);

    // Annotating the new original clears it.
    reader::save_annotated(&s.conn, &s.data, s.file, &Payload::Full(pdf_bytes("annotated 3", 1_000)), true).unwrap();
    assert!(!reader::open_info(&s.conn, s.file).unwrap().original_changed);
}

#[test]
fn save_to_original_replaces_the_file_and_drops_the_copy() {
    let mut s = setup();
    reader::save_annotated(&s.conn, &s.data, s.file, &Payload::Full(pdf_bytes("annotated", 1_000)), true).unwrap();
    let new_bytes = pdf_bytes("annotated into original", 320_000);

    reader::save_to_original(&s.conn, &s.data, s.file, &Payload::Full(new_bytes.clone())).unwrap();

    assert_eq!(fs::read(&s.book).unwrap(), new_bytes);
    assert!(!annotated(&s));
    assert!(!reader::copy_path(&s.data, s.file).exists());
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
fn refuses_to_write_something_that_is_not_a_pdf() {
    let s = setup();
    let before = fs::read(&s.book).unwrap();

    assert!(reader::save_to_original(&s.conn, &s.data, s.file, &Payload::Full(b"<html>oops</html>".to_vec())).is_err());
    assert!(reader::save_annotated(&s.conn, &s.data, s.file, &Payload::Full(b"not a pdf".to_vec()), true).is_err());

    assert_eq!(fs::read(&s.book).unwrap(), before);
    assert!(!reader::copy_path(&s.data, s.file).exists());
}

#[test]
fn appended_saves_copy_the_rest_from_disk() {
    let s = setup();
    let original = fs::read(&s.book).unwrap();
    let base_len = original.len() as u64;
    let tail = b"\n% new objects and xref\n%%EOF\n".to_vec();
    let probe = original[original.len() - 4096..].to_vec();
    let payload = Payload::Append { source: Source::Original, base_len, probe, tail: tail.clone() };

    reader::save_annotated(&s.conn, &s.data, s.file, &payload, true).unwrap();
    let mut expected = original.clone();
    expected.extend_from_slice(&tail);
    assert_eq!(fs::read(reader::copy_path(&s.data, s.file)).unwrap(), expected);
    assert_eq!(fs::read(&s.book).unwrap(), original, "the original is untouched");

    // Appending again to the annotated copy builds on the copy.
    let copy_len = expected.len() as u64;
    let more = b"% second save\n".to_vec();
    let probe = expected[expected.len() - 100..].to_vec();
    let payload = Payload::Append { source: Source::Annotated, base_len: copy_len, probe, tail: more.clone() };
    reader::save_annotated(&s.conn, &s.data, s.file, &payload, false).unwrap();
    expected.extend_from_slice(&more);
    assert_eq!(fs::read(reader::copy_path(&s.data, s.file)).unwrap(), expected);

    // Into the original, after confirmation.
    let probe = original[original.len() - 4096..].to_vec();
    let payload = Payload::Append { source: Source::Original, base_len, probe, tail: tail.clone() };
    reader::save_to_original(&s.conn, &s.data, s.file, &payload).unwrap();
    let mut into_original = original.clone();
    into_original.extend_from_slice(&tail);
    assert_eq!(fs::read(&s.book).unwrap(), into_original);
}

#[test]
fn appended_save_refuses_a_file_that_changed() {
    let s = setup();
    let original = fs::read(&s.book).unwrap();
    let mut wrong_probe = original[original.len() - 4096..].to_vec();
    wrong_probe[0] ^= 0xff;
    let payload = Payload::Append {
        source: Source::Original,
        base_len: original.len() as u64,
        probe: wrong_probe,
        tail: b"% tail\n".to_vec(),
    };

    assert!(reader::save_annotated(&s.conn, &s.data, s.file, &payload, true).is_err());
    assert!(reader::save_to_original(&s.conn, &s.data, s.file, &payload).is_err());
    assert_eq!(fs::read(&s.book).unwrap(), original);
    assert!(!reader::copy_path(&s.data, s.file).exists());
    let folder = s.book.parent().unwrap();
    assert_eq!(fs::read_dir(folder).unwrap().count(), 1, "no temporary file is left behind");
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
