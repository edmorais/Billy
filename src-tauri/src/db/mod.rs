//! Billy's SQLite database: opening, configuration and schema migrations.
//!
//! The database lives in Billy's own data folder, never in a library folder.

pub mod browse;
pub mod libraries;
pub mod marks;
pub mod missing;
pub mod settings;

use crate::error::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Schema migrations, applied in order. The database's `user_version` records how many have run.
const MIGRATIONS: &[&str] = &[
    include_str!("migrations/001_initial.sql"),
    include_str!("migrations/002_thumb_index.sql"),
    include_str!("migrations/003_annotation_deltas.sql"),
    include_str!("migrations/004_ignored_folders.sql"),
];

/// Opens (or creates) the database at `path` and brings its schema up to date.
pub fn open(path: &Path) -> Result<Connection> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut conn = Connection::open(path)?;
    // Other connections (the scanner, the thumbnail worker) may hold the write lock for a moment.
    conn.busy_timeout(Duration::from_secs(30))?;
    conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA synchronous = NORMAL;")?;
    migrate(&mut conn)?;
    Ok(conn)
}

fn migrate(conn: &mut Connection) -> Result<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    for (index, sql) in MIGRATIONS.iter().enumerate() {
        let version = index as i64 + 1;
        if version > current {
            let tx = write_tx(conn)?;
            tx.execute_batch(sql)?;
            tx.execute_batch(&format!("PRAGMA user_version = {version}"))?;
            tx.commit()?;
        }
    }
    Ok(())
}

/// Starts a transaction that will write. It takes the write lock at once, waiting for other
/// writers if needed. A plain (deferred) transaction that reads first and writes later fails at
/// once with "database is locked" when another connection wrote in between, without waiting.
pub fn write_tx(conn: &mut Connection) -> rusqlite::Result<Transaction<'_>> {
    conn.transaction_with_behavior(TransactionBehavior::Immediate)
}

/// Current time in milliseconds since the Unix epoch.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
