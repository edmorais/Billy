// The history of opened documents: whether Billy keeps it, how many documents the History
// view lists, and how it shows them. Kept in Billy's database. The backend records each
// opening while `enabled` is on.

import { api } from "./api";
import type { ViewMode } from "./view";

/** How many documents the sidebar's History section lists. */
export const SIDEBAR_COUNT = 5;
export const SIZE_MIN = 10;
export const SIZE_MAX = 100;

export const history = $state<{ enabled: boolean; size: number; view: ViewMode; library: number | null }>({
  enabled: true,
  size: 30,
  view: "grid-m",
  /** The History view lists this library's documents only; `null` is every library. */
  library: null,
});

/** Bumped when the history changes outside the reader (cleared, turned on or off). */
export const historyChanged = $state({ version: 0 });

let saving: Promise<unknown> = Promise.resolve();

export function setHistory(change: Partial<typeof history>) {
  Object.assign(history, change);
  const value = { ...history };
  saving = saving.then(() => api.setSetting("history", value)).catch(() => {});
}

/** Forgets which documents were opened. */
export async function clearHistory() {
  await api.clearHistory();
  historyChanged.version++;
}

/** With the settings from Billy's database. Values Billy does not know are ignored. */
export function initHistory(saved: Record<string, unknown> | undefined) {
  const s = saved ?? {};
  if (typeof s.enabled === "boolean") history.enabled = s.enabled;
  if (typeof s.size === "number") history.size = Math.min(SIZE_MAX, Math.max(SIZE_MIN, Math.round(s.size)));
  if (typeof s.library === "number" || s.library === null) history.library = s.library as number | null;
  if (typeof s.view === "string" && ["grid-s", "grid-m", "grid-l", "details", "compact"].includes(s.view)) {
    history.view = s.view as ViewMode;
  }
}
