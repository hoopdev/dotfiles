//! Task store write operations.

use crate::task::{dev_store_path, DevQuestion, DevTask, QuestionOption};
use serde_json::Value;
use std::path::{Path, PathBuf};

// ── helpers ──────────────────────────────────────────────────────────────────

pub fn now_iso() -> String {
    // Use SystemTime since chrono not available
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Format as ISO 8601 UTC manually
    let s = secs;
    let sec = s % 60;
    let min = (s / 60) % 60;
    let hour = (s / 3600) % 24;
    let days = s / 86400;
    // Days since epoch to date
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

fn days_to_ymd(mut days: u64) -> (u32, u32, u32) {
    let mut year = 1970u32;
    loop {
        let leap = is_leap(year);
        let ydays = if leap { 366 } else { 365 };
        if days < ydays {
            break;
        }
        days -= ydays;
        year += 1;
    }
    let months = [
        31u64,
        if is_leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u32;
    for m in &months {
        if days < *m {
            break;
        }
        days -= m;
        month += 1;
    }
    (year, month, days as u32 + 1)
}

fn is_leap(y: u32) -> bool {
    y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400))
}

fn vs(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

// ── ID generation ─────────────────────────────────────────────────────────────

fn next_id(prefix: &str, dir: &Path) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (year, month, day) = days_to_ymd(secs / 86400);
    let date = format!("{year:04}{month:02}{day:02}");
    let pattern = format!("{prefix}-{date}-");
    let mut max = 0usize;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if let Some(rest) = name.strip_prefix(&pattern) {
                let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(n) = digits.parse::<usize>() {
                    if n > max {
                        max = n;
                    }
                }
            }
        }
    }
    format!("{prefix}-{date}-{:03}", max + 1)
}

pub fn next_task_id(project_id: &str) -> Result<String, std::io::Error> {
    let dir = dev_store_path()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "HOME not set"))?
        .join(project_id)
        .join("tasks");
    std::fs::create_dir_all(&dir)?;
    Ok(next_id("T", &dir))
}

pub fn next_question_id(project_id: &str) -> Result<String, std::io::Error> {
    let dir = dev_store_path()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "HOME not set"))?
        .join(project_id);
    std::fs::create_dir_all(&dir)?;
    Ok(next_id("Q", &dir))
}

pub fn next_review_id(project_id: &str) -> Result<String, std::io::Error> {
    let dir = dev_store_path()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "HOME not set"))?
        .join(project_id);
    Ok(next_id("R", &dir))
}

pub fn next_test_run_id(project_id: &str) -> Result<String, std::io::Error> {
    let dir = dev_store_path()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "HOME not set"))?
        .join(project_id);
    Ok(next_id("V", &dir))
}

pub fn next_review_id_in(dir: &Path) -> Result<String, std::io::Error> {
    std::fs::create_dir_all(dir)?;
    Ok(next_id("R", dir))
}

pub fn next_test_run_id_in(dir: &Path) -> Result<String, std::io::Error> {
    std::fs::create_dir_all(dir)?;
    Ok(next_id("V", dir))
}

// ── task lookup ───────────────────────────────────────────────────────────────

/// Find the task directory for a given task_id by scanning all projects.
pub fn find_task_dir(task_id: &str) -> Option<PathBuf> {
    let store = dev_store_path()?;
    let entries = std::fs::read_dir(&store).ok()?;
    for project_entry in entries.flatten() {
        let p = project_entry.path().join("tasks").join(task_id);
        if p.join("task.json").exists() {
            return Some(p);
        }
    }
    None
}

/// Find the project directory that owns a given task_id.
pub fn find_project_dir_for_task(task_id: &str) -> Option<PathBuf> {
    let store = dev_store_path()?;
    let entries = std::fs::read_dir(&store).ok()?;
    for project_entry in entries.flatten() {
        let p = project_entry.path();
        if p.join("tasks").join(task_id).join("task.json").exists() {
            return Some(p);
        }
    }
    None
}

/// Find the project directory by scanning questions.jsonl for a question_id.
pub fn find_project_dir_for_question(question_id: &str) -> Option<PathBuf> {
    let store = dev_store_path()?;
    let entries = std::fs::read_dir(&store).ok()?;
    for project_entry in entries.flatten() {
        let p = project_entry.path();
        let qfile = p.join("questions.jsonl");
        if let Ok(content) = std::fs::read_to_string(&qfile) {
            for line in content.lines() {
                if let Ok(v) = serde_json::from_str::<Value>(line) {
                    if v.get("id").and_then(|x| x.as_str()) == Some(question_id) {
                        return Some(p);
                    }
                }
            }
        }
    }
    None
}

// ── task CRUD ─────────────────────────────────────────────────────────────────

