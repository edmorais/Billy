<script lang="ts">
  import type { Doc, FolderNode, Library } from "$lib/api";
  import { collator, displayName } from "$lib/view";
  import Icon from "./Icon.svelte";
  import FolderTreeNode from "./FolderTreeNode.svelte";

  interface Props {
    libraries: Library[];
    libraryId: number | null;
    childrenOf: Map<number | null, FolderNode[]>;
    expanded: Set<number>;
    folderId: number | null;
    version: string;
    onSelectLibrary: (id: number) => void;
    onSelectFolder: (id: number | null) => void;
    onToggle: (id: number) => void;
    onCreateLibrary: (name: string) => Promise<boolean>;
    onOpenSettings: () => void;
    onFolderMenu: (event: MouseEvent, node: FolderNode) => void;
    onLibraryMenu: (event: MouseEvent, library: Library) => void;
    onOpenAppSettings: () => void;
    /** The last opened documents in any library, newest first; empty hides the section. */
    history: Doc[];
    /** The History view is showing. */
    historyActive: boolean;
    /** The History section shows only its heading. */
    historyCollapsed: boolean;
    onToggleHistory: () => void;
    onOpenDoc: (doc: Doc) => void;
    onDocMenu: (event: MouseEvent, doc: Doc) => void;
    onShowHistory: () => void;
  }

  let {
    libraries,
    libraryId,
    childrenOf,
    expanded,
    folderId,
    version,
    onSelectLibrary,
    onSelectFolder,
    onToggle,
    onCreateLibrary,
    onOpenSettings,
    onFolderMenu,
    onLibraryMenu,
    onOpenAppSettings,
    history,
    historyActive,
    historyCollapsed,
    onToggleHistory,
    onOpenDoc,
    onDocMenu,
    onShowHistory,
  }: Props = $props();

  function libraryMenu(event: MouseEvent, library: Library) {
    event.preventDefault();
    onLibraryMenu(event, library);
  }

  let creating = $state(false);
  let newName = $state("");
  let roots = $derived(
    [...(childrenOf.get(null) ?? [])].sort((a, b) => collator.compare(a.name, b.name)),
  );
  let current = $derived(libraries.find((l) => l.id === libraryId));

  async function create(event: SubmitEvent) {
    event.preventDefault();
    if (await onCreateLibrary(newName)) {
      newName = "";
      creating = false;
    }
  }

  function focus(node: HTMLInputElement) {
    node.focus();
  }
</script>

