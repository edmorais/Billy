<script lang="ts">
  // Billy's own title bar, in place of the Windows one. The left part continues the sidebar;
  // the rest continues the main area. Drag it to move the window; double-click to maximize.
  import { onDestroy, onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Icon from "./Icon.svelte";

  interface Props {
    reading?: boolean;
    /** Shows the file-name search, just before the window buttons. */
    showSearch?: boolean;
    search?: string;
    searchInput?: HTMLInputElement;
  }

  let { reading = false, showSearch = false, search = $bindable(""), searchInput = $bindable() }: Props = $props();

  const win = getCurrentWindow();
  let maximized = $state(false);
  let focused = $state(true);
  let unlisten: (() => void)[] = [];

  async function refresh() {
    try {
      maximized = await win.isMaximized();
    } catch {
      maximized = false;
    }
  }

  onMount(async () => {
    refresh();
    try {
      unlisten.push(await win.onResized(refresh));
      unlisten.push(await win.onFocusChanged(({ payload }) => (focused = payload)));
    } catch {
      // Not in a Tauri window (for example the development mock).
    }
  });

  onDestroy(() => unlisten.forEach((fn) => fn()));
</script>

<header class="titlebar" class:reading class:blurred={!focused} data-tauri-drag-region>
  <div class="side" data-tauri-drag-region>
    <img class="mark" src="/billy-mark.svg" alt="" width="15" height="19" draggable="false" />
    <span class="name display">Billy</span>
  </div>
  <div class="rest" data-tauri-drag-region></div>
  {#if showSearch}
    <label class="search">
      <Icon name="search" size={14} />
      <input
        bind:this={searchInput}
        bind:value={search}
        placeholder="Search file names"
        aria-label="Search file names"
        title="Search file names (Ctrl+F)"
        onkeydown={(e) => {
          if (e.key === "Escape") {
            search = "";
            (e.currentTarget as HTMLInputElement).blur();
          }
        }}
      />
      {#if search}
        <button class="icon-btn clear" aria-label="Clear search" onclick={() => (search = "")}>
          <Icon name="x" size={12} />
        </button>
      {/if}
    </label>
  {/if}
  <div class="controls">
    <button class="control" aria-label="Minimize" title="Minimize" onclick={() => win.minimize()}>
      <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true"><path d="M0 5h10" stroke="currentColor" /></svg>
    </button>
    <button
      class="control"
      aria-label={maximized ? "Restore" : "Maximize"}
      title={maximized ? "Restore" : "Maximize"}
      onclick={async () => {
        await win.toggleMaximize();
        refresh();
      }}
    >
      {#if maximized}
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
          <path d="M2.5 2.5V.5h7v7h-2M.5 2.5h7v7h-7z" fill="none" stroke="currentColor" />
        </svg>
      {:else}
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
          <rect x=".5" y=".5" width="9" height="9" fill="none" stroke="currentColor" />
        </svg>
      {/if}
    </button>
    <button class="control close" aria-label="Close" title="Close" onclick={() => win.close()}>
      <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true"><path d="M0 0l10 10M10 0L0 10" stroke="currentColor" /></svg>
    </button>
  </div>
</header>

<style>
  .titlebar {
    display: flex;
    flex: none;
    height: var(--titlebar-h);
    background: var(--bg);
    user-select: none;
  }

  /* While reading, the bar takes the reader's colour from edge to edge. */
  .titlebar.reading,
  .titlebar.reading .side {
    background: var(--panel);
    border-right-color: transparent;
  }

  .titlebar.reading {
    border-bottom: 1px solid var(--line);
  }

  .side {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: none;
    width: var(--sidebar-w, 260px);
    padding: 0 14px;
    border-right: 1px solid var(--line);
    background: var(--panel);
  }

  /* Clicks fall through to the drag region underneath. */
  .side > * {
    pointer-events: none;
  }

  .name {
    font-size: 14px;
    font-weight: 600;
  }

  .rest {
    flex: 1;
    min-width: 24px;
  }

  .search {
    display: flex;
    align-items: center;
    align-self: center;
    gap: 6px;
    flex: 0 1 260px;
    min-width: 120px;
    height: 26px;
    margin-right: 10px;
    padding: 0 3px 0 8px;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    background: var(--raised);
    color: var(--muted);
    user-select: text;
  }

  .search:focus-within {
    border-color: var(--accent);
  }

  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--fg);
  }

  .clear {
    width: 20px;
    height: 20px;
  }

  .blurred .name,
  .blurred .control {
    opacity: 0.55;
  }

  .controls {
    display: flex;
  }

  .control {
    display: grid;
    place-items: center;
    width: 46px;
    height: 100%;
    padding: 0;
    border: 0;
    border-radius: 0;
    background: transparent;
    color: var(--fg);
  }

  .control:hover {
    background: var(--hover);
  }

  .control.close:hover {
    background: #c42b1c;
    color: #ffffff;
  }

  .control svg {
    shape-rendering: crispEdges;
  }
</style>
