# Billy: project plan

Billy is a standalone Windows app for browsing and reading PDF documents kept in folders on disk. It is named after the "Billy" bookshelves.

Status: agreed. Nothing is built yet.

## 1. Goals

1. Add folders and their subfolders to a library. Billy never changes the original folders and never writes its own data there.
2. Keep several libraries.
3. Sync a library with changes in its folders.
4. Browse libraries with PDF thumbnails.
5. When browsing a folder, optionally include documents in its subfolders.
6. Choose between several view modes and sort orders.
7. Pin documents as favorites for each library, and show favorites from subfolders on request.
8. Right-click a document to open it in the default PDF reader or to show it in File Explorer.
9. Read and annotate PDFs inside Billy.

Out of scope for the first version: full-text search inside PDFs, tags, ratings, other file types, cloud or network storage, and languages other than English.

## 2. Decisions so far

| Topic | Decision |
|---|---|
| App shell | Tauri 2: a native Windows app with a Rust backend and a web UI shown in WebView2. No web server. |
| UI framework | Svelte 5 with TypeScript, built with Vite. SvelteKit in single-page mode (static adapter, no server), as Tauri recommends. |
| PDF reader | Mozilla PDF.js, the reader inside Firefox, including its annotation editor. |
| Thumbnails and PDF metadata | PDFium, the PDF engine inside Chrome, called from Rust through the `pdfium-render` crate. |
| Database | SQLite, through the `rusqlite` crate. |
| Size | About 5,000 PDFs, mostly books. All on local disks. |
| Annotations | Saved by default in Billy's own data folder, as only the bytes PDF.js appends to the original (not a full copy). Two extra commands: "Save to original file" and "Export copy…". |
| Overlap | One folder can belong to several libraries. One library can have several unrelated root folders. |
| Missing files | Billy keeps the data of missing files. Deleting it always needs confirmation. |
| Sync | The first scan happens when you add a folder. After that, sync is something you choose for each folder. |
| Search | Filename search only. |
| Document state | Read or unread. Favorites and read status are separate for each library. |
| Updates | Billy updates itself from GitHub Releases. A setting turns this off. |
| Reader layout | The reader takes the place of the library view, with a Back button. |
| Look | Light and dark modes and several color and type themes. Pages are never inverted. |
| Language | English only. |
| Distribution | Mainly for personal use. Installers are published as GitHub Releases. |

## 3. Architecture

```
┌──────────────────────── Billy.exe (Tauri 2) ─────────────────────────┐
│  WebView2 (UI)                         Rust backend                  │
│  ┌──────────────────────────┐  IPC    ┌───────────────────────────┐  │
│  │ Svelte app               │◄──────► │ commands (libraries, …)   │  │
│  │  - library browser       │ events  │ scanner + sync + watcher  │  │
│  │  - settings, themes      │         │ thumbnail worker (PDFium) │  │
│  │ PDF.js viewer (iframe)   │         │ SQLite (rusqlite)         │  │
│  └──────────────────────────┘         │ shell: open, reveal       │  │
│        ▲ thumb:// and pdf:// protocols└───────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────┘
        reads only ▼                         writes ▼
   your PDF folders                  %APPDATA%\Billy, %LOCALAPPDATA%\Billy
```

