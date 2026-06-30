//! dev task subcommands — task store CRUD, questions, plan.

use dev_core::*;
use serde_json::Value;
use std::path::Path;

// ── JSON output helpers ───────────────────────────────────────────────────────

fn json_ok(task_id: &str, project_id: &str, phase: &str, message: &str) {
    println!(r#"{{"ok":true,"task_id":"{task_id}","project_id":"{project_id}","phase":"{phase}","message":"{message}"}}"#);
}

fn json_err(error: &str, message: &str, task_id: &str) -> ! {
    eprintln!(r#"{{"ok":false,"error":"{error}","message":"{message}","task_id":"{task_id}"}}"#);
    std::process::exit(1);
}

fn read_task_json(task_dir: &Path) -> Value {
    let content = std::fs::read_to_string(task_dir.join("task.json"))
        .unwrap_or_else(|_| "{}".to_string());
    serde_json::from_str(&content).unwrap_or_default()
}

fn vs(v: &Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

// ── dispatch ──────────────────────────────────────────────────────────────────

pub fn run(args: &[String]) {
    let sub = match args.first() {
        Some(s) => s.as_str(),
        None => { usage(); std::process::exit(1); }
    };
    let rest = &args[1..];
    match sub {
        "new"           => cmd_new(rest),
        "list"          => cmd_list(rest),
        "show"          => cmd_show(rest),
        "context"       => cmd_context(rest),
        "events"        => cmd_events(rest),
        "ask"           => cmd_ask(rest),
        "answer"        => cmd_answer(rest),
        "write-plan"    => cmd_write_plan(rest),
        "approve"       => cmd_approve(rest),
        "reject"        => cmd_reject(rest),
        "plan"          => cmd_plan(rest),
        "dispatch"      => cmd_dispatch(rest),
        "attach"        => cmd_attach(rest),
        "logs"          => cmd_logs(rest),
        "kill"          => cmd_kill(rest),
        "write-handoff" => cmd_write_handoff(rest),
        "handoff"       => cmd_handoff(rest),
        "harvest"       => cmd_harvest(rest),
        "diff"          => cmd_diff(rest),
        "test"          => cmd_test(rest),
        "review"        => cmd_review(rest),
        "fix"           => cmd_fix(rest),
        "pr"            => cmd_pr(rest),
        _ => { eprintln!("dev task: unknown command '{sub}'"); usage(); std::process::exit(1); }
    }
}

fn usage() {
    eprintln!("Usage: dev task <command> [args...]");
    eprintln!();
    eprintln!("  new <project> --title <t> [--brief <b>]    Create task");
    eprintln!("  list [project] [--phase <p>] [--json]      List tasks");
    eprintln!("  show <task-id> [--json]                    Show task");
    eprintln!("  context <task-id> [--markdown|--json]      Agent context");
    eprintln!("  events <task-id> [--json]                  Task events");
    eprintln!("  ask <task-id> <question> [--severity s]    Open question");
    eprintln!("  answer <q-id> <answer> [--json]            Answer question");
    eprintln!("  write-plan <task-id> [--file f] [--json]   Save plan");
    eprintln!("  approve <task-id> [--json]                 Approve plan");
    eprintln!("  reject <task-id> [--reason r] [--json]     Reject task");
    eprintln!("  plan <task-id> [--tool t] [--model m]      Start planning agent");
    eprintln!("  dispatch <task-id> [--tool t] [--worktree b] [--model m]  Impl agent");
    eprintln!("  attach <task-id>                           Attach to agent");
    eprintln!("  logs <task-id> [-f] [--json]               Tail logs");
    eprintln!("  kill <task-id> [--json]                    Kill agent");
    eprintln!("  write-handoff <task-id> [--file f] [--json]  Write handoff");
    eprintln!("  handoff <task-id> [--markdown|--json]      Show handoff");
    eprintln!("  harvest <task-id> [--json]                 Update from worktree");
    eprintln!("  diff <task-id> [--stat] [--json]           Show diff");
    eprintln!("  test <task-id> [--cmd c] [--json]          Run validation");
    eprintln!("  review <task-id> [--tool t] [--json]       Review diff");
    eprintln!("  fix <task-id> [--tool t] [--json]          Dispatch fix agent");
    eprintln!("  pr <task-id> [--title t] [--base b] [--json]  Create PR");
}

// ── new ───────────────────────────────────────────────────────────────────────

fn cmd_new(args: &[String]) {
    let mut project_id = String::new();
    let mut title = String::new();
    let mut brief: Option<String> = None;
    let mut priority: Option<String> = None;
    let mut json_out = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--title"    => { i += 1; title = args.get(i).cloned().unwrap_or_default(); }
            "--brief"    => { i += 1; brief = Some(args.get(i).cloned().unwrap_or_default()); }
            "--priority" => { i += 1; priority = Some(args.get(i).cloned().unwrap_or_default()); }
            "--json"     => { json_out = true; }
            a if project_id.is_empty() => { project_id = a.to_string(); }
            _ => {}
        }
        i += 1;
    }
    if project_id.is_empty() || title.is_empty() {
        eprintln!("Usage: dev task new <project> --title <title> [--brief <text>]");
        std::process::exit(1);
    }
    let task = task_new(&project_id, &title, brief.as_deref(), priority.as_deref())
        .unwrap_or_else(|e| { eprintln!("error: {e}"); std::process::exit(1); });
    if json_out {
        json_ok(&task.id, &task.project_id, "draft", "task created");
    } else {
        println!("created: {} ({})", task.id, task.project_id);
    }
}

// ── list ──────────────────────────────────────────────────────────────────────

fn cmd_list(args: &[String]) {
    let mut project_filter: Option<String> = None;
    let mut phase_filter: Option<String> = None;
    let mut json_out = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--phase" => { i += 1; phase_filter = Some(args.get(i).cloned().unwrap_or_default()); }
            "--json"  => { json_out = true; }
            a => { project_filter = Some(a.to_string()); }
        }
        i += 1;
    }
    let (tasks, _) = load_dev_tasks();
    let tasks: Vec<_> = tasks.iter()
        .filter(|t| project_filter.as_ref().map_or(true, |p| &t.project_id == p))
        .filter(|t| phase_filter.as_ref().map_or(true, |p| &t.phase == p))
        .collect();
    if json_out {
        let arr: Vec<Value> = tasks.iter().map(|t| serde_json::json!({
            "id": t.id, "project_id": t.project_id, "title": t.title,
            "phase": t.phase, "priority": t.priority
        })).collect();
        println!("{}", serde_json::to_string(&arr).unwrap());
    } else {
        if tasks.is_empty() { println!("(no tasks)"); return; }
        for t in &tasks {
            println!("{:<20}  {:<14}  {:<12}  {}", t.id, t.project_id, t.phase, t.title);
        }
    }
}

