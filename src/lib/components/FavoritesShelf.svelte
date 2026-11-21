<script lang="ts">
  // The favorites pinned in the current folder, shown as a shelf above the documents.
  import type { Doc } from "$lib/api";
  import { displayName } from "$lib/view";
  import Cover from "./Cover.svelte";
  import Icon from "./Icon.svelte";

  interface Props {
    docs: Doc[];
    total: number;
    open: boolean;
    includeSubfolders: boolean;
    selected: string | null;
    onToggleOpen: () => void;
    onToggleSubfolders: () => void;
    onSelect: (key: string) => void;
    onOpen: (doc: Doc) => void;
    onContextMenu: (event: MouseEvent, doc: Doc) => void;
  }

  let {
    docs,
    total,
    open,
    includeSubfolders,
    selected,
    onToggleOpen,
    onToggleSubfolders,
    onSelect,
    onOpen,
    onContextMenu,
  }: Props = $props();
</script>

<section class="shelf" aria-label="Favorites">
  <header>
    <button class="title" aria-expanded={open} onclick={onToggleOpen}>
      <Icon name={open ? "chevron-down" : "chevron-right"} size={12} />
      <span class="star"><Icon name="star" size={14} filled /></span>
      Favorites
      <span class="muted">{docs.length}</span>
    </button>
    {#if open}
      <button
        class="toggle"
        class:on={includeSubfolders}
        aria-pressed={includeSubfolders}
        title="Also show favorites pinned in subfolders"
        onclick={onToggleSubfolders}
      >
        <Icon name="subfolders" size={14} /> From subfolders
      </button>
    {/if}
  </header>

  {#if open}
    {#if docs.length === 0}
      <p class="muted empty">
        No favorites directly in this folder.
        {#if total > 0}{total} in subfolders; turn on "From subfolders" to see them.{/if}
      </p>
    {:else}
      <div class="strip" role="listbox" aria-label="Favorites">
        {#each docs as doc (doc.fileId)}
          <div
            class="item"
            class:selected={selected === `d${doc.fileId}`}
            role="option"
            aria-selected={selected === `d${doc.fileId}`}
            title={doc.path}
            onclick={() => onSelect(`d${doc.fileId}`)}
            ondblclick={() => onOpen(doc)}
            oncontextmenu={(e) => {
              e.preventDefault();
              onSelect(`d${doc.fileId}`);
              onContextMenu(e, doc);
            }}
            onkeydown={(e) => e.key === "Enter" && onOpen(doc)}
            tabindex="0"
          >
            <div class="cover-box"><Cover {doc} /></div>
            <span class="name">{displayName(doc)}</span>
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</section>

<style>
  .shelf {
    flex: none;
    padding: 6px 16px 10px;
    border-bottom: 1px solid var(--line);
    background: color-mix(in srgb, var(--panel) 45%, var(--bg));
  }

  header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .title {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 6px 0 2px;
    border: 0;
    border-radius: var(--radius);
    background: transparent;
    font-weight: 600;
  }

  .title:hover {
    background: var(--hover);
  }

  .star {
    display: grid;
    color: var(--accent);
  }

  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 24px;
    padding: 0 8px;
    border: 1px solid transparent;
    border-radius: var(--radius);
    background: transparent;
    color: var(--muted);
    font-size: 12px;
  }

  .toggle:hover {
    background: var(--hover);
    color: var(--fg);
  }

  .toggle.on {
    border-color: color-mix(in srgb, var(--accent) 40%, transparent);
    background: var(--selection);
    color: var(--fg);
  }

  .empty {
    margin: 6px 0 0 22px;
    font-size: 12px;
  }

  .strip {
    display: flex;
    gap: 6px;
    padding-top: 8px;
    overflow-x: auto;
  }

  .item {
    display: flex;
    flex-direction: column;
    flex: none;
    width: 92px;
    padding: 6px;
    border-radius: var(--radius);
    font-size: 9px;
    cursor: default;
    outline-offset: -2px;
  }

  .item:hover {
    background: var(--hover);
  }

  .item.selected {
    background: var(--selection);
  }

  .cover-box {
    display: flex;
    align-items: flex-end;
    justify-content: center;
    width: 80px;
    height: 112px;
  }

  .name {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
    padding-top: 4px;
    font-size: 11px;
    line-height: 1.25;
    overflow-wrap: anywhere;
  }
</style>
