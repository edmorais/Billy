# Billy

Billy is a Windows app for browsing and reading the PDF documents in your folders. It is named after the bookshelves.

Billy is in early development. See [docs/PLAN.md](docs/PLAN.md) for the plan and milestones.

## Development

Requirements:

- Node.js LTS
- Rust (stable, MSVC toolchain), installed with rustup
- Visual Studio 2022 Build Tools with the "Desktop development with C++" workload
- WebView2 Runtime (included in Windows 11)

Install the dependencies:

```powershell
npm install
```

Run the app in development mode. This starts a local Vite dev server for hot reload. Release builds do not use a server.

```powershell
npm run tauri dev
```

Build the release app and the installer:

```powershell
npm run tauri build
```

The installer is written to `src-tauri/target/release/bundle/nsis/`.

## License

This project is released under the [MIT License](LICENSE).