// ── show ──────────────────────────────────────────────────────────────────────

fn cmd_show(args: &[String]) {
    let mut task_id = String::new();
    let mut json_out = false;
    for a in args {
        match a.as_str() {
            "--json" => json_out = true,
            a => if task_id.is_empty() { task_id = a.to_string(); }
        }
    }
    if task_id.is_empty() { eprintln!("Usage: dev task show <task-id>"); std::process::exit(1); }
    let tdir = find_task_dir(&task_id)
        .unwrap_or_else(|| { eprintln!("not found: {task_id}"); std::process::exit(1); });
    let v = read_task_json(&tdir);
    if json_out {
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
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
}

// ── context ───────────────────────────────────────────────────────────────────

fn cmd_context(args: &[String]) {
    let mut task_id = String::new();
    let mut markdown = false;
    let mut json_out = false;
    for a in args {
        match a.as_str() {
            "--markdown" => markdown = true,
            "--json"     => json_out = true,
            a => if task_id.is_empty() { task_id = a.to_string(); }
        }
    }
    if task_id.is_empty() { eprintln!("Usage: dev task context <task-id> [--markdown|--json]"); std::process::exit(1); }
    let tdir = find_task_dir(&task_id)
        .unwrap_or_else(|| { eprintln!("not found: {task_id}"); std::process::exit(1); });
    let v = read_task_json(&tdir);
    let pdir = find_project_dir_for_task(&task_id).unwrap();

    let read_md = |name: &str| std::fs::read_to_string(tdir.join(name)).unwrap_or_default();
    let read_pmd = |name: &str| std::fs::read_to_string(pdir.join(name)).unwrap_or_default();

    let brief = read_md("brief.md");
    let plan = if tdir.join("approved-plan.md").exists() { read_md("approved-plan.md") } else { read_md("plan.md") };
    let project_md = read_pmd("project.md");

    // Open questions
    let qfile = pdir.join("questions.jsonl");
    let open_questions: Vec<Value> = std::fs::read_to_string(&qfile).unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|q| q.get("task_id").and_then(|x| x.as_str()) == Some(&task_id)
                  && q.get("status").and_then(|x| x.as_str()) == Some("open"))
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
        println!("{}", serde_json::to_string_pretty(&ctx).unwrap());
    } else if markdown {
        println!("# Task Context: {task_id}");
        println!();
        if !project_md.is_empty() { println!("## Project\n\n{project_md}\n"); }
        if !brief.is_empty() { println!("## Brief\n\n{brief}\n"); }
        if !plan.is_empty() { println!("## Plan\n\n{plan}\n"); }
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
        println!("## Task JSON\n\n```json\n{}\n```", serde_json::to_string_pretty(&v).unwrap());
    } else {
        // Human readable
        println!("task: {} ({})", vs(&v, "id"), vs(&v, "phase"));
        println!("title: {}", vs(&v, "title"));
        if !brief.is_empty() { println!("\n--- brief ---\n{brief}"); }
        if !plan.is_empty()  { println!("\n--- plan ---\n{}", &plan[..plan.len().min(500)]); }
    }
}

// ── events ────────────────────────────────────────────────────────────────────

fn cmd_events(args: &[String]) {
    let mut task_id = String::new();
    let mut json_out = false;
    for a in args {
        match a.as_str() {
            "--json" => json_out = true,
            a => if task_id.is_empty() { task_id = a.to_string(); }
        }
    }
    if task_id.is_empty() { eprintln!("Usage: dev task events <task-id>"); std::process::exit(1); }
    let tdir = find_task_dir(&task_id)
        .unwrap_or_else(|| { eprintln!("not found: {task_id}"); std::process::exit(1); });
    let content = std::fs::read_to_string(tdir.join("events.jsonl")).unwrap_or_default();
    let events: Vec<Value> = content.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect();
    if json_out {
        println!("{}", serde_json::to_string(&events).unwrap());
    } else {
        for ev in &events {
            let ts = ev.get("ts").and_then(|x| x.as_str()).unwrap_or("?");
            let t = ev.get("type").and_then(|x| x.as_str()).unwrap_or("?");
            let msg = ev.get("message").and_then(|x| x.as_str()).unwrap_or("");
            println!("{ts}  {t:<25}  {msg}");
        }
    }
}

// ── ask ───────────────────────────────────────────────────────────────────────

