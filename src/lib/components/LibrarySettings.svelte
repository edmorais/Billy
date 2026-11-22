<script lang="ts">
  import { ask, open } from "@tauri-apps/plugin-dialog";
  import { api, type Library, type Root } from "$lib/api";
  import Icon from "./Icon.svelte";

  interface Props {
    library: Library;
    scanning: Record<number, number>;
    /** Bumped by the parent when roots may have changed, so the list reloads. */
    version: number;
    onClose: () => void;
    onChanged: () => void;
    onDeleted: () => void;
    onError: (message: string) => void;
    /** Number of missing files in the library, and how to clean them up. */
    missingCount: number;
    onCleanUp: () => void;
  }

  let { library, scanning, version, onClose, onChanged, onDeleted, onError, missingCount, onCleanUp }: Props =
    $props();

  async function setAutoSync(root: Root, on: boolean) {
    try {
      const updated = await api.setAutoSync(root.id, on);
      roots = roots.map((r) => (r.id === updated.id ? updated : r));
    } catch (err) {
      onError(String(err));
    }
  }

  function syncNow(root: Root) {
    api.syncRoot(root.id).catch((err) => onError(String(err)));
  }

  function syncAll() {
    api.syncLibrary(library.id).catch((err) => onError(String(err)));
  }

  let roots = $state<Root[]>([]);
  let ignored = $state<string[]>([]);
  let name = $state("");
  let dialog: HTMLDialogElement;

  async function load() {
    try {
      [roots, ignored] = await Promise.all([api.listRoots(library.id), api.listIgnored(library.id)]);
    } catch (err) {
      onError(String(err));
    }
  }

  $effect(() => {
    void version;
    void library.id;
    load();
  });

  $effect(() => {
    name = library.name;
  });

  $effect(() => {
    dialog.showModal();
  });

  async function rename(event: SubmitEvent) {
    event.preventDefault();
    if (name.trim() === library.name) return;
    try {
      await api.renameLibrary(library.id, name);
      onChanged();
    } catch (err) {
      onError(String(err));
    }
  }

  async function addFolder() {
    const path = await open({ directory: true, multiple: false, title: `Add a folder to ${library.name}` });
    if (typeof path !== "string") return;
    try {
      await api.addRoot(library.id, path);
      await load();
      onChanged();
    } catch (err) {
      onError(String(err));
    }
  }

  async function removeFolder(root: Root) {
    const confirmed = await ask(
      `Remove this folder from "${library.name}"?\n\n${root.path}\n\nThe folder and its files are not touched.`,
      { title: "Remove folder", kind: "warning", okLabel: "Remove folder", cancelLabel: "Cancel" },
    );
    if (!confirmed) return;
    const purge = await ask(
      "Also delete the favorites and read status of the documents in this folder?\n\nIf you keep them, they come back when you add the folder again.",
      { title: "Remove folder", kind: "info", okLabel: "Delete them", cancelLabel: "Keep them" },
    );
    try {
      await api.removeRoot(root.id, purge);
      await load();
      onChanged();
    } catch (err) {
      onError(String(err));
    }
  }

  /** Where an ignored folder sits: its path inside the library folder that holds it. */
  function ignoredLabel(path: string): { root: string; rest: string } {
    const lower = path.toLowerCase();
    const root = roots
      .map((r) => r.path)
      .filter((r) => lower.startsWith(r.toLowerCase() + "\\"))
      .sort((a, b) => b.length - a.length)[0];
    return root ? { root, rest: path.slice(root.length + 1) } : { root: "", rest: path };
  }

  async function ignoreSubfolder() {
    const path = await open({
      directory: true,
      multiple: false,
      title: `Choose a subfolder for ${library.name} to ignore`,
      defaultPath: roots[0]?.path,
    });
    if (typeof path !== "string") return;
    try {
      ignored = await api.ignoreFolder(library.id, path);
      onChanged();
    } catch (err) {
      onError(String(err));
    }
  }

  async function stopIgnoring(path: string) {
    try {
      ignored = await api.unignoreFolder(library.id, path);
      onChanged();
    } catch (err) {
      onError(String(err));
    }
  }

  let rebuilding = $state<number | null>(null);

  async function rebuildCovers() {
    try {
      rebuilding = await api.rebuildCovers(library.id);
    } catch (err) {
      onError(String(err));
    }
  }

  async function deleteLibrary() {
    const confirmed = await ask(
      `Delete the library "${library.name}"?\n\nBilly will forget its folders, favorites and read status. Your files and folders are not touched.`,
      { title: "Delete library", kind: "warning", okLabel: "Delete library", cancelLabel: "Cancel" },
    );
    if (!confirmed) return;
    try {
      await api.deleteLibrary(library.id);
      onDeleted();
    } catch (err) {
      onError(String(err));
    }
  }

  function formatDate(ms: number | null) {
    return ms ? new Date(ms).toLocaleString() : "never";
  }
</script>

