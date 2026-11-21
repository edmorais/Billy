<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import {
    api,
    onScanFinished,
    onScanProgress,
    onThumbsError,
    onThumbsUpdated,
    type Doc,
    type FolderNode,
    type Library,
    type ScanReport,
  } from "$lib/api";
  import CleanUpDialog from "$lib/components/CleanUpDialog.svelte";
  import Reader from "$lib/components/Reader.svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import {
    collator,
    fold,
    formatSize,
    readSettings,
    sortDocs,
    defaultSettings,
    type SortField,
    type ViewSettings,
  } from "$lib/view";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import Toolbar from "$lib/components/Toolbar.svelte";
  import DocumentView, { type Group, type Item } from "$lib/components/DocumentView.svelte";
  import LibrarySettings from "$lib/components/LibrarySettings.svelte";
  import ContextMenu, { type MenuEntry } from "$lib/components/ContextMenu.svelte";
  import FavoritesShelf from "$lib/components/FavoritesShelf.svelte";

  let version = $state("");
  let libraries = $state<Library[]>([]);
  let libraryId = $state<number | null>(null);
  let settings = $state<ViewSettings>({ ...defaultSettings });
  let folders = $state<FolderNode[]>([]);
  let docs = $state<Doc[]>([]);
  let search = $state("");
  let selected = $state<string | null>(null);
  let expanded = $state(new Set<number>());
  let scanning = $state<Record<number, number>>({});
  let error = $state("");
  let notice = $state("");
  let showSettings = $state(false);
  let rootsVersion = $state(0);
  let treeVersion = $state(0);
  let docsVersion = $state(0);
  let searchInput = $state<HTMLInputElement>();
  let firstName = $state("");
  let thumbsLeft = $state(0);

  // Ask the backend to render what is on screen first. Debounced, so fast scrolling only
  // sends the place where it stops.
  let visibleTimer: ReturnType<typeof setTimeout> | undefined;
  let lastVisible = "";
  function requestThumbs(fileIds: number[]) {
    clearTimeout(visibleTimer);
    visibleTimer = setTimeout(() => {
      const key = fileIds.join(",");
      if (key === lastVisible) return;
      lastVisible = key;
      api.prioritizeThumbs(fileIds).catch(() => {});
    }, 150);
  }

  function applyThumbUpdates(updates: { fileId: number; status: "ok" | "error"; pages: number | null; title: string | null; author: string | null }[]) {
    if (updates.length === 0) return;
    const byId = new Map(updates.map((u) => [u.fileId, u]));
    for (const doc of [...docs, ...shelfAll]) {
      const u = byId.get(doc.fileId);
      if (!u) continue;
      doc.thumbStatus = u.status;
      if (u.status === "ok") {
        doc.pages = u.pages;
        doc.title = u.title;
        doc.author = u.author;
      }
    }
    lastVisible = "";
  }

  let library = $derived(libraries.find((l) => l.id === libraryId) ?? null);
  let folderById = $derived(new Map(folders.map((f) => [f.id, f])));
  let childrenOf = $derived.by(() => {
    const map = new Map<number | null, FolderNode[]>();
    for (const f of folders) {
      const list = map.get(f.parentId) ?? [];
      list.push(f);
      map.set(f.parentId, list);
    }
    return map;
  });
  /** When a library has just one folder, the library shows that folder's contents directly. */
  let singleRoot = $derived.by(() => {
    const tops = childrenOf.get(null) ?? [];
    return tops.length === 1 ? tops[0] : null;
  });
  /** The tree as the sidebar shows it, with a single root folder folded into the library. */
  let treeChildren = $derived.by(() => {
    if (!singleRoot) return childrenOf;
    const map = new Map(childrenOf);
    map.set(null, childrenOf.get(singleRoot.id) ?? []);
    return map;
  });
  /** The folder picked by the user; `null` is the library itself. */
  let folderId = $derived(
    settings.folderId !== null && folderById.has(settings.folderId) && settings.folderId !== singleRoot?.id
      ? settings.folderId
      : null,
  );
  /** The folder whose contents are shown. */
  let scopeId = $derived(folderId ?? singleRoot?.id ?? null);
  let searching = $derived(search.trim() !== "");
  let recursive = $derived(settings.includeSubfolders || searching);
  let scanCount = $derived(Object.keys(scanning).length);

  /** The current folder and its parents, from the library down. */
  let ancestry = $derived.by(() => {
    const chain: FolderNode[] = [];
    for (
      let f = folderId === null ? undefined : folderById.get(folderId);
      f && f.id !== singleRoot?.id;
      f = f.parentId === null ? undefined : folderById.get(f.parentId)
    ) {
      chain.unshift(f);
    }
    return chain;
  });

  let crumbs = $derived([
    { id: null as number | null, name: library?.name ?? "" },
    ...ancestry.map((f) => ({ id: f.id as number | null, name: f.name })),
  ]);

  let visibleDocs = $derived.by(() => {
    const query = fold(search.trim());
    const matching = docs.filter(
      (d) =>
        (!query || fold(d.name).includes(query)) &&
        (settings.filter === "all" || (settings.filter === "read") === d.read) &&
        (!settings.favoritesOnly || d.favorite),
    );
    return sortDocs(matching, settings.sort, settings.desc);
  });

  /** Favorites in the current folder and all its subfolders, for the shelf. */
  let shelfAll = $state<Doc[]>([]);
  let shelfDocs = $derived(
    [...(settings.shelfSubfolders ? shelfAll : shelfAll.filter((d) => d.folderId === scopeId))].sort((a, b) =>
      collator.compare(a.name, b.name),
    ),
  );
  let shelfVersion = $state(0);

  let shelfRequest = 0;
  $effect(() => {
    const id = libraryId;
    const folder = scopeId;
    void docsVersion;
    void shelfVersion;
    const request = ++shelfRequest;
    if (id === null) {
      shelfAll = [];
      return;
    }
    api.listDocuments(id, folder, true, true).then((list) => {
      if (request === shelfRequest) shelfAll = list;
    }, showError);
  });

  let subfolders = $derived(
    recursive
      ? []
      : [...(childrenOf.get(scopeId) ?? [])].sort((a, b) => collator.compare(a.name, b.name)),
  );

  const docItem = (doc: Doc): Item => ({ key: `d${doc.fileId}`, kind: "doc", doc });

  /** Path of a folder below the current one, such as "Sub › Deep". */
  function relativeLabel(id: number): string {
    const names: string[] = [];
    for (let f = folderById.get(id); f && f.id !== scopeId; f = f.parentId === null ? undefined : folderById.get(f.parentId)) {
      names.unshift(f.name);
    }
    return names.length ? names.join(" › ") : (folderById.get(id)?.name ?? "");
  }

  let groups = $derived.by((): Group[] => {
    if (recursive && settings.groupBySubfolder) {
      const byFolder = new Map<number, Doc[]>();
      for (const doc of visibleDocs) {
        const list = byFolder.get(doc.folderId) ?? [];
        list.push(doc);
        byFolder.set(doc.folderId, list);
      }
      // Documents directly in the current folder come first, then each subfolder by name.
      return [...byFolder.entries()]
        .map(([id, list]) => ({
          key: `g${id}`,
          here: id === scopeId,
          label: id === scopeId ? `In ${folderById.get(id)?.name ?? "this folder"}` : relativeLabel(id),
          items: list.map(docItem),
        }))
        .sort((a, b) => Number(b.here) - Number(a.here) || collator.compare(a.label, b.label));
    }
    const folderItems: Item[] = subfolders.map((f) => ({ key: `f${f.id}`, kind: "folder", folder: f }));
    return [{ key: "all", label: null, items: [...folderItems, ...visibleDocs.map(docItem)] }];
  });

  let emptyMessage = $derived(
    !library
      ? ""
      : library.rootCount === 0
        ? "This library has no folders yet. Open library settings to add one."
        : searching
          ? `No file names match "${search.trim()}".`
          : settings.favoritesOnly
            ? "No favorites here. Right-click a document and choose Add to favorites."
            : settings.filter !== "all"
              ? `No ${settings.filter} documents here.`
              : "No PDFs in this folder.",
  );

  // Load the folder tree when the library changes or after a scan.
  $effect(() => {
    const id = libraryId;
    void treeVersion;
    if (id === null) {
      folders = [];
      return;
    }
    api.folderTree(id).then((tree) => {
      if (libraryId === id) folders = tree;
    }, showError);
  });

  // Load the documents in scope.
  let docsRequest = 0;
  $effect(() => {
    const id = libraryId;
    const folder = scopeId;
    const deep = recursive;
    const withMissing = settings.showMissing;
    void docsVersion;
    const request = ++docsRequest;
    // The library itself shows only its root folders, unless subfolders are included.
    if (id === null || (folder === null && !deep)) {
      docs = [];
      return;
    }
    api.listDocuments(id, folder, deep, false, withMissing).then((list) => {
      if (request === docsRequest) docs = list;
    }, showError);
  });

  /** Files in this library that disappeared from disk. */
  let missingCount = $state(0);
  let showCleanUp = $state(false);
  $effect(() => {
    const id = libraryId;
    void docsVersion;
    if (id === null) {
      missingCount = 0;
      return;
    }
    api.listMissing(id).then((list) => {
      if (libraryId === id) missingCount = list.length;
    }, showError);
  });

  /** A short summary of what a sync changed, or nothing when it changed nothing. */
  function syncSummary(report: ScanReport): string {
    const parts: string[] = [];
    if (report.newFiles) parts.push(`${report.newFiles} new`);
    if (report.moved) parts.push(`${report.moved} moved or renamed`);
    if (report.missing) parts.push(`${report.missing} missing`);
    if (report.removed) parts.push(`${report.removed} removed`);
    const root = folders.find((f) => f.rootId === report.rootId && f.parentId === null);
    const name = root?.name ?? "Folder";
    if (report.offline) return `${name} is offline, so nothing changed.`;
    return parts.length ? `Synced ${name}: ${parts.join(", ")}.` : "";
  }

  function flash(message: string, ms = 4000) {
    notice = message;
    setTimeout(() => {
      if (notice === message) notice = "";
    }, ms);
  }

  function showError(err: unknown) {
    error = String(err);
  }

  async function loadLibraries() {
    try {
      libraries = await api.listLibraries();
    } catch (err) {
      showError(err);
      return;
    }
    if (libraryId === null || !libraries.some((l) => l.id === libraryId)) {
      const first = libraries[0];
      libraryId = first?.id ?? null;
      settings = readSettings(first?.viewSettings);
    }
  }

  function selectLibrary(id: number) {
    if (id === libraryId) return;
    libraryId = id;
    settings = readSettings(libraries.find((l) => l.id === id)?.viewSettings);
    search = "";
    selected = null;
    expanded = new Set();
  }

  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  function change(update: Partial<ViewSettings>) {
    settings = { ...settings, ...update };
    const id = libraryId;
    const snapshot = { ...settings };
    const lib = libraries.find((l) => l.id === id);
    if (lib) lib.viewSettings = snapshot as unknown as Record<string, unknown>;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      if (id !== null) api.saveViewSettings(id, snapshot).catch(showError);
    }, 300);
  }

  function navigate(id: number | null) {
    if (id === singleRoot?.id) id = null;
    selected = null;
    search = "";
    if (id !== null) {
      // Expand the folder's parents in the tree so it stays visible.
      const open = new Set(expanded);
      for (let f = folderById.get(id); f && f.parentId !== null; f = folderById.get(f.parentId)) open.add(f.parentId);
      expanded = open;
    }
    change({ folderId: id });
  }

  function goUp() {
    if (ancestry.length === 0) return;
    const current = ancestry[ancestry.length - 1];
    navigate(current.parentId);
    selected = `f${current.id}`;
  }

  function toggle(id: number) {
    const open = new Set(expanded);
    if (open.has(id)) open.delete(id);
    else open.add(id);
    expanded = open;
  }

  function sortBy(field: SortField) {
    change(field === settings.sort ? { desc: !settings.desc } : { sort: field, desc: false });
  }

  function openItem(item: Item) {
    if (item.kind === "folder") navigate(item.folder.id);
    else openDoc(item.doc);
  }

  /** The document open in the reader, if any. The library stays underneath, as it was. */
  let reading = $state<Doc | null>(null);

  function openDoc(doc: Doc) {
    if (doc.missing) return flash("This file is no longer on disk.");
    if (doc.offline) return flash("This file's folder is unavailable. Is its drive connected?");
    menu = null;
    reading = doc;
  }

  function openWithDefaultApp(doc: Doc) {
    api.openWithDefaultApp(doc.fileId).catch(showError);
  }

  async function discardAnnotations(doc: Doc) {
    const confirmed = await ask(
      `Discard the annotations Billy saved for "${doc.name}"?\n\nThey will be lost. The original file is not touched.`,
      { title: "Discard annotations", kind: "warning", okLabel: "Discard", cancelLabel: "Cancel" },
    );
    if (!confirmed) return;
    try {
      await api.discardAnnotations(doc.fileId);
      copiesOf(doc.fileId).forEach((d) => (d.annotated = false));
    } catch (err) {
      showError(err);
    }
  }

  async function exportSavedCopy(doc: Doc) {
    if (doc.size > 100 * 1024 * 1024) {
      notice = `Exporting a large file (${formatSize(doc.size)}). This can take a little while.`;
    }
    try {
      const path = await api.exportSavedCopy(doc.fileId);
      notice = "";
      if (path) flash(`Exported to ${path}`);
    } catch (err) {
      showError(err);
    }
  }

  /** Every loaded copy of a document: the main list and the shelf hold separate objects. */
  function copiesOf(fileId: number): Doc[] {
    return [...docs, ...shelfAll].filter((d) => d.fileId === fileId);
  }

  async function setFavorite(doc: Doc, favorite: boolean) {
    if (libraryId === null) return;
    copiesOf(doc.fileId).forEach((d) => (d.favorite = favorite));
    try {
      await api.setFavorite(libraryId, [doc.fileId], favorite);
    } catch (err) {
      copiesOf(doc.fileId).forEach((d) => (d.favorite = !favorite));
      showError(err);
    }
    shelfVersion++;
  }

  async function setRead(doc: Doc, read: boolean) {
    if (libraryId === null) return;
    copiesOf(doc.fileId).forEach((d) => (d.read = read));
    try {
      await api.setRead(libraryId, [doc.fileId], read);
    } catch (err) {
      copiesOf(doc.fileId).forEach((d) => (d.read = !read));
      showError(err);
    }
  }

  async function copyText(text: string) {
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      // Older WebView2 builds may refuse the async clipboard; fall back to a hidden field.
      const field = document.createElement("textarea");
      field.value = text;
      document.body.append(field);
      field.select();
      document.execCommand("copy");
      field.remove();
    }
    notice = "Path copied.";
    setTimeout(() => (notice = ""), 2000);
  }

  let menu = $state<{ x: number; y: number; entries: MenuEntry[] } | null>(null);

  function folderPath(folder: FolderNode): string {
    return folder.relPath ? `${folder.rootPath}\\${folder.relPath}` : folder.rootPath;
  }

  function openMenu(point: { x: number; y: number }, item: Item) {
    if (item.kind === "folder") {
      const folder = item.folder;
      menu = {
        ...point,
        entries: [
          { label: "Open", icon: "folder", action: () => navigate(folder.id) },
          { label: "Open in File Explorer", icon: "folder-open", disabled: folder.offline, action: () => api.openFolder(folder.id).catch(showError) },
          { label: "Sync now", icon: "sync", action: () => syncFolder(folder) },
          "separator",
          { label: "Copy path", icon: "copy", action: () => copyText(folderPath(folder)) },
        ],
      };
      return;
    }
    const doc = item.doc;
    const unavailable = doc.missing || doc.offline;
    menu = {
      ...point,
      entries: [
        { label: "Open in Billy", icon: "book", disabled: unavailable, action: () => openDoc(doc) },
        { label: "Open with default app", icon: "external", disabled: unavailable, action: () => openWithDefaultApp(doc) },
        { label: "Show in folder", icon: "folder-open", disabled: unavailable, action: () => api.showInFolder(doc.fileId).catch(showError) },
        ...(doc.missing
          ? ([{ label: "Clean up missing files…", icon: "alert", action: () => (showCleanUp = true) }] as MenuEntry[])
          : []),
        "separator",
        doc.favorite
          ? { label: "Remove from favorites", icon: "star", action: () => setFavorite(doc, false) }
          : { label: "Add to favorites", icon: "star", filled: true, action: () => setFavorite(doc, true) },
        doc.read
          ? { label: "Mark as unread", icon: "check-circle", action: () => setRead(doc, false) }
          : { label: "Mark as read", icon: "check-circle", action: () => setRead(doc, true) },
        ...(doc.annotated
          ? ([
              "separator",
              { label: "Export annotated copy…", icon: "external", action: () => exportSavedCopy(doc) },
              { label: "Discard annotations…", icon: "x", action: () => discardAnnotations(doc) },
            ] as MenuEntry[])
          : []),
        "separator",
        { label: "Copy path", icon: "copy", action: () => copyText(doc.path) },
      ],
    };
  }

  /** Menu for a library in the sidebar, and for its "All of …" item. */
  function openLibraryMenu(event: MouseEvent, lib: Library) {
    menu = {
      x: event.clientX,
      y: event.clientY,
      entries: [
        {
          label: lib.rootCount > 1 ? "Sync all folders now" : "Sync now",
          icon: "sync",
          disabled: lib.rootCount === 0,
          action: () => api.syncLibrary(lib.id).catch(showError),
        },
        "separator",
        {
          label: "Library settings…",
          icon: "settings",
          action: () => {
            selectLibrary(lib.id);
            showSettings = true;
          },
        },
      ],
    };
  }

  /** Syncs the root folder that holds this folder. */
  function syncFolder(folder: FolderNode) {
    scanning[folder.rootId] = 0;
    api.syncRoot(folder.rootId).catch(showError);
  }

  function onShelfMenu(event: MouseEvent, doc: Doc) {
    openMenu({ x: event.clientX, y: event.clientY }, { key: `d${doc.fileId}`, kind: "doc", doc });
  }

  async function createLibrary(name: string): Promise<boolean> {
    try {
      const created = await api.createLibrary(name);
      await loadLibraries();
      selectLibrary(created.id);
      showSettings = true;
      return true;
    } catch (err) {
      showError(err);
      return false;
    }
  }

  function refreshAll() {
    loadLibraries();
    treeVersion++;
    docsVersion++;
    rootsVersion++;
  }

  function onGlobalKeydown(event: KeyboardEvent) {
    if (reading) return; // The reader handles its own keys.
    const target = event.target as HTMLElement;
    const typing = target.matches?.("input, select, textarea") ?? false;
    if (event.ctrlKey && event.key.toLowerCase() === "f") {
      event.preventDefault();
      searchInput?.focus();
      searchInput?.select();
    } else if (event.key === "Backspace" && !typing && !showSettings && !showCleanUp && !menu) {
      event.preventDefault();
      goUp();
    }
  }

  let unlisten: UnlistenFn[] = [];

  onMount(async () => {
    api.appVersion().then((v) => (version = v), showError);
    for (const id of await api.scanningRoots().catch(() => [])) scanning[id] = 0;
    unlisten.push(
      await onScanProgress((p) => {
        scanning[p.rootId] = p.filesFound;
      }),
      await onScanFinished((f) => {
        delete scanning[f.rootId];
        if (f.error) showError(f.error);
        else if (f.report?.errors.length) {
          error = `${f.report.errors.length} item(s) could not be read and were skipped. First: ${f.report.errors[0]}`;
        } else if (f.report) {
          const summary = syncSummary(f.report);
          if (summary) flash(summary);
        }
        refreshAll();
      }),
      await onThumbsUpdated(({ updates, remaining }) => {
        thumbsLeft = remaining;
        applyThumbUpdates(updates);
      }),
      await onThumbsError((message) => showError(`Thumbnails are unavailable: ${message}`)),
    );
    api.thumbsRemaining().then((n) => (thumbsLeft = n), () => {});
    await loadLibraries();
  });

  onDestroy(() => {
    unlisten.forEach((fn) => fn());
    clearTimeout(saveTimer);
    clearTimeout(visibleTimer);
  });
