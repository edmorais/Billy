-- Subfolders that a library leaves out when it scans its folders. Absolute paths.
CREATE TABLE ignored_folders (
    library_id INTEGER NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    path       TEXT    NOT NULL COLLATE NOCASE,
    added_at   INTEGER NOT NULL,
    PRIMARY KEY (library_id, path)
);
