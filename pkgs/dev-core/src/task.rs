use serde_json::Value;

#[derive(Clone, Debug, Default)]
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

#[derive(Clone, Debug, Default)]
pub struct QuestionOption {
    pub id: String,
    pub label: String,
    pub impact: String,
}

#[derive(Clone, Debug, Default)]
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

#[derive(Clone, Debug, Default)]
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
                    .filter(|e| {
                        e.path().extension().and_then(|x| x.to_str()) == Some("md")
                    })
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

/// Load all tasks and open questions from ~/.dev/projects/.
pub fn load_dev_tasks() -> (Vec<DevTask>, Vec<DevQuestion>) {
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
