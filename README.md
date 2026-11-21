<p align="center">
  <img src="src-tauri/icons/icon.png" alt="Billy's icon: two shelves of colorful books" width="128" height="128">
</p>

<h1 align="center">Billy</h1>

<p align="center">Your PDF bookshelves, on Windows.</p>

Billy shows the PDFs in your folders as a library you can browse, with real covers, and lets you read and annotate them. It is named after the bookshelves.

Billy only reads your folders. It never moves, renames or changes your files, and it never writes its own data into your folders. The one exception is "Save to original file" in the reader, which always asks first.

## What Billy does

- **Libraries.** Make one or more libraries, and add any folders to each. Subfolders come along.
- **Covers.** Billy shows the first page of each PDF as its cover, and reads the page count, title and author.
- **Browsing.** Small, medium and large grids, a details table and a compact list. Sort by name, title, author, page count, file size, date modified or date added. Search file names. Include the documents in subfolders, grouped by folder if you like.
- **Favorites.** Pin documents as favorites in each library. Each folder shows its favorites on a shelf at the top, optionally with those in its subfolders.
- **Read or unread.** Mark documents as read, and show only unread ones.
- **Reading.** Billy's reader is the one in Firefox (Mozilla's PDF.js). It reopens each book at the page where you stopped.
- **Annotating.** Highlight, add notes, draw and sign. Billy keeps your annotations in its own folder, so your PDF stays as it was. You can also export an annotated copy, or save the annotations into the original file.
- **Staying in sync.** Billy notices new, moved, renamed and deleted files. Turn on Auto-sync for a folder to keep it up to date while Billy runs, or sync by hand.
- **Right-click.** Open any document in your usual PDF app, or show it in File Explorer.
- **Themes.** Light, dark, or follow Windows. Six color themes and several fonts, including any font on your computer.

## Installing

1. Go to the [latest release](https://github.com/edmorais/Billy/releases/latest).
2. Download `Billy_…_x64-setup.exe`.
3. Run it. Billy installs for your user account only, so it doesn't need administrator rights.

Billy isn't signed with a paid certificate, so Windows SmartScreen may warn you the first time. Choose **More info**, then **Run anyway**.

Billy needs Windows 10 or 11, 64-bit. It uses Microsoft Edge WebView2, which is part of Windows 11 and comes with most up-to-date copies of Windows 10.

## Getting started

1. Open Billy and name your first library, for example "Books".
2. Choose **Add folder…** and pick a folder with PDFs. Billy finds every PDF in it and its subfolders, then makes the covers in the background.
3. Browse with the folder tree on the left. Double-click a document to read it.

## Updates

Billy checks for a new version once a day and asks before installing it. You can turn this off, or check by hand, in **Settings**.

## Where Billy keeps its data

| What | Where |
|---|---|
| Libraries, favorites, read status, settings | `%APPDATA%\Billy` |
| Your annotations (usually a few kilobytes each) | `%APPDATA%\Billy\annotations` |
| Covers, and annotated copies when needed | `%LOCALAPPDATA%\Billy` |

Uninstalling Billy leaves these folders, so your libraries are still there if you install it again. Delete them by hand to remove everything.

## Help and feedback

Report problems or ideas on the [Issues page](https://github.com/edmorais/Billy/issues).

## For developers

See [DEVELOPERS.md](DEVELOPERS.md) for how to build Billy.

## License

Billy is free software under the [MIT License](LICENSE). It includes third-party software, listed in [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
