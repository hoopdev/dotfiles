//! dev task subcommands — task store CRUD, questions, plan.

use clap::Subcommand;
use dev_core::*;
use serde_json::Value;
use std::path::Path;

// ── JSON output helpers ───────────────────────────────────────────────────────

fn json_ok(task_id: &str, project_id: &str, phase: &str, message: &str) {
    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "task_id": task_id,
            "project_id": project_id,
            "phase": phase,
            "message": message,
        })
    );
}

/// Emit the `--json` failure receipt for a dispatch that never launched. Uses
/// serde so an error message with quotes/backslashes/newlines can't produce
/// invalid JSON on stdout.
fn json_dispatch_err(task_id: &str, msg: &str) {
    println!(
        "{}",
        serde_json::json!({
            "ok": false,
            "error": "dispatch_failed",
            "message": msg,
            "task_id": task_id,
        })
    );
}

fn read_task_json(task_dir: &Path) -> Value {
    let content =
        std::fs::read_to_string(task_dir.join("task.json")).unwrap_or_else(|_| "{}".to_string());
    serde_json::from_str(&content).unwrap_or_default()
}

fn vs(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn truncate_chars(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// Launch a background agent in-process via [`dev_core::agent::dispatch`] and
/// return its run id.
///
/// This is the one bridge from the task store (`~/.dev/projects`) to the run
/// registry (`~/.dev/runs`). Callers launch first and only mutate task state
/// (phase/links/events) when the launch actually succeeded, then record
/// `links.run_id` back onto the task. `supervise` is `false`: the task already
/// exists, so dispatch must not create a second one. (This replaced a
/// `dev agent dispatch --json` self-subprocess.)
fn spawn_dispatch(
    project_id: &str,
    tool: &str,
    worktree: Option<&str>,
    model: Option<&str>,
    prompt: &str,
) -> Result<String, String> {
    let cfg = dev_core::config::Config::load_or_default();
    let opts = dev_core::agent::DispatchOpts {
        backend: tool,
        task: prompt,
        model,
        effort: None,
        sandbox: None,
        worktree,
        supervise: false,
    };
    match dev_core::agent::dispatch(&cfg, project_id, &opts) {
        Ok(v) => Ok(vs(&v, "id")),
        Err(e) => Err(format!("{e:#}")),
    }
}

// ── clap subcommand tree ──────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum TaskCmd {
    /// Create task.
    New {
        project: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        brief: Option<String>,
        #[arg(long)]
        priority: Option<String>,
    },
    /// List tasks.
    List {
        project: Option<String>,
        #[arg(long)]
        phase: Option<String>,
    },
    /// Show task.
    Show { task_id: Option<String> },
    /// Agent context.
    Context {
        task_id: Option<String>,
        #[arg(long)]
        markdown: bool,
    },
    /// Task events.
    Events { task_id: Option<String> },
    /// Open question.
    Ask {
        task_id: Option<String>,
        question: Option<String>,
        #[arg(long, default_value = "blocking")]
        severity: String,
        #[arg(long, default_value = "behavior")]
        category: String,
        #[arg(long)]
        context: Option<String>,
        #[arg(long)]
        recommendation: Option<String>,
    },
    /// Answer question.
    Answer {
        question_id: Option<String>,
        answer: Option<String>,
    },
    /// Save plan.
    WritePlan {
        task_id: Option<String>,
        #[arg(long)]
        file: Option<String>,
    },
    /// Approve plan.
    Approve { task_id: Option<String> },
    /// Reject task.
    Reject {
        task_id: Option<String>,
        #[arg(long)]
        reason: Option<String>,
    },
    /// Start planning agent.
    Plan {
        task_id: Option<String>,
        #[arg(long, default_value = "claude")]
        tool: String,
        #[arg(long)]
        model: Option<String>,
    },
    /// Dispatch implementation agent.
    Dispatch {
        task_id: Option<String>,
        #[arg(long, default_value = "claude")]
        tool: String,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        worktree: Option<String>,
    },
    /// Attach to agent.
    Attach { task_id: Option<String> },
    /// Tail logs.
    Logs {
        task_id: Option<String>,
        #[arg(short = 'f')]
        follow: bool,
    },
    /// Kill agent.
    Kill { task_id: Option<String> },
    /// Write handoff.
    WriteHandoff {
        task_id: Option<String>,
        #[arg(long)]
        file: Option<String>,
    },
    /// Show handoff.
    Handoff {
        task_id: Option<String>,
        #[arg(long)]
        markdown: bool,
    },
    /// Update from worktree.
    Harvest { task_id: Option<String> },
    /// Show diff.
    Diff {
        task_id: Option<String>,
        #[arg(long)]
        stat: bool,
    },
    /// Run validation.
    Test {
        task_id: Option<String>,
        #[arg(long)]
        cmd: Option<String>,
    },
    /// Review diff.
    Review {
        task_id: Option<String>,
        #[arg(long)]
        tool: Option<String>,
    },
    /// Dispatch fix agent.
    Fix {
        task_id: Option<String>,
        #[arg(long, default_value = "claude")]
        tool: String,
        #[arg(long)]
        model: Option<String>,
    },
    /// Create PR.
    Pr {
        task_id: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long, default_value = "main")]
        base: String,
        #[arg(long)]
        draft: bool,
    },
}

// ── dispatch ──────────────────────────────────────────────────────────────────

