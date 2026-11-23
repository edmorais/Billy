# Billy for developers

How to build Billy, how it is put together, and how to release it. For what Billy does, see the [README](README.md). For the design and the milestones, see [docs/PLAN.md](docs/PLAN.md).

## Stack

- **App shell:** Tauri 2, with a Rust backend in `src-tauri/` and a web UI shown in WebView2. No web server runs in release builds.
- **UI:** Svelte 5 and TypeScript, built with Vite. SvelteKit in single-page mode (static adapter).
- **Reader:** Mozilla PDF.js, the prebuilt viewer, in an iframe. Billy feeds it byte ranges through its backend.
- **Thumbnails and metadata:** PDFium, through the `pdfium-render` crate.
- **Data:** SQLite through `rusqlite`, in `%APPDATA%\Billy\billy.db`. Thumbnails in `%LOCALAPPDATA%\Billy\thumbs`.

## Requirements

- Node.js LTS
- Rust (stable, MSVC toolchain), installed with rustup
- Visual Studio 2022 Build Tools with the "Desktop development with C++" workload
- WebView2 Runtime (included in Windows 11)

## Setting up

Install the npm dependencies:

```powershell
npm install
```

Download the pinned PDFium library (thumbnails) and PDF.js viewer (reader). Both scripts check a SHA-256 hash before using the download:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\fetch-pdfium.ps1
```

```powershell
powershell -ExecutionPolicy Bypass -File scripts\fetch-pdfjs.ps1
```

## Running

Run the app in development mode. This starts a local Vite dev server for hot reload:

```powershell
npm run tauri dev
```

To work on the UI in a normal browser, run `npm run dev` and open http://localhost:1420. Outside Tauri, a fake backend with 5,000 books (`src/lib/dev/mock.ts`) answers the UI's calls. Add `?single` for a library with one folder, or `?big` for a large sample PDF in the reader.

To try a build without touching your own libraries, set `BILLY_DATA_DIR` to an empty folder before starting Billy. It then keeps its database and covers there:

```powershell
$env:BILLY_DATA_DIR = "$env:TEMP\billy-test"; npm run tauri dev
```

Set it whenever Billy is started from inside a packaged app (an MSIX app, such as some terminals and AI assistants). Windows gives such apps a private copy of `%APPDATA%` that falls behind your real data, while covers in `%LOCALAPPDATA%` may still be the real ones.

## Testing

```powershell
cd src-tauri
cargo test
```

The tests include checks that scanning, syncing and making thumbnails never change a library folder. Type-check the UI with `npm run check`.

## Icons

`scripts/make-icons.mjs` draws the app icon (`src-tauri/icons/icon-source.svg`) and the mark used inside the app (`static/billy-mark.svg`). After changing it:

```powershell
node scripts\make-icons.mjs
```

```powershell
npm run tauri icon src-tauri/icons/icon-source.svg
```

Then delete the `android` and `ios` folders that the second command also creates in `src-tauri/icons`.

## Building

Release builds sign the update files, so they need the private signing key (see below). Without the key, build with updater files turned off:

```powershell
npm run tauri build -- --config src-tauri/tauri.ci.conf.json
```

The installer is written to `src-tauri/target/release/bundle/nsis/`.

## Updates and signing

Installed copies of Billy check `https://github.com/edmorais/Billy/releases/latest/download/latest.json` once a day (users can turn this off). They install an update only if it is signed with Billy's release key. The public half of the key is in `src-tauri/tauri.conf.json`, under `plugins.updater.pubkey`.

The private key and its password live outside the repository, in `%USERPROFILE%\.tauri\billy.key` and `billy.key.password`. Keep a backup somewhere safe: without them, no new version can update existing installs. GitHub Actions gets them from two repository secrets:

- `TAURI_SIGNING_PRIVATE_KEY`: the contents of `billy.key`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: the contents of `billy.key.password`

To make a signed build locally:

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content "$env:USERPROFILE\.tauri\billy.key" -Raw; $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = Get-Content "$env:USERPROFILE\.tauri\billy.key.password" -Raw; npm run tauri build
```

## Releasing

1. Set the new version in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and `package.json`.
2. Commit, then tag the commit with the same version and push the tag:

```powershell
git tag v1.0.3
```

```powershell
git push origin v1.0.3
```

The Release workflow (`.github/workflows/release.yml`) checks that the tag matches the version, runs the tests, builds and signs Billy, and publishes a GitHub Release with the installer and `latest.json`.

## License

Billy is released under the [MIT License](LICENSE). The third-party software it includes is listed in [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
