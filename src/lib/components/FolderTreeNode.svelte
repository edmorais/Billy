<script lang="ts">
  import type { FolderNode } from "$lib/api";
  import { collator } from "$lib/view";
  import Icon from "./Icon.svelte";
  import FolderTreeNode from "./FolderTreeNode.svelte";

  interface Props {
    node: FolderNode;
    depth: number;
    childrenOf: Map<number | null, FolderNode[]>;
    expanded: Set<number>;
    currentId: number | null;
    onSelect: (id: number) => void;
    onToggle: (id: number) => void;
    onContextMenu: (event: MouseEvent, node: FolderNode) => void;
  }

  let { node, depth, childrenOf, expanded, currentId, onSelect, onToggle, onContextMenu }: Props = $props();

  let children = $derived(
    [...(childrenOf.get(node.id) ?? [])].sort((a, b) => collator.compare(a.name, b.name)),
  );
  let isOpen = $derived(expanded.has(node.id));
</script>

<li role="treeitem" aria-selected={node.id === currentId} aria-expanded={children.length ? isOpen : undefined}>
  <div
    class="row"
    class:current={node.id === currentId}
    class:offline={node.offline}
    style:padding-left="{4 + depth * 14}px"
    title={node.parentId === null ? node.rootPath : node.name}
  >
    {#if children.length}
      <button class="twisty" aria-label={isOpen ? "Collapse" : "Expand"} onclick={() => onToggle(node.id)} tabindex="-1">
        <Icon name={isOpen ? "chevron-down" : "chevron-right"} size={12} />
      </button>
    {:else}
      <span class="twisty"></span>
    {/if}
    <button
      class="label"
      onclick={() => onSelect(node.id)}
      oncontextmenu={(e) => {
        e.preventDefault();
        onContextMenu(e, node);
      }}
    >
      <Icon name="folder" size={14} />
      <span class="name">{node.name}</span>
      <span class="count">{node.offline ? "offline" : node.total}</span>
    </button>
  </div>
  {#if isOpen && children.length}
    <ul role="group">
      {#each children as child (child.id)}
        <FolderTreeNode
          node={child}
          depth={depth + 1}
          {childrenOf}
          {expanded}
          {currentId}
          {onSelect}
          {onToggle}
          {onContextMenu}
        />
      {/each}
    </ul>
  {/if}
</li>

<style>
  li {
    list-style: none;
  }

  ul {
    margin: 0;
    padding: 0;
  }

  .row {
    display: flex;
    align-items: center;
    height: 26px;
    border-radius: var(--radius);
  }

  .row:hover {
    background: var(--hover);
  }

  .row.current {
    background: var(--selection);
  }

  .row.offline {
    opacity: 0.6;
  }

  .twisty {
    display: grid;
    place-items: center;
    flex: none;
    width: 18px;
    height: 18px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
  }

  .label {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    min-width: 0;
    height: 100%;
    padding: 0 6px 0 2px;
    border: 0;
    background: transparent;
    text-align: left;
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
    font-variant-numeric: tabular-nums;
  }
</style>
