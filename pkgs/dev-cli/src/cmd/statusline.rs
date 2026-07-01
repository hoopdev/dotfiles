//! `dev statusline` — Claude Code's `statusLine.command`. Reads the per-response
//! JSON payload from stdin, caches the normalized rate limits to
//! `~/.cache/claude/usage.json` (for `dev usage` / the TUI), and prints the
//! `5h:X% 7d:Y%` bar line. Replaces the bash+jq `~/.claude/statusline.sh`; all the
//! normalization/formatting lives in `dev_core::statusline`.

use dev_core::statusline;
use serde_json::Value;
use std::io::Read;

pub fn statusline() {
    let mut input = String::new();
    let _ = std::io::stdin().read_to_string(&mut input);
    // Tolerate an empty / unparseable payload: still refresh `updated_at` so the
    // cache doesn't look stale, and print nothing (no five-hour figure to show).
    let payload: Value = serde_json::from_str(input.trim()).unwrap_or(Value::Null);

    let now = statusline::now_secs();
    let cache = statusline::build_cache(&payload, now);

    if let Some(path) = statusline::usage_cache_path() {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(bytes) = serde_json::to_vec_pretty(&cache) {
            // Atomic replace: write a temp sibling then rename, so a concurrent
            // reader (`dev usage`, the TUI) never sees a half-written file.
            let mut tmp = path.clone().into_os_string();
            tmp.push(".tmp");
            let tmp = std::path::PathBuf::from(tmp);
            if std::fs::write(&tmp, &bytes).is_ok() {
                let _ = std::fs::rename(&tmp, &path);
            }
        }
    }

    if let Some(line) = statusline::status_line(&cache) {
        println!("{line}");
    }
}
