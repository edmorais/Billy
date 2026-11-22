// Fake backend for working on the UI in a normal browser (`npm run dev`, then open
// http://localhost:1420). It is only loaded in development, outside Tauri.

import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { Doc, FolderNode, Library, Root } from "$lib/api";

const words =
  "Light Shadow River Garden Stone Atlas Theory Practice History Notes Design Form Space Time Image Sound Field Map Code Signal Memory Glass Paper Machine Body City Voice Line Color Motion".split(" ");
const authors = ["Ada Ribeiro", "Tomás Vale", "Inês Costa", "J. Martin", "K. Okafor", "Lena Fischer", "R. Duarte", null];

function rng(seed: number) {
  return () => {
    seed = (seed * 1664525 + 1013904223) >>> 0;
    return seed / 2 ** 32;
  };
}

function build() {
  const rand = rng(42);
  const pick = <T>(list: T[]) => list[Math.floor(rand() * list.length)];
  const now = Date.now();
  const folders: FolderNode[] = [];
  const docs: Doc[] = [];
  let folderId = 1;
  let fileId = 1;

  const addFolder = (rootId: number, parentId: number | null, name: string, relPath: string, rootPath: string) => {
    const node: FolderNode = { id: folderId++, rootId, parentId, name, relPath, rootPath, offline: false, count: 0, total: 0 };
    folders.push(node);
    return node;
  };

  const addDocs = (folder: FolderNode, n: number) => {
    for (let i = 0; i < n; i++) {
      const title = `${pick(words)} ${pick(["and", "of", "in", "without"])} ${pick(words)}`;
      const name = `${title}${rand() < 0.2 ? ` (${Math.floor(rand() * 20) + 1})` : ""}.pdf`;
      const hasMeta = rand() < 0.7;
      docs.push({
        fileId: fileId,
        name,
        path: `${folder.rootPath}\\${folder.relPath ? folder.relPath + "\\" : ""}${name}`,
        folderId: folder.id,
        size: Math.floor(rand() * 80_000_000) + 50_000,
        modified: now - Math.floor(rand() * 3e11),
        added: now - Math.floor(rand() * 1e9),
        pages: hasMeta ? Math.floor(rand() * 600) + 20 : null,
        title: hasMeta ? title : null,
        author: hasMeta ? pick(authors) : null,
        fingerprint: `f${fileId}`,
        thumbStatus: "pending",
        read: rand() < 0.3,
        favorite: false,
        // A few files have disappeared from disk, to try the missing-file views.
        missing: fileId % 997 === 0,
        offline: false,
        annotated: false,
      });
      fileId++;
    }
    folder.count += n;
  };

  const books = addFolder(1, null, "Books", "", "D:\\Books");
  addDocs(books, 12);
  for (let s = 0; s < 40; s++) {
    const shelf = addFolder(1, books.id, `${pick(words)} ${s + 1}`, `Shelf ${s + 1}`, "D:\\Books");
    addDocs(shelf, 90);
    if (s % 5 === 0) {
      const sub = addFolder(1, shelf.id, "Archive", `Shelf ${s + 1}\\Archive`, "D:\\Books");
      addDocs(sub, 150);
    }
  }
  // Open http://localhost:1420/?single to try a library with only one folder.
  if (!new URLSearchParams(location.search).has("single")) {
    const papers = addFolder(2, null, "Papers", "", "E:\\Papers");
    addDocs(papers, 40);
  }

  // Totals, deepest first.
  for (const f of [...folders].reverse()) {
    f.total += f.count;
    const parent = folders.find((p) => p.id === f.parentId);
    if (parent) parent.total += f.total;
  }
  return { folders, docs };
}

