//! `dev usage` — Claude rate-limit summary from ~/.cache/claude/usage.json
//! (populated by the statusline hook in coder.nix).

use serde_json::Value;

fn usage_path() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let cache = std::env::var("XDG_CACHE_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{home}/.cache"));
    format!("{cache}/claude/usage.json")
}

pub fn usage(json_out: bool) {
    let path = usage_path();
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    if json_out {
        let out = if text.trim().is_empty() {
            "{}".to_string()
        } else {
            text
        };
        println!("{}", out.trim_end());
        return;
    }
    if text.trim().is_empty() {
        println!("no usage data ({path})");
        return;
    }
    let Ok(v) = serde_json::from_str::<Value>(&text) else {
        println!("no usage data (unparseable {path})");
        return;
    };
    // Treat an already-reset window as 0% even if the cache is stale (shared with
    // the statusline writer + the TUI reader via dev_core::statusline).
    let now = dev_core::statusline::now_secs();
    let five = dev_core::statusline::window_pct(v.get("five_hour"), now);
    let seven = dev_core::statusline::window_pct(v.get("seven_day"), now);
    match (five, seven) {
        (Some(f), Some(s)) => println!("5h:{f:.0}% 7d:{s:.0}%"),
        (Some(f), None) => println!("5h:{f:.0}%"),
        _ => println!("no usage data"),
    }
}
