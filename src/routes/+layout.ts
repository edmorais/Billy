// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static with a fallback to index.html to put the site in SPA mode
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
export const ssr = false;

import { applyCachedTheme } from "$lib/theme.svelte";
import { applyCachedDisplay } from "$lib/display.svelte";

export async function load() {
  // The last used theme, text size and sidebar width, before anything is drawn.
  applyCachedTheme();
  applyCachedDisplay();
  // In a plain browser during development, use a fake backend so the UI can be worked on.
  // Release builds drop this code, because import.meta.env.DEV is false there.
  if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
    const { installMock } = await import("$lib/dev/mock");
    installMock();
  }
}