pub fn run(cmd: TaskCmd, json: bool) -> anyhow::Result<()> {
    match cmd {
        TaskCmd::New {
            project,
            title,
            brief,
            priority,
        } => cmd_new(project, title, brief, priority, json),
        TaskCmd::List { project, phase } => cmd_list(project, phase, json),
        TaskCmd::Show { task_id } => cmd_show(task_id, json),
        TaskCmd::Context { task_id, markdown } => cmd_context(task_id, markdown, json),
        TaskCmd::Events { task_id } => cmd_events(task_id, json),
        TaskCmd::Ask {
            task_id,
            question,
            severity,
            category,
            context,
            recommendation,
        } => cmd_ask(
            task_id,
            question,
            severity,
            category,
            context,
            recommendation,
            json,
        ),
        TaskCmd::Answer {
            question_id,
            answer,
        } => cmd_answer(question_id, answer, json),
        TaskCmd::WritePlan { task_id, file } => cmd_write_plan(task_id, file, json),
        TaskCmd::Approve { task_id } => cmd_approve(task_id, json),
        TaskCmd::Reject { task_id, reason } => cmd_reject(task_id, reason, json),
        TaskCmd::Plan {
            task_id,
            tool,
            model,
        } => cmd_plan(task_id, tool, model, json),
        TaskCmd::Dispatch {
            task_id,
            tool,
            model,
            worktree,
        } => cmd_dispatch(task_id, tool, model, worktree, json),
        TaskCmd::Attach { task_id } => cmd_attach(task_id),
        TaskCmd::Logs { task_id, follow } => cmd_logs(task_id, follow, json),
        TaskCmd::Kill { task_id } => cmd_kill(task_id, json),
        TaskCmd::WriteHandoff { task_id, file } => cmd_write_handoff(task_id, file, json),
        TaskCmd::Handoff {
            task_id,
            markdown: _,
        } => cmd_handoff(task_id, json),
        TaskCmd::Harvest { task_id } => cmd_harvest(task_id, json),
        TaskCmd::Diff { task_id, stat } => cmd_diff(task_id, stat, json),
        TaskCmd::Test { task_id, cmd } => cmd_test(task_id, cmd, json),
        TaskCmd::Review { task_id, tool } => cmd_review(task_id, tool, json),
        TaskCmd::Fix {
            task_id,
            tool,
            model,
        } => cmd_fix(task_id, tool, model, json),
        TaskCmd::Pr {
            task_id,
            title,
            base,
            draft,
        } => cmd_pr(task_id, title, base, draft, json),
    }
}

// ── new ───────────────────────────────────────────────────────────────────────

fn cmd_new(
    project: Option<String>,
    title: Option<String>,
    brief: Option<String>,
    priority: Option<String>,
    json_out: bool,
) -> anyhow::Result<()> {
    let project_id = project.unwrap_or_default();
    let title = title.unwrap_or_default();
    if project_id.is_empty() || title.is_empty() {
        eprintln!("Usage: dev task new <project> --title <title> [--brief <text>]");
        std::process::exit(1);
    }
    let task =
        task_new(&project_id, &title, brief.as_deref(), priority.as_deref()).unwrap_or_else(|e| {
            eprintln!("error: {e}");
            std::process::exit(1);
        });
    if json_out {
        json_ok(&task.id, &task.project_id, "draft", "task created");
    } else {
        println!("created: {} ({})", task.id, task.project_id);
    }
    Ok(())
}

// ── list ──────────────────────────────────────────────────────────────────────

fn cmd_list(project: Option<String>, phase: Option<String>, json_out: bool) -> anyhow::Result<()> {
    let project_filter = project;
    let phase_filter = phase;
    let (tasks, _) = load_dev_tasks();
    let tasks: Vec<_> = tasks
        .iter()
        .filter(|t| project_filter.as_ref().is_none_or(|p| &t.project_id == p))
        .filter(|t| phase_filter.as_ref().is_none_or(|p| &t.phase == p))
        .collect();
    if json_out {
        let arr: Vec<Value> = tasks
            .iter()
            .map(|t| {
                serde_json::json!({
                    "id": t.id, "project_id": t.project_id, "title": t.title,
                    "phase": t.phase, "priority": t.priority
                })
            })
            .collect();
        println!("{}", serde_json::to_string(&arr)?);
    } else {
        if tasks.is_empty() {
            println!("(no tasks)");
            return Ok(());
        }
        for t in &tasks {
            println!(
                "{:<20}  {:<14}  {:<12}  {}",
                t.id, t.project_id, t.phase, t.title
            );
        }
    }
    Ok(())
}

// ── show ──────────────────────────────────────────────────────────────────────

fn cmd_show(task_id: Option<String>, json_out: bool) -> anyhow::Result<()> {
    let task_id = task_id.unwrap_or_default();
    if task_id.is_empty() {
        eprintln!("Usage: dev task show <task-id>");
        std::process::exit(1);
    }
    let tdir = find_task_dir(&task_id).unwrap_or_else(|| {
        eprintln!("not found: {task_id}");
        std::process::exit(1);
    });
    let v = read_task_json(&tdir);
    if json_out {
        println!("{}", serde_json::to_string_pretty(&v)?);
    } else {
        println!("id:       {}", vs(&v, "id"));
        println!("title:    {}", vs(&v, "title"));
        println!("phase:    {}", vs(&v, "phase"));
        println!("project:  {}", vs(&v, "project_id"));
        println!("priority: {}", vs(&v, "priority"));
        println!("tool:     {}", vs(&v, "assigned_tool"));
        println!("worktree: {}", vs(&v, "worktree_branch"));
        println!("created:  {}", vs(&v, "created_at"));
        println!("updated:  {}", vs(&v, "updated_at"));
    }
    Ok(())
}

// ── context ───────────────────────────────────────────────────────────────────

