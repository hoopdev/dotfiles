//! `dev snapshot` — one machine-readable snapshot of the whole fleet.
//!
//! Single data endpoint for the Zellij plugin and (soon) the TUI. The plugin
//! deserializes the `tasks`/`questions` fields into `dev_core::BoardSnapshot`
//! (serde ignores the extra `agents`/`git`/`usage` fields), so this stays
//! backward-compatible while giving richer consumers the live fleet state too.

use dev_core::config::Config;
use dev_core::{agent, git, load_board_snapshot, now_iso};
use serde_json::{json, Map, Value};

/// Claude rate-limit cache (`~/.cache/claude/usage.json`), or `{}` if absent /
/// unreadable — usage is best-effort and must never fail the snapshot.
fn usage_value() -> Value {
    let home = std::env::var("HOME").unwrap_or_default();
    let cache = std::env::var("XDG_CACHE_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{home}/.cache"));
    let path = format!("{cache}/claude/usage.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .unwrap_or_else(|| json!({}))
}

pub fn snapshot(json_out: bool) {
    let board = load_board_snapshot();
    let cfg = Config::load_or_default();
    let agents = agent::ps(&cfg);
    let git_status = git::status_all(&cfg, &cfg.list_projects());
    let usage = usage_value();

    if json_out {
        let mut map = Map::new();
        map.insert("generated_at".into(), Value::String(now_iso()));
        map.insert(
            "tasks".into(),
            serde_json::to_value(&board.tasks).unwrap_or_else(|_| json!([])),
        );
        map.insert(
            "questions".into(),
            serde_json::to_value(&board.questions).unwrap_or_else(|_| json!([])),
        );
        map.insert("agents".into(), Value::Array(agents));
        map.insert("git".into(), Value::Array(git_status));
        map.insert("usage".into(), usage);
        match serde_json::to_string(&Value::Object(map)) {
            Ok(s) => println!("{s}"),
            Err(e) => {
                eprintln!("dev snapshot: serialize failed: {e}");
                std::process::exit(1);
            }
        }
    } else {
        let blocking = board
            .questions
            .iter()
            .filter(|q| q.severity == "blocking")
            .count();
        let running = agents
            .iter()
            .filter(|a| a.get("status").and_then(|s| s.as_str()) == Some("running"))
            .count();
        let dirty = git_status
            .iter()
            .filter(|r| r.get("changes").and_then(|c| c.as_i64()).unwrap_or(0) > 0)
            .count();
        println!(
            "tasks: {}  questions: {} ({} blocking)  agents: {} ({} running)  dirty repos: {}",
            board.tasks.len(),
            board.questions.len(),
            blocking,
            agents.len(),
            running,
            dirty,
        );
    }
}
