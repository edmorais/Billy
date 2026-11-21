// Typed wrappers around the Rust backend's commands and events.

import { convertFileSrc, invoke } from "@tauri-apps/api/core";
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
  thumbStatus: "pending" | "ok" | "error";
  read: boolean;
  favorite: boolean;
  /** Gone from disk; Billy keeps the record until a clean-up. */
  missing: boolean;
  /** Its folder is unavailable, for example an unplugged drive. */
  offline: boolean;
}

export interface MissingFile {
  fileId: number;
  path: string;
  missingSince: number;
  favorite: boolean;
  read: boolean;
  annotated: boolean;
}

export interface ThumbUpdate {
  fileId: number;
  status: "ok" | "error";
  pages: number | null;
  title: string | null;
  author: string | null;
}

export interface ThumbsUpdated {
  updates: ThumbUpdate[];
  remaining: number;
}

/** URL of a cached thumbnail, served by the backend's thumb:// protocol. */
export function thumbUrl(fingerprint: string): string {
  return convertFileSrc(fingerprint, "thumb");
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
  moved: number;
  missing: number;
  removed: number;
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
  listDocuments: (
    libraryId: number,
    folderId: number | null,
    recursive: boolean,
    favoritesOnly = false,
    includeMissing = false,
  ) => invoke<Doc[]>("list_documents", { libraryId, folderId, recursive, favoritesOnly, includeMissing }),
  syncRoot: (rootId: number) => invoke<void>("sync_root", { rootId }),
  syncLibrary: (libraryId: number) => invoke<void>("sync_library", { libraryId }),
  setAutoSync: (rootId: number, on: boolean) => invoke<Root>("set_auto_sync", { rootId, on }),
  listMissing: (libraryId: number) => invoke<MissingFile[]>("list_missing", { libraryId }),
  cleanUpMissing: (libraryId: number, fileIds: number[]) =>
    invoke<void>("clean_up_missing", { libraryId, fileIds }),
  setFavorite: (libraryId: number, fileIds: number[], favorite: boolean) =>
    invoke<void>("set_favorite", { libraryId, fileIds, favorite }),
  setRead: (libraryId: number, fileIds: number[], read: boolean) =>
    invoke<void>("set_read", { libraryId, fileIds, read }),
  openWithDefaultApp: (fileId: number) => invoke<void>("open_with_default_app", { fileId }),
  showInFolder: (fileId: number) => invoke<void>("show_in_folder", { fileId }),
  openFolder: (folderId: number) => invoke<void>("open_folder", { folderId }),
  listRoots: (libraryId: number) => invoke<Root[]>("list_roots", { libraryId }),
  scanningRoots: () => invoke<number[]>("scanning_roots"),
  addRoot: (libraryId: number, path: string) => invoke<Root>("add_root", { libraryId, path }),
  removeRoot: (rootId: number, purge: boolean) => invoke<void>("remove_root", { rootId, purge }),
  prioritizeThumbs: (fileIds: number[]) => invoke<void>("prioritize_thumbs", { fileIds }),
  thumbsRemaining: () => invoke<number>("thumbs_remaining"),
};

export function onThumbsUpdated(handler: (u: ThumbsUpdated) => void): Promise<UnlistenFn> {
  return listen<ThumbsUpdated>("thumbs-updated", (e) => handler(e.payload));
}

export function onThumbsError(handler: (message: string) => void): Promise<UnlistenFn> {
  return listen<string>("thumbs-error", (e) => handler(e.payload));
}

export function onScanProgress(handler: (p: ScanProgress) => void): Promise<UnlistenFn> {
  return listen<ScanProgress>("scan-progress", (e) => handler(e.payload));
}

export function onScanFinished(handler: (f: ScanFinished) => void): Promise<UnlistenFn> {
  return listen<ScanFinished>("scan-finished", (e) => handler(e.payload));
}
