use rusqlite::Connection;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

/// Shared app state. Background work opens its own connection from `db_path`.
pub struct AppState {
    pub db: Mutex<Connection>,
    pub db_path: PathBuf,
    /// Roots being scanned right now.
    pub scanning: Mutex<HashSet<i64>>,
}

impl AppState {
    pub fn new(db: Connection, db_path: PathBuf) -> Self {
        Self { db: Mutex::new(db), db_path, scanning: Mutex::new(HashSet::new()) }
    }

    pub fn db(&self) -> MutexGuard<'_, Connection> {
        self.db.lock().unwrap()
    }
}
