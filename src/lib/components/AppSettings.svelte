<script lang="ts">
  // App-wide settings: appearance, color theme and type theme. Changes apply at once.
  import { api } from "$lib/api";
  import {
    applyTheme,
    cleanFontName,
    colorThemes,
    currentSettings,
    customTypeTheme,
    fontInstalled,
    theme,
    typeThemes,
    type Appearance,
    type ThemeSettings,
  } from "$lib/theme.svelte";
  import Icon from "./Icon.svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { checkForUpdates, installUpdate, setAutoUpdates, updates } from "$lib/updates.svelte";
  import { formatDate } from "$lib/view";

  let {
    version,
    onClose,
    onError,
  }: { version: string; onClose: () => void; onError: (message: string) => void } = $props();

  const REPO = "https://github.com/edmorais/Billy";
  const open = (url: string) => openUrl(url).catch((err) => onError(String(err)));

  let dialog: HTMLDialogElement;

  $effect(() => {
    dialog.showModal();
  });

  // Saves run one after another, so quick changes are stored in the order they were made.
  let saving: Promise<unknown> = Promise.resolve();

  function choose(change: Partial<ThemeSettings>) {
    const settings: ThemeSettings = { ...currentSettings(), ...change };
    applyTheme(settings);
    saving = saving.then(() => api.setSetting("theme", settings)).catch((err) => onError(String(err)));
  }

  // The custom font: typed by the user, applied after a short pause in typing.
  let customName = $state(theme.customFont);
  let customTimer: ReturnType<typeof setTimeout> | undefined;
  let custom = $derived(customTypeTheme(customName));
  let installed = $derived(fontInstalled(customName));

  function typeCustom(value: string) {
    customName = value;
    clearTimeout(customTimer);
    customTimer = setTimeout(() => choose({ typeTheme: "custom", customFont: cleanFontName(value) }), 400);
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

  <div class="body">
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
      <div
        class="card custom"
        role="radio"
        tabindex="0"
        aria-checked={theme.typeTheme === "custom"}
        class:on={theme.typeTheme === "custom"}
        onclick={() => choose({ typeTheme: "custom", customFont: cleanFontName(customName) })}
        onkeydown={(e) => {
          if (e.key === "Enter" || e.key === " ") choose({ typeTheme: "custom", customFont: cleanFontName(customName) });
        }}
      >
        <span class="specimen">
          <span class="big" style:font-family={custom.display}>Billy</span>
          <span class="small" style:font-family={custom.ui}>The quick brown fox reads 1,234 pages.</span>
        </span>
        <span class="label">Custom</span>
        <input
          class="field"
          placeholder="Font name, e.g. Georgia"
          aria-label="Custom font name"
          spellcheck="false"
          value={customName}
          oninput={(e) => typeCustom(e.currentTarget.value)}
          onclick={(e) => e.stopPropagation()}
        />
        <span class="desc" class:warn={customName.trim() !== "" && !installed}>
          {#if customName.trim() === ""}
            Any font on this computer
          {:else if installed}
            Installed on this computer
          {:else}
            Not found; Segoe UI is used instead
          {/if}
        </span>
      </div>
    </div>
  </section>

  <section>
    <h3>Updates</h3>
    <label class="check">
      <input type="checkbox" checked={updates.auto} onchange={(e) => setAutoUpdates(e.currentTarget.checked)} />
      Check for updates automatically, once a day
    </label>
    <div class="update-row">
      <button class="btn" onclick={checkForUpdates} disabled={updates.status === "checking" || updates.status === "downloading"}>
        <Icon name="sync" size={14} /> Check now
      </button>
      <span class="hint inline" role="status">
        {#if updates.status === "checking"}
          Checking…
        {:else if updates.status === "none"}
          Billy {version} is the latest version.
        {:else if updates.status === "available"}
          Billy {updates.version} is available.
        {:else if updates.status === "downloading"}
          Downloading… {Math.round(updates.progress * 100)}%
        {:else if updates.status === "error"}
          Could not check for updates: {updates.error}
        {:else if updates.lastCheck}
          Last checked {formatDate(updates.lastCheck)}.
        {/if}
      </span>
      {#if updates.status === "available"}
        <button class="btn primary" onclick={installUpdate}>Update and restart</button>
      {/if}
    </div>
  </section>

  <section class="about">
    <h3>About</h3>
    <div class="about-row">
      <img src="/billy-mark.svg" alt="" width="40" height="40" />
      <div>
        <div class="display about-name">Billy {version}</div>
        <div class="hint inline">Your PDF bookshelves. Free software under the MIT License.</div>
      </div>
    </div>
    <div class="links">
      <button class="link" onclick={() => open(REPO)}>Website</button>
      <button class="link" onclick={() => open(`${REPO}/releases`)}>Release notes</button>
      <button class="link" onclick={() => open(`${REPO}/issues`)}>Report a problem</button>
      <button class="link" onclick={() => open(`${REPO}/blob/main/THIRD-PARTY-NOTICES.md`)}>Third-party software</button>
    </div>
  </section>
  </div>

  <footer>
    <button class="btn" onclick={() => dialog.close()}>Done</button>
  </footer>
</dialog>

<style>
  /* The dialog follows the window: it grows and shrinks with it (up to a comfortable width),
     the cards reflow, and only the middle scrolls, so the title and Done stay in view. */
  dialog {
    width: min(1120px, calc(100vw - 64px));
    max-height: calc(100vh - 64px);
    padding: 0;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--bg);
    color: var(--fg);
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.3);
  }

  dialog[open] {
    display: flex;
    flex-direction: column;
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 0.35);
  }

  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  header {
    flex: none;
    border-bottom: 1px solid var(--line);
    margin-bottom: 14px;
  }

  footer {
    flex: none;
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

  .check {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .check input {
    margin: 0;
    accent-color: var(--accent);
  }

  .update-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 10px;
  }

  .hint.inline {
    margin: 0;
  }

  .about-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .about-name {
    font-size: 15px;
    font-weight: 600;
  }

  .links {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    margin-top: 10px;
  }

  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    text-decoration: underline;
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
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 10px;
  }

  .card.custom {
    cursor: pointer;
  }

  .card.custom .field {
    width: 100%;
    margin-top: 4px;
  }

  .desc.warn {
    color: var(--danger);
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