- **The UI and the backend talk only through Tauri's IPC** (commands and events) and two custom URL protocols. `thumb://` serves cached thumbnail images. `pdf://` streams PDF data to the reader. Neither uses the network.
- **Billy only reads your folders, and one module enforces this.** All file access to library folders goes through a module in `fs_guard.rs` that opens files read-only. One function in the whole code base can write to a library folder: "Save to original file", which runs only after you confirm it. A test fingerprints a sample folder tree before and after a scan and a sync, and fails if anything changed.
- **Billy's own data lives in two places:**
  - `%APPDATA%\Billy\billy.db` holds libraries, favorites, read status and settings.
  - `%APPDATA%\Billy\annotations\` holds the annotations: for each annotated document, only the bytes PDF.js appended to the original (usually a few kilobytes). These are user data, so they live in the roaming profile.
  - `%LOCALAPPDATA%\Billy\annotations\` holds a full annotated copy only in the rare case where a save is not an append. Full copies never go in the roaming profile.
  - `%LOCALAPPDATA%\Billy\thumbs\` is the thumbnail cache. Billy can rebuild it.

## 4. Data model

```
libraries        id, name, created_at, view_settings (JSON)
roots            id, library_id, path, auto_sync, status (ok|offline), added_at, last_synced_at
folders          id, root_id, parent_id, rel_path, name
files            id, path (unique), size, mtime, fingerprint, page_count,
                 title, author, thumb_status, missing_since