fn cmd_context(task_id: Option<String>, markdown: bool, json_out: bool) -> anyhow::Result<()> {
    let task_id = task_id.unwrap_or_default();
    if task_id.is_empty() {
        eprintln!("Usage: dev task context <task-id> [--markdown|--json]");
        std::process::exit(1);
    }
    let tdir = find_task_dir(&task_id).unwrap_or_else(|| {
        eprintln!("not found: {task_id}");
        std::process::exit(1);
    });
    let v = read_task_json(&tdir);
    let pdir = find_project_dir_for_task(&task_id).unwrap();

    let read_md = |name: &str| std::fs::read_to_string(tdir.join(name)).unwrap_or_default();
    let read_pmd = |name: &str| std::fs::read_to_string(pdir.join(name)).unwrap_or_default();

    let brief = read_md("brief.md");
    let plan = if tdir.join("approved-plan.md").exists() {
        read_md("approved-plan.md")
    } else {
        read_md("plan.md")
    };
    let project_md = read_pmd("project.md");

    // Open questions
    let qfile = pdir.join("questions.jsonl");
    let open_questions: Vec<Value> = std::fs::read_to_string(&qfile)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|q| {
            q.get("task_id").and_then(|x| x.as_str()) == Some(&task_id)
                && q.get("status").and_then(|x| x.as_str()) == Some("open")
        })
        .collect();

    if json_out {
        let ctx = serde_json::json!({
            "task_id": task_id,
            "task": v,
            "brief": brief,
            "plan": plan,
            "project_md": project_md,
            "open_questions": open_questions,
        });
        println!("{}", serde_json::to_string_pretty(&ctx)?);
    } else if markdown {
        println!("# Task Context: {task_id}");
        println!();
        if !project_md.is_empty() {
            println!("## Project\n\n{project_md}\n");
        }
        if !brief.is_empty() {
            println!("## Brief\n\n{brief}\n");
        }
        if !plan.is_empty() {
            println!("## Plan\n\n{plan}\n");
        }
        if !open_questions.is_empty() {
            println!("## Open Questions\n");
            for q in &open_questions {
                let qid = q.get("id").and_then(|x| x.as_str()).unwrap_or("");
                let qtext = q.get("question").and_then(|x| x.as_str()).unwrap_or("");
                let sev = q.get("severity").and_then(|x| x.as_str()).unwrap_or("");
                println!("- [{qid}] ({sev}) {qtext}");
            }
            println!();
        }
        println!(
            "## Task JSON\n\n```json\n{}\n```",
            serde_json::to_string_pretty(&v)?
        );
    } else {
        // Human readable
        println!("task: {} ({})", vs(&v, "id"), vs(&v, "phase"));
        println!("title: {}", vs(&v, "title"));
        if !brief.is_empty() {
            println!("\n--- brief ---\n{brief}");
        }
        if !plan.is_empty() {
            println!("\n--- plan ---\n{}", truncate_chars(&plan, 500));
        }
    }
    Ok(())
}

// ── events ────────────────────────────────────────────────────────────────────

fn cmd_events(task_id: Option<String>, json_out: bool) -> anyhow::Result<()> {
    let task_id = task_id.unwrap_or_default();
    if task_id.is_empty() {
        eprintln!("Usage: dev task events <task-id>");
        std::process::exit(1);
    }
    let tdir = find_task_dir(&task_id).unwrap_or_else(|| {
        eprintln!("not found: {task_id}");
        std::process::exit(1);
    });
    let content = std::fs::read_to_string(tdir.join("events.jsonl")).unwrap_or_default();
    let events: Vec<Value> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect();
    if json_out {
        println!("{}", serde_json::to_string(&events)?);
    } else {
        for ev in &events {
            let ts = ev.get("ts").and_then(|x| x.as_str()).unwrap_or("?");
            let t = ev.get("type").and_then(|x| x.as_str()).unwrap_or("?");
            let msg = ev.get("message").and_then(|x| x.as_str()).unwrap_or("");
            println!("{ts}  {t:<25}  {msg}");
        }
    }
    Ok(())
}

// ── ask ───────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn cmd_ask(
    task_id: Option<String>,
    question: Option<String>,
    severity: String,
    category: String,
    context: Option<String>,
    recommendation: Option<String>,
    json_out: bool,
) -> anyhow::Result<()> {
    let task_id = task_id.unwrap_or_default();
    let question = question.unwrap_or_default();
    if task_id.is_empty() || question.is_empty() {
        eprintln!(
            "Usage: dev task ask <task-id> <question> [--severity blocking] [--category behavior]"
        );
        std::process::exit(1);
    }
    let tdir = find_task_dir(&task_id).unwrap_or_else(|| {
        eprintln!("not found: {task_id}");
        std::process::exit(1);
    });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let pdir = find_project_dir_for_task(&task_id).unwrap();

    let q = question_new(
        &pdir,
        &task_id,
        &project_id,
        &question,
        &severity,
        &category,
        vec![],
        recommendation.as_deref(),
        context.as_deref(),
    )
    .unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });

    event_append(
        &tdir,
        "question_opened",
        "agent",
        &format!("question opened: {}", q.id),
        Some(serde_json::json!({"question_id": q.id, "severity": severity})),
    )
    .ok();

    // If blocking, set phase to needs_spec
    if severity == "blocking" {
        task_phase_set(&tdir, "needs_spec", "dev", "blocking question opened").ok();
        if json_out {
            json_ok(
                &task_id,
                &project_id,
                "needs_spec",
                &format!("question opened: {}", q.id),
            );
        } else {
            println!("question: {} (needs_spec)", q.id);
        }
    } else {
        let phase = vs(&v, "phase");
        if json_out {
            json_ok(
                &task_id,
                &project_id,
                &phase,
                &format!("question opened: {}", q.id),
            );
        } else {
            println!("question: {}", q.id);
        }
    }
    Ok(())
}

// ── answer ────────────────────────────────────────────────────────────────────

