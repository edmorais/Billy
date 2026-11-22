<script lang="ts">
  // Lists files that disappeared from disk and, after the user confirms, forgets them.
  import { api, type Library, type MissingFile } from "$lib/api";
  import { formatDate } from "$lib/view";
  import { openDialog } from "$lib/modal.svelte";
  import Icon from "./Icon.svelte";

  interface Props {
    library: Library;
    onClose: () => void;
    onDone: (count: number) => void;
    onError: (message: string) => void;
  }

  let { library, onClose, onDone, onError }: Props = $props();

  let files = $state<MissingFile[]>([]);
  let chosen = $state(new Set<number>());
  let loading = $state(true);
  let working = $state(false);
  let dialog: HTMLDialogElement;

  let withState = $derived(files.filter((f) => chosen.has(f.fileId) && (f.favorite || f.read || f.annotated)).length);

  $effect(() => openDialog(dialog));

  $effect(() => {
    api.listMissing(library.id).then(
      (list) => {
        files = list;
        chosen = new Set(list.map((f) => f.fileId));
        loading = false;
      },
      (err) => onError(String(err)),
    );
  });

  function toggle(id: number) {
    const next = new Set(chosen);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    chosen = next;
  }

  async function forget() {
    working = true;
    try {
      await api.cleanUpMissing(library.id, [...chosen]);
      onDone(chosen.size);
      dialog.close();
    } catch (err) {
      onError(String(err));
    } finally {
      working = false;
    }
  }
</script>

<dialog bind:this={dialog} onclose={onClose} aria-labelledby="cleanup-title">
  <header>
    <h2 id="cleanup-title">Clean up missing files</h2>
    <button class="icon-btn" aria-label="Close" onclick={() => dialog.close()}><Icon name="x" /></button>
  </header>

  <div class="body">
    {#if loading}
      <p class="muted">Looking for missing files…</p>
    {:else if files.length === 0}
      <p class="muted">Nothing to clean up. Every document in "{library.name}" is where Billy expects it.</p>
    {:else}
      <p>
        These files are no longer on disk. Billy kept their records in case they come back. Forgetting
        them deletes Billy's records for them in "{library.name}", including favorites, read status and
        annotated copies.
      </p>
      <ul>
        {#each files as file (file.fileId)}
          <li>
            <label>
              <input type="checkbox" checked={chosen.has(file.fileId)} onchange={() => toggle(file.fileId)} />
              <span class="path">{file.path}</span>
            </label>
            <span class="tags muted">
              {#if file.favorite}<span title="Favorite"><Icon name="star" size={12} filled /></span>{/if}
              {#if file.read}<span title="Read"><Icon name="check" size={12} /></span>{/if}
              {#if file.annotated}<span>annotated</span>{/if}
              <span>missing since {formatDate(file.missingSince)}</span>
            </span>
          </li>
        {/each}
      </ul>
      {#if withState > 0}
        <p class="warning">
          {withState} of the chosen files {withState === 1 ? "has" : "have"} favorites, read status or annotations
          that will be lost. This can't be undone.
        </p>
      {/if}
    {/if}
  </div>

  <footer>
    <button class="btn" onclick={() => dialog.close()}>Cancel</button>
    <button class="btn danger" disabled={chosen.size === 0 || working} onclick={forget}>
      Forget {chosen.size} {chosen.size === 1 ? "file" : "files"}
    </button>
  </footer>
</dialog>

<style>
  dialog {
    width: min(720px, calc(100vw - 48px));
    padding: 0;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--bg);
    color: var(--fg);
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.3);
  }


  header,
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 18px;
  }

  footer {
    justify-content: flex-end;
    gap: 8px;
    border-top: 1px solid var(--line);
  }

  h2 {
    margin: 0;
    font-size: 16px;
  }

  .body {
    padding: 0 18px 12px;
  }

  .body p {
    margin: 0 0 10px;
  }

  ul {
    max-height: 45vh;
    margin: 0;
    padding: 0;
    overflow: auto;
    list-style: none;
    border-top: 1px solid var(--line);
  }

  li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 0;
    border-bottom: 1px solid var(--line);
  }

  label {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
  }

  .path {
    overflow-wrap: anywhere;
    user-select: text;
  }

  .tags {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
    font-size: 12px;
  }

  .warning {
    margin-top: 10px !important;
    color: var(--danger);
  }
</style>