entries          library_id, root_id, file_id, folder_id, date_added     -- one file in one library
favorites        library_id, file_id, pinned_at
read_state       library_id, file_id, read (bool), changed_at
file_state       file_id, last_page, last_opened_at
annotations      file_id, copy_path, source_fingerprint, updated_at, base_len, base_check
```

- **`files` holds one record for each PDF on disk.** The same path in two libraries shares that record, its thumbnail and its annotations. Membership in a library and the date added are stored in `entries`.
- **The fingerprint is the file size plus an xxHash3 hash of the first and last 64 KB of the file.** It is quick to compute, even for 5,000 books, and it lets sync recognize a file that was moved or renamed.
- **Favorites and read status belong to a library.** A book can be read in one library and unread in another. The last page you viewed belongs to the file and is the same in every library.

## 5. Features in detail

### 5.1 Libraries and folders

- **Creating, renaming and deleting libraries.** Deleting a library removes only Billy's records, never files.
- **Adding a root folder** runs a first scan of all PDFs in that folder and its subfolders. The scan skips hidden folders, system folders and folders whose names start with `.`.
- **Overlapping roots:** a root folder can sit inside another root folder of the same library. Billy sees that the files are the same and does not list them twice.
- **Removing a root folder** takes its entries out of that library. Billy also offers to delete the related favorites and read status in that library.

### 5.2 Sync

- **Each root folder has an Auto-sync switch, which is off by default.**
  - When it is on, Billy rescans the folder at startup and watches it for changes while the app runs, using the `notify` crate with debouncing.
  - When it is off, the folder changes only when you click "Sync now". That command is in the folder's right-click menu and in library settings.
- **New files** are added to the library.
- **Changed files** (different size or modification time) are fingerprinted again. Their thumbnail and metadata are refreshed.
- **Moved or renamed files** are recognized by their fingerprint. Favorites, read status and annotations follow the file. This works whichever folder is synced first, and across libraries. A file on an offline folder is never taken as "moved", because its drive is just not there.
- **Changes during a sync** are not lost: if a folder changes while it is being synced, Billy syncs it again when the first sync ends. The watcher waits until a folder has been quiet for 2 seconds, so copying many books gives one sync.
- **Missing files** are marked as missing, not deleted. By default they are hidden, and you can show them as greyed-out entries. "Clean up missing files…" lists them and asks for confirmation before deleting their favorites, read status and annotations.
- **Offline root folders** happen, for example, when a drive is unplugged. Billy marks the folder as offline and changes nothing. Its files are shown as unavailable.

### 5.3 Browsing

- **Layout:**
  - The left sidebar has the library list and then the folder tree of the current library.
  - The main area has a toolbar, then the favorites shelf, then the document view.
- **Toolbar controls:**
  - breadcrumb path
  - filename search, covering the current folder and all its subfolders
  - **Include subfolders** switch
  - view mode
  - sort field and direction
  - **Group by subfolder** switch, which only appears when subfolders are included
  - filter: all, unread, or read
- **View modes:**
  - a grid in small, medium and large sizes
  - a detailed list with small thumbnails and columns
  - a compact list
- **Sort fields:**
  - name
  - date modified
  - date added
  - file size
  - page count
  - title from the PDF
  - author from the PDF
- **Billy remembers the view settings for each library.** It may later remember them for each folder.
- **The grid and lists are virtualized**, so only visible items are drawn. Large folders and the "Include subfolders" view of a whole library stay fast with 5,000 items.
- **Keyboard:** arrow keys move the selection, Enter opens a document, Backspace goes up a folder, and Ctrl+F jumps to search.

### 5.4 Favorites

- **Pinning** adds a document to the current library's favorites.
- **The favorites shelf** sits at the top of a folder view and shows the pinned documents in that folder. Its own "include subfolders" switch also shows pinned documents from all subfolders. That switch works separately from the main one.
- **A "Favorites only" filter** in the toolbar shows nothing but favorites in the document view.

### 5.5 Right-click menu

- Open in Billy
- Open with default app
- Show in folder: opens Explorer with the file selected, through `SHOpenFolderAndSelectItems`
- Pin or unpin
- Mark as read or unread
- Copy path
- These two appear only when the document has annotations:
  - Export annotated copy…
  - Discard annotations…

### 5.6 Thumbnails and metadata

- **The thumbnail is page 1**, rendered by PDFium at 400 px wide and saved as JPEG (quality 82, about 65 KB each). The image crate only writes lossless WebP, which would be several times larger. Cache files are named by fingerprint, so the cache stays valid after a file moves.
- **Each file is rendered once by a background worker.** The same pass reads the page count, title and author. Files visible on screen go first.
- **Progress is shown in a status bar.** Measured on real books: about 4 covers a second, limited by disk reads, so a first pass over 5,000 books takes about 20 minutes in the background. PDFium serializes all calls, so more threads would not help; separate worker processes could, if this ever matters.
- **Encrypted or broken PDFs** get a placeholder image and an error mark. The scan continues.

### 5.7 Reader

- **The reader is the standard PDF.js viewer,** a pinned version kept in `vendor/pdfjs/`, inside an iframe. Billy's own bar sits on top, with Back, the document title and a Save menu. The viewer is styled to match the current theme. In dark mode only the controls change color and the pages keep their own colors.
- **Built-in features:** find in document, sidebar with page thumbnails and outline, zoom, rotation, page modes and printing.
- **Annotation tools:** highlight, free text, freehand ink, image or stamp, and signature.
- **Saving annotations:**
  - **Ctrl+S** saves the annotations in Billy's data folder. PDF.js saves by appending to the PDF, so Billy stores only the appended bytes, plus the length of the original they extend and a hash of the original's last 4 KB. When the document is opened again, Billy joins the original's first part with those bytes as it reads them. A badge marks the document as annotated.
  - **"Save to original file…"** writes the annotations into your PDF after you confirm.
  - **"Export copy…"** writes an annotated PDF wherever you choose.
  - **Large files:** PDF.js reads the whole document before saving, so saving a large book takes a while (about 2 minutes for 800 MB). For files over 100 MB, Billy says so while it saves, exports or writes the original.
- **If the original file changes after you annotate it,** Billy warns you when you open it.
  - If the change was appended to the file (as many PDF programs save), the annotated version can still be rebuilt. You choose between the version you annotated, with your annotations, and the new version without them.
  - If the file was replaced completely, the annotations can no longer be shown. Billy says so, opens the file, and keeps the annotations until you discard them.
- **Billy remembers the last page** for each file. "Mark as read" is a manual action.

### 5.8 Themes

- **Every color and font in the UI comes from CSS custom properties** (design tokens). A theme is one set of token values with a light and a dark variant.
- **Color themes and type themes are separate.** A type theme is a pair of fonts bundled with the app, never loaded from the web.
- **Samples:** in the visual design phase I will make 3 or 4 color themes and 3 type themes for you to choose from.
- **The appearance setting** offers Light, Dark, or Follow Windows.

### 5.9 Updates

- **Billy checks GitHub Releases for a new version at startup,** using Tauri's updater plugin. The check runs at most once a day.
- **When a new version exists,** Billy shows a notice with the release notes and an "Update and restart" button. Nothing installs without that click.
- **Updates are signed with the Tauri updater key**, so Billy installs only releases built by this project's GitHub Actions workflow. The private key is kept as a GitHub Actions secret.
- **A setting turns the automatic check off.** Settings also has a "Check for updates now" button.

## 6. Project layout

```
Billy/
  src/                    Svelte app (routes, components, stores, themes)
  src-tauri/
    src/
      main.rs, lib.rs
      db/                 schema, migrations, queries
      scan/               scanner, fingerprint, sync, watcher
      thumbs/             PDFium worker, cache
      fs_guard.rs         read-only access to library folders
      shell.rs            open with default app, reveal in Explorer
      protocols.rs        thumb:// and pdf://
    resources/pdfium.dll  PDFium binary (pinned version)
  vendor/pdfjs/           PDF.js viewer (pinned version) and update script
  tests/fixtures/         generated sample PDFs and folder trees
  docs/PLAN.md            this file
