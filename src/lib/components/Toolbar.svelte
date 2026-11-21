<script lang="ts">
  import { sortFields, viewModes, type ReadFilter, type SortField, type ViewSettings } from "$lib/view";
  import Icon from "./Icon.svelte";

  interface Crumb {
    id: number | null;
    name: string;
  }

  interface Props {
    crumbs: Crumb[];
    settings: ViewSettings;
    search: string;
    searchInput?: HTMLInputElement;
    onNavigate: (id: number | null) => void;
    onUp: () => void;
    onChange: (change: Partial<ViewSettings>) => void;
  }

  let {
    crumbs,
    settings,
    search = $bindable(),
    searchInput = $bindable(),
    onNavigate,
    onUp,
    onChange,
  }: Props = $props();
</script>

<div class="toolbar">
  <div class="row">
    <button class="icon-btn" title="Up one folder (Backspace)" aria-label="Up one folder" disabled={crumbs.length < 2} onclick={onUp}>
      <Icon name="arrow-up" />
    </button>
    <nav class="crumbs" aria-label="Location">
      {#each crumbs as crumb, i (crumb.id ?? "library")}
        {#if i > 0}<Icon name="chevron-right" size={12} />{/if}
        {#if i === crumbs.length - 1}
          <span class="crumb current" aria-current="location">{crumb.name}</span>
        {:else}
          <button class="crumb" onclick={() => onNavigate(crumb.id)}>{crumb.name}</button>
        {/if}
      {/each}
    </nav>
    <label class="search">
      <Icon name="search" size={14} />
      <input
        bind:this={searchInput}
        bind:value={search}
        placeholder="Search file names"
        aria-label="Search file names"
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
  </div>

  <div class="row controls">
    <button
      class="toggle"
      class:on={settings.includeSubfolders}
      aria-pressed={settings.includeSubfolders}
      title="Include documents in subfolders"
      onclick={() => onChange({ includeSubfolders: !settings.includeSubfolders })}
    >
      <Icon name="subfolders" /> Subfolders
    </button>
    {#if settings.includeSubfolders || search}
      <button
        class="toggle"
        class:on={settings.groupBySubfolder}
        aria-pressed={settings.groupBySubfolder}
        title="Group documents by subfolder"
        onclick={() => onChange({ groupBySubfolder: !settings.groupBySubfolder })}
      >
        <Icon name="group" /> Group by folder
      </button>
    {/if}

    <span class="sep"></span>

    <button
      class="toggle"
      class:on={settings.favoritesOnly}
      aria-pressed={settings.favoritesOnly}
      title="Show only favorites"
      onclick={() => onChange({ favoritesOnly: !settings.favoritesOnly })}
    >
      <Icon name="star" filled={settings.favoritesOnly} /> Favorites only
    </button>
    <select
      class="field"
      aria-label="Show"
      value={settings.filter}
      onchange={(e) => onChange({ filter: e.currentTarget.value as ReadFilter })}
    >
      <option value="all">All documents</option>
      <option value="unread">Unread</option>
      <option value="read">Read</option>
    </select>

    <span class="sep"></span>

    <select
      class="field"
      aria-label="Sort by"
      value={settings.sort}
      onchange={(e) => onChange({ sort: e.currentTarget.value as SortField })}
    >
      {#each sortFields as field (field.id)}
        <option value={field.id}>{field.label}</option>
      {/each}
    </select>
    <button
      class="icon-btn"
      title={settings.desc ? "Descending" : "Ascending"}
      aria-label={settings.desc ? "Sort descending" : "Sort ascending"}
      onclick={() => onChange({ desc: !settings.desc })}
    >
      <Icon name={settings.desc ? "sort-desc" : "sort-asc"} />
    </button>

    <span class="spacer"></span>

    <div class="segmented" role="radiogroup" aria-label="View">
      {#each viewModes as mode (mode.id)}
        <button
          class="icon-btn"
          class:on={settings.view === mode.id}
          role="radio"
          aria-checked={settings.view === mode.id}
          title={mode.label}
          aria-label={mode.label}
          onclick={() => onChange({ view: mode.id })}
        >
          <Icon name={mode.id} />
        </button>
      {/each}
    </div>
  </div>
</div>

<style>
  .toolbar {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 16px 8px;
    border-bottom: 1px solid var(--line);
  }

  .row {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .crumbs {
    display: flex;
    align-items: center;
    gap: 2px;
    flex: 1;
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
  }

  .crumb {
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 3px 6px;
    border: 0;
    border-radius: var(--radius);
    background: transparent;
    color: var(--muted);
    font-size: 15px;
  }

  button.crumb:hover {
    background: var(--hover);
    color: var(--fg);
  }

  .crumb.current {
    flex-shrink: 0;
    max-width: 50%;
    color: var(--fg);
    font-weight: 600;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 260px;
    height: 30px;
    padding: 0 4px 0 8px;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    background: var(--raised);
    color: var(--muted);
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
    width: 22px;
    height: 22px;
  }

  .controls {
    flex-wrap: wrap;
    row-gap: 4px;
  }

  .toggle {
    white-space: nowrap;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 10px 0 8px;
    border: 1px solid transparent;
    border-radius: var(--radius);
    background: transparent;
    color: var(--muted);
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

  .sep {
    width: 1px;
    height: 18px;
    margin: 0 6px;
    background: var(--line);
  }

  .spacer {
    flex: 1;
  }

  .segmented {
    display: flex;
    gap: 2px;
  }
</style>
