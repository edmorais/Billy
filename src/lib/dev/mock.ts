// Fake backend for working on the UI in a normal browser (`npm run dev`, then open
// http://localhost:1420). It is only loaded in development, outside Tauri.

import { mockIPC } from "@tauri-apps/api/mocks";
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
        read: rand() < 0.3,
        favorite: false,
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
  const papers = addFolder(2, null, "Papers", "", "E:\\Papers");
  addDocs(papers, 40);

  // Totals, deepest first.
  for (const f of [...folders].reverse()) {
    f.total += f.count;
    const parent = folders.find((p) => p.id === f.parentId);
    if (parent) parent.total += f.total;
  }
  return { folders, docs };
}

export function installMock() {
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
          return "0.1.0 (mock)";
        case "list_libraries":
          return [library];
        case "save_view_settings":
          library.viewSettings = a.settings;
          return null;
        case "folder_tree":
          return folders;
        case "list_documents": {
          if (a.folderId === null) return docs;
          const scope = a.recursive ? descendants(a.folderId) : new Set([a.folderId]);
          return docs.filter((d) => scope.has(d.folderId));
        }
        case "list_roots":
          return roots;
        case "scanning_roots":
          return [];
        default:
          console.warn("mock: unhandled command", cmd, args);
          return null;
      }
    },
    { shouldMockEvents: true },
  );
}
