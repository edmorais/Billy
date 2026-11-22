<script lang="ts" module>
  import type { IconName } from "./Icon.svelte";

  export interface NavPage {
    id: string;
    label: string;
    icon: IconName;
    /** A count shown next to the label, such as missing files. */
    badge?: number;
  }
</script>

<script lang="ts">
  // The list of sections on the left of a settings dialog. One section shows at a time.
  // Up and Down move between sections, like the tabs they are.
  import Icon from "./Icon.svelte";

  let {
    pages,
    current = $bindable(),
    label,
  }: { pages: NavPage[]; current: string; label: string } = $props();

  let buttons: HTMLButtonElement[] = $state([]);

  function onkeydown(event: KeyboardEvent) {
    const i = pages.findIndex((p) => p.id === current);
    const next =
      event.key === "ArrowDown" ? (i + 1) % pages.length
      : event.key === "ArrowUp" ? (i - 1 + pages.length) % pages.length
      : event.key === "Home" ? 0
      : event.key === "End" ? pages.length - 1
      : -1;
    if (next < 0) return;
    event.preventDefault();
    current = pages[next].id;
    buttons[next]?.focus();
  }
</script>

<nav aria-label={label}>
  <div role="tablist" aria-orientation="vertical" tabindex="-1" {onkeydown}>
    {#each pages as page, i (page.id)}
      <button
        bind:this={buttons[i]}
        role="tab"
        id="tab-{page.id}"
        aria-selected={current === page.id}
        aria-controls="panel-{page.id}"
        tabindex={current === page.id ? 0 : -1}
        class:on={current === page.id}
        onclick={() => (current = page.id)}
      >
        <Icon name={page.icon} size={15} />
        <span class="label">{page.label}</span>
        {#if page.badge}<span class="badge">{page.badge}</span>{/if}
      </button>
    {/each}
  </div>
</nav>

<style>
  nav {
    flex: none;
    width: 190px;
    padding: 10px 8px;
    border-right: 1px solid var(--line);
    background: var(--panel);
    overflow-y: auto;
  }

  [role="tablist"] {
    display: flex;
    flex-direction: column;
    gap: 2px;
    outline: none;
  }

  button {
    display: flex;
    align-items: center;
    gap: 9px;
    height: 32px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--radius);
    background: transparent;
    color: var(--muted);
    text-align: left;
  }

  button:hover {
    background: var(--hover);
    color: var(--fg);
  }

  button.on {
    background: var(--selection);
    color: var(--fg);
    font-weight: 600;
  }

  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .badge {
    font-size: 11px;
    font-weight: 600;
    color: var(--danger);
    font-variant-numeric: tabular-nums;
  }
</style>
