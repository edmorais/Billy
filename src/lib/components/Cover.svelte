<script lang="ts">
  // Placeholder cover drawn from the document's name. Milestone 3 shows the real first page.
  import type { Doc } from "$lib/api";
  import { displayName } from "$lib/view";

  let { doc, small = false }: { doc: Doc; small?: boolean } = $props();

  function hue(text: string): number {
    let h = 0;
    for (let i = 0; i < text.length; i++) h = (h * 31 + text.charCodeAt(i)) >>> 0;
    return h % 360;
  }

  let title = $derived(doc.title?.trim() || displayName(doc));
  let h = $derived(hue(doc.fingerprint));
</script>

<div class="cover" class:small style:--h={h}>
  {#if !small}
    <span class="title">{title}</span>
    {#if doc.author}<span class="author">{doc.author}</span>{/if}
  {/if}
</div>

<style>
  .cover {
    --h: 30;
    position: relative;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    width: 100%;
    height: 100%;
    padding: 12% 10%;
    overflow: hidden;
    border-radius: 2px 4px 4px 2px;
    background: linear-gradient(90deg, hsl(var(--h) 30% 32%) 0 5%, hsl(var(--h) 32% 42%) 5%);
    color: hsl(var(--h) 40% 94%);
    box-shadow: var(--shadow);
    font-family: var(--font-cover);
  }

  .cover.small {
    border-radius: 1px 2px 2px 1px;
  }

  .title {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 5;
    line-clamp: 5;
    overflow: hidden;
    font-size: 1em;
    line-height: 1.2;
    overflow-wrap: anywhere;
  }

  .author {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.8em;
    opacity: 0.8;
  }
</style>
