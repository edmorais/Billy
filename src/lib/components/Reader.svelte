<script lang="ts">
  // The reader: Mozilla's PDF.js viewer in an iframe, with Billy's own bar on top.
  //
  // PDF.js gets the document through a range transport: it asks for the byte ranges it needs
  // and Billy's backend reads them from disk, so big books open quickly and nothing is served
  // over a URL. Annotations are saved by Billy, never by PDF.js's own download button.
  import { onDestroy, onMount } from "svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { api, type Doc, type OpenInfo, type PdfSource, type SavedPdf } from "$lib/api";
  import { displayName, formatSize } from "$lib/view";
  import { currentPalette, theme } from "$lib/theme.svelte";
  import Icon from "./Icon.svelte";
  import ContextMenu, { type MenuEntry } from "./ContextMenu.svelte";

  interface Props {
    doc: Doc;
    onClose: () => void;
    /** Called after something that the library view shows has changed. */
    onChanged: () => void;
    onToggleRead: (doc: Doc) => void;
    onToggleFavorite: (doc: Doc) => void;
    onError: (message: string) => void;
  }

  let { doc, onClose, onChanged, onToggleRead, onToggleFavorite, onError: reportError }: Props = $props();

  // Messages show in the reader's own bar; the library's status bar is hidden underneath.
  let message = $state<{ text: string; error: boolean } | null>(null);
  let messageTimer: ReturnType<typeof setTimeout> | undefined;
  /** Shows a message in the reader's bar. With `sticky`, it stays until the next message. */
  function show(text: string, error = false, sticky = false) {
    message = { text, error };
    clearTimeout(messageTimer);
    if (!sticky) messageTimer = setTimeout(() => (message = null), error ? 8000 : 6000);
  }

  /** Files above this size take a while to save, because PDF.js reads all of them first. */
  const LARGE = 100 * 1024 * 1024;

  /** For large files, says that the action may take a while, until it finishes. */
  function noteIfLarge(action: string) {
    const size = Math.max(info?.originalSize ?? 0, info?.annotatedSize ?? 0);
    if (size > LARGE) {
      show(`${action}… This file is large (${formatSize(size)}), so it can take a minute or two.`, false, true);
    }
  }
  const onNotice = (text: string) => show(text);
  const onError = (text: string) => {
    show(text, true);
    reportError(text);
  };

  /** Bytes per request. PDF.js asks for ranges in multiples of this. */
  const CHUNK = 256 * 1024;

  let frame: HTMLIFrameElement;
  let viewerWindow: any = null;
  let app: any = null;
  let info = $state<OpenInfo | null>(null);
  let source = $state<PdfSource>("original");
  let dirty = $state(false);
  let busy = $state(false);
  let loading = $state(true);
  let choice = $state<"changed" | "unsaved" | null>(null);
  let choose: ((answer: string) => void) | null = null;
  let menu = $state<{ x: number; y: number; entries: MenuEntry[] } | null>(null);
  let dirtyTimer: ReturnType<typeof setInterval> | undefined;
  let pageTimer: ReturnType<typeof setTimeout> | undefined;

  let viewerReady: Promise<void>;
  let resolveReady: () => void;
  viewerReady = new Promise((resolve) => (resolveReady = resolve));

  // PDF.js tells the parent page when it has loaded, before it starts. This is where its
  // options are set.
  function onViewerLoaded(event: Event) {
    const w = (event as CustomEvent).detail?.source;
    if (!frame || w !== frame.contentWindow) return;
    viewerWindow = w;
    const options = w.PDFViewerApplicationOptions;
    options.set("disablePreferences", true);
    options.set("defaultUrl", "");
    options.set("disableHistory", true);
    options.set("viewOnLoad", 1); // Billy restores the last page itself.
    options.set("enableScripting", false);
    options.set("enableSignatureEditor", true);
    options.set("enableHighlightFloatingButton", true);
    options.set("enableAltTextModelDownload", false);
    options.set("viewerCssTheme", theme.mode === "dark" ? 2 : 1);

    // The viewer's own security policy allows stylesheets from the app, not inline styles.
    const link = w.document.createElement("link");
    link.rel = "stylesheet";
    link.href = "/billy-viewer.css";
    w.document.head.append(link);
    w.addEventListener("keydown", onKeydown, true);
    themeViewer();

    w.PDFViewerApplication.initializedPromise.then(() => {
      app = w.PDFViewerApplication;
      resolveReady();
    });
  }

  document.addEventListener("webviewerloaded", onViewerLoaded);

  /** Gives PDF.js's toolbar and surroundings Billy's colors and light or dark mode. Pages keep
   *  their own colors. */
  function themeViewer() {
    const w = viewerWindow;
    if (!w) return;
    const root = w.document.documentElement as HTMLElement;
    const p = currentPalette();
    const mix = (color: string, amount: number, base = "transparent") => `color-mix(in srgb, ${color} ${amount}%, ${base})`;
    const vars: Record<string, string> = {
      "--main-color": p.fg,
      "--body-bg-color": mix(p.fg, theme.mode === "dark" ? 6 : 12, p.bg),
      "--toolbar-bg-color": p.panel,
      "--sidebar-toolbar-bg-color": p.panel,
      "--sidebar-narrow-bg-color": p.panel,
      "--toolbar-border-color": p.line,
      "--toolbar-icon-bg-color": p.fg,
      "--field-color": p.fg,
      "--field-bg-color": p.raised,
      "--field-border-color": p.line,
      "--button-hover-color": mix(p.fg, 10),
      "--toggled-btn-bg-color": mix(p.accent, 28),
      "--dropdown-btn-bg-color": p.raised,
      "--doorhanger-bg-color": p.raised,
      "--doorhanger-border-color": p.line,
      "--doorhanger-hover-color": p.fg,
      "--separator-color": p.line,
      "--progressBar-color": p.accent,
      "--treeitem-color": p.fg,
      "--treeitem-hover-color": p.fg,
    };
    root.style.colorScheme = theme.mode;
    for (const [name, value] of Object.entries(vars)) root.style.setProperty(name, value);
  }

  // Follow theme changes while the reader is open.
  $effect(() => {
    void theme.mode;
    void theme.colorTheme;
    themeViewer();
  });

  function onKeydown(event: KeyboardEvent) {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
      event.stopImmediatePropagation();
      save();
    } else if (event.altKey && event.key === "ArrowLeft") {
      event.preventDefault();
      event.stopImmediatePropagation();
      close();
    }
  }

  /** Asks one of the in-reader questions and waits for the answer. */
  function askChoice(kind: "changed" | "unsaved"): Promise<string> {
    choice = kind;
    return new Promise((resolve) => {
      choose = (answer) => {
        choice = null;
        choose = null;
        resolve(answer);
      };
    });
  }

  async function open(which: PdfSource, page?: number) {
    if (!info) return;
    loading = true;
    source = which;
    await viewerReady;
    const w = viewerWindow;
    const size = which === "annotated" ? info.annotatedSize! : info.originalSize;
    const fileId = info.fileId;
    // Copy each chunk into an array that belongs to the viewer's own window: PDF.js stalls
    // on arrays whose memory belongs to this (the parent) window.
    const read = async (begin: number, end: number) => {
      const bytes = new Uint8Array(await api.readPdf(fileId, which, begin, end - begin));
      const copy = new w.Uint8Array(bytes.length);
      copy.set(bytes);
      return copy;
    };

    // Small files are read in one go. Larger ones are read in ranges as PDF.js needs them.
    // (PDF.js's range transport stalls when its first chunk already holds the whole file.)
    let params: Record<string, unknown>;
    if (size <= 2 * CHUNK) {
      params = { data: await read(0, size) };
    } else {
      const transport = new w.pdfjsLib.PDFDataRangeTransport(size, await read(0, CHUNK));
      transport.requestDataRange = (begin: number, end: number) => {
        read(begin, end).then(
          (chunk) => transport.onDataRange(begin, chunk),
          (err) => onError(String(err)),
        );
      };
      // Only read what is needed to show the pages, rather than the whole book in the
      // background. Saving reads the rest when it needs it.
      params = { range: transport, rangeChunkSize: CHUNK, disableAutoFetch: true };
    }

    const startPage = page ?? info.lastPage ?? 1;
    const restore = () => {
      app.eventBus.off("pagesinit", restore);
      if (startPage > 1 && startPage <= app.pagesCount) app.page = startPage;
      loading = false;
    };
    app.eventBus.on("pagesinit", restore);
    try {
      await app.open({ ...params, filename: info.name });
    } catch (err) {
      loading = false;
      onError(`Billy could not open this PDF: ${err}`);
    }
    dirty = false;
  }

  function onPageChanging(event: { pageNumber: number }) {
    clearTimeout(pageTimer);
    const fileId = doc.fileId;
    pageTimer = setTimeout(() => api.setLastPage(fileId, event.pageNumber).catch(() => {}), 500);
  }

  /** Bytes of the open version that are sent along with an appended save, as a check. */
  const PROBE = 4096;

  /**
   * The document with its annotations, ready to send. PDF.js saves annotations by appending
   * to the version that was open, so usually only the appended bytes are sent and Billy copies
   * the rest from disk. That keeps saving cheap even for books of hundreds of megabytes.
   */
  async function currentSave(): Promise<SavedPdf> {
    const data: Uint8Array = await app.pdfDocument.saveDocument();
    const baseLen = (source === "annotated" ? info?.annotatedSize : info?.originalSize) ?? 0;
    if (baseLen > PROBE && data.length > baseLen) {
      const body = new Uint8Array(PROBE + data.length - baseLen);
      body.set(data.subarray(baseLen - PROBE, baseLen), 0);
      body.set(data.subarray(baseLen), PROBE);
      return {
        body,
        headers: { mode: "append", source, "base-length": String(baseLen), "probe-length": String(PROBE) },
      };
    }
    return { body: new Uint8Array(data), headers: { mode: "full", source } };
  }

  function markSaved() {
    app.pdfDocument?.annotationStorage.resetModified();
    dirty = false;
  }

  /** Saves the annotations in Billy's own folder. */
  async function save(): Promise<boolean> {
    if (!app?.pdfDocument || busy) return false;
    busy = true;
    noteIfLarge("Saving annotations");
    try {
      await api.saveAnnotations(doc.fileId, await currentSave());
      markSaved();
      doc.annotated = true;
      onChanged();
      onNotice("Annotations saved in Billy. The original file is unchanged.");
      return true;
    } catch (err) {
      onError(String(err));
      return false;
    } finally {
      busy = false;
    }
  }

  async function saveToOriginal() {
    if (!app?.pdfDocument || !info) return;
    const large = info.originalSize > LARGE ? `\n\nThis file is large (${formatSize(info.originalSize)}), so saving can take a minute or two.` : "";
    const confirmed = await ask(
      `Write the annotations into the original file?\n\n${info.path}\n\nThis changes the file in your folder, and other PDF readers will show the annotations. Billy's saved annotations for it are then removed.${large}`,
      { title: "Save to original file", kind: "warning", okLabel: "Save to original", cancelLabel: "Cancel" },
    );
    if (!confirmed) return;
    busy = true;
    noteIfLarge("Saving to the original file");
    const page = app.page;
    try {
      await api.saveToOriginal(doc.fileId, await currentSave());
      markSaved();
      doc.annotated = false;
      onChanged();
      onNotice("Annotations saved into the original file.");
      // The file on disk changed, so read it afresh.
      info = await api.openDocument(doc.fileId);
      await open("original", page);
    } catch (err) {
      onError(String(err));
    } finally {
      busy = false;
    }
  }

  async function exportCopy() {
    if (!app?.pdfDocument) return;
    busy = true;
    noteIfLarge("Preparing the copy");
    try {
      const path = await api.exportCopy(doc.fileId, await currentSave());
      if (path) onNotice(`Exported to ${path}`);
      else message = null;
    } catch (err) {
      onError(String(err));
    } finally {
      busy = false;
    }
  }

  async function discard() {
    const confirmed = await ask(
      "Discard the annotations Billy saved for this document?\n\nThey will be lost. The original file is not touched.",
      { title: "Discard annotations", kind: "warning", okLabel: "Discard", cancelLabel: "Cancel" },
    );
    if (!confirmed) return;
    const page = app?.page;
    try {
      await api.discardAnnotations(doc.fileId);
      doc.annotated = false;
      onChanged();
      info = await api.openDocument(doc.fileId);
      await open("original", page);
    } catch (err) {
      onError(String(err));
    }
  }

  export async function close() {
    if (dirty) {
      const answer = await askChoice("unsaved");
      if (answer === "cancel") return;
      if (answer === "save" && !(await save())) return;
    }
    onClose();
  }

  function openSaveMenu(event: MouseEvent) {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    menu = {
      x: rect.left,
      y: rect.bottom + 4,
      entries: [
        { label: "Save annotations (Ctrl+S)", icon: "check", disabled: !dirty, action: save },
        { label: "Save to original file…", icon: "alert", action: saveToOriginal },
        { label: "Export copy…", icon: "external", action: exportCopy },
        "separator",
        { label: "Discard annotations…", icon: "x", disabled: !doc.annotated, action: discard },
      ],
    };
  }

  function onParentKeydown(event: KeyboardEvent) {
    if (choice || menu) return;
    onKeydown(event);
  }

  onMount(async () => {
    window.addEventListener("keydown", onParentKeydown, true);
    try {
      info = await api.openDocument(doc.fileId);
    } catch (err) {
      onError(String(err));
      onClose();
      return;
    }
    let which: PdfSource = info.annotatedSize !== null ? "annotated" : "original";
    if (info.annotatedSize !== null && info.originalChanged) {
      which = (await askChoice("changed")) === "original" ? "original" : "annotated";
    }
    if (info.annotationsBroken) {
      show(
        "This file was replaced after you annotated it, so Billy can't show your annotations on it. They are kept until you discard them.",
        true,
        true,
      );
    }
    await viewerReady;
    app.eventBus.on("pagechanging", onPageChanging);
    dirtyTimer = setInterval(() => {
      if (app) dirty = !!app._annotationStorageModified;
    }, 400);
    await open(which);
  });

  onDestroy(() => {
    document.removeEventListener("webviewerloaded", onViewerLoaded);
    window.removeEventListener("keydown", onParentKeydown, true);
    viewerWindow?.removeEventListener("keydown", onKeydown, true);
    app?.eventBus.off("pagechanging", onPageChanging);
    clearInterval(dirtyTimer);
    clearTimeout(pageTimer);
    clearTimeout(messageTimer);
  });