<dialog bind:this={dialog} onclose={onClose} aria-labelledby="settings-title">
  <header>
    <h2 id="settings-title">Library settings</h2>
    <button class="icon-btn" aria-label="Close" onclick={() => dialog.close()}><Icon name="x" /></button>
  </header>

  <form class="rename" onsubmit={rename}>
    <label for="library-name">Name</label>
    <input id="library-name" class="field" bind:value={name} />
    <button class="btn" type="submit" disabled={!name.trim() || name.trim() === library.name}>Rename</button>
  </form>

  <section>
    <div class="section-head">
      <h3>Folders</h3>
      <div class="actions">
        {#if roots.length > 1}
          <button class="btn" onclick={syncAll} disabled={roots.every((r) => r.id in scanning)}>
            <Icon name="sync" size={14} /> Sync all
          </button>
        {/if}
        <button class="btn primary" onclick={addFolder}><Icon name="plus" size={14} /> Add folder…</button>
      </div>
    </div>
    {#if roots.length === 0}
      <p class="muted">No folders yet. Billy reads the PDFs in the folders you add, and never changes them.</p>
    {:else}
      <ul>
        {#each roots as root (root.id)}
          <li>
            <Icon name="folder" />
            <div class="root">
              <span class="path">{root.path}</span>
              <span class="muted">
                {#if root.id in scanning}
                  Scanning… {scanning[root.id]} PDFs so far
                {:else if root.status === "offline"}
                  Offline: the folder can't be found. Its records are kept.
                {:else}
                  {root.fileCount} PDFs · last synced {formatDate(root.lastSyncedAt)}
                {/if}
              </span>
              <label class="switch" title="Sync when Billy starts, and whenever the folder changes while Billy runs">
                <input type="checkbox" checked={root.autoSync} onchange={(e) => setAutoSync(root, e.currentTarget.checked)} />
                Auto-sync
              </label>
            </div>
            <button class="btn" onclick={() => syncNow(root)} disabled={root.id in scanning}>
              <Icon name="sync" size={14} /> Sync now
            </button>
            <button class="btn" onclick={() => removeFolder(root)} disabled={root.id in scanning}>Remove</button>
          </li>
        {/each}
      </ul>
      <p class="hint muted">
        With Auto-sync off, a folder only changes in Billy when you choose Sync now. Files that disappear are kept
        as missing until you clean them up.
      </p>
    {/if}
  </section>

  {#if roots.length > 0}
    <section>
      <div class="section-head">
        <h3>Ignored folders</h3>
        <button class="btn" onclick={ignoreSubfolder}><Icon name="eye-off" size={14} /> Ignore a subfolder…</button>
      </div>
      {#if ignored.length === 0}
        <p class="muted hint">
          None. To leave a subfolder out, right-click it in the sidebar and choose Ignore this folder, or add it here.
        </p>
      {:else}
        <ul>
          {#each ignored as path (path)}
            {@const where = ignoredLabel(path)}
            <li>
              <Icon name="eye-off" />
              <div class="root">
                <span class="path">{where.rest}</span>
                {#if where.root}<span class="muted">in {where.root}</span>{/if}
              </div>
              <button class="btn" onclick={() => stopIgnoring(path)}>Stop ignoring</button>
            </li>
          {/each}
        </ul>
        <p class="hint muted">
          Billy leaves these folders out when it syncs. Favorites and read status are kept, in case you stop ignoring
          one.
        </p>
      {/if}
    </section>
  {/if}

  {#if roots.length > 0}
    <section>
      <div class="section-head">
        <h3>Covers</h3>
        <button class="btn" onclick={rebuildCovers} disabled={rebuilding !== null}>
          <Icon name="sync" size={14} /> Rebuild covers
        </button>
      </div>
      <p class="muted hint">
        {#if rebuilding !== null}
          Making {rebuilding} {rebuilding === 1 ? "cover" : "covers"} again, in the background.
        {:else}
          Makes the covers of this library's documents again, and reads their page count, title and author. Billy
          already remakes covers that have gone missing by itself.
        {/if}
      </p>
    </section>
  {/if}

  {#if missingCount > 0}
    <section class="missing">
      <span>
        {missingCount} {missingCount === 1 ? "file has" : "files have"} disappeared from disk.
      </span>
      <button class="btn" onclick={onCleanUp}>Clean up missing files…</button>
    </section>
  {/if}

  <footer>
    <button class="btn danger" onclick={deleteLibrary}>Delete library…</button>
    <button class="btn" onclick={() => dialog.close()}>Done</button>
  </footer>
</dialog>

<style>
  dialog {
    width: min(640px, calc(100vw - 48px));
    max-height: calc(100vh - 48px);
    overflow: auto;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--bg);
    color: var(--fg);
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.3);
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 0.35);
  }

  header,
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 18px;
  }

  footer {
    border-top: 1px solid var(--line);
  }

  h2 {
    margin: 0;
    font-size: 16px;
  }

  h3 {
    margin: 0;
    font-size: 13px;
  }

  .rename {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 18px 14px;
  }

  .rename input {
    flex: 1;
  }

  section {
    padding: 0 18px 16px;
  }

  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 0;
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    max-height: 40vh;
    overflow: auto;
  }

  li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 0;
    border-top: 1px solid var(--line);
    color: var(--muted);
  }

  .root {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }

  .path {
    color: var(--fg);
    overflow-wrap: anywhere;
    user-select: text;
  }

  .root .muted {
    font-size: 12px;
  }

  .actions {
    display: flex;
    gap: 8px;
  }

  .switch {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-top: 4px;
    color: var(--fg);
    font-size: 12px;
  }

  .switch input {
    margin: 0;
    accent-color: var(--accent);
  }

  .hint {
    margin: 8px 0 0;
    font-size: 12px;
  }

  .missing {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin: 0 18px 16px;
    padding: 10px 12px;
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--danger) 10%, transparent);
  }
</style>
