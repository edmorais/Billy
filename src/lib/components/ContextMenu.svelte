<script lang="ts" module>
  import type { IconName } from "./Icon.svelte";

  export type MenuEntry =
    | { label: string; icon?: IconName; filled?: boolean; action: () => void; disabled?: boolean }
    | "separator";
</script>

<script lang="ts">
  // A right-click menu. Opens at the pointer, stays inside the window, and works with the
  // keyboard: arrows to move, Enter to choose, Escape to close.
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";

  interface Props {
    x: number;
    y: number;
    entries: MenuEntry[];
    onClose: () => void;
  }

  let { x, y, entries, onClose }: Props = $props();

  let menu: HTMLDivElement;
  let left = $state(0);
  let top = $state(0);

  onMount(() => {
    const rect = menu.getBoundingClientRect();
    left = Math.max(4, Math.min(x, window.innerWidth - rect.width - 4));
    top = Math.max(4, Math.min(y, window.innerHeight - rect.height - 4));
    buttons()[0]?.focus();
  });

  function buttons(): HTMLButtonElement[] {
    return [...menu.querySelectorAll<HTMLButtonElement>("button:not(:disabled)")];
  }

  function onKeydown(event: KeyboardEvent) {
    const list = buttons();
    const index = list.indexOf(document.activeElement as HTMLButtonElement);
    if (event.key === "ArrowDown") list[(index + 1) % list.length]?.focus();
    else if (event.key === "ArrowUp") list[(index - 1 + list.length) % list.length]?.focus();
    else if (event.key === "Escape" || event.key === "Tab") onClose();
    else return;
    event.preventDefault();
    event.stopPropagation();
  }

  function choose(entry: Exclude<MenuEntry, "separator">) {
    onClose();
    entry.action();
  }
</script>

<!-- Clicking anywhere outside closes the menu. -->
<div
  class="backdrop"
  role="presentation"
  onpointerdown={onClose}
  oncontextmenu={(e) => {
    e.preventDefault();
    onClose();
  }}
></div>

<div
  class="menu"
  role="menu"
  tabindex="-1"
  bind:this={menu}
  style:left="{left}px"
  style:top="{top}px"
  onkeydown={onKeydown}
>
  {#each entries as entry, i (i)}
    {#if entry === "separator"}
      <div class="separator" role="separator"></div>
    {:else}
      <button role="menuitem" disabled={entry.disabled} onclick={() => choose(entry)}>
        <span class="icon">{#if entry.icon}<Icon name={entry.icon} size={14} filled={entry.filled} />{/if}</span>
        {entry.label}
      </button>
    {/if}
  {/each}
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
  }

  .menu {
    position: fixed;
    z-index: 51;
    min-width: 220px;
    padding: 4px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--raised);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.18);
    outline: none;
  }

  button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 30px;
    padding: 0 10px 0 6px;
    border: 0;
    border-radius: 5px;
    background: transparent;
    text-align: left;
    white-space: nowrap;
  }

  button:hover:not(:disabled),
  button:focus-visible {
    outline: none;
    background: var(--selection);
  }

  button:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .icon {
    display: grid;
    place-items: center;
    width: 18px;
    color: var(--muted);
  }

  .separator {
    height: 1px;
    margin: 4px 6px;
    background: var(--line);
  }
</style>