</script>

<!-- Billy has its own right-click menus; the browser's menu (Back, Reload…) is only kept in text fields. -->
<svelte:window
  onkeydown={onGlobalKeydown}
  oncontextmenu={(e) => {
    if (!(e.target as HTMLElement).matches?.("input, textarea")) e.preventDefault();
  }}
/>

{#if menu}
  <ContextMenu
    x={menu.x}
    y={menu.y}
    entries={menu.entries}
    onClose={() => {
      menu = null;
      document.querySelector<HTMLElement>(".scroller")?.focus();
    }}
  />
{/if}

<div class="app">
  <Sidebar
    {libraries}
    {libraryId}
    childrenOf={treeChildren}
    {expanded}
    {folderId}
    {version}
    onSelectLibrary={selectLibrary}
    onSelectFolder={navigate}
    onToggle={toggle}
    onCreateLibrary={createLibrary}
    onOpenSettings={() => (showSettings = true)}
    onFolderMenu={(e, node) => openMenu({ x: e.clientX, y: e.clientY }, { key: `f${node.id}`, kind: "folder", folder: node })}
    onLibraryMenu={openLibraryMenu}
  />

  <main>
    {#if library}
      <Toolbar
        {crumbs}
        {settings}
        bind:search
        bind:searchInput
        onNavigate={navigate}
        onUp={goUp}
        onChange={change}
      />
      {#if shelfAll.length > 0}
        <FavoritesShelf
          docs={shelfDocs}
          total={shelfAll.length}
          open={settings.shelfOpen}
          includeSubfolders={settings.shelfSubfolders}
          {selected}
          onToggleOpen={() => change({ shelfOpen: !settings.shelfOpen })}
          onToggleSubfolders={() => change({ shelfSubfolders: !settings.shelfSubfolders })}
          onSelect={(key) => (selected = key)}
          onOpen={openDoc}
          onContextMenu={onShelfMenu}
        />
      {/if}
      <DocumentView
        mode={settings.view}
        {groups}
        {selected}
        sort={settings.sort}
        desc={settings.desc}
        {emptyMessage}
        onSelect={(key) => (selected = key)}
        onOpen={openItem}
        onSort={sortBy}
        onVisible={requestThumbs}
        onContextMenu={openMenu}
      />
    {:else}
      <div class="welcome">
        <h1>Welcome to Billy</h1>
        <p class="muted">A library collects one or more folders of PDFs. Billy reads them and never changes them.</p>
        <form
          onsubmit={async (e) => {
            e.preventDefault();
            if (await createLibrary(firstName)) firstName = "";
          }}
        >
          <input class="field" bind:value={firstName} placeholder="Name your first library, e.g. Books" aria-label="Library name" />
          <button class="btn primary" type="submit" disabled={!firstName.trim()}>Create library</button>
        </form>
      </div>
    {/if}

    <footer class="status">
      {#if error}
        <span class="error" role="alert">{error}</span>
        <button class="link" onclick={() => (error = "")}>Dismiss</button>
      {:else if notice}
        <span>{notice}</span>
      {:else if library}
        <span>
          {visibleDocs.length}
          {visibleDocs.length === 1 ? "document" : "documents"}{#if subfolders.length}, {subfolders.length}
            {subfolders.length === 1 ? "folder" : "folders"}{/if}
        </span>
      {/if}
      <span class="spacer"></span>
      {#if scanCount}
        <span class="muted">Scanning… {Object.values(scanning).reduce((a, b) => a + b, 0)} PDFs found</span>
      {/if}
      {#if thumbsLeft > 0}
        <span class="muted">Making covers… {thumbsLeft} left</span>
      {/if}
      {#if missingCount > 0 && library}
        <span class="missing">
          {missingCount} missing ·
          <button class="link" onclick={() => change({ showMissing: !settings.showMissing })}>
            {settings.showMissing ? "Hide" : "Show"}
          </button>
          ·
          <button class="link" onclick={() => (showCleanUp = true)}>Clean up…</button>
        </span>
      {/if}
    </footer>
  </main>
</div>

{#if showSettings && library}
  <LibrarySettings
    {library}
    {scanning}
    version={rootsVersion}
    onClose={() => (showSettings = false)}
    onChanged={refreshAll}
    onDeleted={() => {
      showSettings = false;
      libraryId = null;
      refreshAll();
    }}
    onError={showError}
    {missingCount}
    onCleanUp={() => (showCleanUp = true)}
  />
{/if}

{#if reading}
  <Reader
    doc={reading}
    onClose={() => {
      const fileId = reading?.fileId;
      reading = null;
      docsVersion++;
      if (fileId !== undefined) selected = `d${fileId}`;
      requestAnimationFrame(() => document.querySelector<HTMLElement>(".scroller")?.focus());
    }}
    onChanged={() => {
      shelfVersion++;
    }}
    onToggleRead={(doc) => setRead(doc, !doc.read)}
    onToggleFavorite={(doc) => setFavorite(doc, !doc.favorite)}
    onError={showError}
  />
{/if}

{#if showCleanUp && library}
  <CleanUpDialog
    {library}
    onClose={() => (showCleanUp = false)}
    onDone={(count) => {
      flash(`Forgot ${count} missing ${count === 1 ? "file" : "files"}.`);
      refreshAll();
    }}
    onError={showError}
  />
{/if}

<style>
  .app {
    display: grid;
    grid-template-columns: 260px 1fr;
    height: 100vh;
  }

  main {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }

  .welcome {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 24px;
    text-align: center;
  }

  .welcome h1 {
    margin: 0;
    font-size: 24px;
  }

  .welcome p {
    max-width: 420px;
    margin: 0 0 12px;
  }

  .welcome form {
    display: flex;
    gap: 8px;
  }

  .welcome input {
    width: 280px;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 28px;
    padding: 0 16px;
    border-top: 1px solid var(--line);
    background: var(--panel);
    font-size: 12px;
  }

  .spacer {
    flex: 1;
  }

  .error {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--danger);
  }

  .missing {
    color: var(--danger);
  }

  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    text-decoration: underline;
  }
</style>
