# Third-party software in Billy

Billy is released under the [MIT License](LICENSE). It includes and builds on the work below. Their full license texts are installed with Billy, in its `licenses` folder, and are in this repository under [src-tauri/licenses](src-tauri/licenses).

## Included in the app

| Software | What Billy uses it for | License |
|---|---|---|
| [PDF.js](https://github.com/mozilla/pdf.js) by Mozilla | The reader and its annotation tools | Apache License 2.0 (`licenses/pdfjs-LICENSE.txt`) |
| [PDFium](https://pdfium.googlesource.com/pdfium/), built by [pdfium-binaries](https://github.com/bblanchon/pdfium-binaries) | Covers, page counts, titles and authors | BSD 3-Clause and Apache License 2.0 (`licenses/pdfium-LICENSE.txt`) |
| [Inter](https://github.com/rsms/inter) | The Clean type theme | SIL Open Font License 1.1 (`licenses/fonts/inter-OFL.txt`) |
| [Literata](https://github.com/googlefonts/literata) | The Bookish type theme | SIL Open Font License 1.1 (`licenses/fonts/literata-OFL.txt`) |
| [Source Sans 3](https://github.com/adobe-fonts/source-sans) | The Bookish type theme | SIL Open Font License 1.1 (`licenses/fonts/source-sans-3-OFL.txt`) |
| [Fraunces](https://github.com/undercasetype/Fraunces) | The Editorial type theme | SIL Open Font License 1.1 (`licenses/fonts/fraunces-OFL.txt`) |
| [IBM Plex Sans](https://github.com/IBM/plex) | The Editorial type theme | SIL Open Font License 1.1 (`licenses/fonts/ibm-plex-sans-OFL.txt`) |
| [SQLite](https://sqlite.org), through rusqlite | Billy's database | Public domain |

The fonts are packaged by [Fontsource](https://fontsource.org).

## Libraries Billy is built with

These are compiled into Billy. Each is under a permissive license (MIT, Apache 2.0, or both, unless noted):

- [Tauri](https://tauri.app) and its dialog, opener, updater and process plugins
- [Svelte](https://svelte.dev) and [SvelteKit](https://svelte.dev/docs/kit)
- [rusqlite](https://github.com/rusqlite/rusqlite), [pdfium-render](https://github.com/ajrcarey/pdfium-render), [image](https://github.com/image-rs/image), [notify](https://github.com/notify-rs/notify) (CC0 1.0, MIT or Apache 2.0), [walkdir](https://github.com/BurntSushi/walkdir) (MIT or Unlicense), [xxhash-rust](https://github.com/DoumanAsh/xstd) (BSL 1.0), [serde](https://serde.rs), [thiserror](https://github.com/dtolnay/thiserror) and [dunce](https://gitlab.com/kornelski/dunce) (CC0 1.0, MIT or Apache 2.0)

The complete list of Rust crates, with their licenses, is in `src-tauri/Cargo.lock`. The npm packages are in `package-lock.json`.
