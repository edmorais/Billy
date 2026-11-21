// Updates from GitHub Releases, through Tauri's updater plugin.
//
// Billy checks at most once a day, if the user allows it, and never installs anything
// without a click. Updates are signed; the updater refuses anything not signed with Billy's
// release key (its public key is in tauri.conf.json).

import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { api } from "./api";

export type UpdateStatus = "idle" | "checking" | "none" | "available" | "downloading" | "error";

export const updates = $state({
  /** Check automatically, once a day. */
  auto: true,
  lastCheck: 0,
  status: "idle" as UpdateStatus,
  version: "",
  notes: "",
  error: "",
  /** Download progress, 0 to 1, while downloading. */
  progress: 0,
  /** The user chose "Later" for this version. */
  dismissed: "",
});

const DAY = 24 * 60 * 60 * 1000;
let pending: Update | null = null;

function save() {
  api.setSetting("updates", { auto: updates.auto, lastCheck: updates.lastCheck }).catch(() => {});
}

/** Reads the saved choice and, if allowed and due, checks a little after startup. */
export function initUpdates(saved: Record<string, unknown> | undefined) {
  updates.auto = saved?.auto !== false;
  updates.lastCheck = typeof saved?.lastCheck === "number" ? saved.lastCheck : 0;
  if (updates.auto && Date.now() - updates.lastCheck > DAY) {
    setTimeout(() => checkForUpdates(), 5000);
  }
}

export function setAutoUpdates(on: boolean) {
  updates.auto = on;
  save();
}

/** Asks GitHub whether a newer version exists. */
export async function checkForUpdates() {
  if (updates.status === "checking" || updates.status === "downloading") return;
  updates.status = "checking";
  updates.error = "";
  try {
    const update = await check();
    updates.lastCheck = Date.now();
    save();
    if (update) {
      pending = update;
      updates.version = update.version;
      updates.notes = update.body ?? "";
      updates.status = "available";
    } else {
      updates.status = "none";
    }
  } catch (err) {
    updates.status = "error";
    updates.error = String(err);
  }
}

/** Downloads and installs the update, then restarts Billy. */
export async function installUpdate() {
  if (!pending) return;
  updates.status = "downloading";
  updates.progress = 0;
  let total = 0;
  let received = 0;
  try {
    await pending.downloadAndInstall((event) => {
      if (event.event === "Started") total = event.data.contentLength ?? 0;
      else if (event.event === "Progress") {
        received += event.data.chunkLength;
        updates.progress = total ? received / total : 0;
      }
    });
    await relaunch();
  } catch (err) {
    updates.status = "error";
    updates.error = String(err);
  }
}
