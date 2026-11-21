//! Tests for scanning, including the guarantee that Billy never changes a library folder.

use billy_lib::db::{self, libraries};
use billy_lib::scan::{fingerprint::fingerprint, scan_root};
use rusqlite::Connection;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tempfile::TempDir;

/// Writes a small fake PDF. The scanner does not parse PDFs, so the content only needs to differ.
fn write_pdf(path: &Path, label: &str, size: usize) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut bytes = format!("%PDF-1.7\n% {label}\n").into_bytes();
    bytes.resize(size.max(bytes.len()), b'x');
    bytes.extend_from_slice(format!("\n% end {label}\n%%EOF\n").as_bytes());
    fs::write(path, bytes).unwrap();
}

/// Builds this tree under `dir`:
///
/// ```text
/// Books/a.pdf
/// Books/notes.txt            not a PDF
/// Books/~$lock.pdf           Office lock file, ignored
/// Books/.cache/hidden.pdf    dot folder, ignored
/// Books/Sub/b.PDF            upper-case extension
/// Books/Sub/Deep/big.pdf     larger than the fingerprint chunks
/// ```
fn make_tree(dir: &Path) -> PathBuf {
    let books = dir.join("Books");
    write_pdf(&books.join("a.pdf"), "a", 1_000);
    fs::write(books.join("notes.txt"), "not a pdf").unwrap();
    write_pdf(&books.join("~$lock.pdf"), "lock", 100);
    write_pdf(&books.join(".cache").join("hidden.pdf"), "hidden", 100);
    write_pdf(&books.join("Sub").join("b.PDF"), "b", 5_000);
    write_pdf(&books.join("Sub").join("Deep").join("big.pdf"), "big", 300_000);
    billy_lib::fs_guard::canonical_dir(&books).unwrap()
}

/// Everything about a folder tree that a write could change: each item's kind, size,
/// modification time and full content hash.
///
/// Metadata is read from each item itself. On Windows, the copy in the parent folder's listing
/// (which `DirEntry::metadata` returns) is updated lazily and can show stale folder times.
fn snapshot(dir: &Path) -> BTreeMap<PathBuf, (bool, u64, u128, u64)> {
    let mut map = BTreeMap::new();
    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry.unwrap();
        let meta = fs::metadata(entry.path()).unwrap();
        let mtime = meta.modified().unwrap().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let hash = if meta.is_file() {
            xxhash_rust::xxh3::xxh3_64(&fs::read(entry.path()).unwrap())
        } else {
            0
        };
        map.insert(entry.path().to_path_buf(), (meta.is_dir(), meta.len(), mtime, hash));
    }
    map
}

struct Setup {
    _dirs: (TempDir, TempDir),
    conn: Connection,
    books: PathBuf,
}

/// A library tree and a database in two separate temporary folders.
fn setup() -> Setup {
    let tree = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();
    let books = make_tree(tree.path());
    let conn = db::open(&data.path().join("billy.db")).unwrap();
    Setup { _dirs: (tree, data), conn, books }
}

fn add_and_scan(conn: &mut Connection, library_id: i64, path: &Path) -> i64 {
    let root = libraries::insert_root(conn, library_id, &path.to_string_lossy()).unwrap();
    scan_root(conn, root.id, &mut |_| {}).unwrap();
    root.id
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).unwrap()
}

#[test]
fn scanning_never_changes_the_library_folder() {
    let mut s = setup();
    let before = snapshot(&s.books);

    let library = libraries::create_library(&s.conn, "Books").unwrap();
    let root_id = add_and_scan(&mut s.conn, library.id, &s.books);
    scan_root(&mut s.conn, root_id, &mut |_| {}).unwrap();
    let sub = add_and_scan(&mut s.conn, library.id, &s.books.join("Sub"));
    let removed = libraries::remove_root(&s.conn, sub).unwrap();
    for other in removed.overlapping_roots {
        scan_root(&mut s.conn, other, &mut |_| {}).unwrap();
    }
    libraries::purge_orphan_state(&s.conn, library.id).unwrap();

    assert_eq!(before, snapshot(&s.books), "the library folder changed");
}

