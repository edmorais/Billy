//! Tests for syncing: new, moved, renamed, deleted and returning files, and offline folders.

use billy_lib::db::{self, browse, libraries, marks, missing};
use billy_lib::scan::{ScanReport, refresh_root_status, scan_root};
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn write_pdf(path: &Path, label: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut bytes = format!("%PDF-1.7\n% {label}\n").into_bytes();
    bytes.resize(5_000, b'x');
    bytes.extend_from_slice(format!("\n% end {label}\n%%EOF\n").as_bytes());
    fs::write(path, bytes).unwrap();
}

struct Setup {
    _dirs: (TempDir, TempDir),
    conn: Connection,
    top: PathBuf,
}

fn setup() -> Setup {
    let tree = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();
    let top = billy_lib::fs_guard::canonical_dir(tree.path()).unwrap();
    let conn = db::open(&data.path().join("billy.db")).unwrap();
    Setup { _dirs: (tree, data), conn, top }
}

fn library_with_root(conn: &mut Connection, name: &str, root: &Path) -> (i64, i64) {
    let library = libraries::create_library(conn, name).unwrap();
    let root = libraries::insert_root(conn, library.id, &root.to_string_lossy()).unwrap();
    scan_root(conn, root.id, &mut |_| {}).unwrap();
    (library.id, root.id)
}

fn sync(conn: &mut Connection, root_id: i64) -> ScanReport {
    scan_root(conn, root_id, &mut |_| {}).unwrap()
}

fn doc<'a>(docs: &'a [browse::Document], name: &str) -> &'a browse::Document {
    docs.iter().find(|d| d.name == name).unwrap_or_else(|| panic!("{name} not listed"))
}

fn docs(conn: &Connection, library: i64) -> Vec<browse::Document> {
    browse::list_documents(conn, library, None, true, false).unwrap()
}

#[test]
fn rename_keeps_the_same_document_and_its_state() {
    let mut s = setup();
    let books = s.top.join("Books");
    write_pdf(&books.join("old name.pdf"), "a");
    let (library, root) = library_with_root(&mut s.conn, "L", &books);
    let id = doc(&docs(&s.conn, library), "old name.pdf").file_id;
    marks::set_favorite(&mut s.conn, library, &[id], true).unwrap();
    marks::set_read(&mut s.conn, library, &[id], true).unwrap();

    fs::rename(books.join("old name.pdf"), books.join("new name.pdf")).unwrap();
    let report = sync(&mut s.conn, root);

    assert_eq!((report.moved, report.new_files, report.missing), (1, 0, 0));
    let list = docs(&s.conn, library);
    assert_eq!(list.len(), 1);
    let renamed = doc(&list, "new name.pdf");
    assert_eq!(renamed.file_id, id);
    assert!(renamed.favorite && renamed.read);
}

#[test]
fn move_to_another_folder_updates_the_folder() {
    let mut s = setup();
    let books = s.top.join("Books");
    write_pdf(&books.join("a.pdf"), "a");
    fs::create_dir_all(books.join("Shelf")).unwrap();
    let (library, root) = library_with_root(&mut s.conn, "L", &books);

    fs::rename(books.join("a.pdf"), books.join("Shelf").join("a.pdf")).unwrap();
    let report = sync(&mut s.conn, root);

    assert_eq!(report.moved, 1);
    let tree = browse::folder_tree(&s.conn, library).unwrap();
    let shelf = tree.iter().find(|n| n.name == "Shelf").unwrap();
    assert_eq!((shelf.count, shelf.total), (1, 1));
    let root_node = tree.iter().find(|n| n.parent_id.is_none()).unwrap();
    assert_eq!(root_node.count, 0);
}

