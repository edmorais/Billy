// Browsing choices, sorting and formatting helpers.

import type { Doc } from "./api";

export type ViewMode = "grid-s" | "grid-m" | "grid-l" | "details" | "compact";
export type SortField = "name" | "modified" | "added" | "size" | "pages" | "title" | "author";
export type ReadFilter = "all" | "unread" | "read";

/** Stored for each library in its `view_settings`. */
export interface ViewSettings {
  view: ViewMode;
  sort: SortField;
  desc: boolean;
  includeSubfolders: boolean;
  groupBySubfolder: boolean;
  filter: ReadFilter;
  /** Show only favorites in the main view. */
  favoritesOnly: boolean;
  /** The favorites shelf also shows favorites from subfolders. */
  shelfSubfolders: boolean;
  /** The favorites shelf is expanded. */
  shelfOpen: boolean;
  /** Show files that disappeared from disk, greyed out. */
  showMissing: boolean;
  /** Folder shown when the library was last open. `null` is the library itself. */
  folderId: number | null;
}

export const defaultSettings: ViewSettings = {
  view: "grid-m",
  sort: "name",
  desc: false,
  includeSubfolders: false,
  groupBySubfolder: false,
  filter: "all",
  favoritesOnly: false,
  shelfSubfolders: true,
  shelfOpen: true,
  showMissing: false,
  folderId: null,
};

export function readSettings(raw: Record<string, unknown> | undefined): ViewSettings {
  return { ...defaultSettings, ...(raw as Partial<ViewSettings>) };
}

export const viewModes: { id: ViewMode; label: string }[] = [
  { id: "grid-s", label: "Small grid" },
  { id: "grid-m", label: "Medium grid" },
  { id: "grid-l", label: "Large grid" },
  { id: "details", label: "Details" },
  { id: "compact", label: "Compact list" },
];

export const sortFields: { id: SortField; label: string }[] = [
  { id: "name", label: "Name" },
  { id: "modified", label: "Date modified" },
  { id: "added", label: "Date added" },
  { id: "size", label: "File size" },
  { id: "pages", label: "Page count" },
  { id: "title", label: "Title" },
  { id: "author", label: "Author" },
];

/** Natural order: "Book 2" before "Book 10", ignoring case and accents. */
export const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" });

/** Sorts documents. Missing titles, authors and page counts always go last. */
export function sortDocs(docs: Doc[], field: SortField, desc: boolean): Doc[] {
  const dir = desc ? -1 : 1;
  const byName = (a: Doc, b: Doc) => collator.compare(a.name, b.name);
  const compare = (a: Doc, b: Doc): number => {
    switch (field) {
      case "name":
        return dir * byName(a, b);
      case "modified":
        return dir * (a.modified - b.modified) || byName(a, b);
      case "added":
        return dir * (a.added - b.added) || byName(a, b);
      case "size":
        return dir * (a.size - b.size) || byName(a, b);
      case "pages":
        return nullsLast(a.pages, b.pages, (x, y) => dir * (x - y)) || byName(a, b);
      case "title":
        return nullsLast(blank(a.title), blank(b.title), (x, y) => dir * collator.compare(x, y)) || byName(a, b);
      case "author":
        return nullsLast(blank(a.author), blank(b.author), (x, y) => dir * collator.compare(x, y)) || byName(a, b);
    }
  };
  return [...docs].sort(compare);
}

function blank(value: string | null): string | null {
  return value && value.trim() ? value : null;
}

function nullsLast<T>(a: T | null, b: T | null, compare: (a: T, b: T) => number): number {
  if (a === null && b === null) return 0;
  if (a === null) return 1;
  if (b === null) return -1;
  return compare(a, b);
}

/** Lower case without accents, for search. */
export function fold(text: string): string {
  return text.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();
}

export function displayName(doc: Doc): string {
  return doc.name.replace(/\.pdf$/i, "");
}

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

const dateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: "medium" });

export function formatDate(ms: number | null): string {
  return ms ? dateFormat.format(new Date(ms)) : "";
}
