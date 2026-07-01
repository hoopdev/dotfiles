use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Machine-readable snapshot the `dev snapshot --json` command emits and the
/// Zellij board / TUI deserialize. Keeping the wire types here (always-on, no
/// process/ssh features) means every consumer renders exactly what the CLI
/// computes — one source of truth. The Zellij plugin fetches this via
/// `run_command(["dev","snapshot","--json"])` instead of reading the host
/// filesystem (which a WASM plugin cannot do without broad `FullHdAccess`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BoardSnapshot {
    pub tasks: Vec<DevTask>,
    pub questions: Vec<DevQuestion>,
}

/// Build a [`BoardSnapshot`] from the on-disk task store.
pub fn load_board_snapshot() -> BoardSnapshot {
    let (tasks, questions) = load_dev_tasks();
    BoardSnapshot { tasks, questions }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DevTask {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub phase: String,
    pub priority: String,
    pub created_at: String,
    pub updated_at: String,
    pub assigned_tool: Option<String>,
    pub review_status: String,
    pub test_status: String,
    pub diff_files_count: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct QuestionOption {
    pub id: String,
    pub label: String,
    pub impact: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DevQuestion {
    pub id: String,
    pub task_id: String,
    pub project_id: String,
    pub severity: String,
    pub category: String,
    pub question: String,
    pub agent_recommendation: Option<String>,
    pub context: String,
    pub options: Vec<QuestionOption>,
}

pub fn dev_store_path() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(std::path::PathBuf::from(home).join(".dev/projects"))
}

// ── Task detail (for the right-panel in TaskBoard) ────────────────────────────

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TaskDetail {
    pub task_id: String,
    pub brief: String,          // from brief.md, first 300 chars
    pub plan_summary: String,   // from plan.md or approved-plan.md, first 500 chars
    pub handoff: String,        // from handoff.md, first 300 chars
    pub review_summary: String, // latest review content, first 200 chars
}

pub fn load_task_detail(task_id: &str) -> Option<TaskDetail> {
    let store = dev_store_path()?;
    let entries = std::fs::read_dir(&store).ok()?;
    for project_entry in entries.flatten() {
        let task_path = project_entry.path().join("tasks").join(task_id);
        if !task_path.exists() {
            continue;
        }

        let read_first = |path: &std::path::Path, max: usize| -> String {
            std::fs::read_to_string(path)
                .unwrap_or_default()
                .chars()
                .take(max)
                .collect()
        };

        let brief = read_first(&task_path.join("brief.md"), 300);
        let plan_summary = if task_path.join("approved-plan.md").exists() {
            read_first(&task_path.join("approved-plan.md"), 500)
        } else {
            read_first(&task_path.join("plan.md"), 500)
        };
        let handoff = read_first(&task_path.join("handoff.md"), 300);

        // Latest review
        let review_summary = {
            let reviews_dir = task_path.join("reviews");
            let mut latest = String::new();
            if let Ok(rd) = std::fs::read_dir(&reviews_dir) {
                let mut mds: Vec<_> = rd
                    .flatten()
                    .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("md"))
                    .collect();
                mds.sort_by_key(|e| e.file_name());
                if let Some(last) = mds.last() {
                    latest = read_first(&last.path(), 200);
                }
            }
            latest
        };

        return Some(TaskDetail {
            task_id: task_id.to_string(),
            brief,
            plan_summary,
            handoff,
            review_summary,
        });
    }
    None
}

fn vs(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

// ── dispatched-run cards (Layer 1: dispatch → board) ─────────────────────────
//
// A plain `dev agent dispatch` writes only a run record (`~/.dev/runs/*.meta`),
// never a task, so the board never grew from a dispatch. To close that gap the
// board also surfaces *bare* runs — ones not yet bound to a task via
// `links.run_id` — as lightweight cards in the existing `implementing`/"Running"
// lane. This is a file-only reader (no Config/ssh) so it stays usable from the
// always-on `task` module.

/// Local run registry dir (`~/.dev/runs`).
fn runs_dir() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(std::path::PathBuf::from(home).join(".dev").join("runs"))
}

/// Trailing epoch of a `<tool>-<project>-<epoch>` run id.
fn run_epoch(id: &str) -> Option<u64> {
    id.rsplit('-').next().and_then(|s| s.parse::<u64>().ok())
}

/// Bare dispatched runs surfaced as `implementing` board cards. `linked` is the
/// set of run ids already owned by a task (so they aren't shown twice). Only runs
/// from the last 48h are shown, so the board doesn't accrete every historical run
/// (`dev agent prune` still GCs the metas).
fn runs_as_tasks(linked: &std::collections::HashSet<String>) -> Vec<DevTask> {
    const RECENT_SECS: u64 = 48 * 3600;
    let now = {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    };
    let dir = match runs_dir() {
        Some(d) => d,
        None => return Vec::new(),
    };
    let rd = match std::fs::read_dir(&dir) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for e in rd.flatten() {
        if e.path().extension().map(|x| x != "meta").unwrap_or(true) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(e.path()) else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        let id = vs(&v, "id").unwrap_or_default();
        if id.is_empty() || linked.contains(&id) {
            continue;
        }
        if let Some(ep) = run_epoch(&id) {
            if now.saturating_sub(ep) > RECENT_SECS {
                continue;
            }
        }
        let title: String = vs(&v, "task")
            .unwrap_or_default()
            .chars()
            .take(120)
            .collect();
        let started = vs(&v, "started").unwrap_or_default();
        out.push(DevTask {
            id,
            project_id: vs(&v, "project").unwrap_or_default(),
            title,
            phase: "implementing".into(),
            priority: "normal".into(),
            created_at: started.clone(),
            updated_at: started,
            assigned_tool: vs(&v, "tool"),
            review_status: "none".into(),
            test_status: "unknown".into(),
            diff_files_count: 0,
        });
    }
    out
}

/// Load all tasks and open questions from ~/.dev/projects/.
pub fn load_dev_tasks() -> (Vec<DevTask>, Vec<DevQuestion>) {
    let store = match dev_store_path() {
        Some(p) => p,
        None => return (Vec::new(), Vec::new()),
    };
    let mut tasks = Vec::new();
    let mut questions = Vec::new();
    // Run ids already owned by a task — used to dedupe the bare-run cards below.
    let mut linked_runs: std::collections::HashSet<String> = std::collections::HashSet::new();

    let entries = match std::fs::read_dir(&store) {
        Ok(e) => e,
        Err(_) => return (Vec::new(), Vec::new()),
    };

    for project_entry in entries.flatten() {
        let project_path = project_entry.path();
        if !project_path.is_dir() {
            continue;
        }

        // Load open questions from questions.jsonl
        let qfile = project_path.join("questions.jsonl");
        if let Ok(content) = std::fs::read_to_string(&qfile) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if let Ok(v) = serde_json::from_str::<Value>(line) {
                    if v.get("status").and_then(|s| s.as_str()) != Some("open") {
                        continue;
                    }
                    let options = v
                        .get("options")
                        .and_then(|o| o.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|opt| {
                                    Some(QuestionOption {
                                        id: vs(opt, "id")?,
                                        label: vs(opt, "label").unwrap_or_default(),
                                        impact: vs(opt, "impact").unwrap_or_default(),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    questions.push(DevQuestion {
                        id: vs(&v, "id").unwrap_or_default(),
                        task_id: vs(&v, "task_id").unwrap_or_default(),
                        project_id: vs(&v, "project_id").unwrap_or_default(),
                        severity: vs(&v, "severity").unwrap_or_else(|| "blocking".into()),
                        category: vs(&v, "category").unwrap_or_default(),
                        question: vs(&v, "question").unwrap_or_default(),
                        agent_recommendation: vs(&v, "agent_recommendation"),
                        context: vs(&v, "context").unwrap_or_default(),
                        options,
                    });
                }
            }
        }

        // Load tasks from tasks/*/task.json
        let tasks_dir = project_path.join("tasks");
        if let Ok(task_entries) = std::fs::read_dir(&tasks_dir) {
            for task_entry in task_entries.flatten() {
                let task_path = task_entry.path();
                if !task_path.is_dir() {
                    continue;
                }
                let task_json_path = task_path.join("task.json");
                if let Ok(content) = std::fs::read_to_string(&task_json_path) {
                    if let Ok(v) = serde_json::from_str::<Value>(&content) {
                        if let Some(rid) = v
                            .pointer("/links/run_id")
                            .and_then(|x| x.as_str())
                            .filter(|s| !s.is_empty())
                        {
                            linked_runs.insert(rid.to_string());
                        }
                        tasks.push(DevTask {
                            id: vs(&v, "id").unwrap_or_default(),
                            project_id: vs(&v, "project_id").unwrap_or_default(),
                            title: vs(&v, "title").unwrap_or_default(),
                            phase: vs(&v, "phase").unwrap_or_else(|| "draft".into()),
                            priority: vs(&v, "priority").unwrap_or_else(|| "normal".into()),
                            created_at: vs(&v, "created_at").unwrap_or_default(),
                            updated_at: vs(&v, "updated_at").unwrap_or_default(),
                            assigned_tool: vs(&v, "assigned_tool"),
                            review_status: v
                                .pointer("/summary/review_status")
                                .and_then(|s| s.as_str())
                                .unwrap_or("none")
                                .into(),
                            test_status: v
                                .pointer("/summary/test_status")
                                .and_then(|s| s.as_str())
                                .unwrap_or("unknown")
                                .into(),
                            diff_files_count: v
                                .pointer("/summary/diff_files")
                                .and_then(|a| a.as_array())
                                .map(|a| a.len())
                                .unwrap_or(0),
                        });
                    }
                }
            }
        }
    }

    // Surface bare dispatched runs (not owned by any task) as board cards.
    tasks.extend(runs_as_tasks(&linked_runs));

    tasks.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    (tasks, questions)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `BoardSnapshot` wire format is the contract between `dev snapshot
    /// --json` and the Zellij plugin's `run_command` parse. Round-trip it so a
    /// field rename can't silently blank the board.
    #[test]
    fn board_snapshot_round_trips() {
        let snap = BoardSnapshot {
            tasks: vec![DevTask {
                id: "T-20260701-001".into(),
                project_id: "proj".into(),
                title: "wire up snapshot".into(),
                phase: "implementing".into(),
                priority: "high".into(),
                created_at: "2026-07-01T00:00:00Z".into(),
                updated_at: "2026-07-01T01:00:00Z".into(),
                assigned_tool: Some("claude".into()),
                review_status: "none".into(),
                test_status: "unknown".into(),
                diff_files_count: 3,
            }],
            questions: vec![DevQuestion {
                id: "Q-20260701-001".into(),
                task_id: "T-20260701-001".into(),
                project_id: "proj".into(),
                severity: "blocking".into(),
                category: "design".into(),
                question: "which store?".into(),
                agent_recommendation: Some("sqlite".into()),
                context: "ctx".into(),
                options: vec![QuestionOption {
                    id: "a".into(),
                    label: "sqlite".into(),
                    impact: "low".into(),
                }],
            }],
        };
        let json = serde_json::to_string(&snap).unwrap();
        let back: BoardSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snap, back);
    }

    /// Forward-compat: an unknown extra field and missing optionals must not
    /// break the plugin's parse (`#[serde(default)]`).
    #[test]
    fn board_snapshot_tolerates_partial_json() {
        let json = r#"{"tasks":[{"id":"T-1","title":"t","future_field":42}]}"#;
        let snap: BoardSnapshot = serde_json::from_str(json).unwrap();
        assert_eq!(snap.tasks.len(), 1);
        assert_eq!(snap.tasks[0].id, "T-1");
        assert_eq!(snap.tasks[0].phase, ""); // defaulted, not panicked
        assert!(snap.questions.is_empty());
    }
}
