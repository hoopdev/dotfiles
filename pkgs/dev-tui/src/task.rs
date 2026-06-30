//! Task tracker — records dispatched tasks and their lifecycle.
//!
//! Tasks are persisted to `~/.dev/tui-tasks.jsonl` (one JSON object per line).

use std::time::{SystemTime, UNIX_EPOCH};
use serde_json::Value;

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
            model: v.get("model").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            task_text: v.get("task_text").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            status: TaskStatus::from_str(
                v.get("status").and_then(|x| x.as_str()).unwrap_or("pending"),
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

pub(crate) fn reconcile_tasks(tasks: &mut Vec<Task>, envs: &[crate::model::Env]) {
    let now = unix_now();
    let mut changed = false;

    for task in tasks.iter_mut() {
        match task.status {
            TaskStatus::Pending | TaskStatus::Running => {
                // Check if there's a matching agent running for this target
                let agent_match = envs.iter()
                    .find(|e| e.name == task.target)
                    .and_then(|e| {
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
                    let has_error = envs.iter()
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

// ── dev task store types ──────────────────────────────────────────────────────

#[derive(Clone, Debug, Default)]
pub(crate) struct DevTask {
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

#[derive(Clone, Debug, Default)]
pub(crate) struct QuestionOption {
    pub id: String,
    pub label: String,
    pub impact: String,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct DevQuestion {
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

fn dev_store_path() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(std::path::PathBuf::from(home).join(".dev/projects"))
}

fn vs(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Load all tasks and open questions from ~/.dev/projects/.
pub(crate) fn load_dev_tasks() -> (Vec<DevTask>, Vec<DevQuestion>) {
    let store = match dev_store_path() {
        Some(p) => p,
        None => return (Vec::new(), Vec::new()),
    };
    let mut tasks = Vec::new();
    let mut questions = Vec::new();

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

    tasks.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    (tasks, questions)
}
