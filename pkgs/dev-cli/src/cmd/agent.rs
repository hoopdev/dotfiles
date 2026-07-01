//! `dev agent` — the full agent lifecycle over the `~/.dev/runs` registry:
//! ps / dispatch / logs / kill / start / attach / review / watch / runs / prune.
//! `logs`/`kill` accept a project (newest run / all live agents) or a run id.

use super::interactive;
use clap::Subcommand;
use dev_core::agent;
use dev_core::config::{Config, Target};
use dev_core::ssh;
use serde_json::json;
use std::process::Command;

#[derive(Subcommand)]
pub enum AgentCmd {
    /// List running agents across all projects.
    Ps,
    /// Tail an agent's log (last 200 lines; `-f` follows).
    Logs {
        reference: String,
        #[arg(short, long)]
        follow: bool,
    },
    /// Launch a background agent on a project.
    Dispatch {
        project: String,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        effort: Option<String>,
        #[arg(long)]
        sandbox: Option<String>,
        #[arg(long)]
        worktree: Option<String>,
        /// Track this dispatch as a task so the agent can raise blocking questions
        /// into the Inbox (`dev task ask`) and leave a handoff. Local targets only.
        #[arg(long)]
        supervise: bool,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        task: Vec<String>,
    },
    /// Stop the running agent(s) for a project/run.
    Kill { reference: String },
    /// Open an interactive agent on a project: reconnect to a live agent, else
    /// resume the newest session (`--fresh` starts a new one). `--backend` picks
    /// which; omit it to reconnect / fzf-pick.
    Attach {
        reference: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        /// Start a fresh session instead of resuming the last one.
        #[arg(long)]
        fresh: bool,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        extra: Vec<String>,
    },
    /// Dispatch a code-review agent on a project (a specialised `dispatch`).
    Review {
        project: String,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        effort: Option<String>,
        #[arg(long)]
        worktree: Option<String>,
        #[arg(long)]
        base: Option<String>,
    },
    /// Print an agent's final result — the last assistant message (claude) or the
    /// run log (others). Richer than `logs`; `--full` prints the whole log.
    Output {
        reference: String,
        #[arg(long)]
        full: bool,
    },
    /// Continue a run with a new instruction: re-dispatch a background agent seeded
    /// with the prior run's result, on the same project/backend/worktree.
    Followup {
        reference: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        text: Vec<String>,
    },
    /// Poll agent state and push a Telegram note on finish / waiting / error.
    Watch {
        #[arg(long, default_value_t = 30)]
        interval: u64,
        #[arg(long)]
        once: bool,
    },
    /// List recorded runs (state: running | exited).
    Runs { project: Option<String> },
    /// GC the run registry — remove finished run records.
    Prune {
        project: Option<String>,
        /// Also remove live runs (default keeps them).
        #[arg(long)]
        all: bool,
        /// Only remove runs older than N days.
        #[arg(long)]
        older: Option<u64>,
    },
}

pub fn run(cmd: &AgentCmd, json: bool) {
    match cmd {
        AgentCmd::Ps => ps(json),
        AgentCmd::Logs { reference, follow } => logs(reference, *follow, json),
        AgentCmd::Dispatch {
            project,
            backend,
            model,
            effort,
            sandbox,
            worktree,
            supervise,
            task,
        } => dispatch(
            project, backend, model, effort, sandbox, worktree, *supervise, task, json,
        ),
        AgentCmd::Kill { reference } => kill(reference, json),
        AgentCmd::Attach {
            reference,
            backend,
            fresh,
            extra,
        } => attach(reference, backend.as_deref(), *fresh, extra),
        AgentCmd::Review {
            project,
            backend,
            model,
            effort,
            worktree,
            base,
        } => review(
            project,
            backend,
            model,
            effort,
            worktree,
            base.as_deref(),
            json,
        ),
        AgentCmd::Output { reference, full } => output(reference, *full, json),
        AgentCmd::Followup { reference, text } => followup(reference, text, json),
        AgentCmd::Watch { interval, once } => watch(*interval, *once),
        AgentCmd::Runs { project } => runs(project.as_deref(), json),
        AgentCmd::Prune {
            project,
            all,
            older,
        } => prune(project.as_deref(), *all, *older),
    }
}