fn cmd_answer(
    question_id: Option<String>,
    answer: Option<String>,
    json_out: bool,
) -> anyhow::Result<()> {
    let question_id = question_id.unwrap_or_default();
    let answer = answer.unwrap_or_default();
    if question_id.is_empty() || answer.is_empty() {
        eprintln!("Usage: dev task answer <question-id> <answer>");
        std::process::exit(1);
    }
    let pdir = find_project_dir_for_question(&question_id).unwrap_or_else(|| {
        eprintln!("question not found: {question_id}");
        std::process::exit(1);
    });
    question_answer(&pdir, &question_id, &answer).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });

    // Find task_id from the question
    let qfile = pdir.join("questions.jsonl");
    let content = std::fs::read_to_string(&qfile).unwrap_or_default();
    let task_id: String = content
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|v| v.get("id").and_then(|x| x.as_str()) == Some(&question_id))
        .map(|v| {
            v.get("task_id")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string()
        })
        .next()
        .unwrap_or_default();

    if !task_id.is_empty() {
        if let Some(tdir) = find_task_dir(&task_id) {
            let remaining = blocking_questions_open(&pdir, &task_id);
            event_append(
                &tdir,
                "question_answered",
                "human",
                &format!("answered {question_id}: {answer}"),
                Some(serde_json::json!({"question_id": question_id})),
            )
            .ok();
            let v = read_task_json(&tdir);
            let project_id = vs(&v, "project_id");
            let phase = vs(&v, "phase");
            if dev_core::task_service::phase_after_answer(remaining, &phase).is_some() {
                task_phase_set(&tdir, "planning", "dev", "blocking questions resolved").ok();
                if json_out {
                    json_ok(
                        &task_id,
                        &project_id,
                        "planning",
                        "answered; blocking questions resolved",
                    );
                } else {
                    println!("answered: {question_id} → task {task_id} → planning");
                }
            } else {
                if json_out {
                    json_ok(
                        &task_id,
                        &project_id,
                        &phase,
                        &format!("answered; {remaining} blocking remaining"),
                    );
                } else {
                    println!("answered: {question_id} ({remaining} blocking remaining)");
                }
            }
            return Ok(());
        }
    }
    if json_out {
        println!(r#"{{"ok":true,"question_id":"{question_id}","message":"answered"}}"#);
    } else {
        println!("answered: {question_id}");
    }
    Ok(())
}

// ── write-plan ────────────────────────────────────────────────────────────────

fn cmd_write_plan(
    task_id: Option<String>,
    file: Option<String>,
    json_out: bool,
) -> anyhow::Result<()> {
    let task_id = task_id.unwrap_or_default();
    if task_id.is_empty() {
        eprintln!("Usage: dev task write-plan <task-id> [--file path] [--json]");
        std::process::exit(1);
    }
    let tdir = find_task_dir(&task_id).unwrap_or_else(|| {
        eprintln!("not found: {task_id}");
        std::process::exit(1);
    });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let pdir = find_project_dir_for_task(&task_id).unwrap();

    let content = if let Some(f) = file {
        std::fs::read_to_string(&f).unwrap_or_else(|e| {
            eprintln!("error reading {f}: {e}");
            std::process::exit(1);
        })
    } else {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .unwrap_or_default();
        buf
    };

    plan_write(&tdir, &content).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });

    let remaining = blocking_questions_open(&pdir, &task_id);
    let new_phase = dev_core::task_service::phase_after_plan(remaining);
    task_phase_set(&tdir, new_phase, "agent", "plan written").unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });

    if json_out {
        json_ok(&task_id, &project_id, new_phase, "plan written");
    } else {
        println!("plan written → {new_phase}");
    }
    Ok(())
}

// ── approve ───────────────────────────────────────────────────────────────────

fn cmd_approve(task_id: Option<String>, json_out: bool) -> anyhow::Result<()> {
    let task_id = task_id.unwrap_or_default();
    if task_id.is_empty() {
        eprintln!("Usage: dev task approve <task-id>");
        std::process::exit(1);
    }
    let tdir = find_task_dir(&task_id).unwrap_or_else(|| {
        eprintln!("not found: {task_id}");
        std::process::exit(1);
    });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let pdir = find_project_dir_for_task(&task_id).unwrap();

    if !tdir.join("plan.md").exists() {
        if json_out {
            println!(
                r#"{{"ok":false,"error":"plan_missing","message":"plan.md does not exist","task_id":"{task_id}"}}"#
            );
        } else {
            eprintln!("error: plan.md does not exist");
        }
        std::process::exit(1);
    }
    let remaining = blocking_questions_open(&pdir, &task_id);
    if remaining > 0 {
        if json_out {
            println!(
                r#"{{"ok":false,"error":"blocking_questions_open","message":"cannot approve while {remaining} blocking questions are open","task_id":"{task_id}"}}"#
            );
        } else {
            eprintln!("error: {remaining} blocking question(s) open");
        }
        std::process::exit(1);
    }
    plan_approve(&tdir).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });
    task_phase_set(&tdir, "approved", "human", "plan approved").unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });

    if json_out {
        json_ok(&task_id, &project_id, "approved", "plan approved");
    } else {
        println!("approved: {task_id}");
    }
    Ok(())
}

// ── reject ────────────────────────────────────────────────────────────────────

fn cmd_reject(
    task_id: Option<String>,
    reason: Option<String>,
    json_out: bool,
) -> anyhow::Result<()> {
    let task_id = task_id.unwrap_or_default();
    if task_id.is_empty() {
        eprintln!("Usage: dev task reject <task-id> [--reason text]");
        std::process::exit(1);
    }
    let tdir = find_task_dir(&task_id).unwrap_or_else(|| {
        eprintln!("not found: {task_id}");
        std::process::exit(1);
    });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let msg = format!(
        "task rejected{}",
        reason
            .as_ref()
            .map(|r| format!(": {r}"))
            .unwrap_or_default()
    );
    event_append(&tdir, "task_rejected", "human", &msg, None).ok();
    task_phase_set(&tdir, "rejected", "human", &msg).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });

    if json_out {
        json_ok(&task_id, &project_id, "rejected", &msg);
    } else {
        println!("rejected: {task_id}");
    }
    Ok(())
}

// ── plan (planning agent) ─────────────────────────────────────────────────────

fn cmd_plan(
    task_id: Option<String>,
    tool: String,
    model: Option<String>,
    json_out: bool,
) -> anyhow::Result<()> {
    let task_id = task_id.unwrap_or_default();
    if task_id.is_empty() {
        eprintln!("Usage: dev task plan <task-id> [--tool t] [--model m]");
        std::process::exit(1);
    }
    let tdir = find_task_dir(&task_id).unwrap_or_else(|| {
        eprintln!("not found: {task_id}");
        std::process::exit(1);
    });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");

    let prompt = dev_core::task_service::plan_prompt(&task_id, &project_id);

    // Launch first; only transition the task if the agent actually started.
    let run_id = match spawn_dispatch(&project_id, &tool, None, model.as_deref(), &prompt) {
        Ok(id) => id,
        Err(e) => {
            if json_out {
                json_dispatch_err(&task_id, &e);
            } else {
                eprintln!("error: planning agent not dispatched: {e}");
            }
            std::process::exit(1);
        }
    };

    task_phase_set(&tdir, "planning", "dev", "planning agent dispatched").ok();
    if !run_id.is_empty() {
        task_set_link(&tdir, "run_id", serde_json::json!(run_id)).ok();
    }
    event_append(
        &tdir,
        "agent_dispatched",
        "dev",
        &format!("planning agent dispatched (tool={tool})"),
        Some(serde_json::json!({ "tool": tool, "run_id": run_id })),
    )
    .ok();

    if json_out {
        json_ok(
            &task_id,
            &project_id,
            "planning",
            "planning agent dispatched",
        );
    } else {
        println!("planning: {task_id} (project={project_id} tool={tool})");
    }
    Ok(())
}

