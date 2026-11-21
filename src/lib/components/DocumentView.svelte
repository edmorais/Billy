<script lang="ts" module>
  import type { Doc, FolderNode } from "$lib/api";

  export type Item =
    | { key: string; kind: "folder"; folder: FolderNode }
    | { key: string; kind: "doc"; doc: Doc };

  export interface Group {
    key: string;
    /** `null` for an ungrouped list. */
    label: string | null;
    items: Item[];
  }
</script>

<script lang="ts">
  // A virtualized grid or list: only the rows on screen are drawn, so 5,000 documents scroll smoothly.
  import { displayName, formatDate, formatSize, type SortField, type ViewMode } from "$lib/view";
  import Cover from "./Cover.svelte";
  import Icon from "./Icon.svelte";

  interface Props {
    mode: ViewMode;
    groups: Group[];
    selected: string | null;
    sort: SortField;
    desc: boolean;
    emptyMessage: string;
    onSelect: (key: string) => void;
    onOpen: (item: Item) => void;
    onSort: (field: SortField) => void;
    /** Called with the documents on screen that still need a thumbnail. */
    onVisible: (fileIds: number[]) => void;
    /** Right-click, the Menu key or Shift+F10 on an item. */
    onContextMenu: (point: { x: number; y: number }, item: Item) => void;
  }

  let { mode, groups, selected, sort, desc, emptyMessage, onSelect, onOpen, onSort, onVisible, onContextMenu }: Props =
    $props();

  type Row =
    | { kind: "header"; label: string; count: number; top: number; height: number }
    | { kind: "items"; items: Item[]; chips: boolean; top: number; height: number };

  const PAD = 16;
  const GAP = 8;
  const HEADER = 36;
  /** In the grids, folders are compact chips rather than full-size tiles. */
  const CHIP_W = 220;
  const CHIP_H = 44;
  const coverWidths = { "grid-s": 104, "grid-m": 144, "grid-l": 196 } as const;

  let scroller: HTMLDivElement;
  let width = $state(0);
  let height = $state(0);
  let scrollTop = $state(0);

  let isGrid = $derived(mode.startsWith("grid"));
  let coverW = $derived(isGrid ? coverWidths[mode as keyof typeof coverWidths] : 0);
  let tileW = $derived(coverW + 16);
  let rowH = $derived(isGrid ? Math.round(coverW * 1.42) + 8 + 52 : mode === "details" ? 52 : 28);
  let cols = $derived(isGrid ? Math.max(1, Math.floor((width - 2 * PAD + GAP) / (tileW + GAP))) : 1);
  let chipCols = $derived(isGrid ? Math.max(1, Math.floor((width - 2 * PAD + GAP) / (CHIP_W + GAP))) : 1);

  let rows = $derived.by(() => {
    const out: Row[] = [];
    let top = isGrid ? PAD : 0;
    const gap = isGrid ? GAP : 0;
    for (const group of groups) {
      if (group.label !== null) {
        out.push({ kind: "header", label: group.label, count: group.items.length, top, height: HEADER });
        top += HEADER;
      }
      const folderItems = group.items.filter((i) => i.kind === "folder");
      const docItems = group.items.filter((i) => i.kind === "doc");
      const chips = isGrid && folderItems.length > 0;
      const runs: [Item[], number, number, boolean][] = [
        [folderItems, chips ? chipCols : cols, chips ? CHIP_H : rowH, chips],
        [docItems, cols, rowH, false],
      ];
      for (const [list, perRow, h, isChips] of runs) {
        for (let i = 0; i < list.length; i += perRow) {
          out.push({ kind: "items", items: list.slice(i, i + perRow), chips: isChips, top, height: h });
          top += h + gap;
        }
        if (isChips && list.length) top += GAP;
      }
    }
    return out;
  });

  let totalHeight = $derived(rows.length ? rows[rows.length - 1].top + rows[rows.length - 1].height + PAD : 0);
  let itemCount = $derived(groups.reduce((n, g) => n + g.items.length, 0));

  /** Rows that overlap the visible area, plus a few above and below. */
  let visible = $derived.by(() => {
    const margin = rowH * 2;
    const from = scrollTop - margin;
    const to = scrollTop + height + margin;
    let lo = 0;
    let hi = rows.length;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (rows[mid].top + rows[mid].height < from) lo = mid + 1;
      else hi = mid;
    }
    const out: Row[] = [];
    for (let i = lo; i < rows.length && rows[i].top < to; i++) out.push(rows[i]);
    return out;
  });

  // Ask for thumbnails of what is on screen first.
  $effect(() => {
    const ids: number[] = [];
    for (const row of visible) {
      if (row.kind !== "items") continue;
      for (const item of row.items) {
        if (item.kind === "doc" && item.doc.thumbStatus === "pending") ids.push(item.doc.fileId);
      }
    }
    if (ids.length) onVisible(ids);
  });

  /** Where each item sits, for keyboard navigation. */
  let positions = $derived.by(() => {
    const map = new Map<string, { row: number; col: number }>();
    rows.forEach((row, r) => {
      if (row.kind === "items") row.items.forEach((item, c) => map.set(item.key, { row: r, col: c }));
    });
    return map;
  });

  let flat = $derived(groups.flatMap((g) => g.items));

  function itemAt(row: number, col: number): Item | undefined {
    const r = rows[row];
    return r?.kind === "items" ? r.items[Math.min(col, r.items.length - 1)] : undefined;
  }

  function nextItemRow(from: number, step: 1 | -1): number {
    for (let r = from + step; r >= 0 && r < rows.length; r += step) if (rows[r].kind === "items") return r;
    return -1;
  }

  function reveal(key: string) {
    const pos = positions.get(key);
    if (!pos || !scroller) return;
    const row = rows[pos.row];
    const headerAbove = pos.row > 0 && rows[pos.row - 1].kind === "header" ? HEADER : 0;
    if (row.top - headerAbove < scroller.scrollTop) scroller.scrollTop = row.top - headerAbove - (isGrid ? PAD : 0);
    else if (row.top + row.height > scroller.scrollTop + height)
      scroller.scrollTop = row.top + row.height - height + (isGrid ? PAD : 0);
  }

  function move(key: string | undefined) {
    if (!key) return;
    onSelect(key);
    reveal(key);
  }

  function onKeydown(event: KeyboardEvent) {
    if (flat.length === 0) return;
    const pos = selected ? positions.get(selected) : undefined;
    const index = selected ? flat.findIndex((i) => i.key === selected) : -1;
    switch (event.key) {
      case "ArrowRight":
        move(flat[Math.min(index + 1, flat.length - 1)]?.key);
        break;
      case "ArrowLeft":
        move(flat[Math.max(index - 1, 0)]?.key);
        break;
      case "ArrowDown":
      case "ArrowUp": {
        if (!pos) {
          move(flat[0].key);
          break;
        }
        const r = nextItemRow(pos.row, event.key === "ArrowDown" ? 1 : -1);
        if (r >= 0) move(itemAt(r, pos.col)?.key);
        break;
      }
      case "Home":
        move(flat[0].key);
        break;
      case "End":
        move(flat[flat.length - 1].key);
        break;
      case "Enter": {
        const item = flat[index];
        if (item) onOpen(item);
        break;
      }
      case "ContextMenu":
      case "F10": {
        if (event.key === "F10" && !event.shiftKey) return;
        const item = flat[index];
        const element = item && document.getElementById(item.key);
        if (!item || !element) return;
        const rect = element.getBoundingClientRect();
        onContextMenu({ x: rect.left + Math.min(rect.width / 2, 40), y: rect.top + Math.min(rect.height / 2, 40) }, item);
        break;
      }
      default:
        return;
    }
    event.preventDefault();
  }

  // Back to the top when the content changes, for example after opening a folder.
  $effect(() => {
    void groups;
    if (scroller) scroller.scrollTop = 0;
  });

  const columns: { field: SortField | null; label: string; align?: "end" }[] = [
    { field: "name", label: "Name" },
    { field: "title", label: "Title" },
    { field: "author", label: "Author" },
    { field: "pages", label: "Pages", align: "end" },
    { field: "size", label: "Size", align: "end" },
    { field: "modified", label: "Modified" },
    { field: "added", label: "Added" },
  ];
