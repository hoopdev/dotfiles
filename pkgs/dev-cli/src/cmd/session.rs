//! `dev session` — list/resume Claude sessions for a project.
//!
//! Sessions live in Claude's own store; `list` reads the per-project
//! `sessions-index.json` under `~/.claude/projects/<enc>/`, `resume` opens an
//! interactive `claude --resume`.

use super::interactive;
use dev_core::config::{Config, Target};
use serde_json::Value;

/// Claude encodes a project path by replacing `/` (and `.`) with `-`.
fn claude_project_dir(path: &str) -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let enc: String = path
        .chars()
        .map(|c| if c == '/' || c == '.' { '-' } else { c })
        .collect();
    Some(
        std::path::PathBuf::from(home)
            .join(".claude/projects")
            .join(enc),
    )
}

fn local_path(cfg: &Config, name: &str) -> Option<String> {
    match cfg.resolve(name)? {
        Target::Local { path, .. } => Some(path),
        _ => None,
    }
}

pub fn list(name: Option<String>, json_out: bool) {
    let cfg = Config::load_or_default();
    let n = interactive::require_project(&cfg, name.as_deref());
    let Some(path) = local_path(&cfg, &n) else {
        eprintln!("dev session list: '{n}' is not a local project");
        std::process::exit(1);
    };
    let index = claude_project_dir(&path).map(|d| d.join("sessions-index.json"));
    let sessions: Vec<Value> = index
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    if json_out {
        println!("{}", serde_json::to_string(&sessions).unwrap());
        return;
    }
    if sessions.is_empty() {
        println!("no sessions for {n}");
        return;
    }
    for s in &sessions {
        let id = s.get("sessionId").and_then(|v| v.as_str()).unwrap_or("?");
        let started = s.get("startedAt").and_then(|v| v.as_str()).unwrap_or("");
        println!("{id}  {started}");
    }
}

pub fn resume(name: Option<String>, session_id: Option<String>) {
    let cfg = Config::load_or_default();
    let n = interactive::require_project(&cfg, name.as_deref());
    let Some(t) = cfg.resolve(&n) else {
        eprintln!("dev session resume: unknown project '{n}'");
        std::process::exit(1);
    };
    let cmd = match session_id {
        Some(id) => format!("claude --resume {}", dev_core::ssh::sh_quote(&id)),
        None => "claude --resume".to_string(),
    };
    interactive::exec_on_target(&cfg, &t, Some(&cmd));
}