<aside>
  <div class="sections">
  <div class="section-head">
    <h2>Libraries</h2>
    <button class="icon-btn" title="New library" aria-label="New library" onclick={() => (creating = !creating)}>
      <Icon name="plus" size={14} />
    </button>
  </div>
  <ul class="libraries">
    {#each libraries as library (library.id)}
      <li>
        <button
          class="library"
          class:current={library.id === libraryId}
          onclick={() => onSelectLibrary(library.id)}
          oncontextmenu={(e) => libraryMenu(e, library)}
        >
          <span class="name">{library.name}</span>
          <span class="count">{library.documentCount}</span>
        </button>
      </li>
    {/each}
  </ul>
  {#if creating}
    <form class="create" onsubmit={create}>
      <input
        class="field"
        bind:value={newName}
        placeholder="Library name"
        aria-label="Library name"
        use:focus
        onkeydown={(e) => e.key === "Escape" && (creating = false)}
      />
      <button class="btn primary" type="submit" disabled={!newName.trim()}>Add</button>
    </form>
  {/if}

  {#if current}
    <div class="section-head">
      <h2>Folders</h2>
      <button class="icon-btn" title="Library settings" aria-label="Library settings" onclick={onOpenSettings}>
        <Icon name="settings" size={14} />
      </button>
    </div>
    <div class="tree">
      <button
        class="all"
        class:current={folderId === null}
        onclick={() => onSelectFolder(null)}
        oncontextmenu={(e) => libraryMenu(e, current)}
      >
        <Icon name="shelf" size={14} />
        <span class="name">All of {current.name}</span>
        <span class="count">{current.documentCount}</span>
      </button>
      {#if current.rootCount === 0}
        <p class="hint">
          No folders yet.
          <button class="link" onclick={onOpenSettings}>Add a folder</button>
        </p>
      {:else}
        <ul role="tree" aria-label="Folders">
          {#each roots as root (root.id)}
            <FolderTreeNode
              node={root}
              depth={0}
              {childrenOf}
              {expanded}
              currentId={folderId}
              onSelect={onSelectFolder}
              {onToggle}
              onContextMenu={onFolderMenu}
            />
          {/each}
        </ul>
      {/if}
    </div>
  {/if}

  {#if history.length > 0}
    <!-- Pushed to the bottom of the sidebar when there is room. -->
    <section class="history-section" aria-label="History">
    <div class="section-head">
      <h2>
        <button
          class="collapse"
          aria-expanded={!historyCollapsed}
          aria-controls="recent-list"
          title={historyCollapsed ? "Show the recent documents" : "Hide the recent documents"}
          onclick={onToggleHistory}
        >
          <Icon name={historyCollapsed ? "chevron-right" : "chevron-down"} size={12} />
          History
        </button>
      </h2>
      <button
        class="icon-btn"
        class:on={historyActive}
        title="Show the full history"
        aria-label="Show the full history"
        aria-pressed={historyActive}
        onclick={onShowHistory}
      >
        <Icon name="arrow-right" size={14} />
      </button>
    </div>
    <ul class="history" id="recent-list" aria-label="Recently opened" hidden={historyCollapsed}>
      {#each history.slice(0, 5) as doc (doc.fileId)}
        <li>
          <button
            class="recent"
            title={`${doc.path}${doc.libraryId !== libraryId ? `\nIn ${libraries.find((l) => l.id === doc.libraryId)?.name ?? "another library"}` : ""}`}
            onclick={() => onOpenDoc(doc)}
            oncontextmenu={(e) => {
              e.preventDefault();
              onDocMenu(e, doc);
            }}
          >
            <Icon name="book" size={14} />
            <span class="name">{displayName(doc)}</span>
          </button>
        </li>
      {/each}
    </ul>
    </section>
  {/if}
  </div>

  <div class="foot">
    <button class="settings" onclick={onOpenAppSettings}>
      <Icon name="settings" size={14} /> Settings
    </button>
    <span class="version">Billy {version}</span>
  </div>
</aside>

<style>
  aside {
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 12px 8px;
    background: var(--panel);
    border-right: 1px solid var(--line);
  }

  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 4px 6px 8px;
  }

  .sections > .section-head:first-child {
    padding-top: 6px;
  }

  h2 {
    margin: 0;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--muted);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  /* Libraries, folders and history scroll together, as one column, when they do not fit. */
  .sections {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  .history-section {
    margin-top: auto;
  }

  /* The History heading is also its collapse toggle. */
  .collapse {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-left: -4px;
    padding: 2px 4px;
    border: 0;
    border-radius: var(--radius);
    background: transparent;
    color: inherit;
    font: inherit;
    letter-spacing: inherit;
    text-transform: inherit;
  }

  .collapse:hover {
    background: var(--hover);
    color: var(--fg);
  }

  .library,
  .all {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    height: 28px;
    padding: 0 8px;
    border: 0;
    border-radius: var(--radius);
    background: transparent;
    text-align: left;
  }

  .library:hover,
  .all:hover {
    background: var(--hover);
  }

  .library.current,
  .all.current {
    background: var(--selection);
  }

  .library.current .name {
    font-weight: 600;
  }

  .all {
    color: var(--muted);
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--fg);
  }

  .count {
    font-size: 11px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }

  .history {
    flex: none;
  }

  .recent {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    width: 100%;
    height: 28px;
    padding: 0 8px;
    border: 0;
    border-radius: var(--radius);
    background: transparent;
    text-align: left;
  }

  .recent:hover {
    background: var(--hover);
  }

  .create {
    display: flex;
    gap: 6px;
    padding: 4px;
  }

  .create input {
    flex: 1;
    min-width: 0;
  }

  .tree {
    flex: none;
  }

  .hint {
    padding: 8px;
    color: var(--muted);
  }

  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    text-decoration: underline;
  }

  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-top: 8px;
    padding: 8px 4px 0;
    border-top: 1px solid var(--line);
  }

  .settings {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 8px;
    border: 0;
    border-radius: var(--radius);
    background: transparent;
    color: var(--muted);
  }

  .settings:hover {
    background: var(--hover);
    color: var(--fg);
  }

  .version {
    font-size: 11px;
    color: var(--muted);
  }
</style>
