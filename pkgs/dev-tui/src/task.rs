//! Task tracker — records dispatched tasks and their lifecycle.
//!
//! Tasks are persisted to `~/.dev/tui-tasks.jsonl` (one JSON object per line).

use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TaskStatus {
    Pending,
    Running,
    Success,
    Error,
    Killed,
}

impl TaskStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Success => "success",
            TaskStatus::Error => "error",
            TaskStatus::Killed => "killed",
        }
    }

    pub(crate) fn from_str(s: &str) -> Self {
        match s {
            "running" => TaskStatus::Running,
            "success" => TaskStatus::Success,
            "error" => TaskStatus::Error,
            "killed" => TaskStatus::Killed,
            _ => TaskStatus::Pending,
        }
    }
}

#[derive(Clone)]
pub(crate) struct Task {
    pub id: u64,
    pub target: String,
    pub tool: String,
    pub model: String,
    pub task_text: String,
    pub status: TaskStatus,
    pub started_at: i64,
    pub finished_at: Option<i64>,
}

impl Task {
    pub(crate) fn new(target: String, tool: String, model: String, task_text: String) -> Self {
        let now = unix_now();
        Self {
            id: now as u64,
            target,
            tool,
            model,
            task_text,
            status: TaskStatus::Pending,
            started_at: now,
            finished_at: None,
        }
    }

    pub(crate) fn to_json(&self) -> Value {
        let mut m = serde_json::Map::new();
        m.insert("id".into(), Value::Number(self.id.into()));
        m.insert("target".into(), Value::String(self.target.clone()));
        m.insert("tool".into(), Value::String(self.tool.clone()));
        m.insert("model".into(), Value::String(self.model.clone()));
        m.insert("task_text".into(), Value::String(self.task_text.clone()));
        m.insert("status".into(), Value::String(self.status.as_str().into()));
        m.insert("started_at".into(), Value::Number(self.started_at.into()));
        match self.finished_at {
            Some(ts) => m.insert("finished_at".into(), Value::Number(ts.into())),
            None => m.insert("finished_at".into(), Value::Null),
        };
        Value::Object(m)
    }

    pub(crate) fn from_json(v: &Value) -> Option<Self> {
        Some(Self {
            id: v.get("id")?.as_u64()?,
            target: v.get("target")?.as_str()?.to_string(),
            tool: v.get("tool")?.as_str()?.to_string(),
            model: v
                .get("model")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            task_text: v
                .get("task_text")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            status: TaskStatus::from_str(
                v.get("status")
                    .and_then(|x| x.as_str())
                    .unwrap_or("pending"),
            ),
            started_at: v.get("started_at")?.as_i64()?,
            finished_at: v.get("finished_at").and_then(|x| x.as_i64()),
        })
    }
}

fn tasks_path() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    Some(format!("{home}/.dev/tui-tasks.jsonl"))
}

pub(crate) fn load_tasks() -> Vec<Task> {
    let path = match tasks_path() {
        Some(p) => p,
        None => return Vec::new(),
    };
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    let all: Vec<Task> = data
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter_map(|v| Task::from_json(&v))
        .collect();
    // Keep most recent 200
    if all.len() > 200 {
        all[all.len() - 200..].to_vec()
    } else {
        all
    }
}

pub(crate) fn save_task(task: &Task) {
    let path = match tasks_path() {
        Some(p) => p,
        None => return,
    };
    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let line = serde_json::to_string(&task.to_json()).unwrap_or_default();
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(f, "{line}");
    }
}

pub(crate) fn save_all_tasks(tasks: &[Task]) {
    let path = match tasks_path() {
        Some(p) => p,
        None => return,
    };
    if let Some(parent) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    use std::io::Write;
    if let Ok(mut f) = std::fs::File::create(&path) {
        for task in tasks {
            let line = serde_json::to_string(&task.to_json()).unwrap_or_default();
            let _ = writeln!(f, "{line}");
        }
    }
}

pub(crate) fn reconcile_tasks(tasks: &mut [Task], envs: &[crate::model::Env]) {
    let now = unix_now();
    let mut changed = false;

    for task in tasks.iter_mut() {
        match task.status {
            TaskStatus::Pending | TaskStatus::Running => {
                // Check if there's a matching agent running for this target
                let agent_match = envs.iter().find(|e| e.name == task.target).and_then(|e| {
                    e.agents.iter().find(|a| {
                        let status = a.status.as_str();
                        status == "running" || status == "busy" || status == "waiting"
                    })
                });

                if let Some(agent) = agent_match {
                    if task.status != TaskStatus::Running {
                        task.status = TaskStatus::Running;
                        changed = true;
                    }
                    // Check if agent has error status
                    if agent.status == "error" {
                        task.status = TaskStatus::Error;
                        task.finished_at = Some(now);
                        changed = true;
                    }
                } else if task.status == TaskStatus::Running {
                    // Was running but agent is gone — check if any agent with
                    // error status exists for this target
                    let has_error = envs
                        .iter()
                        .find(|e| e.name == task.target)
                        .map(|e| e.agents.iter().any(|a| a.status == "error"))
                        .unwrap_or(false);
                    if has_error {
                        task.status = TaskStatus::Error;
                    } else {
                        task.status = TaskStatus::Success;
                    }
                    task.finished_at = Some(now);
                    changed = true;
                } else if task.status == TaskStatus::Pending {
                    // Pending for >24h → Killed
                    if now - task.started_at > 86400 {
                        task.status = TaskStatus::Killed;
                        task.finished_at = Some(now);
                        changed = true;
                    }
                }
            }
            _ => {} // Success, Error, Killed — terminal states
        }
    }

    if changed {
        save_all_tasks(tasks);
    }
}

pub(crate) fn format_elapsed(secs: i64) -> String {
    if secs < 0 {
        return "0s".to_string();
    }
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    if days > 0 {
        format!("{days}d{hours}h")
    } else if hours > 0 {
        format!("{hours}h{minutes}m")
    } else if minutes > 0 {
        format!("{minutes}m{seconds}s")
    } else {
        format!("{seconds}s")
    }
}

pub(crate) fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ── dev task store types (re-exported from dev-core) ─────────────────────────
pub(crate) use dev_core::{load_dev_tasks, load_task_detail, DevQuestion, DevTask, TaskDetail};
