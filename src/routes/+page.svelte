<script lang="ts">
  // Temporary library manager for milestone 1. Milestone 2 replaces it with the browser.
  import { onDestroy, onMount } from "svelte";
  import { ask, message, open } from "@tauri-apps/plugin-dialog";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import { api, onScanFinished, onScanProgress, type Library, type Root } from "$lib/api";

  let version = $state("");
  let libraries = $state<Library[]>([]);
  let selectedId = $state<number | null>(null);
  let roots = $state<Root[]>([]);
  let scanning = $state<Record<number, number>>({});
  let newName = $state("");
  let editingName = $state<string | null>(null);
  let error = $state("");

  let selected = $derived(libraries.find((l) => l.id === selectedId) ?? null);
  let unlisten: UnlistenFn[] = [];

  async function run<T>(task: () => Promise<T>): Promise<T | undefined> {
    error = "";
    try {
      return await task();
    } catch (err) {
      error = String(err);
    }
  }

  async function loadLibraries() {
    libraries = (await run(api.listLibraries)) ?? [];
    if (selectedId === null || !libraries.some((l) => l.id === selectedId)) {
      selectedId = libraries[0]?.id ?? null;
    }
    await loadRoots();
  }

  async function loadRoots() {
    roots = selectedId === null ? [] : ((await run(() => api.listRoots(selectedId!))) ?? []);
  }

  async function select(id: number) {
    selectedId = id;
    editingName = null;
    await loadRoots();
  }

  async function createLibrary(event: SubmitEvent) {
    event.preventDefault();
    const library = await run(() => api.createLibrary(newName));
    if (library) {
      newName = "";
      selectedId = library.id;
      await loadLibraries();
    }
  }

  async function saveName(event: SubmitEvent) {
    event.preventDefault();
    if (!selected || editingName === null) return;
    const renamed = await run(() => api.renameLibrary(selected!.id, editingName!));
    if (renamed) {
      editingName = null;
      await loadLibraries();
    }
  }

  async function deleteLibrary() {
    if (!selected) return;
    const confirmed = await ask(
      `Delete the library "${selected.name}"?\n\nBilly will forget its folders, favorites and read status. Your files and folders are not touched.`,
      { title: "Delete library", kind: "warning", okLabel: "Delete library", cancelLabel: "Cancel" },
    );
    if (!confirmed) return;
    await run(() => api.deleteLibrary(selected!.id));
    selectedId = null;
    await loadLibraries();
  }

  async function addFolder() {
    if (!selected) return;
    const path = await open({ directory: true, multiple: false, title: `Add a folder to ${selected.name}` });
    if (typeof path !== "string") return;
    const root = await run(() => api.addRoot(selected!.id, path));
    if (root) {
      scanning[root.id] = 0;
      await loadLibraries();
    }
  }

  async function removeFolder(root: Root) {
    const confirmed = await ask(
      `Remove this folder from "${selected?.name}"?\n\n${root.path}\n\nThe folder and its files are not touched.`,
      { title: "Remove folder", kind: "warning", okLabel: "Remove folder", cancelLabel: "Cancel" },
    );
    if (!confirmed) return;
    const purge = await ask(
      "Also delete the favorites and read status of the documents in this folder?\n\nIf you keep them, they come back when you add the folder again.",
      { title: "Remove folder", kind: "info", okLabel: "Delete them", cancelLabel: "Keep them" },
    );
    await run(() => api.removeRoot(root.id, purge));
    await loadLibraries();
  }

  function formatDate(ms: number | null) {
    return ms ? new Date(ms).toLocaleString() : "never";
  }

  onMount(async () => {
    version = (await run(api.appVersion)) ?? "";
    for (const id of (await run(api.scanningRoots)) ?? []) scanning[id] = 0;
    unlisten.push(
      await onScanProgress((p) => {
        scanning[p.rootId] = p.filesFound;
      }),
      await onScanFinished(async (f) => {
        delete scanning[f.rootId];
        if (f.error) {
          error = f.error;
        } else if (f.report && f.report.errors.length > 0) {
          await message(
            `${f.report.errors.length} item(s) could not be read and were skipped:\n\n${f.report.errors.slice(0, 10).join("\n")}`,
            { title: "Scan finished", kind: "warning" },
          );
        }
        await loadLibraries();
      }),
    );
    await loadLibraries();
  });

  onDestroy(() => unlisten.forEach((fn) => fn()));
</script>

