//! Tests for thumbnails and metadata, using small PDFs generated here.

use billy_lib::db::{self, libraries};
use billy_lib::scan::scan_root;
use billy_lib::thumbs::{self, THUMB_WIDTH};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Writes a valid PDF with `pages` pages of 300×400 points, each a colored rectangle, and
/// the given title and author in its Info dictionary.
fn write_pdf(path: &Path, title: &str, author: &str, pages: usize) {
    let mut objects: Vec<String> = Vec::new();
    let page_ids: Vec<usize> = (0..pages).map(|i| 4 + i * 2).collect();
    objects.push("<< /Type /Catalog /Pages 2 0 R >>".into());
    let kids: Vec<String> = page_ids.iter().map(|id| format!("{id} 0 R")).collect();
    objects.push(format!("<< /Type /Pages /Kids [{}] /Count {pages} >>", kids.join(" ")));
    objects.push(format!("<< /Title ({title}) /Author ({author}) >>"));
    for (i, id) in page_ids.iter().enumerate() {
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 400] /Contents {} 0 R >>",
            id + 1
        ));
        let content = format!("0.{} 0.4 0.7 rg 20 20 260 360 re f", (i % 9) + 1);
        objects.push(format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()));
    }

    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R /Info 3 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, out).unwrap();
}

fn pdfium_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("resources")
}

#[test]
fn renders_cover_and_reads_metadata() {
    let dir = TempDir::new().unwrap();
    let pdf = dir.path().join("book.pdf");
    let out = dir.path().join("cache").join("cover.jpg");
    write_pdf(&pdf, "A Field Guide", "Ada Ribeiro", 3);
    let pdfium = thumbs::pdfium(&pdfium_dir()).unwrap();

    let info = thumbs::render(pdfium, &pdf, &out).unwrap();

    assert_eq!(info.pages, 3);
    assert_eq!(info.title.as_deref(), Some("A Field Guide"));
    assert_eq!(info.author.as_deref(), Some("Ada Ribeiro"));
    let cover = image::open(&out).unwrap();
    assert_eq!(cover.width(), THUMB_WIDTH as u32);
    assert_eq!(cover.height(), (THUMB_WIDTH as u32) * 4 / 3);
    assert!(!out.with_extension("tmp").exists());
}

#[test]
fn damaged_pdf_is_marked_as_error_and_good_ones_as_ok() {
    let tree = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();
    let books = tree.path().join("Books");
    write_pdf(&books.join("good.pdf"), "Good", "Someone", 1);
    fs::write(books.join("broken.pdf"), b"%PDF-1.7\nthis is not really a pdf").unwrap();

    let mut conn = db::open(&data.path().join("billy.db")).unwrap();
    let library = libraries::create_library(&conn, "Books").unwrap();
    let root = libraries::insert_root(&conn, library.id, &books.to_string_lossy()).unwrap();
    scan_root(&mut conn, root.id, &mut |_| {}).unwrap();
    assert_eq!(thumbs::remaining(&conn).unwrap(), 2);

    let pdfium = thumbs::pdfium(&pdfium_dir()).unwrap();
    let cache = data.path().join("thumbs");
    let mut updates = Vec::new();
    while let Some(id) = thumbs::next_pending(&conn).unwrap() {
        updates.push(thumbs::process(&conn, pdfium, &cache, id).unwrap().unwrap());
    }

    assert_eq!(thumbs::remaining(&conn).unwrap(), 0);
    let mut statuses: Vec<(String, Option<i64>)> =
        updates.iter().map(|u| (u.status.clone(), u.pages)).collect();
    statuses.sort();
    assert_eq!(statuses, vec![("error".to_string(), None), ("ok".to_string(), Some(1))]);
    assert_eq!(fs::read_dir(&cache).unwrap().count(), 1, "only the good PDF has a cover");

    // Already done: processing again does nothing.
    assert!(thumbs::process(&conn, pdfium, &cache, updates[0].file_id).unwrap().is_none());
}

#[test]
fn making_thumbnails_never_changes_the_library_folder() {
    let tree = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();
    let books = tree.path().join("Books");
    for i in 0..5 {
        write_pdf(&books.join(format!("b{i}.pdf")), &format!("Book {i}"), "A", 2);
    }
    let before: Vec<_> = fs::read_dir(&books)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            let m = fs::metadata(e.path()).unwrap();
            (e.path(), m.len(), m.modified().unwrap(), fs::read(e.path()).unwrap())
        })
        .collect();
    let dir_mtime = fs::metadata(&books).unwrap().modified().unwrap();

    let mut conn = db::open(&data.path().join("billy.db")).unwrap();
    let library = libraries::create_library(&conn, "Books").unwrap();
    let root = libraries::insert_root(&conn, library.id, &books.to_string_lossy()).unwrap();
    scan_root(&mut conn, root.id, &mut |_| {}).unwrap();
    let pdfium = thumbs::pdfium(&pdfium_dir()).unwrap();
    while let Some(id) = thumbs::next_pending(&conn).unwrap() {
        thumbs::process(&conn, pdfium, &data.path().join("thumbs"), id).unwrap();
    }

    let after: Vec<_> = fs::read_dir(&books)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            let m = fs::metadata(e.path()).unwrap();
            (e.path(), m.len(), m.modified().unwrap(), fs::read(e.path()).unwrap())
        })
        .collect();
    assert_eq!(before, after);
    assert_eq!(dir_mtime, fs::metadata(&books).unwrap().modified().unwrap());
}

#[test]
fn protocol_serves_only_cached_fingerprints() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("abc-123.jpg"), b"jpeg").unwrap();
    fs::write(dir.path().join("secret.txt"), b"no").unwrap();

    assert_eq!(thumbs::serve(dir.path(), "/abc-123").status(), 200);
    assert_eq!(thumbs::serve(dir.path(), "/abc-123").body(), b"jpeg");
    assert_eq!(thumbs::serve(dir.path(), "/def-456").status(), 404);
    assert_eq!(thumbs::serve(dir.path(), "/../secret").status(), 404);
    assert_eq!(thumbs::serve(dir.path(), "/secret").status(), 404);
}

/// Times rendering 200 generated PDFs. Run with
/// `cargo test --release --test thumbs -- --ignored --nocapture render_speed`.
#[test]
#[ignore]
fn render_speed() {
    let dir = TempDir::new().unwrap();
    let pdfium = thumbs::pdfium(&pdfium_dir()).unwrap();
    for i in 0..200 {
        write_pdf(&dir.path().join(format!("{i}.pdf")), &format!("Book {i}"), "A", 50);
    }
    let start = std::time::Instant::now();
    for i in 0..200 {
        thumbs::render(pdfium, &dir.path().join(format!("{i}.pdf")), &dir.path().join(format!("{i}.jpg"))).unwrap();
    }
    println!("200 covers in {:?}", start.elapsed());
}