fn runs(project: Option<&str>, json_out: bool) {
    let cfg = Config::load_or_default();
    let rows = agent::list_runs(&cfg, project);
    if json_out {
        println!("{}", serde_json::to_string(&rows).unwrap());
        return;
    }
    if rows.is_empty() {
        println!("no runs");
        return;
    }
    println!(
        "{:<30} {:<9} {:<8} {:<18} STARTED",
        "ID", "TOOL", "STATE", "PROJECT"
    );
    for r in &rows {
        println!(
            "{:<30} {:<9} {:<8} {:<18} {}",
            r["id"].as_str().unwrap_or("?"),
            r["tool"].as_str().unwrap_or("-"),
            r["state"].as_str().unwrap_or("-"),
            r["project"].as_str().unwrap_or("-"),
            r["started"].as_str().unwrap_or("")
        );
    }
}

fn prune(project: Option<&str>, all: bool, older: Option<u64>) {
    let cfg = Config::load_or_default();
    // keep_alive = !all → by default we never remove a still-running run.
    let n = agent::prune(&cfg, project, !all, older);
    println!("pruned {n} run(s)");
}

/// Open a fresh interactive `backend` in a project (also the claude/codex/…
/// shortcuts and `attach --fresh`). `extra` is appended verbatim.
pub fn start_fresh(backend: &str, project: &Option<String>, extra: &[String]) {
    let cfg = Config::load_or_default();
    let n = interactive::require_project(&cfg, project.as_deref());
    let Some(t) = cfg.resolve(&n) else {
        eprintln!("dev {backend}: unknown project '{n}'");
        std::process::exit(1);
    };
    interactive::exec_on_target(&cfg, &t, Some(&append_extra(backend, extra)));
}

/// Append shell-quoted `extra` args to a base command (backend name or a resume
/// command produced by [`agent::attach_command`]).
fn append_extra(base: &str, extra: &[String]) -> String {
    if extra.is_empty() {
        base.to_string()
    } else {
        let q: Vec<String> = extra.iter().map(|a| ssh::sh_quote(a)).collect();
        format!("{base} {}", q.join(" "))
    }
}

