<script lang="ts">
  // The toolbar of the History view: a way back, the title, a library filter and the view
  // buttons. The history is always in the order the documents were opened, so there is
  // nothing to sort.
  import type { Library } from "$lib/api";
  import { viewModes, type ViewMode } from "$lib/view";
  import Icon from "./Icon.svelte";

  interface Props {
    view: ViewMode;
    count: number;
    libraries: Library[];
    /** Shows only this library's documents; `null` is every library. */
    library: number | null;
    onView: (view: ViewMode) => void;
    onLibrary: (library: number | null) => void;
    onBack: () => void;
  }

  let { view, count, libraries, library, onView, onLibrary, onBack }: Props = $props();
</script>

<div class="bar">
  <button class="icon-btn" title="Back to the library (Backspace)" aria-label="Back to the library" onclick={onBack}>
    <Icon name="arrow-left" />
  </button>
  <Icon name="clock" />
  <h1 class="display">History</h1>
  <span class="muted">Newest first · up to {count}</span>
  <span class="spacer"></span>
  {#if libraries.length > 1}
    <select
      class="field"
      aria-label="Library"
      value={library ?? ""}
      onchange={(e) => onLibrary(e.currentTarget.value === "" ? null : Number(e.currentTarget.value))}
    >
      <option value="">All libraries</option>
      {#each libraries as lib (lib.id)}
        <option value={lib.id}>{lib.name}</option>
      {/each}
    </select>
  {/if}
  <div class="segmented" role="radiogroup" aria-label="View">
    {#each viewModes as mode (mode.id)}
      <button
        class="icon-btn"
        class:on={view === mode.id}
        role="radio"
        aria-checked={view === mode.id}
        title={mode.label}
        aria-label={mode.label}
        onclick={() => onView(mode.id)}
      >
        <Icon name={mode.id} />
      </button>
    {/each}
  </div>
</div>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 50px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--line);
    color: var(--muted);
  }

  h1 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--fg);
  }

  .muted {
    font-size: 12px;
  }

  .spacer {
    flex: 1;
  }

  .segmented {
    display: flex;
    gap: 2px;
  }
</style>