fn cmd_ask(args: &[String]) {
    let mut task_id = String::new();
    let mut question = String::new();
    let mut severity = "blocking".to_string();
    let mut category = "behavior".to_string();
    let mut context: Option<String> = None;
    let mut recommendation: Option<String> = None;
    let mut json_out = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--severity"       => { i += 1; severity = args.get(i).cloned().unwrap_or_default(); }
            "--category"       => { i += 1; category = args.get(i).cloned().unwrap_or_default(); }
            "--context"        => { i += 1; context = Some(args.get(i).cloned().unwrap_or_default()); }
            "--recommendation" => { i += 1; recommendation = Some(args.get(i).cloned().unwrap_or_default()); }
            "--json" => json_out = true,
            a => {
                if task_id.is_empty() { task_id = a.to_string(); }
                else if question.is_empty() { question = a.to_string(); }
            }
        }
        i += 1;
    }
    if task_id.is_empty() || question.is_empty() {
        eprintln!("Usage: dev task ask <task-id> <question> [--severity blocking] [--category behavior]");
        std::process::exit(1);
    }
    let tdir = find_task_dir(&task_id)
        .unwrap_or_else(|| { eprintln!("not found: {task_id}"); std::process::exit(1); });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let pdir = find_project_dir_for_task(&task_id).unwrap();

    let q = question_new(
        &pdir, &task_id, &project_id,
        &question, &severity, &category,
        vec![], recommendation.as_deref(), context.as_deref(),
    ).unwrap_or_else(|e| { eprintln!("error: {e}"); std::process::exit(1); });

    event_append(&tdir, "question_opened", "agent",
        &format!("question opened: {}", q.id),
        Some(serde_json::json!({"question_id": q.id, "severity": severity}))).ok();

    // If blocking, set phase to needs_spec
    if severity == "blocking" {
        task_phase_set(&tdir, "needs_spec", "dev", "blocking question opened").ok();
        if json_out {
            json_ok(&task_id, &project_id, "needs_spec", &format!("question opened: {}", q.id));
        } else {
            println!("question: {} (needs_spec)", q.id);
        }
    } else {
        let phase = vs(&v, "phase");
        if json_out {
            json_ok(&task_id, &project_id, &phase, &format!("question opened: {}", q.id));
        } else {
            println!("question: {}", q.id);
        }
    }
}

// ── answer ────────────────────────────────────────────────────────────────────

fn cmd_answer(args: &[String]) {
    let mut question_id = String::new();
    let mut answer = String::new();
    let mut json_out = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => json_out = true,
            a => {
                if question_id.is_empty() { question_id = a.to_string(); }
                else if answer.is_empty() { answer = a.to_string(); }
            }
        }
        i += 1;
    }
    if question_id.is_empty() || answer.is_empty() {
        eprintln!("Usage: dev task answer <question-id> <answer>");
        std::process::exit(1);
    }
    let pdir = find_project_dir_for_question(&question_id)
        .unwrap_or_else(|| { eprintln!("question not found: {question_id}"); std::process::exit(1); });
    question_answer(&pdir, &question_id, &answer)
        .unwrap_or_else(|e| { eprintln!("error: {e}"); std::process::exit(1); });

    // Find task_id from the question
    let qfile = pdir.join("questions.jsonl");
    let content = std::fs::read_to_string(&qfile).unwrap_or_default();
    let task_id: String = content.lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|v| v.get("id").and_then(|x| x.as_str()) == Some(&question_id))
        .map(|v| v.get("task_id").and_then(|x| x.as_str()).unwrap_or("").to_string())
        .next()
        .unwrap_or_default();

    if !task_id.is_empty() {
        if let Some(tdir) = find_task_dir(&task_id) {
            let remaining = blocking_questions_open(&pdir, &task_id);
            event_append(&tdir, "question_answered", "human",
                &format!("answered {question_id}: {answer}"),
                Some(serde_json::json!({"question_id": question_id}))).ok();
            let v = read_task_json(&tdir);
            let project_id = vs(&v, "project_id");
            let phase = vs(&v, "phase");
            if remaining == 0 && phase == "needs_spec" {
                task_phase_set(&tdir, "planning", "dev", "blocking questions resolved").ok();
                if json_out {
                    json_ok(&task_id, &project_id, "planning", "answered; blocking questions resolved");
                } else {
                    println!("answered: {question_id} → task {task_id} → planning");
                }
            } else {
                if json_out {
                    json_ok(&task_id, &project_id, &phase, &format!("answered; {remaining} blocking remaining"));
                } else {
                    println!("answered: {question_id} ({remaining} blocking remaining)");
                }
            }
            return;
        }
    }
    if json_out {
        println!(r#"{{"ok":true,"question_id":"{question_id}","message":"answered"}}"#);
    } else {
        println!("answered: {question_id}");
    }
}

// ── write-plan ────────────────────────────────────────────────────────────────

fn cmd_write_plan(args: &[String]) {
    let mut task_id = String::new();
    let mut file: Option<String> = None;
    let mut json_out = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--file" => { i += 1; file = Some(args.get(i).cloned().unwrap_or_default()); }
            "--json" => json_out = true,
            a => if task_id.is_empty() { task_id = a.to_string(); }
        }
        i += 1;
    }
    if task_id.is_empty() { eprintln!("Usage: dev task write-plan <task-id> [--file path] [--json]"); std::process::exit(1); }
    let tdir = find_task_dir(&task_id)
        .unwrap_or_else(|| { eprintln!("not found: {task_id}"); std::process::exit(1); });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let pdir = find_project_dir_for_task(&task_id).unwrap();

    let content = if let Some(f) = file {
        std::fs::read_to_string(&f).unwrap_or_else(|e| { eprintln!("error reading {f}: {e}"); std::process::exit(1); })
    } else {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf).unwrap_or_default();
        buf
    };

    plan_write(&tdir, &content).unwrap_or_else(|e| { eprintln!("error: {e}"); std::process::exit(1); });

    let remaining = blocking_questions_open(&pdir, &task_id);
    let new_phase = if remaining > 0 { "needs_spec" } else { "planned" };
    task_phase_set(&tdir, new_phase, "agent", "plan written")
        .unwrap_or_else(|e| { eprintln!("error: {e}"); std::process::exit(1); });

    if json_out { json_ok(&task_id, &project_id, new_phase, "plan written"); }
    else { println!("plan written → {new_phase}"); }
}