#[test]
fn deleted_file_is_missing_until_it_returns() {
    let mut s = setup();
    let books = s.top.join("Books");
    write_pdf(&books.join("a.pdf"), "a");
    write_pdf(&books.join("b.pdf"), "b");
    let (library, root) = library_with_root(&mut s.conn, "L", &books);
    let a = doc(&docs(&s.conn, library), "a.pdf").file_id;
    marks::set_favorite(&mut s.conn, library, &[a], true).unwrap();
    let saved = fs::read(books.join("a.pdf")).unwrap();

    fs::remove_file(books.join("a.pdf")).unwrap();
    let report = sync(&mut s.conn, root);
    assert_eq!(report.missing, 1);
    assert_eq!(docs(&s.conn, library).len(), 1, "missing files are hidden");
    let with_missing = browse::list_documents_with(&s.conn, library, None, true, false, true).unwrap();
    assert!(doc(&with_missing, "a.pdf").missing);
    let listed = missing::list_missing(&s.conn, library).unwrap();
    assert_eq!(listed.len(), 1);
    assert!(listed[0].favorite);

    // A second sync does not count it again.
    assert_eq!(sync(&mut s.conn, root).missing, 0);

    // It comes back with its favorite intact.
    fs::write(books.join("a.pdf"), saved).unwrap();
    sync(&mut s.conn, root);
    let back = docs(&s.conn, library);
    assert_eq!(back.len(), 2);
    assert!(doc(&back, "a.pdf").favorite);
    assert!(missing::list_missing(&s.conn, library).unwrap().is_empty());
}

#[test]
fn move_between_roots_of_one_library_is_followed_whatever_the_sync_order() {
    let mut s = setup();
    let one = s.top.join("One");
    let two = s.top.join("Two");
    write_pdf(&one.join("a.pdf"), "a");
    fs::create_dir_all(&two).unwrap();
    let library = libraries::create_library(&s.conn, "L").unwrap().id;
    let r1 = libraries::insert_root(&s.conn, library, &one.to_string_lossy()).unwrap().id;
    let r2 = libraries::insert_root(&s.conn, library, &two.to_string_lossy()).unwrap().id;
    sync(&mut s.conn, r1);
    sync(&mut s.conn, r2);
    let id = doc(&docs(&s.conn, library), "a.pdf").file_id;
    marks::set_favorite(&mut s.conn, library, &[id], true).unwrap();

    fs::rename(one.join("a.pdf"), two.join("a.pdf")).unwrap();
    // The old folder is synced first, so the file is missing for a moment.
    assert_eq!(sync(&mut s.conn, r1).missing, 1);
    assert_eq!(sync(&mut s.conn, r2).moved, 1);

    let list = docs(&s.conn, library);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].file_id, id);
    assert!(list[0].favorite);
    assert!(missing::list_missing(&s.conn, library).unwrap().is_empty());
}

#[test]
fn moving_out_of_one_library_into_another() {
    let mut s = setup();
    let one = s.top.join("One");
    let two = s.top.join("Two");
    write_pdf(&one.join("a.pdf"), "a");
    fs::create_dir_all(&two).unwrap();
    let (lib1, r1) = library_with_root(&mut s.conn, "First", &one);
    let (lib2, r2) = library_with_root(&mut s.conn, "Second", &two);

    fs::rename(one.join("a.pdf"), two.join("a.pdf")).unwrap();
    sync(&mut s.conn, r2);

    assert_eq!(docs(&s.conn, lib2).len(), 1);
    assert_eq!(docs(&s.conn, lib1).len(), 0, "it left the first library");
    assert_eq!(sync(&mut s.conn, r1).missing, 0, "and is not missing there");
}

#[test]
fn a_copy_is_a_new_document() {
    let mut s = setup();
    let books = s.top.join("Books");
    write_pdf(&books.join("a.pdf"), "a");
    let (library, root) = library_with_root(&mut s.conn, "L", &books);

    fs::copy(books.join("a.pdf"), books.join("a copy.pdf")).unwrap();
    let report = sync(&mut s.conn, root);

    assert_eq!((report.new_files, report.moved), (1, 0));
    let list = docs(&s.conn, library);
    assert_ne!(doc(&list, "a.pdf").file_id, doc(&list, "a copy.pdf").file_id);
}

#[test]
fn offline_folder_changes_nothing() {
    let mut s = setup();
    let books = s.top.join("Books");
    write_pdf(&books.join("a.pdf"), "a");
    let (library, root) = library_with_root(&mut s.conn, "L", &books);

    // Like an unplugged drive.
    let away = s.top.join("Away");
    fs::rename(&books, &away).unwrap();
    let report = sync(&mut s.conn, root);
    assert!(report.offline);
    assert_eq!(report.missing, 0);
    let list = docs(&s.conn, library);
    assert_eq!(list.len(), 1);
    assert!(list[0].offline);

    // Even another library finding the same content elsewhere does not take it away.
    let (other, _) = library_with_root(&mut s.conn, "Other", &away);
    assert_eq!(docs(&s.conn, other).len(), 1);
    assert_ne!(docs(&s.conn, other)[0].file_id, list[0].file_id);

    fs::remove_dir_all(&away).unwrap();
    write_pdf(&books.join("a.pdf"), "a");
    refresh_root_status(&s.conn).unwrap();
    assert!(!docs(&s.conn, library)[0].offline);
}

