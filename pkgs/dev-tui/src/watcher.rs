//! File-system watcher for ~/.dev/projects/ — sends Req::DevTasks on change.

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

use crate::data::Req;

/// Start a watcher on `~/.dev/projects/`.
/// Returns `Some(watcher)` — caller must keep the watcher alive (drop = stop).
/// Returns `None` if the directory doesn't exist or notify fails.
pub fn start_task_watcher(req_tx: Sender<Req>) -> Option<RecommendedWatcher> {
    let store = dev_core::dev_store_path()?;

    // 500 ms debounce: ignore bursts of events (agent writing multiple files)
    let last_sent: Arc<Mutex<Instant>> =
        Arc::new(Mutex::new(Instant::now() - Duration::from_secs(10)));

    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            if res.is_ok() {
                let mut last = last_sent.lock().unwrap();
                if last.elapsed() >= Duration::from_millis(500) {
                    *last = Instant::now();
                    let _ = req_tx.send(Req::DevTasks);
                }
            }
        },
        Config::default(),
    )
    .ok()?;

    watcher.watch(&store, RecursiveMode::Recursive).ok()?;
    Some(watcher)
}
