//! Watches root folders that have Auto-sync turned on, and asks for a sync when they change.
//!
//! Changes come in bursts (copying a folder of books makes many events), so a root is synced
//! only after it has been quiet for a moment. Watching never writes anything to the folder.

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

/// How long a folder must be quiet before it is synced.
const QUIET: Duration = Duration::from_secs(2);

pub struct SyncWatcher {
    watchers: Mutex<HashMap<i64, RecommendedWatcher>>,
    changes: Mutex<Sender<i64>>,
}

impl SyncWatcher {
    /// `on_change` is called with a root id once that root has been quiet for [`QUIET`].
    pub fn new(on_change: impl Fn(i64) + Send + 'static) -> Self {
        let (tx, rx) = mpsc::channel::<i64>();
        std::thread::spawn(move || {
            let mut pending: HashMap<i64, Instant> = HashMap::new();
            loop {
                let now = Instant::now();
                let wait = pending
                    .values()
                    .map(|&last| (last + QUIET).saturating_duration_since(now))
                    .min()
                    .unwrap_or(Duration::from_secs(3600));
                match rx.recv_timeout(wait) {
                    Ok(root_id) => {
                        pending.insert(root_id, Instant::now());
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
                let now = Instant::now();
                let due: Vec<i64> =
                    pending.iter().filter(|(_, last)| now.duration_since(**last) >= QUIET).map(|(id, _)| *id).collect();
                for root_id in due {
                    pending.remove(&root_id);
                    on_change(root_id);
                }
            }
        });
        Self { watchers: Mutex::new(HashMap::new()), changes: Mutex::new(tx) }
    }

    /// Starts watching a root folder and its subfolders. Replaces any earlier watch of it.
    pub fn watch(&self, root_id: i64, path: &Path) -> notify::Result<()> {
        let tx = self.changes.lock().unwrap().clone();
        let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            match event {
                // Reads, including Billy's own, are not changes.
                Ok(event) if matches!(event.kind, EventKind::Access(_)) => {}
                // Anything else, including an error such as a dropped event buffer, means
                // the folder may have changed.
                _ => {
                    let _ = tx.send(root_id);
                }
            }
        })?;
        watcher.watch(path, RecursiveMode::Recursive)?;
        self.watchers.lock().unwrap().insert(root_id, watcher);
        Ok(())
    }

    pub fn unwatch(&self, root_id: i64) {
        self.watchers.lock().unwrap().remove(&root_id);
    }

    pub fn is_watching(&self, root_id: i64) -> bool {
        self.watchers.lock().unwrap().contains_key(&root_id)
    }
}
