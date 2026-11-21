// Typed wrappers around the Rust backend's commands and events.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface Library {
  id: number;
  name: string;
  createdAt: number;
  documentCount: number;
  rootCount: number;
  viewSettings: Record<string, unknown>;
}

export interface FolderNode {
  id: number;
  rootId: number;
  parentId: number | null;
  name: string;
  relPath: string;
  rootPath: string;
  offline: boolean;
  /** Documents directly in this folder. */
  count: number;
  /** Documents in this folder and its subfolders. */
  total: number;
}

export interface Doc {
  fileId: number;
  name: string;
  path: string;
  folderId: number;
  size: number;
  modified: number;
  added: number;
  pages: number | null;
  title: string | null;
  author: string | null;
  fingerprint: string;
  read: boolean;
  favorite: boolean;
}

export interface Root {
  id: number;
  libraryId: number;
  path: string;
  autoSync: boolean;
  status: "ok" | "offline";
  addedAt: number;
  lastSyncedAt: number | null;
  fileCount: number;
}

export interface ScanProgress {
  rootId: number;
  filesFound: number;
}

export interface ScanReport {
  rootId: number;
  files: number;
  newFiles: number;
  offline: boolean;
  errors: string[];
}

export interface ScanFinished {
  rootId: number;
  report: ScanReport | null;
  error: string | null;
}

export const api = {
  appVersion: () => invoke<string>("app_version"),
  listLibraries: () => invoke<Library[]>("list_libraries"),
  createLibrary: (name: string) => invoke<Library>("create_library", { name }),
  renameLibrary: (id: number, name: string) => invoke<Library>("rename_library", { id, name }),
  deleteLibrary: (id: number) => invoke<void>("delete_library", { id }),
  saveViewSettings: (id: number, settings: object) =>
    invoke<void>("save_view_settings", { id, settings }),
  folderTree: (libraryId: number) => invoke<FolderNode[]>("folder_tree", { libraryId }),
  listDocuments: (libraryId: number, folderId: number | null, recursive: boolean) =>
    invoke<Doc[]>("list_documents", { libraryId, folderId, recursive }),
  listRoots: (libraryId: number) => invoke<Root[]>("list_roots", { libraryId }),
  scanningRoots: () => invoke<number[]>("scanning_roots"),
  addRoot: (libraryId: number, path: string) => invoke<Root>("add_root", { libraryId, path }),
  removeRoot: (rootId: number, purge: boolean) => invoke<void>("remove_root", { rootId, purge }),
};

export function onScanProgress(handler: (p: ScanProgress) => void): Promise<UnlistenFn> {
  return listen<ScanProgress>("scan-progress", (e) => handler(e.payload));
}

export function onScanFinished(handler: (f: ScanFinished) => void): Promise<UnlistenFn> {
  return listen<ScanFinished>("scan-finished", (e) => handler(e.payload));
}