// ── approve ───────────────────────────────────────────────────────────────────

fn cmd_approve(args: &[String]) {
    let mut task_id = String::new();
    let mut json_out = false;
    for a in args {
        match a.as_str() {
            "--json" => json_out = true,
            a => if task_id.is_empty() { task_id = a.to_string(); }
        }
    }
    if task_id.is_empty() { eprintln!("Usage: dev task approve <task-id>"); std::process::exit(1); }
    let tdir = find_task_dir(&task_id)
        .unwrap_or_else(|| { eprintln!("not found: {task_id}"); std::process::exit(1); });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let pdir = find_project_dir_for_task(&task_id).unwrap();

    if !tdir.join("plan.md").exists() {
        if json_out { println!(r#"{{"ok":false,"error":"plan_missing","message":"plan.md does not exist","task_id":"{task_id}"}}"#); }
        else { eprintln!("error: plan.md does not exist"); }
        std::process::exit(1);
    }
    let remaining = blocking_questions_open(&pdir, &task_id);
    if remaining > 0 {
        if json_out { println!(r#"{{"ok":false,"error":"blocking_questions_open","message":"cannot approve while {remaining} blocking questions are open","task_id":"{task_id}"}}"#); }
        else { eprintln!("error: {remaining} blocking question(s) open"); }
        std::process::exit(1);
    }
    plan_approve(&tdir).unwrap_or_else(|e| { eprintln!("error: {e}"); std::process::exit(1); });
    task_phase_set(&tdir, "approved", "human", "plan approved")
        .unwrap_or_else(|e| { eprintln!("error: {e}"); std::process::exit(1); });

    if json_out { json_ok(&task_id, &project_id, "approved", "plan approved"); }
    else { println!("approved: {task_id}"); }
}

// ── reject ────────────────────────────────────────────────────────────────────

fn cmd_reject(args: &[String]) {
    let mut task_id = String::new();
    let mut reason: Option<String> = None;
    let mut json_out = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--reason" => { i += 1; reason = Some(args.get(i).cloned().unwrap_or_default()); }
            "--json"   => json_out = true,
            a => if task_id.is_empty() { task_id = a.to_string(); }
        }
        i += 1;
    }
    if task_id.is_empty() { eprintln!("Usage: dev task reject <task-id> [--reason text]"); std::process::exit(1); }
    let tdir = find_task_dir(&task_id)
        .unwrap_or_else(|| { eprintln!("not found: {task_id}"); std::process::exit(1); });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let msg = format!("task rejected{}", reason.as_ref().map(|r| format!(": {r}")).unwrap_or_default());
    event_append(&tdir, "task_rejected", "human", &msg, None).ok();
    task_phase_set(&tdir, "rejected", "human", &msg)
        .unwrap_or_else(|e| { eprintln!("error: {e}"); std::process::exit(1); });

    if json_out { json_ok(&task_id, &project_id, "rejected", &msg); }
    else { println!("rejected: {task_id}"); }
}

// ── plan (planning agent) ─────────────────────────────────────────────────────

fn cmd_plan(args: &[String]) {
    let mut task_id = String::new();
    let mut tool = "claude".to_string();
    let mut model: Option<String> = None;
    let mut json_out = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--tool"  => { i += 1; tool  = args.get(i).cloned().unwrap_or_default(); }
            "--model" => { i += 1; model = Some(args.get(i).cloned().unwrap_or_default()); }
            "--json"  => json_out = true,
            a => if task_id.is_empty() { task_id = a.to_string(); }
        }
        i += 1;
    }
    if task_id.is_empty() { eprintln!("Usage: dev task plan <task-id> [--tool t] [--model m]"); std::process::exit(1); }
    let tdir = find_task_dir(&task_id)
        .unwrap_or_else(|| { eprintln!("not found: {task_id}"); std::process::exit(1); });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");

    task_phase_set(&tdir, "planning", "dev", "planning agent dispatched").ok();
    event_append(&tdir, "agent_dispatched", "dev",
        &format!("planning agent dispatched (tool={tool})"), None).ok();

    let prompt = format!(
        "You are planning dev task {task_id} for project {project_id}.\n\n\
         Rules:\n\
         - Do not edit files.\n\
         - Read the shared task context: dev task context {task_id} --markdown\n\
         - If behavior, scope, compatibility, API, UX, migration, release, or validation is ambiguous, run:\n\
             dev task ask {task_id} \"<question>\" --category <category> --severity blocking\n\
           and stop.\n\
         - If there are no blocking questions, write the plan:\n\
             dev task write-plan {task_id}\n\
         - The plan must include: understanding, proposed behavior, files to touch, \
         files not to touch, implementation steps, validation, risks, rollback.\n\
         - Do not implement until the task is approved."
    );

    let mut cmd = std::process::Command::new("dev");
    cmd.args(["agent", "dispatch", &project_id, "--tool", &tool]);
    if let Some(m) = &model { cmd.args(["--model", m]); }
    cmd.arg(&prompt);
    let _ = cmd.status();

    if json_out { json_ok(&task_id, &project_id, "planning", "planning agent dispatched"); }
    else { println!("planning: {task_id} (project={project_id} tool={tool})"); }
}

// ── dispatch (implementation agent) ──────────────────────────────────────────