/// Create a new task in the given project. Returns the created DevTask.
pub fn task_new(
    project_id: &str,
    title: &str,
    brief: Option<&str>,
    priority: Option<&str>,
) -> Result<DevTask, std::io::Error> {
    let store = dev_store_path()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "HOME not set"))?;
    let project_dir = store.join(project_id);
    std::fs::create_dir_all(&project_dir)?;

    // Ensure project.json
    let project_json = project_dir.join("project.json");
    if !project_json.exists() {
        let ts = now_iso();
        let pj = serde_json::json!({
            "id": project_id,
            "created_at": ts,
            "updated_at": ts
        });
        write_json_atomic(&project_json, &pj)?;
    }

    let task_id = next_task_id(project_id)?;
    let task_dir = project_dir.join("tasks").join(&task_id);
    std::fs::create_dir_all(&task_dir)?;
    std::fs::create_dir_all(task_dir.join("reviews"))?;
    std::fs::create_dir_all(task_dir.join("test-results"))?;

    let ts = now_iso();
    let task_json = serde_json::json!({
        "id": task_id,
        "project_id": project_id,
        "title": title,
        "phase": "draft",
        "priority": priority.unwrap_or("normal"),
        "created_at": ts,
        "updated_at": ts,
        "created_by": "human",
        "assigned_tool": null,
        "assigned_model": null,
        "worktree_branch": null,
        "worktree_path": null,
        "scope": {"paths":[],"allowed_paths":[],"forbidden_paths":[],"risk":"unknown"},
        "validation": {"commands":[],"required":true},
        "links": {"run_id":null,"session_id":null,"pr_url":null},
        "summary": {
            "latest_question":null,"latest_handoff":null,
            "diff_files":[],"review_status":"none","test_status":"unknown"
        }
    });
    write_json_atomic(&task_dir.join("task.json"), &task_json)?;

    if let Some(b) = brief {
        std::fs::write(task_dir.join("brief.md"), b)?;
    }

    // Append task_created event
    event_append(
        &task_dir,
        "task_created",
        "human",
        &format!("task created: {title}"),
        None,
    )?;

    Ok(DevTask {
        id: task_id,
        project_id: project_id.to_string(),
        title: title.to_string(),
        phase: "draft".to_string(),
        priority: priority.unwrap_or("normal").to_string(),
        created_at: ts.clone(),
        updated_at: ts,
        assigned_tool: None,
        review_status: "none".to_string(),
        test_status: "unknown".to_string(),
        diff_files_count: 0,
    })
}

// ── events ────────────────────────────────────────────────────────────────────

pub fn event_append(
    task_dir: &Path,
    event_type: &str,
    actor: &str,
    message: &str,
    extra: Option<Value>,
) -> Result<(), std::io::Error> {
    let ts = now_iso();
    let mut ev = extra.unwrap_or_else(|| serde_json::json!({}));
    ev["ts"] = Value::String(ts);
    ev["type"] = Value::String(event_type.to_string());
    ev["actor"] = Value::String(actor.to_string());
    ev["message"] = Value::String(message.to_string());
    let line = serde_json::to_string(&ev).unwrap();
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(task_dir.join("events.jsonl"))?;
    writeln!(f, "{}", line)?;
    Ok(())
}

// ── phase transition ──────────────────────────────────────────────────────────

/// Atomically write `contents` to `path`: write a uniquely-named sibling temp
/// file, then rename it over the target. A concurrent reader (the board / TUI
/// polling the store every few seconds) therefore never observes a half-written
/// file, and a crash mid-write leaves the previous file intact. Same-directory
/// rename is atomic on POSIX. (This bounds *corruption*; a lost update between
/// two writers that both started from the same base is still possible — that
/// would need file locking.)
fn write_atomic(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let base = path.file_name().and_then(|f| f.to_str()).unwrap_or("store");
    let uniq = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = dir.join(format!(".{base}.{}.{uniq}.tmp", std::process::id()));
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}

/// Serialize `v` as pretty JSON and [`write_atomic`] it — no serialize `unwrap`.
fn write_json_atomic(path: &Path, v: &Value) -> std::io::Result<()> {
    let bytes = serde_json::to_vec_pretty(v)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    write_atomic(path, &bytes)
}

