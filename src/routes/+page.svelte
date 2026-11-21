<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  let backend = $state("connecting…");

  onMount(async () => {
    try {
      const version = await invoke<string>("app_version");
      backend = `Billy ${version}`;
    } catch (err) {
      backend = `Backend not reachable: ${err}`;
    }
  });
</script>

<main>
  <h1>Billy</h1>
  <p>Your PDF bookshelves.</p>
  <p class="status">{backend}</p>
</main>

<style>
  :global(:root) {
    --bg: #f6f4ef;
    --fg: #1f1d1a;
    --muted: #6b665d;
    color-scheme: light dark;
  }

  @media (prefers-color-scheme: dark) {
    :global(:root) {
      --bg: #1b1a18;
      --fg: #ece8e1;
      --muted: #a39d92;
    }
  }

  :global(body) {
    margin: 0;
    background: var(--bg);
    color: var(--fg);
    font-family: "Segoe UI Variable", "Segoe UI", system-ui, sans-serif;
  }

  main {
    min-height: 100vh;
    display: grid;
    place-content: center;
    text-align: center;
  }

  h1 {
    margin: 0;
    font-size: 3rem;
    font-weight: 600;
  }

  p {
    margin: 0.5rem 0 0;
  }

  .status {
    color: var(--muted);
    font-size: 0.875rem;
  }
</style>