fn cmd_dispatch(args: &[String]) {
    let mut task_id = String::new();
    let mut tool = "claude".to_string();
    let mut model: Option<String> = None;
    let mut worktree: Option<String> = None;
    let mut json_out = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--tool"     => { i += 1; tool     = args.get(i).cloned().unwrap_or_default(); }
            "--model"    => { i += 1; model    = Some(args.get(i).cloned().unwrap_or_default()); }
            "--worktree" => { i += 1; worktree = Some(args.get(i).cloned().unwrap_or_default()); }
            "--json"     => json_out = true,
            a => if task_id.is_empty() { task_id = a.to_string(); }
        }
        i += 1;
    }
    if task_id.is_empty() { eprintln!("Usage: dev task dispatch <task-id> [--tool t] [--worktree b]"); std::process::exit(1); }
    let tdir = find_task_dir(&task_id)
        .unwrap_or_else(|| { eprintln!("not found: {task_id}"); std::process::exit(1); });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let phase = vs(&v, "phase");

    if phase != "approved" && phase != "needs_fix" {
        if json_out { println!(r#"{{"ok":false,"error":"task_not_approved","message":"phase is {phase} (need approved or needs_fix)","task_id":"{task_id}"}}"#); }
        else { eprintln!("error: phase is {phase} (need approved or needs_fix)"); }
        std::process::exit(1);
    }
    if !tdir.join("approved-plan.md").exists() {
        if json_out { println!(r#"{{"ok":false,"error":"approved_plan_missing","message":"approved-plan.md not found","task_id":"{task_id}"}}"#); }
        else { eprintln!("error: approved-plan.md not found"); }
        std::process::exit(1);
    }

    let wt_branch = worktree.unwrap_or_else(|| {
        format!("task/{}", task_id.to_lowercase().replace(['_', ' '], "-"))
    });

    // Update task.json
    task_update_field(&tdir, "worktree_branch", serde_json::json!(wt_branch)).ok();
    task_update_field(&tdir, "assigned_tool", serde_json::json!(tool)).ok();
    if let Some(m) = &model { task_update_field(&tdir, "assigned_model", serde_json::json!(m)).ok(); }
    task_phase_set(&tdir, "implementing", "dev", "implementation started").ok();
    event_append(&tdir, "implementation_started", "dev", "implementation agent dispatched",
        Some(serde_json::json!({"tool": tool, "worktree": wt_branch}))).ok();

    let prompt = format!(
        "You are implementing approved dev task {task_id} for project {project_id}.\n\n\
         Rules:\n\
         - Read `dev task context {task_id} --markdown`.\n\
         - Implement only the approved plan.\n\
         - Do not broaden scope.\n\
         - If the approved plan is insufficient, run:\n\
             dev task ask {task_id} \"<question>\" --category <category> --severity blocking\n\
           and stop.\n\
         - Run the declared validation commands when feasible.\n\
         - At the end, write a handoff:\n\
             dev task write-handoff {task_id}\n\
           Include: changed files, tests run, results, risks, follow-up."
    );

    let mut cmd = std::process::Command::new("dev");
    cmd.args(["agent", "dispatch", &project_id, "--tool", &tool, "--worktree", &wt_branch]);
    if let Some(m) = &model { cmd.args(["--model", m]); }
    cmd.arg(&prompt);
    let _ = cmd.status();

    if json_out { json_ok(&task_id, &project_id, "implementing", "implementation agent dispatched"); }
    else { println!("dispatched: {task_id} → {project_id} ({tool}, worktree: {wt_branch})"); }
}

// ── attach ────────────────────────────────────────────────────────────────────

fn cmd_attach(args: &[String]) {
    let task_id = args.first().cloned().unwrap_or_default();
    if task_id.is_empty() { eprintln!("Usage: dev task attach <task-id>"); std::process::exit(1); }
    let tdir = find_task_dir(&task_id)
        .unwrap_or_else(|| { eprintln!("not found: {task_id}"); std::process::exit(1); });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let _ = std::process::Command::new("dev").args(["agent", "attach", &project_id]).status();
}

// ── logs ──────────────────────────────────────────────────────────────────────

fn cmd_logs(args: &[String]) {
    let mut task_id = String::new();
    let mut follow = false;
    let mut json_out = false;
    for a in args {
        match a.as_str() {
            "-f"     => follow = true,
            "--json" => json_out = true,
            a => if task_id.is_empty() { task_id = a.to_string(); }
        }
    }
    if task_id.is_empty() { eprintln!("Usage: dev task logs <task-id> [-f]"); std::process::exit(1); }
    let tdir = find_task_dir(&task_id)
        .unwrap_or_else(|| { eprintln!("not found: {task_id}"); std::process::exit(1); });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let mut cmd = std::process::Command::new("dev");
    cmd.args(["agent", "logs", &project_id]);
    if follow  { cmd.arg("-f"); }
    if json_out { cmd.arg("--json"); }
    let _ = cmd.status();
}

// ── kill ──────────────────────────────────────────────────────────────────────

fn cmd_kill(args: &[String]) {
    let mut task_id = String::new();
    let mut json_out = false;
    for a in args {
        match a.as_str() {
            "--json" => json_out = true,
            a => if task_id.is_empty() { task_id = a.to_string(); }
        }
    }
    if task_id.is_empty() { eprintln!("Usage: dev task kill <task-id>"); std::process::exit(1); }
    let tdir = find_task_dir(&task_id)
        .unwrap_or_else(|| { eprintln!("not found: {task_id}"); std::process::exit(1); });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let _ = std::process::Command::new("dev").args(["agent", "kill", &project_id]).status();
    event_append(&tdir, "task_killed", "human", "agent killed", None).ok();
    task_phase_set(&tdir, "killed", "human", "agent killed").ok();
    if json_out { json_ok(&task_id, &project_id, "killed", "agent killed"); }
    else { println!("killed: {task_id}"); }
}

// ── write-handoff ─────────────────────────────────────────────────────────────

fn cmd_write_handoff(args: &[String]) {
    let mut task_id = String::new();
    let mut file: Option<String> = None;
    let mut json_out = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--file" => { i += 1; file = Some(args.get(i).cloned().unwrap_or_default()); }
            "--json" => json_out = true,
            a => if task_id.is_empty() { task_id = a.to_string(); }
        }
        i += 1;
    }
    if task_id.is_empty() { eprintln!("Usage: dev task write-handoff <task-id> [--file path]"); std::process::exit(1); }
    let tdir = find_task_dir(&task_id)
        .unwrap_or_else(|| { eprintln!("not found: {task_id}"); std::process::exit(1); });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let pdir = find_project_dir_for_task(&task_id).unwrap();

    let content = if let Some(f) = file {
        std::fs::read_to_string(&f).unwrap_or_else(|e| { eprintln!("error: {e}"); std::process::exit(1); })
    } else {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf).unwrap_or_default();
        buf
    };
    handoff_write(&tdir, &content)
        .unwrap_or_else(|e| { eprintln!("error: {e}"); std::process::exit(1); });

    let remaining = blocking_questions_open(&pdir, &task_id);
    let new_phase = if remaining > 0 { "needs_spec" } else { "review" };
    task_phase_set(&tdir, new_phase, "agent", "handoff written").ok();

    if json_out { json_ok(&task_id, &project_id, new_phase, "handoff written"); }
    else { println!("handoff written → {new_phase}"); }
}

