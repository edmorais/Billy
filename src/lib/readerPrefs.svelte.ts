// How the reader shows a document when it opens: zoom, scrolling and spreads. Kept in
// Billy's database. The reader's toolbar can still change them for the open document.

import { api } from "./api";

/** PDF.js zoom values. */
export type ReaderZoom = "auto" | "page-fit" | "page-actual";
/** PDF.js scroll modes: vertical, horizontal, wrapped, page by page. */
export type ReaderScroll = 0 | 1 | 2 | 3;
/** PDF.js spread modes: none, odd, even. */
export type ReaderSpread = 0 | 1 | 2;

export const zoomOptions: { value: ReaderZoom; label: string }[] = [
  { value: "auto", label: "Automatic" },
  { value: "page-fit", label: "Page fit" },
  { value: "page-actual", label: "Actual size" },
];

export const scrollOptions: { value: ReaderScroll; label: string }[] = [
  { value: 0, label: "Vertical" },
  { value: 1, label: "Horizontal" },
  { value: 2, label: "Wrapped" },
  { value: 3, label: "Page by page" },
];

export const spreadOptions: { value: ReaderSpread; label: string }[] = [
  { value: 0, label: "Single pages" },
  { value: 1, label: "Odd spreads" },
  { value: 2, label: "Even spreads" },
];

export const readerPrefs = $state<{ zoom: ReaderZoom; scroll: ReaderScroll; spread: ReaderSpread }>({
  zoom: "page-fit",
  scroll: 0,
  spread: 0,
});

let saving: Promise<unknown> = Promise.resolve();

export function setReaderPrefs(change: Partial<typeof readerPrefs>) {
  Object.assign(readerPrefs, change);
  const value = { ...readerPrefs };
  saving = saving.then(() => api.setSetting("reader", value)).catch(() => {});
}

/** With the settings from Billy's database. Values Billy does not know are ignored. */
export function initReaderPrefs(saved: Record<string, unknown> | undefined) {
  const s = saved ?? {};
  if (zoomOptions.some((o) => o.value === s.zoom)) readerPrefs.zoom = s.zoom as ReaderZoom;
  if (scrollOptions.some((o) => o.value === s.scroll)) readerPrefs.scroll = s.scroll as ReaderScroll;
  if (spreadOptions.some((o) => o.value === s.spread)) readerPrefs.spread = s.spread as ReaderSpread;
}