/// Open an interactive agent: reconnect to a live one, else resume the newest
/// session (`fresh` starts a new one instead). `backend_override` skips the
/// picker. This is the merged `start` + `attach`.
fn attach(
    reference: &Option<String>,
    backend_override: Option<&str>,
    fresh: bool,
    extra: &[String],
) {
    let cfg = Config::load_or_default();
    let n = interactive::require_project(&cfg, reference.as_deref());
    let Some(t) = cfg.resolve(&n) else {
        eprintln!("dev agent attach: unknown project '{n}'");
        std::process::exit(1);
    };

    // Live agents on this project (a row with both pid and backend set).
    let rows = agent::ps_project(&cfg, &n);
    let running: Vec<&serde_json::Value> = rows
        .iter()
        .filter(|r| !r.get("pid").map(|v| v.is_null()).unwrap_or(true))
        .filter(|r| r.get("tool").and_then(|v| v.as_str()).is_some())
        .collect();

    // Reconnect to a live agent — unless the user forced a fresh session or
    // named a specific backend to open.
    let reconnect = !fresh && backend_override.is_none();
    let (backend_name, session) = if reconnect && running.len() == 1 {
        row_backend_session(running[0])
    } else if reconnect && running.len() > 1 {
        // Several live agents — let the user choose which to reconnect to.
        let labels: Vec<String> = running
            .iter()
            .map(|r| {
                format!(
                    "{}  pid={}  {}",
                    r["tool"].as_str().unwrap_or("?"),
                    r["pid"],
                    r["status"].as_str().unwrap_or("")
                )
            })
            .collect();
        match interactive::pick(&labels, "attach> ") {
            Some(sel) => {
                let idx = labels.iter().position(|l| *l == sel).unwrap_or(0);
                row_backend_session(running[idx])
            }
            None => return, // user aborted
        }
    } else {
        // Nothing live (or --fresh / --backend): choose a backend and, when
        // resuming, carry the newest run's session for that same backend.
        let newest = agent::newest_run(&cfg, &n);
        let default = agent::default_backend(&cfg, &n, newest.as_ref());
        // Attach is genuinely interactive (a human reconnecting), so let the fzf
        // picker run when the backend is omitted on a TTY.
        let chosen =
            interactive::choose_backend(backend_override, agent::Purpose::Fresh, &default, false);
        let session = newest
            .as_ref()
            .filter(|m| m.get("tool").and_then(|v| v.as_str()) == Some(chosen.as_str()))
            .and_then(|m| m.get("session").and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_string();
        (chosen, session)
    };

    let Some(spec) = agent::backend(&backend_name) else {
        eprintln!("dev agent attach: unknown backend '{backend_name}'");
        std::process::exit(1);
    };
    let base = agent::attach_command(spec, &session, fresh);
    interactive::exec_on_target(&cfg, &t, Some(&append_extra(&base, extra)));
}

/// (backend, session_id) from a `ps` row (wire key stays `tool`).
fn row_backend_session(r: &serde_json::Value) -> (String, String) {
    (
        r["tool"].as_str().unwrap_or("claude").to_string(),
        r["session_id"].as_str().unwrap_or("").to_string(),
    )
}

/// Code review is a `dispatch` with a canned prompt — it shares the same
/// backend/model/effort/worktree resolution and picker behaviour.
#[allow(clippy::too_many_arguments)]
fn review(
    project: &str,
    backend: &Option<String>,
    model: &Option<String>,
    effort: &Option<String>,
    worktree: &Option<String>,
    base: Option<&str>,
    json_out: bool,
) {
    let prompt = agent::review_prompt(base);
    dispatch(
        project,
        backend,
        model,
        effort,
        &None,
        worktree,
        false,
        std::slice::from_ref(&prompt),
        json_out,
    );
}

use std::collections::HashMap;

fn watch_state_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    std::path::PathBuf::from(home)
        .join(".dev")
        .join("watch-state.json")
}