// ── dispatch (implementation agent) ──────────────────────────────────────────

fn cmd_dispatch(
    task_id: Option<String>,
    tool: String,
    model: Option<String>,
    worktree: Option<String>,
    json_out: bool,
) -> anyhow::Result<()> {
    let task_id = task_id.unwrap_or_default();
    if task_id.is_empty() {
        eprintln!("Usage: dev task dispatch <task-id> [--tool t] [--worktree b]");
        std::process::exit(1);
    }
    let tdir = find_task_dir(&task_id).unwrap_or_else(|| {
        eprintln!("not found: {task_id}");
        std::process::exit(1);
    });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let phase = vs(&v, "phase");

    if phase != "approved" && phase != "needs_fix" {
        if json_out {
            println!(
                r#"{{"ok":false,"error":"task_not_approved","message":"phase is {phase} (need approved or needs_fix)","task_id":"{task_id}"}}"#
            );
        } else {
            eprintln!("error: phase is {phase} (need approved or needs_fix)");
        }
        std::process::exit(1);
    }
    if !tdir.join("approved-plan.md").exists() {
        if json_out {
            println!(
                r#"{{"ok":false,"error":"approved_plan_missing","message":"approved-plan.md not found","task_id":"{task_id}"}}"#
            );
        } else {
            eprintln!("error: approved-plan.md not found");
        }
        std::process::exit(1);
    }

    let wt_branch = worktree
        .unwrap_or_else(|| format!("task/{}", task_id.to_lowercase().replace(['_', ' '], "-")));

    let prompt = dev_core::task_service::dispatch_prompt(&task_id, &project_id);

    // Launch first; only mutate task state if the agent actually started.
    let run_id = match spawn_dispatch(
        &project_id,
        &tool,
        Some(&wt_branch),
        model.as_deref(),
        &prompt,
    ) {
        Ok(id) => id,
        Err(e) => {
            if json_out {
                json_dispatch_err(&task_id, &e);
            } else {
                eprintln!("error: implementation agent not dispatched: {e}");
            }
            std::process::exit(1);
        }
    };

    // Update task.json
    task_update_field(&tdir, "worktree_branch", serde_json::json!(wt_branch)).ok();
    task_update_field(&tdir, "assigned_tool", serde_json::json!(tool)).ok();
    if let Some(m) = &model {
        task_update_field(&tdir, "assigned_model", serde_json::json!(m)).ok();
    }
    if !run_id.is_empty() {
        task_set_link(&tdir, "run_id", serde_json::json!(run_id)).ok();
    }
    task_phase_set(&tdir, "implementing", "dev", "implementation started").ok();
    event_append(
        &tdir,
        "implementation_started",
        "dev",
        "implementation agent dispatched",
        Some(serde_json::json!({"tool": tool, "worktree": wt_branch, "run_id": run_id})),
    )
    .ok();

    if json_out {
        json_ok(
            &task_id,
            &project_id,
            "implementing",
            "implementation agent dispatched",
        );
    } else {
        println!("dispatched: {task_id} → {project_id} ({tool}, worktree: {wt_branch})");
    }
    Ok(())
}

// ── attach ────────────────────────────────────────────────────────────────────

fn cmd_attach(task_id: Option<String>) -> anyhow::Result<()> {
    let task_id = task_id.unwrap_or_default();
    if task_id.is_empty() {
        eprintln!("Usage: dev task attach <task-id>");
        std::process::exit(1);
    }
    let tdir = find_task_dir(&task_id).unwrap_or_else(|| {
        eprintln!("not found: {task_id}");
        std::process::exit(1);
    });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let _ = std::process::Command::new("dev")
        .args(["agent", "attach", &project_id])
        .status();
    Ok(())
}

// ── logs ──────────────────────────────────────────────────────────────────────

fn cmd_logs(task_id: Option<String>, follow: bool, json_out: bool) -> anyhow::Result<()> {
    let task_id = task_id.unwrap_or_default();
    if task_id.is_empty() {
        eprintln!("Usage: dev task logs <task-id> [-f]");
        std::process::exit(1);
    }
    let tdir = find_task_dir(&task_id).unwrap_or_else(|| {
        eprintln!("not found: {task_id}");
        std::process::exit(1);
    });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let mut cmd = std::process::Command::new("dev");
    cmd.args(["agent", "logs", &project_id]);
    if follow {
        cmd.arg("-f");
    }
    if json_out {
        cmd.arg("--json");
    }
    let _ = cmd.status();
    Ok(())
}

// ── kill ──────────────────────────────────────────────────────────────────────

fn cmd_kill(task_id: Option<String>, json_out: bool) -> anyhow::Result<()> {
    let task_id = task_id.unwrap_or_default();
    if task_id.is_empty() {
        eprintln!("Usage: dev task kill <task-id>");
        std::process::exit(1);
    }
    let tdir = find_task_dir(&task_id).unwrap_or_else(|| {
        eprintln!("not found: {task_id}");
        std::process::exit(1);
    });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let cfg = dev_core::config::Config::load_or_default();
    let _ = dev_core::agent::kill(&cfg, &project_id);
    event_append(&tdir, "task_killed", "human", "agent killed", None).ok();
    task_phase_set(&tdir, "killed", "human", "agent killed").ok();
    if json_out {
        json_ok(&task_id, &project_id, "killed", "agent killed");
    } else {
        println!("killed: {task_id}");
    }
    Ok(())
}