<div class="app">
  <aside>
    <h1>Billy</h1>
    <h2>Libraries</h2>
    <ul>
      {#each libraries as library (library.id)}
        <li>
          <button class:active={library.id === selectedId} onclick={() => select(library.id)}>
            <span>{library.name}</span>
            <span class="count">{library.documentCount}</span>
          </button>
        </li>
      {/each}
    </ul>
    <form onsubmit={createLibrary}>
      <input bind:value={newName} placeholder="New library name" aria-label="New library name" />
      <button type="submit" disabled={!newName.trim()}>Add</button>
    </form>
    <p class="version">Billy {version}</p>
  </aside>

  <main>
    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}

    {#if selected}
      <header>
        {#if editingName !== null}
          <form onsubmit={saveName} class="rename">
            <input bind:value={editingName} aria-label="Library name" />
            <button type="submit">Save</button>
            <button type="button" onclick={() => (editingName = null)}>Cancel</button>
          </form>
        {:else}
          <h2>{selected.name}</h2>
          <button onclick={() => (editingName = selected!.name)}>Rename</button>
          <button class="danger" onclick={deleteLibrary}>Delete library</button>
        {/if}
      </header>
      <p class="muted">
        {selected.documentCount} documents in {selected.rootCount}
        {selected.rootCount === 1 ? "folder" : "folders"}
      </p>

      <section>
        <div class="section-head">
          <h3>Folders</h3>
          <button onclick={addFolder}>Add folder…</button>
        </div>
        {#if roots.length === 0}
          <p class="muted">No folders yet. Add a folder to fill this library.</p>
        {:else}
          <table>
            <thead>
              <tr><th>Folder</th><th>PDFs</th><th>Status</th><th>Last scan</th><th></th></tr>
            </thead>
            <tbody>
              {#each roots as root (root.id)}
                <tr>
                  <td class="path">{root.path}</td>
                  <td>{root.id in scanning ? scanning[root.id] : root.fileCount}</td>
                  <td>
                    {#if root.id in scanning}
                      <span class="badge scanning">Scanning…</span>
                    {:else if root.status === "offline"}
                      <span class="badge offline">Offline</span>
                    {:else}
                      <span class="badge">OK</span>
                    {/if}
                  </td>
                  <td>{formatDate(root.lastSyncedAt)}</td>
                  <td><button onclick={() => removeFolder(root)} disabled={root.id in scanning}>Remove</button></td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </section>
    {:else}
      <div class="empty">
        <h2>No library yet</h2>
        <p class="muted">Create a library on the left, then add folders to it.</p>
      </div>
    {/if}
  </main>
</div>

<style>
  :global(:root) {
    --bg: #f6f4ef;
    --panel: #ebe7de;
    --fg: #1f1d1a;
    --muted: #6b665d;
    --line: #d6d0c4;
    --accent: #8a5a2b;
    --danger: #a23b2a;
    color-scheme: light dark;
  }

  @media (prefers-color-scheme: dark) {
    :global(:root) {
      --bg: #1b1a18;
      --panel: #24221f;
      --fg: #ece8e1;
      --muted: #a39d92;
      --line: #3a3732;
      --accent: #d29a5c;
      --danger: #e0715f;
    }
  }

  :global(body) {
    margin: 0;
    background: var(--bg);
    color: var(--fg);
    font: 14px/1.4 "Segoe UI Variable", "Segoe UI", system-ui, sans-serif;
  }

  :global(button),
  :global(input) {
    font: inherit;
    color: inherit;
  }

  .app {
    display: grid;
    grid-template-columns: 260px 1fr;
    height: 100vh;
  }

  aside {
    background: var(--panel);
    border-right: 1px solid var(--line);
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow: auto;
  }

  h1 {
    margin: 0 0 8px;
    font-size: 22px;
  }

  h2 {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
  }

  aside h2 {
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size: 11px;
  }

  main h2 {
    font-size: 20px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  aside li button {
    width: 100%;
    display: flex;
    justify-content: space-between;
    padding: 6px 8px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  aside li button:hover {
    background: color-mix(in srgb, var(--fg) 8%, transparent);
  }

  aside li button.active {
    background: color-mix(in srgb, var(--accent) 20%, transparent);
  }

  .count,
  .muted,
  .version {
    color: var(--muted);
  }

  .version {
    margin-top: auto;
    font-size: 12px;
  }

  form {
    display: flex;
    gap: 6px;
  }

  input {
    flex: 1;
    min-width: 0;
    padding: 5px 8px;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--bg);
  }

  button {
    padding: 5px 10px;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--bg);
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.5;
    cursor: default;
  }

  button.danger {
    color: var(--danger);
  }

  main {
    padding: 24px 32px;
    overflow: auto;
  }

  header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  header h2 {
    margin-right: auto;
  }

  .rename {
    flex: 1;
  }

  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 24px;
  }

  h3 {
    margin: 0;
    font-size: 15px;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    margin-top: 12px;
  }

  th,
  td {
    text-align: left;
    padding: 8px;
    border-bottom: 1px solid var(--line);
  }

  th {
    color: var(--muted);
    font-weight: 600;
    font-size: 12px;
  }

  td.path {
    word-break: break-all;
  }

  .badge {
    font-size: 12px;
    padding: 2px 8px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--fg) 8%, transparent);
  }

  .badge.scanning {
    background: color-mix(in srgb, var(--accent) 25%, transparent);
  }

  .badge.offline {
    background: color-mix(in srgb, var(--danger) 25%, transparent);
  }

  .error {
    color: var(--danger);
    background: color-mix(in srgb, var(--danger) 12%, transparent);
    padding: 8px 12px;
    border-radius: 6px;
  }

  .empty {
    margin-top: 20vh;
    text-align: center;
  }

  .empty h2 {
    margin-bottom: 4px;
  }
</style>
