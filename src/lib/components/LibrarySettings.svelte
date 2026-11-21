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
  }

  let { library, scanning, version, onClose, onChanged, onDeleted, onError }: Props = $props();

  let roots = $state<Root[]>([]);
  let name = $state("");
  let dialog: HTMLDialogElement;

  async function load() {
    try {
      roots = await api.listRoots(library.id);
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
      <button class="btn primary" onclick={addFolder}><Icon name="plus" size={14} /> Add folder…</button>
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
                  {root.fileCount} PDFs · last scanned {formatDate(root.lastSyncedAt)}
                {/if}
              </span>
            </div>
            <button class="btn" onclick={() => removeFolder(root)} disabled={root.id in scanning}>Remove</button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <footer>
    <button class="btn danger" onclick={deleteLibrary}>Delete library…</button>
    <button class="btn" onclick={() => dialog.close()}>Done</button>
  </footer>
</dialog>

<style>
  dialog {
    width: min(640px, calc(100vw - 48px));
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
</style>
