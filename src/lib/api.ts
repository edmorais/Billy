// Typed wrappers around the Rust backend's commands and events.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface Library {
  id: number;
  name: string;
  createdAt: number;
  documentCount: number;
  rootCount: number;
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