pub fn task_phase_set(
    task_dir: &Path,
    new_phase: &str,
    actor: &str,
    message: &str,
) -> Result<(), std::io::Error> {
    let json_path = task_dir.join("task.json");
    let content = std::fs::read_to_string(&json_path)?;
    let mut v: Value = serde_json::from_str(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let old_phase = vs(&v, "phase").unwrap_or_else(|| "draft".to_string());
    v["phase"] = Value::String(new_phase.to_string());
    v["updated_at"] = Value::String(now_iso());
    write_json_atomic(&json_path, &v)?;
    event_append(
        task_dir,
        "phase_changed",
        actor,
        message,
        Some(serde_json::json!({"from": old_phase, "to": new_phase})),
    )?;
    Ok(())
}

pub fn task_update_field(task_dir: &Path, key: &str, value: Value) -> Result<(), std::io::Error> {
    let json_path = task_dir.join("task.json");
    let content = std::fs::read_to_string(&json_path)?;
    let mut v: Value = serde_json::from_str(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    v[key] = value;
    v["updated_at"] = Value::String(now_iso());
    write_json_atomic(&json_path, &v)?;
    Ok(())
}

// ── plan ──────────────────────────────────────────────────────────────────────

pub fn plan_write(task_dir: &Path, content: &str) -> Result<(), std::io::Error> {
    std::fs::write(task_dir.join("plan.md"), content)?;
    event_append(task_dir, "plan_written", "agent", "plan written", None)?;
    Ok(())
}

pub fn plan_approve(task_dir: &Path) -> Result<(), std::io::Error> {
    let plan = std::fs::read_to_string(task_dir.join("plan.md"))?;
    std::fs::write(task_dir.join("approved-plan.md"), &plan)?;
    event_append(task_dir, "plan_approved", "human", "plan approved", None)?;
    Ok(())
}

// ── handoff ───────────────────────────────────────────────────────────────────

pub fn handoff_write(task_dir: &Path, content: &str) -> Result<(), std::io::Error> {
    std::fs::write(task_dir.join("handoff.md"), content)?;
    // Update summary.latest_handoff
    let summary_preview: String = content.chars().take(150).collect();
    let json_path = task_dir.join("task.json");
    if let Ok(c) = std::fs::read_to_string(&json_path) {
        if let Ok(mut v) = serde_json::from_str::<Value>(&c) {
            v["summary"]["latest_handoff"] = Value::String(summary_preview);
            v["updated_at"] = Value::String(now_iso());
            let _ = write_json_atomic(&json_path, &v);
        }
    }
    event_append(
        task_dir,
        "handoff_written",
        "agent",
        "handoff written",
        None,
    )?;
    Ok(())
}

// ── questions ─────────────────────────────────────────────────────────────────

pub fn blocking_questions_open(project_dir: &Path, task_id: &str) -> usize {
    let qfile = project_dir.join("questions.jsonl");
    let content = match std::fs::read_to_string(&qfile) {
        Ok(c) => c,
        Err(_) => return 0,
    };
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|v| {
            v.get("task_id").and_then(|x| x.as_str()) == Some(task_id)
                && v.get("status").and_then(|x| x.as_str()) == Some("open")
                && v.get("severity").and_then(|x| x.as_str()) == Some("blocking")
        })
        .count()
}

#[allow(clippy::too_many_arguments)]
pub fn question_new(
    project_dir: &Path,
    task_id: &str,
    project_id: &str,
    question: &str,
    severity: &str,
    category: &str,
    options: Vec<QuestionOption>,
    recommendation: Option<&str>,
    context: Option<&str>,
) -> Result<DevQuestion, std::io::Error> {
    // Generate Q-YYYYMMDD-NNN id
    let qid = next_question_id(project_id)?;
    let ts = now_iso();
    let opts_json: Vec<Value> = options
        .iter()
        .map(|o| {
            serde_json::json!({
                "id": o.id, "label": o.label, "impact": o.impact
            })
        })
        .collect();
    let q = serde_json::json!({
        "id": qid,
        "task_id": task_id,
        "project_id": project_id,
        "status": "open",
        "severity": severity,
        "category": category,
        "question": question,
        "options": opts_json,
        "agent_recommendation": recommendation,
        "context": context.unwrap_or(""),
        "created_at": ts,
        "answered_at": null,
        "answer": null
    });
    use std::io::Write;
    let qfile = project_dir.join("questions.jsonl");
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&qfile)?;
    writeln!(f, "{}", serde_json::to_string(&q).unwrap())?;
    Ok(DevQuestion {
        id: qid,
        task_id: task_id.to_string(),
        project_id: project_id.to_string(),
        severity: severity.to_string(),
        category: category.to_string(),
        question: question.to_string(),
        agent_recommendation: recommendation.map(|s| s.to_string()),
        context: context.unwrap_or("").to_string(),
        options,
    })
}

pub fn question_answer(
    project_dir: &Path,
    question_id: &str,
    answer: &str,
) -> Result<(), std::io::Error> {
    let qfile = project_dir.join("questions.jsonl");
    let content = std::fs::read_to_string(&qfile)?;
    let ts = now_iso();
    let new_content: String = content
        .lines()
        .map(|line| {
            if let Ok(mut v) = serde_json::from_str::<Value>(line) {
                if v.get("id").and_then(|x| x.as_str()) == Some(question_id) {
                    v["status"] = Value::String("answered".to_string());
                    v["answer"] = Value::String(answer.to_string());
                    v["answered_at"] = Value::String(ts.clone());
                    return serde_json::to_string(&v).unwrap_or_else(|_| line.to_string());
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");
    write_atomic(&qfile, (new_content + "\n").as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_atomic_replaces_and_leaves_no_temp() {
        let dir = std::env::temp_dir().join(format!("dev-store-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("task.json");

        write_json_atomic(&path, &serde_json::json!({"a": 1})).unwrap();
        write_json_atomic(&path, &serde_json::json!({"a": 2})).unwrap();

        let back: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            back["a"], 2,
            "second write wins, file is complete valid JSON"
        );

        // The temp file must have been renamed away, not left as litter.
        let leftover_tmp = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leftover_tmp, 0);

        std::fs::remove_dir_all(&dir).ok();
    }
}