#[test]
fn scan_imports_pdfs_and_skips_ignored_items() {
    let mut s = setup();
    let library = libraries::create_library(&s.conn, "Books").unwrap();
    let root = libraries::insert_root(&s.conn, library.id, &s.books.to_string_lossy()).unwrap();

    let report = scan_root(&mut s.conn, root.id, &mut |_| {}).unwrap();

    assert_eq!(report.files, 3);
    assert_eq!(report.new_files, 3);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert_eq!(libraries::get_root(&s.conn, root.id).unwrap().file_count, 3);

    let mut folders: Vec<String> = s
        .conn
        .prepare("SELECT rel_path FROM folders ORDER BY rel_path")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    folders.sort();
    let expected_deep = Path::new("Sub").join("Deep").to_string_lossy().to_string();
    assert_eq!(folders, vec![String::new(), "Sub".to_string(), expected_deep]);

    let big_in_deep: i64 = s
        .conn
        .query_row(
            "SELECT COUNT(*) FROM entries e JOIN folders f ON f.id = e.folder_id
             JOIN files x ON x.id = e.file_id
             WHERE f.name = 'Deep' AND x.path LIKE '%big.pdf'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(big_in_deep, 1);
}

#[test]
fn second_scan_adds_nothing_new() {
    let mut s = setup();
    let library = libraries::create_library(&s.conn, "Books").unwrap();
    let root_id = add_and_scan(&mut s.conn, library.id, &s.books);

    let report = scan_root(&mut s.conn, root_id, &mut |_| {}).unwrap();

    assert_eq!(report.files, 3);
    assert_eq!(report.new_files, 0);
    assert_eq!(count(&s.conn, "SELECT COUNT(*) FROM files"), 3);
    assert_eq!(count(&s.conn, "SELECT COUNT(*) FROM entries"), 3);
}

#[test]
fn new_pdf_is_picked_up_by_next_scan() {
    let mut s = setup();
    let library = libraries::create_library(&s.conn, "Books").unwrap();
    let root_id = add_and_scan(&mut s.conn, library.id, &s.books);

    write_pdf(&s.books.join("Sub").join("c.pdf"), "c", 2_000);
    let report = scan_root(&mut s.conn, root_id, &mut |_| {}).unwrap();

    assert_eq!(report.files, 4);
    assert_eq!(report.new_files, 1);
}

#[test]
fn overlapping_roots_do_not_duplicate_documents() {
    let mut s = setup();
    let library = libraries::create_library(&s.conn, "Books").unwrap();
    let outer = add_and_scan(&mut s.conn, library.id, &s.books);
    add_and_scan(&mut s.conn, library.id, &s.books.join("Sub"));
    assert_eq!(libraries::get_library(&s.conn, library.id).unwrap().document_count, 3);

    // Removing the outer root keeps the documents that the inner root also covers.
    let removed = libraries::remove_root(&s.conn, outer).unwrap();
    assert_eq!(removed.overlapping_roots.len(), 1);
    for other in removed.overlapping_roots {
        scan_root(&mut s.conn, other, &mut |_| {}).unwrap();
    }
    assert_eq!(libraries::get_library(&s.conn, library.id).unwrap().document_count, 2);
}

#[test]
fn libraries_share_file_records() {
    let mut s = setup();
    let one = libraries::create_library(&s.conn, "One").unwrap();
    let two = libraries::create_library(&s.conn, "Two").unwrap();
    add_and_scan(&mut s.conn, one.id, &s.books);
    add_and_scan(&mut s.conn, two.id, &s.books);

    assert_eq!(count(&s.conn, "SELECT COUNT(*) FROM files"), 3);
    assert_eq!(count(&s.conn, "SELECT COUNT(*) FROM entries"), 6);
}

#[test]
fn same_folder_cannot_be_added_twice_to_a_library() {
    let s = setup();
    let library = libraries::create_library(&s.conn, "Books").unwrap();
    let path = s.books.to_string_lossy().to_string();
    libraries::insert_root(&s.conn, library.id, &path).unwrap();
    assert!(libraries::insert_root(&s.conn, library.id, &path.to_uppercase()).is_err());
}

#[test]
fn removing_a_root_keeps_state_unless_purged() {
    let mut s = setup();
    let library = libraries::create_library(&s.conn, "Books").unwrap();
    let root_id = add_and_scan(&mut s.conn, library.id, &s.books);
    s.conn
        .execute(
            "INSERT INTO favorites (library_id, file_id, pinned_at) SELECT ?1, id, 0 FROM files",
            [library.id],
        )
        .unwrap();

    libraries::remove_root(&s.conn, root_id).unwrap();
    assert_eq!(count(&s.conn, "SELECT COUNT(*) FROM favorites"), 3);

    libraries::purge_orphan_state(&s.conn, library.id).unwrap();
    assert_eq!(count(&s.conn, "SELECT COUNT(*) FROM favorites"), 0);
    assert_eq!(count(&s.conn, "SELECT COUNT(*) FROM files"), 3, "file records are kept");
}

#[test]
fn deleting_a_library_removes_its_records() {
    let mut s = setup();
    let library = libraries::create_library(&s.conn, "Books").unwrap();
    add_and_scan(&mut s.conn, library.id, &s.books);

    libraries::delete_library(&s.conn, library.id).unwrap();

    assert_eq!(count(&s.conn, "SELECT COUNT(*) FROM roots"), 0);
    assert_eq!(count(&s.conn, "SELECT COUNT(*) FROM folders"), 0);
    assert_eq!(count(&s.conn, "SELECT COUNT(*) FROM entries"), 0);
}

#[test]
fn missing_root_is_marked_offline() {
    let mut s = setup();
    let library = libraries::create_library(&s.conn, "Books").unwrap();
    let gone = s.books.join("Gone");
    let root = libraries::insert_root(&s.conn, library.id, &gone.to_string_lossy()).unwrap();

    let report = scan_root(&mut s.conn, root.id, &mut |_| {}).unwrap();

    assert!(report.offline);
    assert_eq!(libraries::get_root(&s.conn, root.id).unwrap().status, "offline");
}

#[test]
fn fingerprint_follows_content_not_name() {
    let dir = TempDir::new().unwrap();
    let a = dir.path().join("a.pdf");
    let moved = dir.path().join("elsewhere").join("renamed.pdf");
    let other = dir.path().join("other.pdf");
    write_pdf(&a, "same", 300_000);
    write_pdf(&moved, "same", 300_000);
    write_pdf(&other, "different", 300_000);

    assert_eq!(fingerprint(&a).unwrap(), fingerprint(&moved).unwrap());
    assert_ne!(fingerprint(&a).unwrap(), fingerprint(&other).unwrap());
}

/// Times a first scan and a rescan of 5,000 PDFs in 50 folders.
/// Run with `cargo test --release -- --ignored --nocapture scan_speed`.
#[test]
#[ignore]
fn scan_speed() {
    let tree = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();
    for i in 0..5_000 {
        write_pdf(&tree.path().join(format!("shelf{:02}", i % 50)).join(format!("book{i}.pdf")), &i.to_string(), 150_000);
    }
    let mut conn = db::open(&data.path().join("billy.db")).unwrap();
    let library = libraries::create_library(&conn, "Speed").unwrap();
    let root = libraries::insert_root(&conn, library.id, &tree.path().to_string_lossy()).unwrap();

    let start = std::time::Instant::now();
    let first = scan_root(&mut conn, root.id, &mut |_| {}).unwrap();
    let first_time = start.elapsed();
    let start = std::time::Instant::now();
    scan_root(&mut conn, root.id, &mut |_| {}).unwrap();
    let rescan_time = start.elapsed();

    assert_eq!(first.files, 5_000);
    println!("first scan: {first_time:?}, rescan: {rescan_time:?}");
}

#[cfg(windows)]
#[test]
fn hidden_folders_are_skipped() {
    let mut s = setup();
    let secret = s.books.join("Secret");
    write_pdf(&secret.join("s.pdf"), "s", 100);
    let status = std::process::Command::new("attrib").arg("+h").arg(&secret).status().unwrap();
    assert!(status.success());

    let library = libraries::create_library(&s.conn, "Books").unwrap();
    let root = libraries::insert_root(&s.conn, library.id, &s.books.to_string_lossy()).unwrap();
    let report = scan_root(&mut s.conn, root.id, &mut |_| {}).unwrap();

    assert_eq!(report.files, 3);
}