/** A small valid PDF with three pages of text, for trying the reader. */
function makeSamplePdf(): Uint8Array {
  const pages = 3;
  const objects: string[] = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    `<< /Type /Pages /Kids [${Array.from({ length: pages }, (_, i) => `${4 + i * 2} 0 R`).join(" ")}] /Count ${pages} >>`,
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
  ];
  // With ?big the sample is padded past 1 MB, so the reader reads it in ranges.
  const padding = new URLSearchParams(location.search).has("big") ? " ".repeat(1_200_000) : "";
  objects[2] = `<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /BillyPadding (${padding}) >>`;
  for (let i = 0; i < pages; i++) {
    objects.push(
      `<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 3 0 R >> >> /Contents ${5 + i * 2} 0 R >>`,
    );
    const text = `BT /F1 28 Tf 72 760 Td (Billy sample, page ${i + 1}) Tj /F1 12 Tf 0 -40 Td (Select this text to try highlighting.) Tj ET`;
    objects.push(`<< /Length ${text.length} >>\nstream\n${text}\nendstream`);
  }
  let out = "%PDF-1.7\n";
  const offsets: number[] = [];
  objects.forEach((body, i) => {
    offsets.push(out.length);
    out += `${i + 1} 0 obj\n${body}\nendobj\n`;
  });
  const xref = out.length;
  out += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n`;
  for (const o of offsets) out += `${String(o).padStart(10, "0")} 00000 n \n`;
  out += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  return new TextEncoder().encode(out);
}

const samplePdf = makeSamplePdf();

const appSettings: Record<string, unknown> = {};
let ignored: string[] = [];

export function installMock() {
  mockWindows("main");
  const { folders, docs } = build();
  const library: Library = {
    id: 1,
    name: "Books",
    createdAt: Date.now(),
    documentCount: docs.length,
    rootCount: 2,
    viewSettings: {},
  };
  const roots: Root[] = [
    { id: 1, libraryId: 1, path: "D:\\Books", autoSync: false, status: "ok", addedAt: 0, lastSyncedAt: Date.now(), fileCount: docs.length - 40, },
    { id: 2, libraryId: 1, path: "E:\\Papers", autoSync: false, status: "ok", addedAt: 0, lastSyncedAt: Date.now(), fileCount: 40 },
  ];

  const descendants = (id: number): Set<number> => {
    const set = new Set([id]);
    let grew = true;
    while (grew) {
      grew = false;
      for (const f of folders) if (f.parentId !== null && set.has(f.parentId) && !set.has(f.id)) set.add(f.id), (grew = true);
    }
    return set;
  };

  mockIPC(
    (cmd, args) => {
      const a = args as Record<string, any>;
      switch (cmd) {
        case "app_version":
          return "1.0.1 (mock)";
        case "plugin:webview|set_webview_zoom":
          // The real app zooms the whole window; in a browser, CSS zoom stands in.
          document.documentElement.style.setProperty("zoom", String(a.value ?? a.scaleFactor ?? 1));
          return null;
        case "plugin:updater|check":
          // With ?update, pretend a newer version exists.
          return new URLSearchParams(location.search).has("update")
            ? { rid: 1, currentVersion: "1.0.0-rc.2", version: "1.0.1", date: null, body: "Faster covers and small fixes.", rawJson: {} }
            : null;
        case "get_settings":
          return { ...appSettings };
        case "set_setting":
          appSettings[a.key] = a.value;
          return null;
        case "list_libraries":
          return [library];
        case "save_view_settings":
          library.viewSettings = a.settings;
          return null;
        case "folder_tree":
          return folders;
        case "list_documents": {
          const scope = a.folderId === null ? null : a.recursive ? descendants(a.folderId) : new Set([a.folderId]);
          return docs
            .filter((d) => !scope || scope.has(d.folderId))
            .filter((d) => !a.favoritesOnly || d.favorite)
            .filter((d) => a.includeMissing || !d.missing)
            .map((d) => ({ ...d }));
        }
        case "list_missing":
          return docs
            .filter((d) => d.missing)
            .map((d) => ({ fileId: d.fileId, path: d.path, missingSince: Date.now() - 864e5, favorite: d.favorite, read: d.read, annotated: false }));
        case "clean_up_missing": {
          const gone = new Set<number>(a.fileIds);
          for (let i = docs.length - 1; i >= 0; i--) if (gone.has(docs[i].fileId)) docs.splice(i, 1);
          return null;
        }
        case "sync_root":
        case "sync_library":
          return null;
        case "open_document": {
          const d = docs.find((x) => x.fileId === a.fileId)!;
          return {
            fileId: d.fileId,
            name: d.name,
            path: d.path,
            originalSize: samplePdf.length,
            annotatedSize: null,
            originalChanged: false,
            annotationsBroken: false,
            lastPage: 2,
          };
        }
        case "read_pdf":
          (window as any).__billyReads = ((window as any).__billyReads ?? 0) + 1;
          return samplePdf.slice(a.start, a.start + a.length).buffer;
        case "set_last_page":
        case "save_annotations":
        case "save_to_original":
        case "discard_annotations":
          console.info("mock:", cmd, a);
          return null;
        case "export_copy":
          return "C:\\mock\\export.pdf";
        case "set_auto_sync": {
          const root = roots.find((r) => r.id === a.rootId)!;
          root.autoSync = a.on;
          return { ...root };
        }
        case "set_favorite":
          for (const d of docs) if (a.fileIds.includes(d.fileId)) d.favorite = a.favorite;
          return null;
        case "set_read":
          for (const d of docs) if (a.fileIds.includes(d.fileId)) d.read = a.read;
          return null;
        case "open_with_default_app":
        case "show_in_folder":
        case "open_folder":
          console.info("mock:", cmd, args);
          return null;
        case "list_roots":
          return roots;
        case "list_ignored":
          return [...ignored];
        case "ignore_folder":
          if (!ignored.includes(a.path)) ignored = [...ignored, a.path].sort();
          return [...ignored];
        case "unignore_folder":
          ignored = ignored.filter((p) => p !== a.path);
          return [...ignored];
        case "scanning_roots":
          return [];
        case "prioritize_thumbs":
          return null;
        case "repair_covers":
          return 0;
        case "rebuild_covers":
          return docs.length;
        case "thumbs_remaining":
          return 0;
        default:
          console.warn("mock: unhandled command", cmd, args);
          return null;
      }
    },
    { shouldMockEvents: true },
  );
}
