-- Billy database schema, version 1.
-- Times are milliseconds since the Unix epoch. Paths are absolute Windows paths.

CREATE TABLE libraries (
    id            INTEGER PRIMARY KEY,
    name          TEXT    NOT NULL,
    created_at    INTEGER NOT NULL,
    view_settings TEXT    NOT NULL DEFAULT '{}'
);

-- A root is a folder (with its subfolders) added to a library.
CREATE TABLE roots (
    id             INTEGER PRIMARY KEY,
    library_id     INTEGER NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    path           TEXT    NOT NULL COLLATE NOCASE,
    auto_sync      INTEGER NOT NULL DEFAULT 0,
    status         TEXT    NOT NULL DEFAULT 'ok',   -- ok | offline
    added_at       INTEGER NOT NULL,
    last_synced_at INTEGER,
    file_count     INTEGER NOT NULL DEFAULT 0,       -- PDFs found under this root in the last scan
    UNIQUE (library_id, path)
);

-- Folders under a root. The root folder itself has rel_path ''.
CREATE TABLE folders (
    id        INTEGER PRIMARY KEY,
    root_id   INTEGER NOT NULL REFERENCES roots(id) ON DELETE CASCADE,
    parent_id INTEGER REFERENCES folders(id) ON DELETE CASCADE,
    rel_path  TEXT    NOT NULL COLLATE NOCASE,
    name      TEXT    NOT NULL,
    UNIQUE (root_id, rel_path)
);
CREATE INDEX folders_parent ON folders(parent_id);

-- One row per PDF on disk, shared by every library that contains it.
CREATE TABLE files (
    id            INTEGER PRIMARY KEY,
    path          TEXT    NOT NULL UNIQUE COLLATE NOCASE,
    size          INTEGER NOT NULL,
    mtime         INTEGER NOT NULL,
    fingerprint   TEXT    NOT NULL,
    page_count    INTEGER,
    title         TEXT,
    author        TEXT,
    thumb_status  TEXT    NOT NULL DEFAULT 'pending',  -- pending | ok | error
    missing_since INTEGER
);
CREATE INDEX files_fingerprint ON files(fingerprint);

-- A file's membership in a library.
CREATE TABLE entries (
    library_id INTEGER NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    root_id    INTEGER NOT NULL REFERENCES roots(id) ON DELETE CASCADE,
    file_id    INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    folder_id  INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
    date_added INTEGER NOT NULL,
    PRIMARY KEY (library_id, file_id)
);
CREATE INDEX entries_folder ON entries(folder_id);
CREATE INDEX entries_root ON entries(root_id);

CREATE TABLE favorites (
    library_id INTEGER NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    file_id    INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    pinned_at  INTEGER NOT NULL,
    PRIMARY KEY (library_id, file_id)
);

CREATE TABLE read_state (
    library_id INTEGER NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    file_id    INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    read       INTEGER NOT NULL,
    changed_at INTEGER NOT NULL,
    PRIMARY KEY (library_id, file_id)
);

CREATE TABLE file_state (
    file_id        INTEGER PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE,
    last_page      INTEGER,
    last_opened_at INTEGER
);

CREATE TABLE annotations (
    file_id            INTEGER PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE,
    copy_path          TEXT    NOT NULL,
    source_fingerprint TEXT    NOT NULL,
    updated_at         INTEGER NOT NULL
);

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