#[test]
fn clean_up_forgets_missing_files() {
    let mut s = setup();
    let books = s.top.join("Books");
    write_pdf(&books.join("a.pdf"), "a");
    write_pdf(&books.join("b.pdf"), "b");
    let (library, root) = library_with_root(&mut s.conn, "L", &books);
    let (other, _) = library_with_root(&mut s.conn, "Other", &books);
    let a = doc(&docs(&s.conn, library), "a.pdf").file_id;
    marks::set_favorite(&mut s.conn, library, &[a], true).unwrap();
    marks::set_favorite(&mut s.conn, other, &[a], true).unwrap();

    fs::remove_file(books.join("a.pdf")).unwrap();
    sync(&mut s.conn, root);
    missing::clean_up(&mut s.conn, library, &[a]).unwrap();

    assert!(missing::list_missing(&s.conn, library).unwrap().is_empty());
    let count = |conn: &Connection, sql: &str| -> i64 { conn.query_row(sql, [], |r| r.get(0)).unwrap() };
    assert_eq!(count(&s.conn, &format!("SELECT COUNT(*) FROM favorites WHERE library_id = {library}")), 0);
    assert_eq!(count(&s.conn, "SELECT COUNT(*) FROM files"), 2, "still known to the other library");

    missing::clean_up(&mut s.conn, other, &[a]).unwrap();
    assert_eq!(count(&s.conn, "SELECT COUNT(*) FROM files"), 1, "now forgotten completely");
    assert_eq!(count(&s.conn, "SELECT COUNT(*) FROM favorites"), 0);
}

#[test]
fn clean_up_skips_files_that_came_back() {
    let mut s = setup();
    let books = s.top.join("Books");
    write_pdf(&books.join("a.pdf"), "a");
    let (library, root) = library_with_root(&mut s.conn, "L", &books);
    let a = doc(&docs(&s.conn, library), "a.pdf").file_id;
    let saved = fs::read(books.join("a.pdf")).unwrap();
    fs::remove_file(books.join("a.pdf")).unwrap();
    sync(&mut s.conn, root);
    fs::write(books.join("a.pdf"), saved).unwrap();
    sync(&mut s.conn, root);

    missing::clean_up(&mut s.conn, library, &[a]).unwrap();

    assert_eq!(docs(&s.conn, library).len(), 1);
}

#[test]
fn watcher_reports_a_change_once_the_folder_is_quiet() {
    let dir = TempDir::new().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let watcher = billy_lib::watch::SyncWatcher::new(move |root_id| tx.send(root_id).unwrap());
    watcher.watch(7, dir.path()).unwrap();

    // A burst of changes gives one notification.
    for i in 0..5 {
        write_pdf(&dir.path().join("Sub").join(format!("{i}.pdf")), "x");
    }
    let first = rx.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
    assert_eq!(first, 7);
    assert!(rx.recv_timeout(std::time::Duration::from_millis(2500)).is_err(), "only one sync per burst");

    // After unwatching, changes are ignored.
    watcher.unwatch(7);
    write_pdf(&dir.path().join("later.pdf"), "y");
    assert!(rx.recv_timeout(std::time::Duration::from_secs(3)).is_err());
}

#[test]
fn moving_into_an_ignored_folder_counts_as_missing() {
    let mut s = setup();
    let books = s.top.join("Books");
    write_pdf(&books.join("a.pdf"), "a");
    let (library, root) = library_with_root(&mut s.conn, "L", &books);

    fs::create_dir_all(books.join(".trash")).unwrap();
    fs::rename(books.join("a.pdf"), books.join(".trash").join("a.pdf")).unwrap();
    let report = sync(&mut s.conn, root);

    // Moved into an ignored folder: the old path is gone and nothing new was found.
    assert_eq!(report.missing, 1);
    assert_eq!(docs(&s.conn, library).len(), 0);
}