// ── handoff ───────────────────────────────────────────────────────────────────

fn cmd_handoff(args: &[String]) {
    let mut task_id = String::new();
    let mut json_out = false;
    for a in args {
        match a.as_str() {
            "--json" | "--markdown" => json_out = a == "--json",
            a => if task_id.is_empty() { task_id = a.to_string(); }
        }
    }
    if task_id.is_empty() { eprintln!("Usage: dev task handoff <task-id>"); std::process::exit(1); }
    let tdir = find_task_dir(&task_id)
        .unwrap_or_else(|| { eprintln!("not found: {task_id}"); std::process::exit(1); });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let handoff_path = tdir.join("handoff.md");
    let exists = handoff_path.exists();
    let content = if exists { std::fs::read_to_string(&handoff_path).unwrap_or_default() } else { String::new() };
    if json_out {
        println!("{}", serde_json::json!({"task_id": task_id, "project_id": project_id, "handoff": content, "exists": exists}));
    } else if exists {
        println!("{content}");
    } else {
        println!("(no handoff yet)");
    }
}

// ── harvest ───────────────────────────────────────────────────────────────────

fn cmd_harvest(args: &[String]) {
    let mut task_id = String::new();
    let mut json_out = false;
    for a in args {
        match a.as_str() {
            "--json" => json_out = true,
            a => if task_id.is_empty() { task_id = a.to_string(); }
        }
    }
    if task_id.is_empty() { eprintln!("Usage: dev task harvest <task-id>"); std::process::exit(1); }
    let tdir = find_task_dir(&task_id)
        .unwrap_or_else(|| { eprintln!("not found: {task_id}"); std::process::exit(1); });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");

    // Try git2 if worktree_path is set
    let files = {
        let wt_path = vs(&v, "worktree_path");
        if !wt_path.is_empty() {
            let path = std::path::Path::new(&wt_path);
            dev_core::git::diff_head_to_workdir(path)
                .ok()
                .map(|r| r.files)
        } else {
            None
        }
    };

    let files = if let Some(f) = files {
        f
    } else {
        // Fallback: call dev git diff --json
        let diff_output = std::process::Command::new("dev")
            .args(["git", "diff", &project_id, "--json"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        if let Ok(diff_v) = serde_json::from_str::<Value>(&diff_output) {
            if let Some(arr) = diff_v.get("files").and_then(|x| x.as_array()) {
                arr.iter()
                    .filter_map(|f| f.get("path").and_then(|x| x.as_str()).map(|s| s.to_string()))
                    .collect()
            } else { Vec::new() }
        } else { Vec::new() }
    };

    // Update task.json summary
    let json_path = tdir.join("task.json");
    if let Ok(content) = std::fs::read_to_string(&json_path) {
        if let Ok(mut tv) = serde_json::from_str::<Value>(&content) {
            tv["summary"]["diff_files"] = serde_json::json!(files);
            tv["updated_at"] = serde_json::json!(now_iso());
            if tdir.join("handoff.md").exists() {
                let h: String = std::fs::read_to_string(tdir.join("handoff.md"))
                    .unwrap_or_default().chars().take(150).collect();
                tv["summary"]["latest_handoff"] = serde_json::json!(h);
            }
            let _ = std::fs::write(&json_path, serde_json::to_string_pretty(&tv).unwrap());
        }
    }
    let n = files.len();
    event_append(&tdir, "diff_harvested", "dev", &format!("harvested {n} files"),
        Some(serde_json::json!({"file_count": n}))).ok();

    if json_out { json_ok(&task_id, &project_id, &vs(&v, "phase"), &format!("harvested {n} files")); }
    else { println!("harvested: {n} files ({task_id})"); }
}

// ── diff ──────────────────────────────────────────────────────────────────────

fn cmd_diff(args: &[String]) {
    let mut task_id = String::new();
    let mut stat = false;
    let mut json_out = false;
    for a in args {
        match a.as_str() {
            "--stat" => stat = true,
            "--json" => json_out = true,
            a => if task_id.is_empty() { task_id = a.to_string(); }
        }
    }
    if task_id.is_empty() { eprintln!("Usage: dev task diff <task-id> [--stat] [--json]"); std::process::exit(1); }
    let tdir = find_task_dir(&task_id)
        .unwrap_or_else(|| { eprintln!("not found: {task_id}"); std::process::exit(1); });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let mut cmd = std::process::Command::new("dev");
    cmd.args(["git", "diff", &project_id]);
    if stat    { cmd.arg("--stat"); }
    if json_out { cmd.arg("--json"); }
    let _ = cmd.status();
}

// ── test ──────────────────────────────────────────────────────────────────────

fn cmd_test(args: &[String]) {
    let mut task_id = String::new();
    let mut extra_cmd: Option<String> = None;
    let mut json_out = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--cmd"  => { i += 1; extra_cmd = Some(args.get(i).cloned().unwrap_or_default()); }
            "--json" => json_out = true,
            a => if task_id.is_empty() { task_id = a.to_string(); }
        }
        i += 1;
    }
    if task_id.is_empty() { eprintln!("Usage: dev task test <task-id> [--cmd cmd]"); std::process::exit(1); }
    let tdir = find_task_dir(&task_id)
        .unwrap_or_else(|| { eprintln!("not found: {task_id}"); std::process::exit(1); });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");

    let cmds: Vec<String> = if let Some(c) = extra_cmd {
        vec![c]
    } else {
        v.pointer("/validation/commands")
            .and_then(|x| x.as_array())
            .map(|arr| arr.iter().filter_map(|c| c.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default()
    };

    if cmds.is_empty() {
        if json_out { println!(r#"{{"ok":false,"error":"no_commands","message":"no validation commands defined","task_id":"{task_id}"}}"#); }
        else { eprintln!("no validation commands defined"); }
        std::process::exit(1);
    }

    let vid = next_test_run_id(&project_id).unwrap_or_else(|_| "V-unknown".to_string());
    let mut results = Vec::new();
    let mut passed = 0usize;
    let mut failed = 0usize;
    for c in &cmds {
        let out = std::process::Command::new("dev")
            .args(["run", &project_id, c])
            .output();
        let (ok, output) = match out {
            Ok(o) => (o.status.success(), String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr)),
            Err(e) => (false, e.to_string()),
        };
        if ok { passed += 1; } else { failed += 1; }
        results.push(serde_json::json!({"cmd": c, "ok": ok, "output": &output[..output.len().min(500)]}));
    }

    let test_status = if failed == 0 { "passed" } else if passed == 0 { "failed" } else { "partial" };
    let ts = now_iso();
    let result_json = serde_json::json!({
        "id": vid, "task_id": task_id, "commands": cmds,
        "results": results, "passed": passed, "failed": failed, "ts": ts
    });
    let results_dir = tdir.join("test-results");
    let _ = std::fs::create_dir_all(&results_dir);
    let _ = std::fs::write(results_dir.join(format!("{vid}.json")), serde_json::to_string_pretty(&result_json).unwrap());

    // Update task.json
    let json_path = tdir.join("task.json");
    if let Ok(content) = std::fs::read_to_string(&json_path) {
        if let Ok(mut tv) = serde_json::from_str::<Value>(&content) {
            tv["summary"]["test_status"] = serde_json::json!(test_status);
            tv["updated_at"] = serde_json::json!(now_iso());
            let _ = std::fs::write(&json_path, serde_json::to_string_pretty(&tv).unwrap());
        }
    }
    event_append(&tdir, "test_completed", "dev", &format!("{passed}/{} passed", passed + failed),
        Some(serde_json::json!({"passed": passed, "failed": failed, "vid": vid}))).ok();

    if json_out { println!("{}", serde_json::to_string(&result_json).unwrap()); }
    else { println!("test: {passed} passed, {failed} failed → {test_status}"); }
}

// ── review ────────────────────────────────────────────────────────────────────

fn cmd_review(args: &[String]) {
    let mut task_id = String::new();
    let mut tool: Option<String> = None;
    let mut json_out = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--tool" => { i += 1; tool = Some(args.get(i).cloned().unwrap_or_default()); }
            "--json" => json_out = true,
            a => if task_id.is_empty() { task_id = a.to_string(); }
        }
        i += 1;
    }
    if task_id.is_empty() { eprintln!("Usage: dev task review <task-id> [--tool t]"); std::process::exit(1); }
    let tdir = find_task_dir(&task_id)
        .unwrap_or_else(|| { eprintln!("not found: {task_id}"); std::process::exit(1); });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");

    let mut cmd = std::process::Command::new("dev");
    cmd.args(["agent", "review", &project_id]);
    if let Some(t) = &tool { cmd.args(["--tool", t]); }
    let out = cmd.output().unwrap_or_else(|e| { eprintln!("error: {e}"); std::process::exit(1); });
    let output = String::from_utf8_lossy(&out.stdout).to_string()
        + &String::from_utf8_lossy(&out.stderr);

    // Parse recommendation from output text
    let lower = output.to_lowercase();
    let recommendation = if lower.contains("mergeable") { "mergeable" }
        else if lower.contains("needs_fix") || lower.contains("needs fix") { "needs_fix" }
        else if lower.contains("reject") { "reject" }
        else { "unknown" };

    let rid = next_review_id(&project_id).unwrap_or_else(|_| "R-unknown".to_string());
    let ts = now_iso();
    let reviews_dir = tdir.join("reviews");
    let _ = std::fs::create_dir_all(&reviews_dir);
    let _ = std::fs::write(reviews_dir.join(format!("{rid}.md")), &output);
    let review_json = serde_json::json!({
        "id": rid, "task_id": task_id, "tool": tool.as_deref().unwrap_or("auto"),
        "output": &output[..output.len().min(2000)],
        "recommendation": recommendation, "ts": ts
    });
    let _ = std::fs::write(reviews_dir.join(format!("{rid}.json")), serde_json::to_string_pretty(&review_json).unwrap());

    // Update review_status and phase
    let json_path = tdir.join("task.json");
    if let Ok(content) = std::fs::read_to_string(&json_path) {
        if let Ok(mut tv) = serde_json::from_str::<Value>(&content) {
            tv["summary"]["review_status"] = serde_json::json!(recommendation);
            tv["updated_at"] = serde_json::json!(now_iso());
            let _ = std::fs::write(&json_path, serde_json::to_string_pretty(&tv).unwrap());
        }
    }
    let old_phase = vs(&v, "phase");
    let new_phase = match recommendation {
        "reject"    => "rejected",
        "needs_fix" => "needs_fix",
        "mergeable" => "mergeable",
        _           => &old_phase,
    };
    if new_phase != old_phase {
        task_phase_set(&tdir, new_phase, "dev", &format!("review: {recommendation}")).ok();
    }
    event_append(&tdir, "review_completed", "dev", &format!("review: {recommendation}"),
        Some(serde_json::json!({"rid": rid, "recommendation": recommendation}))).ok();

    if json_out { println!("{}", serde_json::to_string(&review_json).unwrap()); }
    else { print!("{output}"); println!("\n→ {recommendation}"); }
}

// ── fix ───────────────────────────────────────────────────────────────────────

fn cmd_fix(args: &[String]) {
    let mut task_id = String::new();
    let mut tool = "claude".to_string();
    let mut model: Option<String> = None;
    let mut json_out = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--tool"  => { i += 1; tool  = args.get(i).cloned().unwrap_or_default(); }
            "--model" => { i += 1; model = Some(args.get(i).cloned().unwrap_or_default()); }
            "--json"  => json_out = true,
            a => if task_id.is_empty() { task_id = a.to_string(); }
        }
        i += 1;
    }
    if task_id.is_empty() { eprintln!("Usage: dev task fix <task-id> [--tool t]"); std::process::exit(1); }
    let tdir = find_task_dir(&task_id)
        .unwrap_or_else(|| { eprintln!("not found: {task_id}"); std::process::exit(1); });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let phase = vs(&v, "phase");

    if phase != "needs_fix" {
        if json_out { println!(r#"{{"ok":false,"error":"task_not_needs_fix","message":"phase is {phase}","task_id":"{task_id}"}}"#); }
        else { eprintln!("error: phase is {phase} (need needs_fix)"); }
        std::process::exit(1);
    }
    let wt_branch = vs(&v, "worktree_branch");
    task_phase_set(&tdir, "implementing", "dev", "fix agent dispatched").ok();
    event_append(&tdir, "implementation_started", "dev", "fix agent dispatched",
        Some(serde_json::json!({"tool": tool, "worktree": wt_branch}))).ok();

    let prompt = format!(
        "You are fixing dev task {task_id} (phase: needs_fix) for project {project_id}.\n\n\
         Rules:\n\
         - Read `dev task context {task_id} --markdown` for the approved plan.\n\
         - Read `dev task handoff {task_id} --markdown` for the last handoff and review feedback.\n\
         - Fix only the reported issues. Do not change unrelated code.\n\
         - Run declared validation commands.\n\
         - At the end, write a new handoff: dev task write-handoff {task_id}"
    );

    let mut cmd = std::process::Command::new("dev");
    cmd.args(["agent", "dispatch", &project_id, "--tool", &tool]);
    if !wt_branch.is_empty() { cmd.args(["--worktree", &wt_branch]); }
    if let Some(m) = &model { cmd.args(["--model", m]); }
    cmd.arg(&prompt);
    let _ = cmd.status();

    if json_out { json_ok(&task_id, &project_id, "implementing", "fix agent dispatched"); }
    else { println!("fix dispatched: {task_id} ({tool})"); }
}

// ── pr ────────────────────────────────────────────────────────────────────────

fn cmd_pr(args: &[String]) {
    let mut task_id = String::new();
    let mut title: Option<String> = None;
    let mut base = "main".to_string();
    let mut draft = false;
    let mut json_out = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--title" => { i += 1; title = Some(args.get(i).cloned().unwrap_or_default()); }
            "--base"  => { i += 1; base  = args.get(i).cloned().unwrap_or_default(); }
            "--draft" => draft = true,
            "--json"  => json_out = true,
            a => if task_id.is_empty() { task_id = a.to_string(); }
        }
        i += 1;
    }
    if task_id.is_empty() { eprintln!("Usage: dev task pr <task-id> [--title t] [--base b]"); std::process::exit(1); }
    let tdir = find_task_dir(&task_id)
        .unwrap_or_else(|| { eprintln!("not found: {task_id}"); std::process::exit(1); });
    let v = read_task_json(&tdir);
    let project_id = vs(&v, "project_id");
    let phase = vs(&v, "phase");

    if phase != "mergeable" {
        if json_out { println!(r#"{{"ok":false,"error":"task_not_mergeable","message":"phase is {phase}","task_id":"{task_id}"}}"#); }
        else { eprintln!("error: phase is {phase} (need mergeable)"); }
        std::process::exit(1);
    }

    let mut cmd = std::process::Command::new("dev");
    cmd.args(["git", "pr", &project_id, "--base", &base]);
    if let Some(t) = &title { cmd.args(["--title", t]); }
    if draft { cmd.arg("--draft"); }
    if json_out { cmd.arg("--json"); }
    let out = cmd.output().unwrap_or_else(|e| { eprintln!("error: {e}"); std::process::exit(1); });
    let output = String::from_utf8_lossy(&out.stdout).to_string();

    // Extract PR URL
    let url: String = output.lines()
        .find(|l| l.contains("https://") && l.contains("/pull/"))
        .and_then(|l| l.split_whitespace().find(|w| w.starts_with("https://")))
        .unwrap_or("").to_string();

    // Update task.json
    let json_path = tdir.join("task.json");
    if let Ok(content) = std::fs::read_to_string(&json_path) {
        if let Ok(mut tv) = serde_json::from_str::<Value>(&content) {
            tv["links"]["pr_url"] = serde_json::json!(url);
            tv["updated_at"] = serde_json::json!(now_iso());
            let _ = std::fs::write(&json_path, serde_json::to_string_pretty(&tv).unwrap());
        }
    }
    event_append(&tdir, "pr_created", "human", &format!("PR: {url}"),
        Some(serde_json::json!({"url": url}))).ok();
    task_phase_set(&tdir, "merged", "human", "PR created").ok();

    if json_out { print!("{output}"); }
    else { println!("PR: {}", if url.is_empty() { output.trim() } else { &url }); }
}