</script>

{#snippet badges(doc: Doc)}
  {#if doc.missing}
    <span class="badge warn" title="Missing: this file is no longer on disk"><Icon name="alert" size={11} /></span>
  {:else if doc.offline}
    <span class="badge warn" title="Offline: this file's folder is unavailable"><Icon name="alert" size={11} /></span>
  {/if}
  {#if doc.favorite}<span class="badge fav" title="Favorite"><Icon name="star" size={11} filled /></span>{/if}
  {#if doc.read}<span class="badge read" title="Read"><Icon name="check" size={11} /></span>{/if}
  {#if doc.annotated}<span class="badge read" title="Annotated in Billy"><Icon name="pen" size={11} /></span>{/if}
{/snippet}

<div class="view mode-{mode}" style:--cover-w="{coverW}px" style:--tile-w="{tileW}px" style:--row-h="{rowH}px">
  {#if mode === "details"}
    <div class="details-head" role="row">
      {#each columns as column (column.label)}
        <button
          class="col"
          class:end={column.align === "end"}
          class:sorted={column.field === sort}
          role="columnheader"
          aria-sort={column.field === sort ? (desc ? "descending" : "ascending") : "none"}
          onclick={() => column.field && onSort(column.field)}
        >
          {column.label}
          {#if column.field === sort}
            <Icon name={desc ? "chevron-down" : "chevron-up"} size={10} />
          {/if}
        </button>
      {/each}
    </div>
  {/if}

  <div
    class="scroller"
    bind:this={scroller}
    bind:clientWidth={width}
    bind:clientHeight={height}
    onscroll={() => (scrollTop = scroller.scrollTop)}
    onkeydown={onKeydown}
    tabindex="0"
    role="listbox"
    aria-label="Documents"
    aria-activedescendant={selected ?? undefined}
  >
    {#if itemCount === 0}
      <p class="empty">{emptyMessage}</p>
    {:else}
      <div class="spacer" style:height="{totalHeight}px">
        {#each visible as row (row.top)}
          {#if row.kind === "header"}
            <div class="group-header" style:transform="translateY({row.top}px)">
              <Icon name="folder" size={14} />
              <span class="group-label">{row.label}</span>
              <span class="muted">{row.count}</span>
            </div>
          {:else}
            <div class="row" style:transform="translateY({row.top}px)">
              {#each row.items as item (item.key)}
                <div
                  id={item.key}
                  class="item"
                  class:chip={row.chips}
                  class:selected={item.key === selected}
                  class:unavailable={item.kind === "doc" && (item.doc.missing || item.doc.offline)}
                  role="option"
                  tabindex="-1"
                  aria-selected={item.key === selected}
                  title={item.kind === "doc" ? item.doc.path : item.folder.name}
                  onclick={() => onSelect(item.key)}
                  ondblclick={() => onOpen(item)}
                  oncontextmenu={(e) => {
                    e.preventDefault();
                    onSelect(item.key);
                    onContextMenu({ x: e.clientX, y: e.clientY }, item);
                  }}
                  onkeydown={() => {}}
                >
                  {#if row.chips && item.kind === "folder"}
                    <span class="chip-icon"><Icon name="folder" size={18} /></span>
                    <span class="name">{item.folder.name}</span>
                    <span class="meta">{item.folder.total}</span>
                  {:else if isGrid && item.kind === "doc"}
                    <div class="cover-box"><Cover doc={item.doc} /></div>
                    <div class="caption">
                      <span class="name">{displayName(item.doc)}</span>
                      <span class="meta">
                        {@render badges(item.doc)}
                        {item.doc.pages ? `${item.doc.pages} pages` : formatSize(item.doc.size)}
                      </span>
                    </div>
                  {:else if mode === "details"}
                    {#if item.kind === "doc"}
                      <span class="cell name-cell">
                        <span class="mini"><Cover doc={item.doc} small /></span>
                        <span class="name">{displayName(item.doc)}</span>
                        {@render badges(item.doc)}
                      </span>
                      <span class="cell">{item.doc.title ?? ""}</span>
                      <span class="cell">{item.doc.author ?? ""}</span>
                      <span class="cell end">{item.doc.pages ?? ""}</span>
                      <span class="cell end">{formatSize(item.doc.size)}</span>
                      <span class="cell">{formatDate(item.doc.modified)}</span>
                      <span class="cell">{formatDate(item.doc.added)}</span>
                    {:else}
                      <span class="cell name-cell">
                        <span class="mini folder"><Icon name="folder" size={22} /></span>
                        <span class="name">{item.folder.name}</span>
                      </span>
                      <span class="cell muted">{item.folder.total} documents</span>
                    {/if}
                  {:else}
                    <Icon name={item.kind === "doc" ? "book" : "folder"} size={14} />
                    <span class="name">{item.kind === "doc" ? displayName(item.doc) : item.folder.name}</span>
                    {#if item.kind === "doc"}{@render badges(item.doc)}{/if}
                    <span class="meta">
                      {item.kind === "doc" ? formatSize(item.doc.size) : `${item.folder.total} documents`}
                    </span>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }

  .scroller {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    outline: none;
  }

  .spacer {
    position: relative;
  }

  .empty {
    margin: 20vh auto 0;
    max-width: 360px;
    text-align: center;
    color: var(--muted);
  }

  .row,
  .group-header {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
  }

  .group-header {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 16px;
    color: var(--muted);
  }

  .group-label {
    font-family: var(--font-display);
    color: var(--fg);
    font-weight: 600;
  }

  .item {
    border-radius: var(--radius);
    cursor: default;
  }

  .item:hover {
    background: var(--hover);
  }

  .item.selected {
    background: var(--selection);
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--muted);
    font-size: 12px;
    white-space: nowrap;
  }

  .badge {
    display: inline-grid;
    flex: none;
  }

  .badge.fav {
    color: var(--accent);
  }

  .badge.read {
    color: var(--muted);
  }

  .badge.warn {
    color: var(--danger);
  }

  .item.unavailable :global(.thumb),
  .item.unavailable :global(.cover),
  .item.unavailable .name {
    opacity: 0.45;
    filter: grayscale(1);
  }

  /* Grids */
  .mode-grid-s .row,
  .mode-grid-m .row,
  .mode-grid-l .row {
    display: flex;
    gap: 8px;
    padding: 0 16px;
  }

  .mode-grid-s .item,
  .mode-grid-m .item,
  .mode-grid-l .item {
    display: flex;
    flex-direction: column;
    width: var(--tile-w);
    height: var(--row-h);
    padding: 8px;
  }

  .mode-grid-s .item {
    font-size: 10px;
  }

  .mode-grid-m .item {
    font-size: 12px;
  }

  .mode-grid-l .item {
    font-size: 15px;
  }

  .cover-box {
    display: flex;
    align-items: flex-end;
    justify-content: center;
    width: var(--cover-w);
    height: calc(var(--cover-w) * 1.42);
  }

  .item.chip {
    flex-direction: row;
    align-items: center;
    gap: 8px;
    width: 220px;
    height: 44px;
    padding: 0 12px;
    border: 1px solid var(--line);
    background: var(--raised);
    font-size: 13px;
  }

  .item.chip:hover {
    background: color-mix(in srgb, var(--raised) 90%, var(--fg));
  }

  .item.chip.selected {
    border-color: var(--accent);
    background: var(--selection);
  }

  .chip-icon {
    display: grid;
    color: var(--accent);
  }

  .item.chip .name {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
  }

  .caption {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-top: 6px;
    font-size: 12px;
    line-height: 1.3;
  }

  .caption .name {
    font-family: var(--font-display);
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow-wrap: anywhere;
  }

  .caption .meta {
    font-size: 11px;
  }

  /* Details */
  .details-head,
  .mode-details .item {
    display: grid;
    grid-template-columns:
      minmax(220px, 2.2fr) minmax(120px, 1.5fr) minmax(100px, 1fr) 60px 76px 104px 104px;
    align-items: center;
    column-gap: 12px;
    padding: 0 16px;
  }

  .details-head {
    height: 30px;
    border-bottom: 1px solid var(--line);
  }

  .col {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 100%;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
    text-align: left;
  }

  .col.end {
    justify-content: flex-end;
  }

  .col.sorted,
  .col:hover {
    color: var(--fg);
  }

  .mode-details .item {
    height: var(--row-h);
    margin: 0 8px;
    padding: 0 8px;
  }

  .cell {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cell.end {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .name-cell {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .mini {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: none;
    width: 30px;
    height: 42px;
  }

  .mini.folder {
    display: grid;
    place-items: center;
    color: var(--accent);
  }

  /* Compact list */
  .mode-compact .item {
    display: flex;
    align-items: center;
    gap: 8px;
    height: var(--row-h);
    margin: 0 8px;
    padding: 0 8px;
    color: var(--muted);
  }

  .mode-compact .name {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    color: var(--fg);
  }
</style>
