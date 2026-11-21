use crate::reader::Dirs;
use crate::thumbs::ThumbQueue;
use crate::watch::SyncWatcher;
use rusqlite::Connection;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

/// Shared app state. Background work opens its own connection from `db_path`.
pub struct AppState {
    pub db: Mutex<Connection>,
    pub db_path: PathBuf,
    /// Thumbnail cache, in %LOCALAPPDATA%\Billy\thumbs.
    pub thumbs_dir: PathBuf,
    pub thumbs: Arc<ThumbQueue>,
    pub watcher: SyncWatcher,
    /// Roots being scanned right now.
    pub scanning: Mutex<HashSet<i64>>,
    /// Roots that changed while being scanned, and need another scan when it ends.
    pub rescan: Mutex<HashSet<i64>>,
}

impl AppState {
    pub fn new(db: Connection, db_path: PathBuf, thumbs_dir: PathBuf, watcher: SyncWatcher) -> Self {
        Self {
            db: Mutex::new(db),
            db_path,
            thumbs_dir,
            thumbs: Arc::new(ThumbQueue::default()),
            watcher,
            scanning: Mutex::new(HashSet::new()),
            rescan: Mutex::new(HashSet::new()),
        }
    }

    pub fn db(&self) -> MutexGuard<'_, Connection> {
        self.db.lock().unwrap()
    }

    /// Billy's own data folders: %APPDATA%\Billy (roaming) and %LOCALAPPDATA%\Billy (local).
    pub fn dirs(&self) -> Dirs {
        Dirs {
            roaming: self.db_path.parent().map(PathBuf::from).unwrap_or_default(),
            local: self.thumbs_dir.parent().map(PathBuf::from).unwrap_or_default(),
        }
    }
}