// ── write-handoff ─────────────────────────────────────────────────────────────

fn cmd_write_handoff(
    task_id: Option<String>,
    file: Option<String>,
    json_out: bool,
) -> anyhow::Result<()> {
    let task_id = task_id.unwrap_or_default();
    if task_id.is_empty() {
        eprintln!("Usage: dev task write-handoff <task-id> [--file path]");
        std::process::exit(1);
    }
    let tdir = find_task_dir(&task_id).unwrap_or_else(|| {
        eprintln!("not found: {task_id}");
        std::process::exit(1);
    });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let pdir = find_project_dir_for_task(&task_id).unwrap();

    let content = if let Some(f) = file {
        std::fs::read_to_string(&f).unwrap_or_else(|e| {
            eprintln!("error: {e}");
            std::process::exit(1);
        })
    } else {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .unwrap_or_default();
        buf
    };
    handoff_write(&tdir, &content).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });

    let remaining = blocking_questions_open(&pdir, &task_id);
    let new_phase = dev_core::task_service::phase_after_handoff(remaining);
    task_phase_set(&tdir, new_phase, "agent", "handoff written").ok();

    if json_out {
        json_ok(&task_id, &project_id, new_phase, "handoff written");
    } else {
        println!("handoff written → {new_phase}");
    }
    Ok(())
}

// ── handoff ───────────────────────────────────────────────────────────────────

fn cmd_handoff(task_id: Option<String>, json_out: bool) -> anyhow::Result<()> {
    let task_id = task_id.unwrap_or_default();
    if task_id.is_empty() {
        eprintln!("Usage: dev task handoff <task-id>");
        std::process::exit(1);
    }
    let tdir = find_task_dir(&task_id).unwrap_or_else(|| {
        eprintln!("not found: {task_id}");
        std::process::exit(1);
    });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let handoff_path = tdir.join("handoff.md");
    let exists = handoff_path.exists();
    let content = if exists {
        std::fs::read_to_string(&handoff_path).unwrap_or_default()
    } else {
        String::new()
    };
    if json_out {
        println!(
            "{}",
            serde_json::json!({"task_id": task_id, "project_id": project_id, "handoff": content, "exists": exists})
        );
    } else if exists {
        println!("{content}");
    } else {
        println!("(no handoff yet)");
    }
    Ok(())
}

// ── harvest ───────────────────────────────────────────────────────────────────

fn cmd_harvest(task_id: Option<String>, json_out: bool) -> anyhow::Result<()> {
    let task_id = task_id.unwrap_or_default();
    if task_id.is_empty() {
        eprintln!("Usage: dev task harvest <task-id>");
        std::process::exit(1);
    }
    let tdir = find_task_dir(&task_id).unwrap_or_else(|| {
        eprintln!("not found: {task_id}");
        std::process::exit(1);
    });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");

    // Diff HEAD..workdir with git2 — prefer the recorded worktree_path, else the
    // project's local checkout resolved via config. (Replaced a `dev git diff
    // <project> --json` self-subprocess; a remote project, which git2 cannot read
    // from the control machine, now yields an empty file list.)
    let files = {
        let wt_path = vs(&v, "worktree_path");
        let repo_path = if !wt_path.is_empty() {
            Some(std::path::PathBuf::from(wt_path))
        } else {
            match dev_core::config::Config::load_or_default().resolve(&project_id) {
                Some(dev_core::config::Target::Local { path, .. }) => {
                    Some(std::path::PathBuf::from(path))
                }
                _ => None,
            }
        };
        repo_path
            .and_then(|p| dev_core::git::diff_head_to_workdir(&p).ok())
            .map(|r| r.files)
            .unwrap_or_default()
    };

    // Update task.json summary
    let json_path = tdir.join("task.json");
    if let Ok(content) = std::fs::read_to_string(&json_path) {
        if let Ok(mut tv) = serde_json::from_str::<Value>(&content) {
            tv["summary"]["diff_files"] = serde_json::json!(files);
            tv["updated_at"] = serde_json::json!(now_iso());
            if tdir.join("handoff.md").exists() {
                let h: String = std::fs::read_to_string(tdir.join("handoff.md"))
                    .unwrap_or_default()
                    .chars()
                    .take(150)
                    .collect();
                tv["summary"]["latest_handoff"] = serde_json::json!(h);
            }
            let _ = std::fs::write(&json_path, serde_json::to_string_pretty(&tv)?);
        }
    }
    let n = files.len();
    event_append(
        &tdir,
        "diff_harvested",
        "dev",
        &format!("harvested {n} files"),
        Some(serde_json::json!({"file_count": n})),
    )
    .ok();

    if json_out {
        json_ok(
            &task_id,
            &project_id,
            &vs(&v, "phase"),
            &format!("harvested {n} files"),
        );
    } else {
        println!("harvested: {n} files ({task_id})");
    }
    Ok(())
}

// ── diff ──────────────────────────────────────────────────────────────────────

fn cmd_diff(task_id: Option<String>, stat: bool, json_out: bool) -> anyhow::Result<()> {
    let task_id = task_id.unwrap_or_default();
    if task_id.is_empty() {
        eprintln!("Usage: dev task diff <task-id> [--stat] [--json]");
        std::process::exit(1);
    }
    let tdir = find_task_dir(&task_id).unwrap_or_else(|| {
        eprintln!("not found: {task_id}");
        std::process::exit(1);
    });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    // NOTE: kept as a subprocess — this is a terminal/pager passthrough. `.status()`
    // inherits stdio so git can page the diff for the user; there is no in-process
    // git-diff *printer* in dev-core (only file-list helpers), so this is a display
    // handoff, not a data call.
    let mut cmd = std::process::Command::new("dev");
    cmd.args(["git", "diff", &project_id]);
    if stat {
        cmd.arg("--stat");
    }
    if json_out {
        cmd.arg("--json");
    }
    let _ = cmd.status();
    Ok(())
}

// ── test ──────────────────────────────────────────────────────────────────────