fn load_watch_state(p: &std::path::Path) -> HashMap<String, String> {
    std::fs::read_to_string(p)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_watch_state(p: &std::path::Path, m: &HashMap<String, String>) {
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(s) = serde_json::to_string(m) {
        let _ = std::fs::write(p, s);
    }
}

/// Currently-running agents keyed `target:tool` → status.
fn current_running(cfg: &Config) -> HashMap<String, String> {
    let mut cur = HashMap::new();
    for r in agent::ps(cfg) {
        let has_pid = r.get("pid").map(|v| !v.is_null()).unwrap_or(false);
        if let (true, Some(tool)) = (has_pid, r.get("tool").and_then(|v| v.as_str())) {
            let target = r["target"].as_str().unwrap_or("");
            cur.insert(
                format!("{target}:{tool}"),
                r["status"].as_str().unwrap_or("running").to_string(),
            );
        }
    }
    cur
}

/// Poll agent state; notify on **finish** (was running, now gone) and on
/// waiting/error transitions. State is persisted so `--once` (cron) compares
/// against the previous invocation and a loop restart doesn't re-notify.
fn watch(interval: u64, once: bool) {
    let cfg = Config::load_or_default();
    let path = watch_state_path();
    let mut prev = load_watch_state(&path);
    let mut first = true;
    loop {
        let cur = current_running(&cfg);
        // In loop mode the very first tick is a silent baseline; --once always
        // announces (its baseline is the persisted previous invocation).
        if once || !first {
            for (k, s) in &cur {
                if prev.get(k) != Some(s) && matches!(s.as_str(), "waiting" | "error") {
                    dev_core::notify::send(&format!("dev: {k} → {s}"));
                }
            }
            for k in prev.keys() {
                if !cur.contains_key(k) {
                    dev_core::notify::send(&format!("dev: {k} finished"));
                }
            }
        }
        prev = cur;
        save_watch_state(&path, &prev);
        first = false;
        if once {
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(interval));
    }
}

#[allow(clippy::too_many_arguments)]
fn dispatch(
    project: &str,
    backend: &Option<String>,
    model: &Option<String>,
    effort: &Option<String>,
    sandbox: &Option<String>,
    worktree: &Option<String>,
    supervise: bool,
    task: &[String],
    json_out: bool,
) {
    let cfg = Config::load_or_default();
    let taskstr = task.join(" ");
    if taskstr.trim().is_empty() {
        eprintln!("Usage: dev agent dispatch <project> [--backend b] [--supervise] [--worktree b] <task...>");
        std::process::exit(1);
    }
    // --backend wins; else fzf-pick on a TTY; else the project/newest default.
    // `json_out` marks a non-interactive caller (agent/skill/TUI) → no fzf.
    let default = agent::default_backend(&cfg, project, None);
    let backend_s = interactive::choose_backend(
        backend.as_deref(),
        agent::Purpose::Dispatch,
        &default,
        json_out,
    );
    // --model wins; else fzf-pick from the backend's registry on a TTY; else none.
    let model_s = interactive::choose_model(model.as_deref(), &backend_s, json_out);
    let opts = agent::DispatchOpts {
        backend: &backend_s,
        task: &taskstr,
        model: model_s.as_deref(),
        effort: effort.as_deref(),
        sandbox: sandbox.as_deref(),
        worktree: worktree.as_deref(),
        supervise,
    };
    match agent::dispatch(&cfg, project, &opts) {
        Ok(v) => {
            if json_out {
                println!("{}", serde_json::to_string(&v).unwrap());
            } else {
                let task_note = v["task_id"]
                    .as_str()
                    .filter(|t| !t.is_empty())
                    .map(|t| format!(" task={t}"))
                    .unwrap_or_default();
                println!(
                    "dispatched {} on {} (backend={}{}{})",
                    v["id"].as_str().unwrap_or("?"),
                    project,
                    opts.backend,
                    v["pid"]
                        .as_str()
                        .filter(|p| !p.is_empty())
                        .map(|p| format!(" pid={p}"))
                        .unwrap_or_default(),
                    task_note,
                );
                if v["supervise_skipped"].as_bool() == Some(true) {
                    eprintln!(
                        "note: --supervise ignored (task store is local-only; target is remote)"
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("dev agent dispatch: {e:#}");
            std::process::exit(1);
        }
    }
}

fn kill(reference: &str, json_out: bool) {
    let cfg = Config::load_or_default();
    let v = agent::kill(&cfg, reference);
    if json_out {
        println!("{}", serde_json::to_string(&v).unwrap());
    } else if v["ok"].as_bool() == Some(true) {
        let killed: Vec<&str> = v["killed"]
            .as_array()
            .map(|a| a.iter().filter_map(|x| x.as_str()).collect())
            .unwrap_or_default();
        println!(
            "killed {reference}: {}",
            if killed.is_empty() {
                "none".to_string()
            } else {
                killed.join(" ")
            }
        );
    } else {
        eprintln!(
            "dev agent kill: {}",
            v["error"].as_str().unwrap_or("failed")
        );
        std::process::exit(1);
    }
}

fn ps(json_out: bool) {
    let cfg = Config::load_or_default();
    let rows = agent::ps(&cfg);
    if json_out {
        println!("{}", serde_json::to_string(&rows).unwrap());
        return;
    }
    println!("{:<24} {:<10} PID  STATUS/CWD", "PROJECT", "TOOL");
    for r in &rows {
        let t = r["target"].as_str().unwrap_or("?");
        let tool = r["tool"].as_str().unwrap_or("-");
        let status = r["status"].as_str().unwrap_or("");
        let pid = r["pid"]
            .as_i64()
            .map(|p| p.to_string())
            .unwrap_or_else(|| "-".to_string());
        let cwd = r["cwd"].as_str().unwrap_or("");
        let detail = if cwd.is_empty() {
            status.to_string()
        } else {
            format!("{status} {cwd}")
        };
        println!("{t:<24} {tool:<10} pid={pid:<8} {detail}");
    }
}

fn logs(reference: &str, follow: bool, json_out: bool) {
    let cfg = Config::load_or_default();
    if follow && !json_out {
        follow_tail(&cfg, reference);
        return;
    }
    let lines = agent::recent_activity(&cfg, reference, None);
    if json_out {
        println!(
            "{}",
            serde_json::to_string(&json!({ "target": reference, "lines": lines })).unwrap()
        );
    } else {
        for l in &lines {
            println!("{l}");
        }
    }
}

/// `dev agent output` — the run's final result, beyond the scrolling log tail.
fn output(reference: &str, full: bool, json_out: bool) {
    let cfg = Config::load_or_default();
    let lines = agent::final_output(&cfg, reference, full);
    if json_out {
        println!(
            "{}",
            serde_json::to_string(&json!({ "target": reference, "lines": lines })).unwrap()
        );
    } else {
        for l in &lines {
            println!("{l}");
        }
    }
}

/// `dev agent followup` — continue a run with a new instruction (background).
fn followup(reference: &str, text: &[String], json_out: bool) {
    let cfg = Config::load_or_default();
    let instruction = text.join(" ");
    if instruction.trim().is_empty() {
        eprintln!("Usage: dev agent followup <project|run-id> <instruction...>");
        std::process::exit(1);
    }
    match agent::followup(&cfg, reference, &instruction) {
        Ok(v) => {
            if json_out {
                println!("{}", serde_json::to_string(&v).unwrap());
            } else {
                println!(
                    "followup dispatched {} on {}",
                    v["id"].as_str().unwrap_or("?"),
                    v["target"].as_str().unwrap_or(reference),
                );
            }
        }
        Err(e) => {
            eprintln!("dev agent followup: {e:#}");
            std::process::exit(1);
        }
    }
}

/// `-f` streaming — exec `tail -f` on the run's log (local or over ssh), which
/// inherits stdio and streams. claude logs to its own store, so we short-circuit.
fn follow_tail(cfg: &Config, reference: &str) {
    let Some((target, _)) = agent::resolve_run(cfg, reference) else {
        eprintln!("dev agent logs: unknown target/run '{reference}'");
        std::process::exit(1);
    };
    // claude has no `~/.dev/runs/*.log` to `tail -f`; it keeps a JSONL transcript.
    // Codex interactive sessions are the same shape: readable transcript, but no
    // dev-owned file descriptor to stream. Print the current activity snapshot
    // and stop; the TUI polls these transcript-backed backends.
    let Some(log) = agent::run_log_name(cfg, reference) else {
        for l in agent::recent_activity(cfg, reference, None) {
            println!("{l}");
        }
        eprintln!(
            "dev agent logs: no streamable dev run log for '{reference}' \
             (transcript snapshot shown above; `dev agent attach {reference}` for a live session)"
        );
        return;
    };
    match target {
        Target::Local { .. } => {
            if let Some(dir) = agent::runs_dir_local_path() {
                let path = dir.join(&log);
                let _ = Command::new("tail")
                    .arg("-n")
                    .arg("200")
                    .arg("-f")
                    .arg(path)
                    .status();
            }
        }
        Target::Remote { env, .. } => {
            if let Some(e) = cfg.env(&env) {
                let mut c = Command::new("ssh");
                c.args(ssh::ssh_opts(e, false));
                c.arg("-T").arg(&e.host);
                c.arg(format!("tail -n 200 -f \"$HOME/.dev/runs/{log}\""));
                let _ = c.status();
            }
        }
        Target::Env { .. } => {}
    }
}