```

## 7. Milestones

Each milestone ends with a working build and a commit.

| # | Milestone | Done when |
|---|---|---|
| 0 | **Toolchain and scaffold**: install Node LTS, Rust and VS C++ Build Tools. Create the Tauri, Svelte and TypeScript app. Add a CI build on GitHub Actions. | An empty Billy window opens, and CI builds it. |
| 1 | **Data and scanning**: SQLite schema and migrations, libraries and root folders, scanner, fingerprints, the read-only guard and its test. | Adding a folder fills the database. The test shows that no folder was changed. |
| 2 | **Browser**: sidebar, folder tree, grid and lists, virtualization, sort, search, include subfolders, group by subfolder. | You can move through a library of 5,000 files without lag, using placeholder thumbnails. |
| 3 | **Thumbnails and metadata**: PDFium worker, cache, `thumb://` protocol, visible files first. | Real covers, page counts, titles and authors appear. |
| 4 | **Favorites, read status and right-click menu**, including opening files in Windows and showing them in Explorer. | All right-click actions work. |
| 5 | **Sync**: Auto-sync switch, Sync now, watcher, moved and missing files, clean-up with confirmation, offline root folders. | Renaming, moving and deleting files in Explorer give the expected results. |
| 6 | **Reader**: PDF.js integration, byte ranges read through Billy's backend (instead of a `pdf://` protocol), annotations and the three save modes, last page, warning when the original changed. | You can read, annotate, save, reopen and export. |
| 7 | **Visual design**: theme samples, then your choice, then building the themes, a settings page and the app icon. | Themes can be switched in Settings. |
| 8 | **Releases and updates**: NSIS installer, a GitHub Actions release workflow with signed updates, the updater with its on/off setting, third-party licenses (PDF.js and PDFium) and the README. | Pushing a version tag publishes an installer on GitHub Releases, and an older installed Billy offers to update to it. |

## 8. Risks and notes

- **Unsigned installer.** Without a code-signing certificate, Windows SmartScreen shows a warning on first install. That is acceptable for personal use. Signing can be added later.
- **PDFium binary.** Billy uses the prebuilt `pdfium.dll` from the `bblanchon/pdfium-binaries` project, pinned to one version and shipped with the app.
- **PDF.js annotations in the original file.** These follow the PDF standard and show in Acrobat, Edge and Chrome. Some annotation types may look slightly different in other readers.
- **Large books.** PDF.js loads pages as needed, and `pdf://` will support byte ranges, so big scanned books open quickly.

## 9. Settled questions

- **Favorites per folder:** each favorite belongs to a document in one library. It shows on the shelf of the folder that holds it. With the shelf's subfolders switch on, it also shows in the parent folders.
- **Read status:** separate for each library.
- **Auto-sync:** off by default for new root folders.
- **Updates:** automatic, with a setting to turn them off.
- **Thumbnail:** always page 1.
