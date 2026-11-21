<script lang="ts">
  // App-wide settings: appearance, color theme and type theme. Changes apply at once.
  import { api } from "$lib/api";
  import {
    applyTheme,
    colorThemes,
    theme,
    typeThemes,
    type Appearance,
    type ThemeSettings,
  } from "$lib/theme.svelte";
  import Icon from "./Icon.svelte";

  let { onClose, onError }: { onClose: () => void; onError: (message: string) => void } = $props();

  let dialog: HTMLDialogElement;

  $effect(() => {
    dialog.showModal();
  });

  function choose(change: Partial<ThemeSettings>) {
    const settings: ThemeSettings = {
      appearance: theme.appearance,
      colorTheme: theme.colorTheme,
      typeTheme: theme.typeTheme,
      ...change,
    };
    applyTheme(settings);
    api.setSetting("theme", settings).catch((err) => onError(String(err)));
  }

  const appearances: { id: Appearance; label: string }[] = [
    { id: "system", label: "Follow Windows" },
    { id: "light", label: "Light" },
    { id: "dark", label: "Dark" },
  ];
</script>

<dialog bind:this={dialog} onclose={onClose} aria-labelledby="app-settings-title">
  <header>
    <h2 id="app-settings-title">Settings</h2>
    <button class="icon-btn" aria-label="Close" onclick={() => dialog.close()}><Icon name="x" /></button>
  </header>

  <section>
    <h3>Appearance</h3>
    <div class="segmented" role="radiogroup" aria-label="Appearance">
      {#each appearances as option (option.id)}
        <button
          role="radio"
          aria-checked={theme.appearance === option.id}
          class:on={theme.appearance === option.id}
          onclick={() => choose({ appearance: option.id })}
        >
          {option.label}
        </button>
      {/each}
    </div>
    <p class="hint">Pages in the reader keep their own colors in every mode.</p>
  </section>

  <section>
    <h3>Colors</h3>
    <div class="cards" role="radiogroup" aria-label="Color theme">
      {#each colorThemes as option (option.id)}
        {@const p = option[theme.mode]}
        <button
          class="card"
          role="radio"
          aria-checked={theme.colorTheme === option.id}
          class:on={theme.colorTheme === option.id}
          onclick={() => choose({ colorTheme: option.id })}
        >
          <span class="swatch" style:background={p.bg} style:border-color={p.line}>
            <span class="side" style:background={p.panel}>
              <span class="pill" style:background={p.accent}></span>
              <span class="bar" style:background={p.muted}></span>
              <span class="bar short" style:background={p.muted}></span>
            </span>
            <span class="shelf">
              <span class="book" style:background={p.accent}></span>
              <span class="book" style:background={p.fg}></span>
              <span class="book" style:background={p.muted}></span>
              <span class="book tall" style:background={p.raised} style:border-color={p.line}></span>
            </span>
          </span>
          <span class="label">{option.name}</span>
          <span class="desc">{option.description}</span>
        </button>
      {/each}
    </div>
  </section>

  <section>
    <h3>Type</h3>
    <div class="cards" role="radiogroup" aria-label="Type theme">
      {#each typeThemes as option (option.id)}
        <button
          class="card"
          role="radio"
          aria-checked={theme.typeTheme === option.id}
          class:on={theme.typeTheme === option.id}
          onclick={() => choose({ typeTheme: option.id })}
        >
          <span class="specimen">
            <span class="big" style:font-family={option.display}>Billy</span>
            <span class="small" style:font-family={option.ui}>The quick brown fox reads 1,234 pages.</span>
          </span>
          <span class="label">{option.name}</span>
          <span class="desc">{option.description}</span>
        </button>
      {/each}
    </div>
  </section>

  <footer>
    <button class="btn" onclick={() => dialog.close()}>Done</button>
  </footer>
</dialog>

<style>
  dialog {
    width: min(760px, calc(100vw - 48px));
    max-height: calc(100vh - 48px);
    padding: 0;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--bg);
    color: var(--fg);
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.3);
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 0.35);
  }

  header,
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 18px;
  }

  footer {
    justify-content: flex-end;
    border-top: 1px solid var(--line);
  }

  h2 {
    margin: 0;
    font-size: 18px;
  }

  h3 {
    margin: 0 0 8px;
    font-size: 13px;
  }

  section {
    padding: 0 18px 16px;
  }

  .hint {
    margin: 6px 0 0;
    color: var(--muted);
    font-size: 12px;
  }

  .segmented {
    display: inline-flex;
    padding: 2px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--raised);
  }

  .segmented button {
    height: 28px;
    padding: 0 14px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
  }

  .segmented button.on {
    background: var(--accent);
    color: var(--accent-fg);
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 10px;
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--raised);
    text-align: left;
  }

  .card:hover {
    border-color: color-mix(in srgb, var(--accent) 50%, var(--line));
  }

  .card.on {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }

  .label {
    margin-top: 6px;
    font-weight: 600;
  }

  .desc {
    color: var(--muted);
    font-size: 12px;
  }

  .swatch {
    display: flex;
    height: 74px;
    overflow: hidden;
    border: 1px solid;
    border-radius: 5px;
  }

  .side {
    display: flex;
    flex-direction: column;
    gap: 5px;
    width: 34%;
    padding: 8px 6px;
  }

  .pill {
    height: 7px;
    border-radius: 3px;
  }

  .bar {
    height: 4px;
    border-radius: 2px;
    opacity: 0.5;
  }

  .bar.short {
    width: 60%;
  }

  .shelf {
    display: flex;
    align-items: flex-end;
    gap: 4px;
    flex: 1;
    padding: 10px;
  }

  .book {
    width: 14px;
    height: 70%;
    border-radius: 1px 2px 2px 1px;
  }

  .book.tall {
    height: 85%;
    border: 1px solid;
  }

  .specimen {
    display: flex;
    flex-direction: column;
    justify-content: center;
    height: 74px;
    padding: 0 8px;
    border-radius: 5px;
    background: var(--bg);
  }

  .big {
    font-size: 26px;
    font-weight: 600;
    line-height: 1.1;
  }

  .small {
    font-size: 12px;
    color: var(--muted);
  }
</style>
