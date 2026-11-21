// Display preferences: text size (the whole window's zoom) and the sidebar's width.
// Kept in Billy's database, and cached locally so the window opens at the right size.

import { getCurrentWebview } from "@tauri-apps/api/webview";
import { api } from "./api";

export const textSizes = [
  { value: 0.9, label: "Small" },
  { value: 1, label: "Normal" },
  { value: 1.1, label: "Large" },
  { value: 1.25, label: "Larger" },
  { value: 1.5, label: "Largest" },
];

export const SIDEBAR_DEFAULT = 260;
const SIDEBAR_MIN = 200;
const SIDEBAR_MAX = 520;

export const display = $state({ textSize: 1, sidebarWidth: SIDEBAR_DEFAULT });

const CACHE_KEY = "billy.display";
const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
let saving: Promise<unknown> = Promise.resolve();
let saveTimer: ReturnType<typeof setTimeout> | undefined;

function cache() {
  try {
    localStorage.setItem(CACHE_KEY, JSON.stringify(display));
  } catch {
    // Storage may be unavailable; the database has the settings too.
  }
}

function save(delay = 0) {
  cache();
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    const value = { textSize: display.textSize, sidebarWidth: display.sidebarWidth };
    saving = saving.then(() => api.setSetting("display", value)).catch(() => {});
  }, delay);
}

function applyZoom() {
  if (inTauri) {
    getCurrentWebview()
      .setZoom(display.textSize)
      .catch(() => {
        // Fall back to CSS zoom, for example in the development mock.
        document.documentElement.style.setProperty("zoom", String(display.textSize));
      });
  } else {
    document.documentElement.style.setProperty("zoom", String(display.textSize));
  }
}

function applyWidth() {
  document.documentElement.style.setProperty("--sidebar-w", `${display.sidebarWidth}px`);
}

function clampWidth(width: number): number {
  const max = Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, window.innerWidth / display.textSize - 420));
  return Math.round(Math.min(max, Math.max(SIDEBAR_MIN, width)));
}

export function setTextSize(scale: number) {
  display.textSize = scale;
  applyZoom();
  save();
}

/** Sets the sidebar's width. `persist` is false while dragging and true when the drag ends. */
export function setSidebarWidth(width: number, persist = true) {
  display.sidebarWidth = clampWidth(width);
  applyWidth();
  if (persist) save(200);
}

function apply(values: Partial<typeof display>) {
  if (typeof values.textSize === "number") display.textSize = values.textSize;
  if (typeof values.sidebarWidth === "number") display.sidebarWidth = values.sidebarWidth;
  applyZoom();
  applyWidth();
}

/** At startup, before the database answers. */
export function applyCachedDisplay() {
  try {
    apply(JSON.parse(localStorage.getItem(CACHE_KEY) ?? "{}"));
  } catch {
    apply({});
  }
}

/** With the settings from Billy's database. */
export function initDisplay(saved: Record<string, unknown> | undefined) {
  apply((saved ?? {}) as Partial<typeof display>);
  cache();
}