fn cmd_test(
    task_id: Option<String>,
    extra_cmd: Option<String>,
    json_out: bool,
) -> anyhow::Result<()> {
    let task_id = task_id.unwrap_or_default();
    if task_id.is_empty() {
        eprintln!("Usage: dev task test <task-id> [--cmd cmd]");
        std::process::exit(1);
    }
    let tdir = find_task_dir(&task_id).unwrap_or_else(|| {
        eprintln!("not found: {task_id}");
        std::process::exit(1);
    });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");

    let cmds: Vec<String> = if let Some(c) = extra_cmd {
        vec![c]
    } else {
        v.pointer("/validation/commands")
            .and_then(|x| x.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| c.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    };

    if cmds.is_empty() {
        if json_out {
            println!(
                r#"{{"ok":false,"error":"no_commands","message":"no validation commands defined","task_id":"{task_id}"}}"#
            );
        } else {
            eprintln!("no validation commands defined");
        }
        std::process::exit(1);
    }

    let results_dir = tdir.join("test-results");
    let vid = next_test_run_id_in(&results_dir).unwrap_or_else(|_| "V-unknown".to_string());
    let mut results = Vec::new();
    let mut passed = 0usize;
    let mut failed = 0usize;
    for c in &cmds {
        let (ok, output) = match super::run::run_one(&project_id, c, 120) {
            Ok(v) => (
                v.get("ok").and_then(|x| x.as_bool()).unwrap_or(false),
                v.get("stdout").and_then(|x| x.as_str()).unwrap_or("").to_string()
                    + v.get("stderr").and_then(|x| x.as_str()).unwrap_or(""),
            ),
            Err(e) => (false, e.to_string()),
        };
        if ok {
            passed += 1;
        } else {
            failed += 1;
        }
        results
            .push(serde_json::json!({"cmd": c, "ok": ok, "output": truncate_chars(&output, 500)}));
    }

    let test_status = dev_core::task_service::test_status(passed, failed);
    let ts = now_iso();
    let result_json = serde_json::json!({
        "id": vid, "task_id": task_id, "commands": cmds,
        "results": results, "passed": passed, "failed": failed, "ts": ts
    });
    let _ = std::fs::write(
        results_dir.join(format!("{vid}.json")),
        serde_json::to_string_pretty(&result_json)?,
    );

    // Update task.json
    let json_path = tdir.join("task.json");
    if let Ok(content) = std::fs::read_to_string(&json_path) {
        if let Ok(mut tv) = serde_json::from_str::<Value>(&content) {
            dev_core::task_service::set_summary_field(
                &mut tv,
                "test_status",
                serde_json::json!(test_status),
            );
            tv["updated_at"] = serde_json::json!(now_iso());
            let _ = std::fs::write(&json_path, serde_json::to_string_pretty(&tv)?);
        }
    }
    event_append(
        &tdir,
        "test_completed",
        "dev",
        &format!("{passed}/{} passed", passed + failed),
        Some(serde_json::json!({"passed": passed, "failed": failed, "vid": vid})),
    )
    .ok();

    if json_out {
        println!("{}", serde_json::to_string(&result_json)?);
    } else {
        println!("test: {passed} passed, {failed} failed → {test_status}");
    }
    if failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

// ── review ────────────────────────────────────────────────────────────────────

fn cmd_review(task_id: Option<String>, tool: Option<String>, json_out: bool) -> anyhow::Result<()> {
    let task_id = task_id.unwrap_or_default();
    if task_id.is_empty() {
        eprintln!("Usage: dev task review <task-id> [--tool t]");
        std::process::exit(1);
    }
    let tdir = find_task_dir(&task_id).unwrap_or_else(|| {
        eprintln!("not found: {task_id}");
        std::process::exit(1);
    });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");

    // Dispatch a review agent in-process (was a `dev agent review` self-subprocess,
    // itself a background dispatch). Reconstruct the same receipt text the
    // subprocess printed and this command captured, so the recorded review
    // artifact and JSON stay shaped as before.
    let cfg = dev_core::config::Config::load_or_default();
    let backend = tool
        .clone()
        .unwrap_or_else(|| dev_core::agent::default_backend(&cfg, &project_id, None));
    let prompt = dev_core::agent::review_prompt(None);
    let opts = dev_core::agent::DispatchOpts {
        backend: &backend,
        task: &prompt,
        model: None,
        effort: None,
        sandbox: None,
        worktree: None,
        supervise: false,
    };
    let (review_ok, review_exit, output) = match dev_core::agent::dispatch(&cfg, &project_id, &opts) {
        Ok(v) => {
            let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("?");
            let pid_note = v
                .get("pid")
                .and_then(|x| x.as_str())
                .filter(|p| !p.is_empty())
                .map(|p| format!(" pid={p}"))
                .unwrap_or_default();
            (
                true,
                0,
                format!("dispatched {id} on {project_id} (backend={backend}{pid_note})\n"),
            )
        }
        Err(e) => (false, 1, format!("dev agent dispatch: {e:#}\n")),
    };

    let recommendation = dev_core::task_service::review_recommendation(&output, review_ok);

    let reviews_dir = tdir.join("reviews");
    let rid = next_review_id_in(&reviews_dir).unwrap_or_else(|_| "R-unknown".to_string());
    let ts = now_iso();
    let _ = std::fs::write(reviews_dir.join(format!("{rid}.md")), &output);
    let review_json = serde_json::json!({
        "id": rid, "task_id": task_id, "tool": tool.as_deref().unwrap_or("auto"),
        "ok": review_ok, "exit": review_exit,
        "output": truncate_chars(&output, 2000),
        "recommendation": recommendation, "ts": ts
    });
    let _ = std::fs::write(
        reviews_dir.join(format!("{rid}.json")),
        serde_json::to_string_pretty(&review_json)?,
    );

    if !review_ok {
        event_append(
            &tdir,
            "review_failed",
            "dev",
            &format!("review failed: exit {review_exit}"),
            Some(serde_json::json!({"rid": rid, "exit": review_exit})),
        )
        .ok();
        if json_out {
            println!("{}", serde_json::to_string(&review_json)?);
        } else {
            print!("{output}");
            eprintln!("\n→ review failed (exit {review_exit})");
        }
        std::process::exit(if review_exit == 0 { 1 } else { review_exit });
    }

    // Update review_status and phase
    let json_path = tdir.join("task.json");
    if let Ok(content) = std::fs::read_to_string(&json_path) {
        if let Ok(mut tv) = serde_json::from_str::<Value>(&content) {
            dev_core::task_service::set_summary_field(
                &mut tv,
                "review_status",
                serde_json::json!(recommendation),
            );
            tv["updated_at"] = serde_json::json!(now_iso());
            let _ = std::fs::write(&json_path, serde_json::to_string_pretty(&tv)?);
        }
    }
    let old_phase = vs(&v, "phase");
    let new_phase = dev_core::task_service::phase_after_review(recommendation, &old_phase);
    if new_phase != old_phase {
        task_phase_set(
            &tdir,
            new_phase,
            "dev",
            &format!("review: {recommendation}"),
        )
        .ok();
    }
    event_append(
        &tdir,
        "review_completed",
        "dev",
        &format!("review: {recommendation}"),
        Some(serde_json::json!({"rid": rid, "recommendation": recommendation})),
    )
    .ok();

    if json_out {
        println!("{}", serde_json::to_string(&review_json)?);
    } else {
        print!("{output}");
        println!("\n→ {recommendation}");
    }
    Ok(())
}

// ── fix ───────────────────────────────────────────────────────────────────────

fn cmd_fix(
    task_id: Option<String>,
    tool: String,
    model: Option<String>,
    json_out: bool,
) -> anyhow::Result<()> {
    let task_id = task_id.unwrap_or_default();
    if task_id.is_empty() {
        eprintln!("Usage: dev task fix <task-id> [--tool t]");
        std::process::exit(1);
    }
    let tdir = find_task_dir(&task_id).unwrap_or_else(|| {
        eprintln!("not found: {task_id}");
        std::process::exit(1);
    });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let phase = vs(&v, "phase");

    if phase != "needs_fix" {
        if json_out {
            println!(
                r#"{{"ok":false,"error":"task_not_needs_fix","message":"phase is {phase}","task_id":"{task_id}"}}"#
            );
        } else {
            eprintln!("error: phase is {phase} (need needs_fix)");
        }
        std::process::exit(1);
    }
    let wt_branch = vs(&v, "worktree_branch");

    let prompt = dev_core::task_service::fix_prompt(&task_id, &project_id);

    let wt = (!wt_branch.is_empty()).then_some(wt_branch.as_str());
    let run_id = match spawn_dispatch(&project_id, &tool, wt, model.as_deref(), &prompt) {
        Ok(id) => id,
        Err(e) => {
            if json_out {
                json_dispatch_err(&task_id, &e);
            } else {
                eprintln!("error: fix agent not dispatched: {e}");
            }
            std::process::exit(1);
        }
    };

    if !run_id.is_empty() {
        task_set_link(&tdir, "run_id", serde_json::json!(run_id)).ok();
    }
    task_phase_set(&tdir, "implementing", "dev", "fix agent dispatched").ok();
    event_append(
        &tdir,
        "implementation_started",
        "dev",
        "fix agent dispatched",
        Some(serde_json::json!({"tool": tool, "worktree": wt_branch, "run_id": run_id})),
    )
    .ok();

    if json_out {
        json_ok(
            &task_id,
            &project_id,
            "implementing",
            "fix agent dispatched",
        );
    } else {
        println!("fix dispatched: {task_id} ({tool})");
    }
    Ok(())
}

// ── pr ────────────────────────────────────────────────────────────────────────

fn cmd_pr(
    task_id: Option<String>,
    title: Option<String>,
    base: String,
    draft: bool,
    json_out: bool,
) -> anyhow::Result<()> {
    let task_id = task_id.unwrap_or_default();
    if task_id.is_empty() {
        eprintln!("Usage: dev task pr <task-id> [--title t] [--base b]");
        std::process::exit(1);
    }
    let tdir = find_task_dir(&task_id).unwrap_or_else(|| {
        eprintln!("not found: {task_id}");
        std::process::exit(1);
    });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let phase = vs(&v, "phase");

    if phase != "mergeable" {
        if json_out {
            println!(
                r#"{{"ok":false,"error":"task_not_mergeable","message":"phase is {phase}","task_id":"{task_id}"}}"#
            );
        } else {
            eprintln!("error: phase is {phase} (need mergeable)");
        }
        std::process::exit(1);
    }

    // Create the PR in-process (was a `dev git pr` self-subprocess). Rebuild the
    // exact stdout the subprocess produced — the `--json` receipt or the raw
    // git/gh output — so URL extraction and the echoed output are unchanged.
    let cfg = dev_core::config::Config::load_or_default();
    let (pr_ok, pr_url, raw_out) =
        super::git::pr_capture(&cfg, &project_id, title.as_deref(), Some(base.as_str()), draft);
    let output = if json_out {
        format!(
            "{}\n",
            serde_json::to_string(
                &serde_json::json!({ "target": project_id, "ok": pr_ok, "url": pr_url })
            )
            .unwrap_or_default()
        )
    } else {
        raw_out
    };

    // Extract PR URL
    let url: String = output
        .lines()
        .find(|l| l.contains("https://") && l.contains("/pull/"))
        .and_then(|l| l.split_whitespace().find(|w| w.starts_with("https://")))
        .unwrap_or("")
        .to_string();

    // Update task.json (links.pr_url + updated_at, atomically via the store).
    task_set_link(&tdir, "pr_url", serde_json::json!(url)).ok();
    event_append(
        &tdir,
        "pr_created",
        "human",
        &format!("PR: {url}"),
        Some(serde_json::json!({"url": url})),
    )
    .ok();
    task_phase_set(&tdir, "merged", "human", "PR created").ok();

    if json_out {
        print!("{output}");
    } else {
        println!("PR: {}", if url.is_empty() { output.trim() } else { &url });
    }
    Ok(())
}