</script>

<div class="reader">
  <header>
    <button class="icon-btn" title="Back to the library (Alt+Left)" aria-label="Back to the library" onclick={close}>
      <Icon name="arrow-left" />
    </button>
    <div class="title" title={info?.path ?? doc.path}>
      <span class="name">{doc.title?.trim() || displayName(doc)}</span>
      {#if source === "annotated"}
        <span class="tag">With your annotations</span>
      {/if}
      {#if dirty}
        <span class="tag unsaved">Unsaved changes</span>
      {/if}
      {#if message}
        <span class="message" class:error={message.error} role="status">{message.text}</span>
      {/if}
    </div>
    <button
      class="toggle"
      class:on={doc.read}
      aria-pressed={doc.read}
      title={doc.read ? "Mark as unread" : "Mark as read"}
      onclick={() => onToggleRead(doc)}
    >
      <Icon name="check-circle" /> {doc.read ? "Read" : "Mark as read"}
    </button>
    <button
      class="icon-btn"
      class:on={doc.favorite}
      aria-pressed={doc.favorite}
      title={doc.favorite ? "Remove from favorites" : "Add to favorites"}
      aria-label={doc.favorite ? "Remove from favorites" : "Add to favorites"}
      onclick={() => onToggleFavorite(doc)}
    >
      <Icon name="star" filled={doc.favorite} />
    </button>
    <div class="save">
      <button class="btn primary" disabled={!dirty || busy} onclick={save} title="Save annotations in Billy's copy (Ctrl+S)">
        Save
      </button>
      <button class="btn more" aria-label="More save options" onclick={openSaveMenu} disabled={busy}>
        <Icon name="chevron-down" size={12} />
      </button>
    </div>
  </header>

  <div class="viewer">
    <iframe bind:this={frame} src="/pdfjs/web/viewer.html?file=" title="PDF viewer"></iframe>
    {#if loading}
      <div class="loading">Opening…</div>
    {/if}
  </div>
</div>

{#if menu}
  <ContextMenu x={menu.x} y={menu.y} entries={menu.entries} onClose={() => (menu = null)} />
{/if}

{#if choice}
  <div class="scrim" role="presentation">
    <div class="question" role="alertdialog" aria-modal="true" aria-labelledby="question-title">
      {#if choice === "changed"}
        <h2 id="question-title">The original has changed</h2>
        <p>
          This file was changed after you annotated it. You can open the version you annotated, with your annotations,
          or the new version without them. Your annotations are kept either way.
        </p>
        <div class="buttons">
          <button class="btn" onclick={() => choose?.("original")}>Open the new version</button>
          <button class="btn primary" onclick={() => choose?.("annotated")}>Open with my annotations</button>
        </div>
      {:else}
        <h2 id="question-title">Save your annotations?</h2>
        <p>Billy saves them in its own copy. The original file is not changed.</p>
        <div class="buttons">
          <button class="btn" onclick={() => choose?.("cancel")}>Cancel</button>
          <button class="btn danger" onclick={() => choose?.("discard")}>Don't save</button>
          <button class="btn primary" onclick={() => choose?.("save")}>Save</button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .reader {
    position: fixed;
    inset: 0;
    z-index: 20;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }

  header {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 44px;
    padding: 0 10px;
    border-bottom: 1px solid var(--line);
    background: var(--panel);
  }

  .title {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-display);
    font-size: 14px;
    font-weight: 600;
  }

  .tag {
    flex: none;
    padding: 1px 8px;
    border-radius: 999px;
    background: var(--hover);
    color: var(--muted);
    font-size: 11px;
  }

  .message {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted);
    font-size: 12px;
  }

  .message.error {
    color: var(--danger);
  }

  .tag.unsaved {
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    color: var(--fg);
  }

  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 10px 0 8px;
    border: 1px solid transparent;
    border-radius: var(--radius);
    background: transparent;
    color: var(--muted);
    white-space: nowrap;
  }

  .toggle:hover {
    background: var(--hover);
    color: var(--fg);
  }

  .toggle.on {
    color: var(--fg);
  }

  .icon-btn.on {
    color: var(--accent);
  }

  .save {
    display: flex;
  }

  .save .btn.primary {
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
  }

  .save .more {
    padding: 0 6px;
    border-left: 0;
    border-top-left-radius: 0;
    border-bottom-left-radius: 0;
  }

  .viewer {
    position: relative;
    flex: 1;
    min-height: 0;
  }

  iframe {
    display: block;
    width: 100%;
    height: 100%;
    border: 0;
  }

  .loading {
    position: absolute;
    top: 45%;
    left: 0;
    right: 0;
    text-align: center;
    color: var(--muted);
    pointer-events: none;
  }

  .scrim {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: grid;
    place-items: center;
    background: rgb(0 0 0 / 0.35);
  }

  .question {
    width: min(460px, calc(100vw - 48px));
    padding: 18px 20px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--bg);
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.3);
  }

  .question h2 {
    margin: 0 0 8px;
    font-size: 16px;
  }

  .question p {
    margin: 0 0 16px;
    color: var(--muted);
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
