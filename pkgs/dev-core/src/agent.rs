//! Agent backends — the ONE place agent tools are enumerated (port of the bash
//! `_dev_tools_json` registry), plus process discovery (`ps`) and log tailing
//! over the `~/.dev/runs` registry.
//!
//! Gated on `config` (needs [`crate::config::Env`] / [`crate::ssh`]).

use crate::config::{Config, Env, Target};
use crate::ssh;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// How `dev agent attach` resumes a backend's previous session. Turned into an
/// actual command by [`attach_command`] — the ONE place resume commands live
/// (was duplicated in the CLI `attach_cmd` and the TUI's hardcoded per-tool arm).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AttachStyle {
    /// `claude --resume <session>` (bare `claude` when no session is known).
    ResumeSession,
    /// `codex resume --last`.
    ResumeLast,
    /// `opencode --continue`.
    Continue,
    /// No resume concept — always a fresh interactive launch (`agy`).
    Fresh,
}

/// One selectable model for a backend. `id` is what gets passed to `--model`
/// (empty ⇒ "the backend's own default"); `label` is the human/picker string.
#[derive(Clone, Copy)]
pub struct ModelSpec {
    pub label: &'static str,
    pub id: &'static str,
}

/// One agent backend (claude/codex/opencode/agy). The single registry every
/// crate reads — the CLI, the TUI picker, and dispatch validation.
pub struct BackendSpec {
    pub name: &'static str,
    pub interactive: bool,
    pub dispatchable: bool,
    pub review: bool,
    /// How to find running instances: `agents-json` (claude) | `pgrep`.
    pub ps_detect: &'static str,
    /// How `dev agent attach` resumes this backend.
    pub attach: AttachStyle,
    /// Subcommand token for nohup dispatch; empty ⇒ `claude --bg`.
    pub bg_sub: &'static str,
    /// Curated models offered by the picker / `dev models`.
    pub models: &'static [ModelSpec],
}

pub const BACKENDS: &[BackendSpec] = &[
    BackendSpec {
        name: "claude",
        interactive: true,
        dispatchable: true,
        review: true,
        ps_detect: "agents-json",
        attach: AttachStyle::ResumeSession,
        bg_sub: "",
        models: &[
            ModelSpec {
                label: "opus",
                id: "claude-opus-4-8",
            },
            ModelSpec {
                label: "sonnet",
                id: "claude-sonnet-5",
            },
            ModelSpec {
                label: "haiku",
                id: "claude-haiku-4-5-20251001",
            },
            ModelSpec {
                label: "fable",
                id: "claude-fable-5",
            },
        ],
    },
    BackendSpec {
        name: "codex",
        interactive: true,
        dispatchable: true,
        review: true,
        ps_detect: "pgrep",
        attach: AttachStyle::ResumeLast,
        bg_sub: "exec",
        models: &[
            ModelSpec {
                label: "gpt-4o",
                id: "gpt-4o",
            },
            ModelSpec {
                label: "gpt-4.1",
                id: "gpt-4.1",
            },
            ModelSpec {
                label: "gpt-4.5",
                id: "gpt-4.5",
            },
            ModelSpec {
                label: "gpt-5.5",
                id: "gpt-5.5",
            },
            ModelSpec {
                label: "o4-mini",
                id: "o4-mini",
            },
        ],
    },
    BackendSpec {
        name: "opencode",
        interactive: true,
        dispatchable: true,
        review: true,
        ps_detect: "pgrep",
        attach: AttachStyle::Continue,
        bg_sub: "run",
        models: &[
            ModelSpec {
                label: "(default)",
                id: "",
            },
            ModelSpec {
                label: "lite-llm/Qwen3.6-27B",
                id: "lite-llm/unsloth/Qwen3.6-27B-NVFP4",
            },
            ModelSpec {
                label: "vllm-oiwa/Gemma4-31B",
                id: "vllm-oiwa/nvidia/Gemma-4-31B-IT-NVFP4",
            },
        ],
    },
    BackendSpec {
        name: "agy",
        interactive: true,
        dispatchable: true,
        review: true,
        ps_detect: "pgrep",
        attach: AttachStyle::Fresh,
        bg_sub: "-p",
        models: &[
            ModelSpec {
                label: "Gemini 3.5 Flash (Medium)",
                id: "Gemini 3.5 Flash (Medium)",
            },
            ModelSpec {
                label: "Gemini 3.5 Flash (High)",
                id: "Gemini 3.5 Flash (High)",
            },
            ModelSpec {
                label: "Gemini 3.5 Flash (Low)",
                id: "Gemini 3.5 Flash (Low)",
            },
            ModelSpec {
                label: "Gemini 3.1 Pro (Low)",
                id: "Gemini 3.1 Pro (Low)",
            },
            ModelSpec {
                label: "Gemini 3.1 Pro (High)",
                id: "Gemini 3.1 Pro (High)",
            },
            ModelSpec {
                label: "Claude Sonnet 4.6 (Thinking)",
                id: "Claude Sonnet 4.6 (Thinking)",
            },
            ModelSpec {
                label: "Claude Opus 4.6 (Thinking)",
                id: "Claude Opus 4.6 (Thinking)",
            },
            ModelSpec {
                label: "GPT-OSS 120B (Medium)",
                id: "GPT-OSS 120B (Medium)",
            },
        ],
    },
];

pub fn backend(name: &str) -> Option<&'static BackendSpec> {
    BACKENDS.iter().find(|b| b.name == name)
}

/// The interactive command that re-opens `spec` on a target. `fresh` forces a
/// brand-new session (the old `dev agent start`); otherwise resume per the
/// backend's [`AttachStyle`]. This is the single source of resume commands.
pub fn attach_command(spec: &BackendSpec, session: &str, fresh: bool) -> String {
    if fresh {
        return spec.name.to_string();
    }
    match spec.attach {
        AttachStyle::ResumeSession if !session.is_empty() => {
            format!("{} --resume {}", spec.name, ssh::sh_quote(session))
        }
        AttachStyle::ResumeLast => format!("{} resume --last", spec.name),
        AttachStyle::Continue => format!("{} --continue", spec.name),
        AttachStyle::ResumeSession | AttachStyle::Fresh => spec.name.to_string(),
    }
}

/// What a backend picker is being opened for — filters [`backends_for`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// Open a fresh interactive session (any interactive backend).
    Fresh,
    /// Background dispatch (`dispatchable` backends).
    Dispatch,
    /// Code review (`review`-capable backends).
    Review,
}

/// Backends valid for `purpose`, in registry order.
pub fn backends_for(purpose: Purpose) -> Vec<&'static BackendSpec> {
    BACKENDS
        .iter()
        .filter(|b| match purpose {
            Purpose::Fresh => b.interactive,
            Purpose::Dispatch => b.dispatchable,
            Purpose::Review => b.review,
        })
        .collect()
}

/// The default backend for a project: its configured backend, else the newest
/// recorded run's backend, else `claude`. The ONE resolver — dispatch, review,
/// and attach all use it (they previously each had a different fallback chain).
pub fn default_backend(cfg: &Config, project: &str, newest: Option<&Value>) -> String {
    cfg.project_backend(project)
        .or_else(|| {
            newest
                .and_then(|m| m.get("tool").and_then(|v| v.as_str()))
                .map(String::from)
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "claude".to_string())
}

/// `dev backends --json` — the registry as an array of objects.
pub fn registry_json() -> Value {
    Value::Array(
        BACKENDS
            .iter()
            .map(|b| {
                json!({
                    "name": b.name,
                    "interactive": b.interactive,
                    "dispatchable": b.dispatchable,
                    "review": b.review,
                    "ps_detect": b.ps_detect,
                    "attach": format!("{:?}", b.attach),
                    "bg_sub": b.bg_sub,
                    "models": b.models.iter()
                        .map(|m| json!({ "label": m.label, "id": m.id }))
                        .collect::<Vec<_>>(),
                })
            })
            .collect(),
    )
}

// ── small process helpers ───────────────────────────────────────────────────

/// stdout of a command regardless of exit status (pgrep exits 1 on no match).
fn output_stdout(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

/// cwd of a local pid — `lsof` (macOS) with a `/proc` fallback (Linux).
fn local_cwd(pid: &str) -> Option<String> {
    let out = output_stdout("lsof", &["-p", pid, "-a", "-d", "cwd", "-Fn"]);
    for line in out.lines() {
        if let Some(rest) = line.strip_prefix('n') {
            return Some(rest.to_string());
        }
    }
    std::fs::read_link(format!("/proc/{pid}/cwd"))
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
}

fn stopped_row(name: &str, loc: &str, status: &str) -> Value {
    json!({
        "target": name, "location": loc, "tool": Value::Null, "pid": Value::Null,
        "status": status, "kind": Value::Null, "cwd": Value::Null,
        "session_id": Value::Null, "name": Value::Null,
    })
}

/// Build claude agent rows from a `claude agents --json` array, collecting the
/// pids so pgrep can skip them.
fn claude_rows(name: &str, loc: &str, cjson: &Value, claude_pids: &mut Vec<i64>) -> Vec<Value> {
    let mut rows = Vec::new();
    if let Some(arr) = cjson.as_array() {
        for a in arr {
            if let Some(p) = a.get("pid").and_then(|v| v.as_i64()) {
                claude_pids.push(p);
            }
            rows.push(json!({
                "target": name, "location": loc, "tool": "claude",
                "pid": a.get("pid").cloned().unwrap_or(Value::Null),
                "status": a.get("status").and_then(|v| v.as_str()).unwrap_or("running"),
                "kind": a.get("kind").cloned().unwrap_or(Value::Null),
                "cwd": Value::Null,
                "session_id": a.get("sessionId").cloned().unwrap_or(Value::Null),
                "name": a.get("name").cloned().unwrap_or(Value::Null),
            }));
        }
    }
    rows
}

// ── ps: process discovery ───────────────────────────────────────────────────

fn ps_local(name: &str, lp: &str) -> Vec<Value> {
    let cjson: Value =
        serde_json::from_str(output_stdout("claude", &["agents", "--json", "--cwd", lp]).trim())
            .unwrap_or(json!([]));
    let mut claude_pids = Vec::new();
    let mut rows = claude_rows(name, "local", &cjson, &mut claude_pids);

    // pgrep net: catch tools with no JSON interface (and any claude the JSON
    // missed), matched to this project by process cwd.
    for tool in ["claude", "codex", "opencode", "agy"] {
        for pid in output_stdout("pgrep", &["-x", tool])
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
        {
            if tool == "claude" {
                if let Ok(p) = pid.parse::<i64>() {
                    if claude_pids.contains(&p) {
                        continue;
                    }
                }
            }
            if let Some(cwd) = local_cwd(pid) {
                if cwd.starts_with(lp) {
                    rows.push(json!({
                        "target": name, "location": "local", "tool": tool,
                        "pid": pid.parse::<i64>().ok(), "status": "running",
                        "kind": Value::Null, "cwd": cwd,
                        "session_id": Value::Null, "name": Value::Null,
                    }));
                }
            }
        }
    }
    if rows.is_empty() {
        rows.push(stopped_row(name, "local", "stopped"));
    }
    rows
}

fn ps_remote_unix(name: &str, env: &Env, rp: &str) -> Vec<Value> {
    let claude_cmd = format!(
        "if command -v claude >/dev/null 2>&1; then claude agents --json --cwd '{rp}' 2>/dev/null || echo '[]'; else echo '[]'; fi"
    );
    let cjson_str = match ssh::exec_capture(env, "", &claude_cmd) {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
        _ => return vec![stopped_row(name, "remote", "unreachable")],
    };
    let cjson: Value = serde_json::from_str(cjson_str.trim()).unwrap_or(json!([]));
    let mut claude_pids = Vec::new();
    let mut rows = claude_rows(name, "remote", &cjson, &mut claude_pids);

    let others = format!(
        "for _tool in codex opencode agy; do \
           pids=$(pgrep -x \"$_tool\" 2>/dev/null) || continue; \
           while IFS= read -r pid; do \
             cwd=$(readlink /proc/$pid/cwd 2>/dev/null || lsof -p \"$pid\" -a -d cwd -Fn 2>/dev/null | awk '/^n/{{print substr($0,2)}}'); \
             [[ \"$cwd\" == \"{rp}\"* ]] && printf '%s %s %s\\n' \"$_tool\" \"$pid\" \"$cwd\"; \
           done <<< \"$pids\"; \
         done"
    );
    if let Some(out) = ssh::exec_stdout(env, "", &others) {
        for line in out.lines() {
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() == 3 {
                rows.push(json!({
                    "target": name, "location": "remote", "tool": parts[0],
                    "pid": parts[1].parse::<i64>().ok(), "status": "running",
                    "kind": Value::Null, "cwd": parts[2],
                    "session_id": Value::Null, "name": Value::Null,
                }));
            }
        }
    }
    if rows.is_empty() {
        rows.push(stopped_row(name, "remote", "stopped"));
    }
    rows
}

#[cfg(feature = "windows")]
fn ps_remote_windows(name: &str, env: &Env, rp: &str) -> Vec<Value> {
    let cjson = match crate::windows::claude_agents_json(env, rp) {
        Some(v) => v,
        None => return vec![stopped_row(name, "remote", "unreachable")],
    };
    let mut claude_pids = Vec::new();
    let mut rows = claude_rows(name, "remote", &cjson, &mut claude_pids);
    for (tool, pid, cwd) in crate::windows::process_rows(env, rp) {
        rows.push(json!({
            "target": name, "location": "remote", "tool": tool,
            "pid": pid.parse::<i64>().ok(), "status": "running",
            "kind": Value::Null, "cwd": cwd,
            "session_id": Value::Null, "name": Value::Null,
        }));
    }
    if rows.is_empty() {
        rows.push(stopped_row(name, "remote", "stopped"));
    }
    rows
}

fn ps_remote(name: &str, env: &Env, rp: &str) -> Vec<Value> {
    if env.is_windows() {
        #[cfg(feature = "windows")]
        {
            return ps_remote_windows(name, env, rp);
        }
        #[cfg(not(feature = "windows"))]
        {
            return vec![stopped_row(name, "remote", "unreachable")];
        }
    }
    ps_remote_unix(name, env, rp)
}

/// `dev agent ps --json` — every running agent across all projects, discovered
/// in parallel. Row schema:
/// `{target,location,tool,pid,status,kind,cwd,session_id,name}`.
pub fn ps(cfg: &Config) -> Vec<Value> {
    std::thread::scope(|s| {
        let locals: Vec<_> = cfg
            .local
            .iter()
            .map(|l| s.spawn(move || ps_local(&l.name, &l.path)))
            .collect();
        let remotes: Vec<_> = cfg
            .remote
            .iter()
            .map(|r| {
                s.spawn(move || match cfg.env(&r.env) {
                    Some(e) => ps_remote(&r.name, e, &r.path),
                    None => vec![stopped_row(&r.name, "remote", "unreachable")],
                })
            })
            .collect();
        // A panicking discovery worker degrades to an "error" row for that one
        // target rather than crashing all of `ps` (and the TUI that polls it).
        let mut out = Vec::new();
        for (h, l) in locals.into_iter().zip(&cfg.local) {
            match h.join() {
                Ok(rows) => out.extend(rows),
                Err(_) => out.push(stopped_row(&l.name, "local", "error")),
            }
        }
        for (h, r) in remotes.into_iter().zip(&cfg.remote) {
            match h.join() {
                Ok(rows) => out.extend(rows),
                Err(_) => out.push(stopped_row(&r.name, "remote", "error")),
            }
        }
        out
    })
}

/// Discover running agents for a single project (no fleet-wide fan-out) —
/// what `dev agent attach` needs to decide what to reconnect to.
pub fn ps_project(cfg: &Config, name: &str) -> Vec<Value> {
    match cfg.resolve(name) {
        Some(Target::Local { path, .. }) => ps_local(name, &path),
        Some(Target::Remote { env, path, .. }) => match cfg.env(&env) {
            Some(e) => ps_remote(name, e, &path),
            None => vec![stopped_row(name, "remote", "unreachable")],
        },
        _ => Vec::new(),
    }
}

// ── run registry (~/.dev/runs) & logs ───────────────────────────────────────

fn runs_dir_local() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".dev").join("runs"))
}

/// Newest `<id>.meta` (compact JSON) whose `project` == `project`, on the local
/// registry.
fn newest_meta_local(project: &str) -> Option<Value> {
    let dir = runs_dir_local()?;
    let mut metas: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(&dir)
        .ok()?
        .flatten()
        .filter(|e| e.path().extension().map(|x| x == "meta").unwrap_or(false))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    metas.sort_by_key(|(mtime, _)| std::cmp::Reverse(*mtime));
    for (_, p) in metas {
        if let Ok(v) = std::fs::read_to_string(&p).and_then(|t| {
            serde_json::from_str::<Value>(&t)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        }) {
            if v.get("project").and_then(|x| x.as_str()) == Some(project) {
                return Some(v);
            }
        }
    }
    None
}

/// Same, over SSH on a remote registry (mirrors the bash `_dev_run_meta`).
fn newest_meta_remote(env: &Env, project: &str) -> Option<Value> {
    let script = format!(
        "for p in $(ls -t \"$HOME/.dev/runs\"/*.meta 2>/dev/null); do \
           grep -q '\"project\":\"{project}\"' \"$p\" && {{ cat \"$p\"; break; }}; \
         done"
    );
    let out = ssh::exec_stdout(env, "", &script)?;
    serde_json::from_str(out.trim()).ok()
}

fn newest_meta(cfg: &Config, target: &Target) -> Option<Value> {
    match target {
        Target::Local { name, .. } => newest_meta_local(name),
        Target::Remote { name, env, .. } => cfg.env(env).and_then(|e| newest_meta_remote(e, name)),
        Target::Env { .. } => None,
    }
}

/// Public path to the local run registry dir (`~/.dev/runs`).
pub fn runs_dir_local_path() -> Option<PathBuf> {
    runs_dir_local()
}

// ── run listing / resolution / prune ────────────────────────────────────────

/// Parse `<tool>-<project>-<epoch>` → project name.
fn project_from_run_id(id: &str) -> Option<String> {
    let rest = BACKENDS
        .iter()
        .find_map(|t| id.strip_prefix(&format!("{}-", t.name)))?;
    let dash = rest.rfind('-')?;
    let (proj, ep) = (&rest[..dash], &rest[dash + 1..]);
    if !proj.is_empty() && !ep.is_empty() && ep.chars().all(|c| c.is_ascii_digit()) {
        Some(proj.to_string())
    } else {
        None
    }
}

fn epoch_from_run_id(id: &str) -> Option<u64> {
    id.rsplit('-').next().and_then(|s| s.parse().ok())
}

fn read_local_metas() -> Vec<Value> {
    let Some(dir) = runs_dir_local() else {
        return Vec::new();
    };
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut v: Vec<(std::time::SystemTime, Value)> = rd
        .flatten()
        .filter(|e| e.path().extension().map(|x| x == "meta").unwrap_or(false))
        .filter_map(|e| {
            let mtime = e.metadata().ok()?.modified().ok()?;
            let val =
                serde_json::from_str::<Value>(&std::fs::read_to_string(e.path()).ok()?).ok()?;
            Some((mtime, val))
        })
        .collect();
    v.sort_by_key(|(m, _)| std::cmp::Reverse(*m));
    v.into_iter().map(|(_, val)| val).collect()
}

fn read_remote_metas(env: &Env) -> Vec<Value> {
    let script =
        "for f in $(ls -t \"$HOME/.dev/runs\"/*.meta 2>/dev/null); do cat \"$f\"; echo; done";
    ssh::exec_stdout(env, "", script)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l.trim()).ok())
        .collect()
}

fn read_metas(cfg: &Config, target: &Target) -> Vec<Value> {
    match target {
        Target::Local { .. } => read_local_metas(),
        Target::Remote { env, .. } => cfg.env(env).map(read_remote_metas).unwrap_or_default(),
        Target::Env { .. } => Vec::new(),
    }
}

/// Is numeric `pid` alive on the target?
fn pid_alive(cfg: &Config, target: &Target, pid: &str) -> bool {
    if pid.is_empty() || !pid.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    match target {
        Target::Local { .. } => Command::new("kill")
            .args(["-0", pid])
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false),
        Target::Remote { env, .. } => cfg
            .env(env)
            .and_then(|e| ssh::exec_capture(e, "", &format!("kill -0 {pid} 2>/dev/null")).ok())
            .map(|o| o.status.success())
            .unwrap_or(false),
        Target::Env { .. } => false,
    }
}

/// Resolve a project name OR a run id → (target, meta?). A project resolves to
/// its newest run; a run id resolves to that exact run. This is what unifies
/// `logs`/`kill`/`attach` so they accept either.
pub fn resolve_run(cfg: &Config, reference: &str) -> Option<(Target, Option<Value>)> {
    if let Some(t) = cfg.resolve(reference) {
        let meta = newest_meta(cfg, &t);
        return Some((t, meta));
    }
    let project = project_from_run_id(reference)?;
    let t = cfg.resolve(&project)?;
    let meta = read_metas(cfg, &t)
        .into_iter()
        .find(|m| m.get("id").and_then(|v| v.as_str()) == Some(reference));
    Some((t, meta))
}

/// `dev agent runs` — recorded runs (optionally one project), newest first, each
/// enriched with a derived `state` (running | exited).
pub fn list_runs(cfg: &Config, project: Option<&str>) -> Vec<Value> {
    let metas: Vec<Value> = match project {
        Some(p) => match cfg.resolve(p) {
            Some(t) => read_metas(cfg, &t)
                .into_iter()
                .filter(|m| m.get("project").and_then(|v| v.as_str()) == Some(p))
                .collect(),
            None => Vec::new(),
        },
        None => {
            let mut all = read_local_metas();
            let mut seen = std::collections::HashSet::new();
            for r in &cfg.remote {
                if seen.insert(r.env.clone()) {
                    if let Some(e) = cfg.env(&r.env) {
                        all.extend(read_remote_metas(e));
                    }
                }
            }
            all
        }
    };
    metas
        .into_iter()
        .map(|mut m| {
            let proj = m
                .get("project")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let pid = m
                .get("pid")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let alive = cfg
                .resolve(&proj)
                .map(|t| pid_alive(cfg, &t, &pid))
                .unwrap_or(false);
            if let Value::Object(o) = &mut m {
                o.insert(
                    "state".into(),
                    Value::String(if alive { "running" } else { "exited" }.into()),
                );
            }
            m
        })
        .collect()
}

/// Remove run metas (and their logs). `keep_alive` skips runs whose process is
/// still alive; `older_days` skips recent runs. Returns the count removed.
pub fn prune(
    cfg: &Config,
    project: Option<&str>,
    keep_alive: bool,
    older_days: Option<u64>,
) -> usize {
    let cutoff = older_days.map(|d| epoch().saturating_sub(d * 86_400));
    let mut registries: Vec<(Target, Vec<Value>)> = Vec::new();
    match project {
        Some(p) => {
            if let Some(t) = cfg.resolve(p) {
                let metas = read_metas(cfg, &t)
                    .into_iter()
                    .filter(|m| m.get("project").and_then(|v| v.as_str()) == Some(p))
                    .collect();
                registries.push((t, metas));
            }
        }
        None => {
            if let Some(l) = cfg.local.first() {
                if let Some(t) = cfg.resolve(&l.name) {
                    registries.push((t, read_local_metas()));
                }
            }
            let mut seen = std::collections::HashSet::new();
            for r in &cfg.remote {
                if seen.insert(r.env.clone()) {
                    if let Some(t) = cfg.resolve(&r.name) {
                        let metas = read_metas(cfg, &t);
                        registries.push((t, metas));
                    }
                }
            }
        }
    }

    let mut removed = 0usize;
    for (target, metas) in &registries {
        for m in metas {
            let id = m.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() {
                continue;
            }
            let pid = m.get("pid").and_then(|v| v.as_str()).unwrap_or("");
            if keep_alive && pid_alive(cfg, target, pid) {
                continue;
            }
            if let (Some(cut), Some(ep)) = (cutoff, epoch_from_run_id(id)) {
                if ep > cut {
                    continue; // too recent
                }
            }
            let log = m.get("log").and_then(|v| v.as_str()).unwrap_or("");
            if remove_run_files(cfg, target, id, log) {
                removed += 1;
            }
        }
    }
    removed
}

fn remove_run_files(cfg: &Config, target: &Target, id: &str, log: &str) -> bool {
    match target {
        Target::Local { .. } => {
            let Some(dir) = runs_dir_local() else {
                return false;
            };
            let _ = std::fs::remove_file(dir.join(format!("{id}.meta")));
            if !log.is_empty() {
                let _ = std::fs::remove_file(dir.join(log));
            }
            true
        }
        Target::Remote { env, .. } => cfg
            .env(env)
            .and_then(|e| {
                ssh::exec_capture(
                    e,
                    "",
                    &format!("rm -f \"$HOME/.dev/runs/{id}.meta\" \"$HOME/.dev/runs/{log}\" 2>/dev/null; true"),
                )
                .ok()
            })
            .map(|o| o.status.success())
            .unwrap_or(false),
        Target::Env { .. } => false,
    }
}

// ── dispatch / kill ─────────────────────────────────────────────────────────

fn epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Run `cmd` at `cwd` on the target (local `bash -c` / remote ssh), returning
/// stdout. Mirrors the bash `_dev_run_at`.
fn run_at(cfg: &Config, target: &Target, cwd: &str, cmd: &str) -> Option<String> {
    match target {
        Target::Local { .. } => {
            let full = format!("cd {} && {}", ssh::sh_quote(cwd), cmd);
            let out = Command::new("bash")
                .arg("-c")
                .arg(&full)
                .stdin(Stdio::null())
                .output()
                .ok()?;
            Some(String::from_utf8_lossy(&out.stdout).into_owned())
        }
        Target::Remote { env, .. } => ssh::exec_stdout(cfg.env(env)?, cwd, cmd),
        Target::Env { .. } => None,
    }
}

/// Write `content` to `$HOME/.dev/runs/<filename>` on the target.
fn put_run(cfg: &Config, target: &Target, filename: &str, content: &str) -> Option<()> {
    match target {
        Target::Local { .. } => {
            let dir = runs_dir_local()?;
            std::fs::create_dir_all(&dir).ok()?;
            std::fs::write(dir.join(filename), content).ok()?;
            Some(())
        }
        Target::Remote { env, .. } => {
            let e = cfg.env(env)?;
            let cmd =
                format!("mkdir -p \"$HOME/.dev/runs\" && cat > \"$HOME/.dev/runs/{filename}\"");
            let out = ssh::exec_with_stdin(e, "", &cmd, content.as_bytes()).ok()?;
            out.status.success().then_some(())
        }
        Target::Env { .. } => None,
    }
}

/// Ensure a sibling `.dev-worktrees/<repo>-<branch>` worktree exists; echo path.
fn worktree_ensure(cfg: &Config, target: &Target, base: &str, branch: &str) -> String {
    let p = std::path::Path::new(base);
    let repo = p
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let parent = p
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let san: String = branch
        .chars()
        .map(|c| if c == '/' || c == ' ' { '-' } else { c })
        .collect();
    let wt = format!("{parent}/.dev-worktrees/{repo}-{san}");
    let cmd = format!(
        "if [ -d {wt} ]; then :; else \
           git -C {base} worktree add -b {br} {wt} 2>/dev/null || \
           git -C {base} worktree add {wt} {br} 2>/dev/null; fi",
        wt = ssh::sh_quote(&wt),
        base = ssh::sh_quote(base),
        br = ssh::sh_quote(branch),
    );
    let _ = run_at(cfg, target, base, &cmd);
    wt
}

/// Options for [`dispatch`].
#[derive(Default)]
pub struct DispatchOpts<'a> {
    pub backend: &'a str,
    pub task: &'a str,
    pub model: Option<&'a str>,
    pub effort: Option<&'a str>,
    pub sandbox: Option<&'a str>,
    pub worktree: Option<&'a str>,
    /// Supervised dispatch: create a task in the local store, inject the
    /// ask/handoff protocol into the prompt, and link the run back to the task —
    /// so the agent can raise blocking questions into the human's Inbox. Only
    /// meaningful for local targets (the task store lives on the control machine);
    /// silently skipped for remote targets.
    pub supervise: bool,
}

/// Wrap a user task with the supervision protocol so a dispatched agent can raise
/// blocking questions (→ Inbox) and leave a handoff, all keyed to `task_id`.
fn supervised_prompt(task_id: &str, project: &str, user_task: &str) -> String {
    format!(
        "You are working on dev task {task_id} (project {project}).\n\n\
         ── Task ──\n{user_task}\n\n\
         ── Protocol ──\n\
         - If behavior/scope/compatibility/API/UX is ambiguous, or you are blocked, run:\n\
             dev task ask {task_id} \"<question>\" --severity blocking\n\
           then STOP and wait — the answer arrives in the human's Inbox.\n\
         - Read shared context any time: dev task context {task_id} --markdown\n\
         - When finished, write a handoff:  dev task write-handoff {task_id}\n\
           (changed files, tests run + results, risks, follow-up)."
    )
}

/// Launch a background agent and record it in `~/.dev/runs`. Returns
/// `{id,target,tool,pid,session,branch,worktree,ok}`. Port of `_dev_dispatch`.
pub fn dispatch(cfg: &Config, project: &str, opts: &DispatchOpts) -> anyhow::Result<Value> {
    let target = cfg
        .resolve(project)
        .ok_or_else(|| anyhow::anyhow!("unknown project '{project}'"))?;
    let base = match &target {
        Target::Local { path, .. } | Target::Remote { path, .. } => path.clone(),
        Target::Env { .. } => anyhow::bail!("cannot dispatch to an env"),
    };
    // Preflight: the tool must exist on the target, else nohup writes a dead run.
    let avail = run_at(
        cfg,
        &target,
        &base,
        &format!("command -v {} >/dev/null 2>&1 && echo ok", opts.backend),
    );
    if avail.as_deref().map(str::trim) != Some("ok") {
        anyhow::bail!("backend '{}' not found on {}", opts.backend, project);
    }
    let (mut cwd, mut branch, mut wt) = (base.clone(), String::new(), String::new());
    if let Some(b) = opts.worktree {
        branch = b.to_string();
        wt = worktree_ensure(cfg, &target, &base, b);
        cwd = wt.clone();
    }
    let id = format!("{}-{}-{}", opts.backend, project, epoch());
    // Supervised dispatch (local only — the task store lives on the control
    // machine): create a task so the agent can raise blocking questions into the
    // Inbox, and wrap the prompt with the ask/handoff protocol.
    let is_local = matches!(target, Target::Local { .. });
    let supervise = opts.supervise && is_local;
    let supervise_skipped = opts.supervise && !is_local;
    let mut task_id = String::new();
    let effective_task = if supervise {
        let title: String = opts
            .task
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .unwrap_or("dispatched task")
            .chars()
            .take(80)
            .collect();
        match crate::store::task_new(project, &title, Some(opts.task), None) {
            Ok(t) => {
                task_id = t.id;
                supervised_prompt(&task_id, project, opts.task)
            }
            Err(_) => opts.task.to_string(),
        }
    } else {
        opts.task.to_string()
    };
    let qtask = ssh::sh_quote(&effective_task);
    let (mut pid, mut session) = (String::new(), String::new());

    if opts.backend == "claude" {
        let mut flags = String::new();
        if let Some(m) = opts.model {
            flags.push_str(&format!(" --model {}", ssh::sh_quote(m)));
        }
        if let Some(e) = opts.effort {
            flags.push_str(&format!(" --effort {}", ssh::sh_quote(e)));
        }
        let cmd = format!(
            "mkdir -p \"$HOME/.dev/runs\"; claude --bg{flags} -p {qtask} >/dev/null 2>&1 </dev/null || true"
        );
        let _ = run_at(cfg, &target, &cwd, &cmd);
        let cj = run_at(
            cfg,
            &target,
            &cwd,
            &format!(
                "claude agents --json --cwd {} 2>/dev/null",
                ssh::sh_quote(&cwd)
            ),
        )
        .unwrap_or_default();
        if let Ok(Value::Array(mut arr)) = serde_json::from_str::<Value>(cj.trim()) {
            arr.sort_by_key(|a| {
                a.get("startedAt")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string()
            });
            if let Some(a) = arr.last() {
                session = a
                    .get("sessionId")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                pid = a.get("pid").map(pid_to_string).unwrap_or_default();
            }
        }
    } else {
        let spec = backend(opts.backend)
            .ok_or_else(|| anyhow::anyhow!("unknown backend '{}'", opts.backend))?;
        if spec.bg_sub.is_empty() {
            anyhow::bail!("'{}' is not background-dispatchable", opts.backend);
        }
        let mut extra = String::new();
        match opts.backend {
            "codex" => {
                if let Some(m) = opts.model {
                    extra.push_str(&format!(" --model {}", ssh::sh_quote(m)));
                }
                if let Some(s) = opts.sandbox {
                    extra.push_str(&format!(" --sandbox {}", ssh::sh_quote(s)));
                }
                // `codex exec` has no `--ask-for-approval` flag (that's a
                // top-level interactive-only option); set the policy via a
                // config override instead — `never` is the recommended policy
                // for non-interactive runs.
                extra.push_str(" -c approval_policy=never");
            }
            "opencode" => {
                if let Some(m) = opts.model {
                    extra.push_str(&format!(" --model {}", ssh::sh_quote(m)));
                }
                // Keep opencode's default *formatted* output — NOT `--format
                // json`. The run log is what `dev logs` / the TUI display, and
                // nothing decodes opencode's JSON event stream, so raw events
                // are just an unreadable wall. (If a structured transcript view
                // is ever added, re-add `--format json` and decode it there.)
            }
            "agy" => {
                // antigravity: `agy -p <task>` non-interactive; `--model` selects
                // the session model (see `agy models`).
                if let Some(m) = opts.model {
                    extra.push_str(&format!(" --model {}", ssh::sh_quote(m)));
                }
            }
            _ => {}
        }
        let cmd = format!(
            "mkdir -p \"$HOME/.dev/runs\"; \
             nohup {tool} {sub}{extra} {qtask} >\"$HOME/.dev/runs/{id}.log\" 2>&1 </dev/null & \
             echo $!; disown 2>/dev/null; true",
            tool = opts.backend,
            sub = spec.bg_sub,
        );
        pid = run_at(cfg, &target, &cwd, &cmd)
            .unwrap_or_default()
            .trim()
            .to_string();
    }

    let meta = json!({
        "id": id, "tool": opts.backend, "project": project, "branch": branch,
        "worktree": wt, "cwd": cwd, "task": opts.task, "pid": pid,
        "session": session, "log": format!("{id}.log"), "started": crate::store::now_iso(),
    });
    put_run(
        cfg,
        &target,
        &format!("{id}.meta"),
        &serde_json::to_string(&meta)?,
    );
    // Bind the run to its supervised task (dedupes the board's bare-run card and
    // lets follow-up/attach find it). Recorded after the run meta exists.
    if !task_id.is_empty() {
        if let Some(tdir) = crate::store::find_task_dir(&task_id) {
            let _ = crate::store::task_set_link(&tdir, "run_id", json!(id));
            let _ = crate::store::task_update_field(&tdir, "assigned_tool", json!(opts.backend));
            if !branch.is_empty() {
                let _ = crate::store::task_update_field(&tdir, "worktree_branch", json!(branch));
            }
            let _ =
                crate::store::task_phase_set(&tdir, "implementing", "dev", "supervised dispatch");
        }
    }
    Ok(json!({
        "id": id, "target": project, "tool": opts.backend, "pid": pid,
        "session": session, "branch": branch, "worktree": wt,
        "task_id": task_id, "supervise_skipped": supervise_skipped, "ok": true,
    }))
}

fn pid_to_string(v: &Value) -> String {
    match v {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        _ => String::new(),
    }
}

fn kill_pids(cfg: &Config, target: &Target, pids: &[String]) {
    if pids.is_empty() {
        return;
    }
    let cmd = format!("kill {} 2>/dev/null; true", pids.join(" "));
    match target {
        Target::Local { .. } => {
            let _ = Command::new("bash").arg("-c").arg(&cmd).status();
        }
        Target::Remote { env, .. } => {
            if let Some(e) = cfg.env(env) {
                let _ = ssh::exec_capture(e, "", &cmd);
            }
        }
        Target::Env { .. } => {}
    }
}

/// TERM agents. A **project** kills *all* its live agents (by pid, every tool);
/// a **run id** kills only that run's process.
pub fn kill(cfg: &Config, reference: &str) -> Value {
    // Run id → that specific run.
    if project_from_run_id(reference).is_some() {
        if let Some((target, Some(meta))) = resolve_run(cfg, reference) {
            let pid = meta.get("pid").and_then(|v| v.as_str()).unwrap_or("");
            let mut killed = Vec::new();
            if pid_alive(cfg, &target, pid) {
                kill_pids(cfg, &target, std::slice::from_ref(&pid.to_string()));
                killed.push(pid.to_string());
            }
            return json!({"target": reference, "killed": killed, "ok": true});
        }
    }
    // Project → every live agent on it (from ps), by pid.
    let Some(target) = cfg.resolve(reference) else {
        return json!({"target": reference, "ok": false, "error": "unknown target"});
    };
    let pids: Vec<String> = ps_project(cfg, reference)
        .iter()
        .filter_map(|r| r.get("pid").and_then(|v| v.as_i64()).map(|p| p.to_string()))
        .collect();
    kill_pids(cfg, &target, &pids);
    json!({"target": reference, "killed": pids, "ok": true})
}

/// The newest run meta (compact JSON) for `reference`, if any.
pub fn newest_run(cfg: &Config, reference: &str) -> Option<Value> {
    resolve_run(cfg, reference).and_then(|(_, m)| m)
}

/// The `.log` filename recorded in the resolved run (project newest, or a run
/// id), if any.
pub fn run_log_name(cfg: &Config, reference: &str) -> Option<String> {
    let (_, meta) = resolve_run(cfg, reference)?;
    meta?
        .get("log")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Tail (last 200 lines) of the resolved run's log (project newest, or a run id).
/// claude logs to its own store, so it returns a hint line instead.
pub fn logs_snapshot(cfg: &Config, reference: &str) -> Vec<String> {
    log_lines(cfg, reference, Some(200))
}

/// Lines of the resolved run's log. `tail = Some(n)` returns only the last `n`
/// lines (the scrolling log view); `None` returns the whole log (`dev agent
/// output --full`). claude keeps no `~/.dev/runs/*.log`, so it returns a hint.
fn log_lines(cfg: &Config, reference: &str, tail: Option<usize>) -> Vec<String> {
    let Some((target, Some(meta))) = resolve_run(cfg, reference) else {
        return Vec::new();
    };
    let tool = meta.get("tool").and_then(|v| v.as_str()).unwrap_or("");
    if tool == "claude" {
        return vec!["claude logs to its own store — use dev attach".to_string()];
    }
    let log = meta.get("log").and_then(|v| v.as_str()).unwrap_or("");
    if log.is_empty() {
        return Vec::new();
    }
    match &target {
        Target::Local { .. } => {
            let Some(dir) = runs_dir_local() else {
                return Vec::new();
            };
            std::fs::read_to_string(dir.join(log))
                .map(|s| {
                    let all: Vec<&str> = s.lines().collect();
                    let start = tail.map(|n| all.len().saturating_sub(n)).unwrap_or(0);
                    all[start..].iter().map(|s| s.to_string()).collect()
                })
                .unwrap_or_default()
        }
        Target::Remote { env, .. } => {
            let cmd = match tail {
                Some(n) => format!("tail -n {n} \"$HOME/.dev/runs/{log}\" 2>/dev/null"),
                None => format!("cat \"$HOME/.dev/runs/{log}\" 2>/dev/null"),
            };
            cfg.env(env)
                .and_then(|e| ssh::exec_stdout(e, "", &cmd))
                .map(|s| s.lines().map(String::from).collect())
                .unwrap_or_default()
        }
        Target::Env { .. } => Vec::new(),
    }
}

/// The agent's *result* for `reference` — the extraction primitive behind `dev
/// agent output`, richer than the scrolling log. For claude it's the last
/// assistant prose turn from the transcript (what the agent concluded); for other
/// backends it's the run log (`full` ⇒ whole log, else the 200-line tail). This
/// is what makes a dispatched run's outcome retrievable as text/JSON instead of
/// only through `dev agent logs`.
pub fn final_output(cfg: &Config, reference: &str, full: bool) -> Vec<String> {
    // Accept a project name OR a run id (like logs/kill/attach). Using the
    // name-only `cfg.resolve` here meant `dev agent output <run-id>` returned
    // nothing and followup-by-run-id lost its context seed.
    let Some((target, meta)) = resolve_run(cfg, reference) else {
        return Vec::new();
    };
    let tool = meta
        .as_ref()
        .and_then(|m| m.get("tool"))
        .and_then(|v| v.as_str());
    // claude → last assistant message from the transcript.
    let claude_run = tool == Some("claude") || (tool.is_none() && meta.is_none());
    if claude_run {
        let sess = meta
            .as_ref()
            .and_then(|m| m.get("session"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from)
            .or_else(|| live_claude_session(cfg, reference));
        if let Some(s) = sess.filter(|s| is_session_id(s)) {
            let raw = claude_transcript_raw(cfg, &target, &s);
            let msg = last_assistant_message(&raw);
            if !msg.is_empty() {
                return msg;
            }
        }
    }
    if tool == Some("codex")
        || (tool.is_none()
            && live_tool(cfg, target_name(&target).unwrap_or(reference)).as_deref()
                == Some("codex"))
    {
        if tool == Some("codex") {
            let lines = log_lines(cfg, reference, (!full).then_some(200));
            if !lines.is_empty() {
                return lines;
            }
        }
        let raw = codex_transcript_raw(cfg, &target);
        let msg = last_codex_assistant_message(&raw);
        if !msg.is_empty() {
            return msg;
        }
    }
    // non-claude, or claude with no transcript yet: the run log.
    log_lines(cfg, reference, (!full).then_some(200))
}

/// Re-dispatch a follow-up agent for `reference`, seeded with the previous run's
/// result plus a new `text` instruction, on the same project / backend / worktree.
/// This is the non-interactive "continue from the output" primitive (interactive
/// continuation is `dev agent attach`). Returns the new [`dispatch`] receipt.
pub fn followup(cfg: &Config, reference: &str, text: &str) -> anyhow::Result<Value> {
    let (target, meta) = resolve_run(cfg, reference)
        .ok_or_else(|| anyhow::anyhow!("no run found for '{reference}'"))?;
    let project = match &target {
        Target::Local { name, .. } | Target::Remote { name, .. } => name.clone(),
        Target::Env { .. } => anyhow::bail!("cannot follow up on an env"),
    };
    let tool = meta
        .as_ref()
        .and_then(|m| m.get("tool"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(|| default_backend(cfg, &project, meta.as_ref()));
    // Continue in the same worktree branch if the prior run used one.
    let worktree = meta
        .as_ref()
        .and_then(|m| m.get("branch"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);
    // Seed continuity with the previous result (bounded so the prompt stays sane).
    let prior: String = final_output(cfg, reference, false)
        .join("\n")
        .chars()
        .take(4000)
        .collect();
    let prompt = if prior.trim().is_empty() {
        format!("Continue the previous task on project {project}.\n\n── New instruction ──\n{text}")
    } else {
        format!(
            "You are continuing a previous {tool} agent run on project {project}.\n\n\
             ── Previous result ──\n{prior}\n\n── New instruction ──\n{text}"
        )
    };
    let opts = DispatchOpts {
        backend: &tool,
        task: &prompt,
        model: None,
        effort: None,
        sandbox: None,
        worktree: worktree.as_deref(),
        supervise: false,
    };
    dispatch(cfg, &project, &opts)
}

// ── claude transcript activity ───────────────────────────────────────────────
//
// claude doesn't write to `~/.dev/runs/*.log` — it keeps a rich JSONL transcript
// at `~/.claude/projects/<escaped-cwd>/<session>.jsonl`. `logs_snapshot` used to
// give up on claude ("logs to its own store"); instead we distill that transcript
// into a readable activity tail (assistant prose / `⚙ tool` calls / `↳ results`)
// so the TUI detail pane and `dev agent logs` show what claude is *doing*. This
// is the claude-squad idea (surface the agent's live output) adapted to claude's
// on-disk transcript instead of a captured terminal pane.

/// Recent, human-readable activity for `reference` — what `dev agent logs` and
/// the TUI detail pane render. claude → distilled transcript; every other backend
/// → the dispatched run-log tail (unchanged [`logs_snapshot`]). `session` pins a
/// specific claude session (the TUI passes the one it already found via `ps`, so
/// no rediscovery is needed); when `None` we infer it from the newest recorded
/// run, then from a live `ps` lookup.
pub fn recent_activity(cfg: &Config, reference: &str, session: Option<&str>) -> Vec<String> {
    let Some((target, meta)) = resolve_run(cfg, reference) else {
        return Vec::new();
    };
    let project = target_name(&target).unwrap_or(reference);
    // Fast path: the caller already knows the claude session (TUI hot loop).
    if let Some(s) = session {
        return claude_activity(cfg, &target, s);
    }
    // Codex interactive sessions do not have a ~/.dev/runs log. Read Codex's
    // own session transcript when the selected live process is codex, or when
    // the resolved run meta says this is a codex run.
    let meta_tool = meta
        .as_ref()
        .and_then(|m| m.get("tool"))
        .and_then(|v| v.as_str());
    if meta_tool == Some("codex")
        || (meta.is_none() && live_tool(cfg, project).as_deref() == Some("codex"))
    {
        if meta.is_some() {
            let lines = logs_snapshot(cfg, reference);
            if !lines.is_empty() {
                return lines;
            }
        }
        let activity = codex_activity(cfg, &target);
        if !activity.is_empty() {
            return activity;
        }
        return logs_snapshot(cfg, reference);
    }
    if meta.is_some() && meta_tool != Some("claude") {
        // A non-claude run is the newest thing here — show its log.
        return logs_snapshot(cfg, reference);
    }
    // claude run, or nothing recorded: find the session and read the transcript.
    let sess = meta
        .as_ref()
        .and_then(|m| m.get("session"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .or_else(|| live_claude_session(cfg, project));
    match sess {
        Some(s) => claude_activity(cfg, &target, &s),
        None => logs_snapshot(cfg, reference),
    }
}

/// Recent activity for a specific selected backend on a project. The TUI uses
/// this when the user highlights an agent row; otherwise a project-level lookup
/// can accidentally show another backend's newest run for the same project.
pub fn recent_activity_for_tool(
    cfg: &Config,
    reference: &str,
    tool: &str,
    session: Option<&str>,
) -> Vec<String> {
    if tool.is_empty() || tool == "-" {
        return recent_activity(cfg, reference, session);
    }
    let Some((target, meta)) = resolve_run(cfg, reference) else {
        return Vec::new();
    };
    if tool == "claude" {
        if let Some(s) = session {
            return claude_activity(cfg, &target, s);
        }
        let project = target_name(&target).unwrap_or(reference);
        let sess = meta
            .as_ref()
            .filter(|m| m.get("tool").and_then(|v| v.as_str()) == Some("claude"))
            .and_then(|m| m.get("session"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from)
            .or_else(|| live_claude_session(cfg, project));
        return sess
            .map(|s| claude_activity(cfg, &target, &s))
            .unwrap_or_default();
    }
    if tool == "codex" {
        if meta
            .as_ref()
            .and_then(|m| m.get("tool"))
            .and_then(|v| v.as_str())
            == Some("codex")
        {
            let lines = logs_snapshot(cfg, reference);
            if !lines.is_empty() {
                return lines;
            }
        }
        return codex_activity(cfg, &target);
    }
    if meta
        .as_ref()
        .and_then(|m| m.get("tool"))
        .and_then(|v| v.as_str())
        == Some(tool)
    {
        return logs_snapshot(cfg, reference);
    }
    Vec::new()
}

fn target_name(target: &Target) -> Option<&str> {
    match target {
        Target::Local { name, .. } | Target::Remote { name, .. } => Some(name),
        Target::Env { .. } => None,
    }
}

fn target_cwd(target: &Target) -> Option<&str> {
    match target {
        Target::Local { path, .. } | Target::Remote { path, .. } => Some(path),
        Target::Env { .. } => None,
    }
}

fn live_tool(cfg: &Config, project: &str) -> Option<String> {
    ps_project(cfg, project).into_iter().find_map(|r| {
        let has_pid = r.get("pid").map(|v| !v.is_null()).unwrap_or(false);
        has_pid
            .then(|| {
                r.get("tool")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(String::from)
            })
            .flatten()
    })
}

/// First live claude session on `reference`. Interactive claude records no run
/// meta, so its session is only discoverable from `ps` — the CLI needs this; the
/// TUI already carries the session from its own `ps` poll.
fn live_claude_session(cfg: &Config, reference: &str) -> Option<String> {
    ps_project(cfg, reference).into_iter().find_map(|r| {
        if r.get("tool").and_then(|v| v.as_str()) != Some("claude") {
            return None;
        }
        r.get("session_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from)
    })
}

/// Distilled activity for a claude `session` on `target` (local fs / remote ssh).
fn claude_activity(cfg: &Config, target: &Target, session: &str) -> Vec<String> {
    if !is_session_id(session) {
        return Vec::new();
    }
    let raw = claude_transcript_raw(cfg, target, session);
    if raw.is_empty() {
        if let Target::Local { .. } = target {
            return vec!["(no claude transcript yet — attach to view)".to_string()];
        }
    }
    let lines = distill_transcript(&raw);
    if lines.is_empty() {
        vec!["(claude session has no readable activity yet)".to_string()]
    } else {
        lines
    }
}

/// Raw transcript JSONL tail for a claude `session` on `target`. Empty string if
/// none is found yet. Shared by [`claude_activity`] (distilled view) and
/// [`final_output`] (last-message extraction). `session` is validated UUID-ish so
/// it's safe to interpolate into the ssh glob.
fn claude_transcript_raw(cfg: &Config, target: &Target, session: &str) -> String {
    if !is_session_id(session) {
        return String::new();
    }
    match target {
        Target::Local { .. } => match claude_transcript_local(session) {
            Some(p) => read_tail_bytes(&p, 512 * 1024),
            None => String::new(),
        },
        Target::Remote { env, .. } => {
            let Some(e) = cfg.env(env) else {
                return String::new();
            };
            let script = format!(
                "f=$(ls -t \"$HOME/.claude/projects\"/*/{session}.jsonl 2>/dev/null | head -1); \
                 [ -n \"$f\" ] && tail -n 400 \"$f\""
            );
            ssh::exec_stdout(e, "", &script).unwrap_or_default()
        }
        Target::Env { .. } => String::new(),
    }
}

/// Newest local `~/.claude/projects/*/<session>.jsonl`. Globbing by the unique
/// session id sidesteps claude's cwd path-escaping (and worktree subdirs).
fn claude_transcript_local(session: &str) -> Option<PathBuf> {
    let root = PathBuf::from(std::env::var("HOME").ok()?)
        .join(".claude")
        .join("projects");
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for proj in std::fs::read_dir(&root).ok()?.flatten() {
        let p = proj.path().join(format!("{session}.jsonl"));
        if let Ok(mt) = std::fs::metadata(&p).and_then(|m| m.modified()) {
            if best.as_ref().map(|(t, _)| mt > *t).unwrap_or(true) {
                best = Some((mt, p));
            }
        }
    }
    best.map(|(_, p)| p)
}

// ── codex transcript activity ───────────────────────────────────────────────

fn codex_activity(cfg: &Config, target: &Target) -> Vec<String> {
    let raw = codex_transcript_raw(cfg, target);
    if raw.is_empty() {
        return Vec::new();
    }
    distill_codex_transcript(&raw)
}

fn codex_transcript_raw(cfg: &Config, target: &Target) -> String {
    let Some(cwd) = target_cwd(target) else {
        return String::new();
    };
    match target {
        Target::Local { .. } => match codex_transcript_local(cwd) {
            Some(p) => read_tail_bytes(&p, 512 * 1024),
            None => String::new(),
        },
        Target::Remote { env, .. } => {
            let Some(e) = cfg.env(env) else {
                return String::new();
            };
            let needle = json_cwd_fragment(cwd);
            let script = format!(
                "needle={needle}; \
                 f=$(find \"$HOME/.codex/sessions\" \"$HOME/.codex/archived_sessions\" \
                      -type f -name 'rollout-*.jsonl' 2>/dev/null | \
                    while IFS= read -r p; do \
                      head -20 \"$p\" 2>/dev/null | grep -F \"$needle\" >/dev/null || continue; \
                      mt=$(stat -c %Y \"$p\" 2>/dev/null || stat -f %m \"$p\" 2>/dev/null || echo 0); \
                      printf '%s\t%s\n' \"$mt\" \"$p\"; \
                    done | sort -nr | head -1 | cut -f2-); \
                 [ -n \"$f\" ] && tail -n 400 \"$f\"",
                needle = ssh::sh_quote(&needle),
            );
            ssh::exec_stdout(e, "", &script).unwrap_or_default()
        }
        Target::Env { .. } => String::new(),
    }
}

fn codex_transcript_local(cwd: &str) -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let roots = [
        PathBuf::from(&home).join(".codex").join("sessions"),
        PathBuf::from(&home)
            .join(".codex")
            .join("archived_sessions"),
    ];
    let mut files = Vec::new();
    for root in roots {
        collect_codex_session_files(&root, &mut files);
    }
    files.sort_by_key(|(mtime, _)| std::cmp::Reverse(*mtime));
    files
        .into_iter()
        .map(|(_, p)| p)
        .find(|p| codex_session_matches_cwd(p, cwd))
}

fn collect_codex_session_files(
    dir: &std::path::Path,
    out: &mut Vec<(std::time::SystemTime, PathBuf)>,
) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect_codex_session_files(&p, out);
            continue;
        }
        let is_rollout = p
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.starts_with("rollout-") && s.ends_with(".jsonl"))
            .unwrap_or(false);
        if !is_rollout {
            continue;
        }
        if let Ok(mtime) = entry.metadata().and_then(|m| m.modified()) {
            out.push((mtime, p));
        }
    }
}

fn codex_session_matches_cwd(path: &std::path::Path, cwd: &str) -> bool {
    let Ok(text) = read_first_bytes(path, 128 * 1024) else {
        return false;
    };
    text.lines().take(20).any(|line| {
        serde_json::from_str::<Value>(line)
            .ok()
            .and_then(|v| {
                v.pointer("/payload/cwd")
                    .and_then(|x| x.as_str())
                    .map(str::to_string)
            })
            .as_deref()
            == Some(cwd)
    })
}

fn json_cwd_fragment(cwd: &str) -> String {
    let encoded = serde_json::to_string(cwd).unwrap_or_else(|_| "\"\"".to_string());
    format!("\"cwd\":{encoded}")
}

/// Read the last `cap` bytes of a file as UTF-8, dropping a leading partial line.
/// Transcripts grow to many MB over a session; only the tail is interesting.
fn read_tail_bytes(path: &std::path::Path, cap: u64) -> String {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut f) = std::fs::File::open(path) else {
        return String::new();
    };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let start = len.saturating_sub(cap);
    if start > 0 && f.seek(SeekFrom::Start(start)).is_err() {
        return String::new();
    }
    let mut buf = Vec::new();
    if f.read_to_end(&mut buf).is_err() {
        return String::new();
    }
    let mut s = String::from_utf8_lossy(&buf).into_owned();
    if start > 0 {
        if let Some(nl) = s.find('\n') {
            s.drain(..=nl);
        }
    }
    s
}

fn read_first_bytes(path: &std::path::Path, cap: usize) -> std::io::Result<String> {
    use std::io::Read;
    let mut f = std::fs::File::open(path)?;
    let mut buf = Vec::new();
    f.by_ref().take(cap as u64).read_to_end(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// A plausible claude session id (UUID-ish) — guards ssh/path interpolation.
fn is_session_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Distill raw transcript JSONL (one event per line) into activity lines: keep
/// assistant prose, `⚙ tool` invocations, and short `↳ result` echoes; drop
/// thinking, sidechains, and bookkeeping event types. Newest-last, capped.
pub fn distill_transcript(raw: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if v.get("isSidechain").and_then(|x| x.as_bool()) == Some(true) {
            continue; // subagent chatter — not this agent's main thread
        }
        match v.get("type").and_then(|t| t.as_str()) {
            Some("assistant") => distill_assistant(&v, &mut out),
            Some("user") => distill_user(&v, &mut out),
            _ => {} // file-history-snapshot / mode / ai-title / attachment / …
        }
    }
    const KEEP: usize = 80;
    if out.len() > KEEP {
        out.drain(..out.len() - KEEP);
    }
    out
}

/// The text of the *last* assistant turn in a transcript — the agent's final
/// answer/summary. Concatenates that turn's text blocks; ignores tool calls,
/// thinking, and sidechains. A tool-only turn does not clobber the last textual
/// answer. Returns `[]` if the transcript has no assistant prose.
pub fn last_assistant_message(raw: &str) -> Vec<String> {
    let mut last: Vec<String> = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if v.get("isSidechain").and_then(|x| x.as_bool()) == Some(true) {
            continue;
        }
        if v.get("type").and_then(|t| t.as_str()) != Some("assistant") {
            continue;
        }
        let content = v.pointer("/message/content");
        let mut texts: Vec<String> = Vec::new();
        if let Some(s) = content.and_then(|c| c.as_str()) {
            texts.extend(s.lines().map(|l| l.to_string()));
        } else if let Some(arr) = content.and_then(|c| c.as_array()) {
            for c in arr {
                if c.get("type").and_then(|t| t.as_str()) == Some("text") {
                    if let Some(t) = c.get("text").and_then(|x| x.as_str()) {
                        texts.extend(t.lines().map(|l| l.to_string()));
                    }
                }
            }
        }
        if texts.iter().any(|l| !l.trim().is_empty()) {
            last = texts;
        }
    }
    last.into_iter()
        .map(|l| l.trim_end().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

pub fn distill_codex_transcript(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        match v.get("type").and_then(|t| t.as_str()) {
            Some("response_item") => distill_codex_response_item(&v, &mut out),
            Some("event_msg") => distill_codex_event(&v, &mut out),
            _ => {}
        }
    }
    const KEEP: usize = 120;
    if out.len() > KEEP {
        out.drain(..out.len() - KEEP);
    }
    out
}

pub fn last_codex_assistant_message(raw: &str) -> Vec<String> {
    let mut last = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if v.get("type").and_then(|t| t.as_str()) != Some("response_item") {
            continue;
        }
        let payload = v.get("payload").unwrap_or(&Value::Null);
        if payload.get("type").and_then(|t| t.as_str()) != Some("message")
            || payload.get("role").and_then(|r| r.as_str()) != Some("assistant")
        {
            continue;
        }
        let mut texts = Vec::new();
        if let Some(content) = payload.get("content").and_then(|c| c.as_array()) {
            for c in content {
                if matches!(
                    c.get("type").and_then(|t| t.as_str()),
                    Some("output_text" | "text")
                ) {
                    if let Some(text) = c.get("text").and_then(|x| x.as_str()) {
                        texts.extend(text.lines().map(|l| l.trim_end().to_string()));
                    }
                }
            }
        }
        if texts.iter().any(|l| !l.trim().is_empty()) {
            last = texts;
        }
    }
    last.into_iter().filter(|l| !l.trim().is_empty()).collect()
}

fn distill_codex_response_item(v: &Value, out: &mut Vec<String>) {
    let payload = v.get("payload").unwrap_or(&Value::Null);
    match payload.get("type").and_then(|t| t.as_str()) {
        Some("message") => {
            let role = payload
                .get("role")
                .and_then(|r| r.as_str())
                .unwrap_or("assistant");
            let prefix = if role == "user" { "user: " } else { "" };
            if let Some(content) = payload.get("content").and_then(|c| c.as_array()) {
                for c in content {
                    if matches!(
                        c.get("type").and_then(|t| t.as_str()),
                        Some("output_text" | "input_text" | "text")
                    ) {
                        if let Some(text) = c.get("text").and_then(|x| x.as_str()) {
                            push_prefixed_prose(out, prefix, text);
                        }
                    }
                }
            }
        }
        Some("function_call") => {
            let name = payload
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("tool");
            let args = payload
                .get("arguments")
                .and_then(|a| a.as_str())
                .unwrap_or("");
            out.push(format!("tool {name}: {}", clip_codex_args(args)));
        }
        Some("function_call_output") => {
            let text = payload
                .get("output")
                .and_then(|o| o.as_str())
                .map(first_meaningful_line)
                .unwrap_or_default();
            if !text.is_empty() {
                out.push(format!("  -> {}", clip(&text, 120)));
            }
        }
        _ => {}
    }
}

fn distill_codex_event(v: &Value, out: &mut Vec<String>) {
    let payload = v.get("payload").unwrap_or(&Value::Null);
    if payload.get("type").and_then(|t| t.as_str()) != Some("agent_message") {
        return;
    }
    if let Some(message) = payload.get("message").and_then(|m| m.as_str()) {
        push_prefixed_prose(out, "", message);
    }
}

fn push_prefixed_prose(out: &mut Vec<String>, prefix: &str, text: &str) {
    for l in text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .take(8)
    {
        out.push(format!("{prefix}{}", clip(l, 200)));
    }
}

fn clip_codex_args(args: &str) -> String {
    if args.trim().is_empty() {
        return "(no args)".to_string();
    }
    let summary = serde_json::from_str::<Value>(args)
        .ok()
        .and_then(|v| {
            for key in ["cmd", "command", "path", "file_path", "pattern", "query"] {
                if let Some(s) = v.get(key).and_then(|x| x.as_str()) {
                    return Some(s.lines().next().unwrap_or("").to_string());
                }
            }
            None
        })
        .unwrap_or_else(|| args.lines().next().unwrap_or("").to_string());
    clip(&summary, 120)
}

fn first_meaningful_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .to_string()
}

fn distill_assistant(v: &Value, out: &mut Vec<String>) {
    let content = v.pointer("/message/content");
    if let Some(s) = content.and_then(|c| c.as_str()) {
        push_prose(out, s);
        return;
    }
    let Some(arr) = content.and_then(|c| c.as_array()) else {
        return;
    };
    for c in arr {
        match c.get("type").and_then(|t| t.as_str()) {
            Some("text") => {
                if let Some(t) = c.get("text").and_then(|x| x.as_str()) {
                    push_prose(out, t);
                }
            }
            Some("tool_use") => {
                let name = c.get("name").and_then(|x| x.as_str()).unwrap_or("tool");
                out.push(format!(
                    "⚙ {}{}",
                    name,
                    tool_input_summary(name, c.get("input"))
                ));
            }
            _ => {} // thinking, redacted_thinking, …
        }
    }
}

fn distill_user(v: &Value, out: &mut Vec<String>) {
    let content = v.pointer("/message/content");
    if let Some(arr) = content.and_then(|c| c.as_array()) {
        for c in arr {
            if c.get("type").and_then(|t| t.as_str()) == Some("tool_result") {
                let text = tool_result_first_line(c.get("content"));
                if !text.is_empty() {
                    out.push(format!("  ↳ {}", clip(&text, 100)));
                }
            }
        }
        return;
    }
    // A plain string is a real user prompt (not a tool result).
    if let Some(s) = content.and_then(|c| c.as_str()) {
        for l in s.lines().map(str::trim).filter(|l| !l.is_empty()).take(3) {
            out.push(format!("▶ {}", clip(l, 100)));
        }
    }
}

fn push_prose(out: &mut Vec<String>, text: &str) {
    for l in text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .take(6)
    {
        out.push(clip(l, 200));
    }
}

/// A compact one-line summary of a tool call's most salient argument.
fn tool_input_summary(name: &str, input: Option<&Value>) -> String {
    let Some(inp) = input else {
        return String::new();
    };
    let s = |k: &str| inp.get(k).and_then(|v| v.as_str());
    let detail = match name {
        "Bash" => s("command").map(|c| c.lines().next().unwrap_or("").to_string()),
        "Read" | "Edit" | "Write" | "NotebookEdit" => s("file_path").map(short_path),
        "Grep" | "Glob" => s("pattern").map(String::from),
        "Task" => s("description")
            .map(String::from)
            .or_else(|| s("subagent_type").map(String::from)),
        "WebFetch" | "WebSearch" => s("url").or_else(|| s("query")).map(String::from),
        "TodoWrite" => Some("update todos".to_string()),
        _ => None,
    };
    match detail {
        Some(d) if !d.is_empty() => format!(": {}", clip(&d, 100)),
        _ => String::new(),
    }
}

/// First meaningful line of a tool_result (string, or array of text blocks),
/// skipping injected `<system-reminder>` noise.
fn tool_result_first_line(content: Option<&Value>) -> String {
    let text = match content {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|c| c.get("text").and_then(|x| x.as_str()))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    };
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with("<system-reminder>"))
        .unwrap_or("")
        .to_string()
}

/// Last two path components (`…/dir/file.rs`), for compact tool summaries.
fn short_path(p: &str) -> String {
    let mut tail: Vec<&str> = p.rsplit('/').take(2).collect();
    tail.reverse();
    tail.join("/")
}

/// Char-based truncation with an ellipsis (keeps multibyte output intact).
fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max.saturating_sub(1)).collect();
        t.push('…');
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attach_command_per_backend() {
        let claude = backend("claude").unwrap();
        // The session id is shell-quoted for safe interpolation over ssh.
        assert_eq!(
            attach_command(claude, "abc123", false),
            "claude --resume 'abc123'"
        );
        assert_eq!(attach_command(claude, "", false), "claude");
        // fresh ignores the session for every backend.
        assert_eq!(attach_command(claude, "abc123", true), "claude");

        assert_eq!(
            attach_command(backend("codex").unwrap(), "", false),
            "codex resume --last"
        );
        assert_eq!(
            attach_command(backend("opencode").unwrap(), "", false),
            "opencode --continue"
        );
        // agy has no resume concept — always bare.
        assert_eq!(attach_command(backend("agy").unwrap(), "sid", false), "agy");
        assert_eq!(attach_command(backend("agy").unwrap(), "sid", true), "agy");
    }

    #[test]
    fn backends_lookup_and_filter() {
        assert!(backend("claude").is_some());
        assert!(backend("nope").is_none());
        // Every backend is dispatchable/reviewable/interactive today.
        assert_eq!(backends_for(Purpose::Dispatch).len(), BACKENDS.len());
        assert!(backends_for(Purpose::Review)
            .iter()
            .any(|b| b.name == "codex"));
    }

    #[test]
    fn default_backend_fallback_chain() {
        let cfg = Config::default();
        // No project config, no run → claude.
        assert_eq!(default_backend(&cfg, "x", None), "claude");
        // Newest run's backend wins when the project has no configured default.
        let newest = json!({ "tool": "codex" });
        assert_eq!(default_backend(&cfg, "x", Some(&newest)), "codex");
        // An empty recorded backend still falls back to claude.
        let empty = json!({ "tool": "" });
        assert_eq!(default_backend(&cfg, "x", Some(&empty)), "claude");
    }

    #[test]
    fn distill_transcript_extracts_activity() {
        // A realistic slice: assistant prose, a tool_use, then its tool_result,
        // plus a sidechain line and a bookkeeping line that must be dropped.
        let raw = concat!(
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"thinking","thinking":"hmm"},{"type":"text","text":"Editing the file now."}]}}"#,
            "\n",
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","name":"Edit","input":{"file_path":"/Users/x/proj/src/main.rs"}}]}}"#,
            "\n",
            r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","content":"The file was updated.\nok"}]}}"#,
            "\n",
            r#"{"type":"assistant","isSidechain":true,"message":{"role":"assistant","content":[{"type":"text","text":"subagent noise"}]}}"#,
            "\n",
            r#"{"type":"file-history-snapshot","messageId":"z"}"#,
            "\n",
            r#"{"type":"user","message":{"role":"user","content":"fix the bug please"}}"#,
            "\n",
        );
        let out = distill_transcript(raw);
        assert!(
            out.contains(&"Editing the file now.".to_string()),
            "{out:?}"
        );
        assert!(out.contains(&"⚙ Edit: src/main.rs".to_string()), "{out:?}");
        assert!(
            out.contains(&"  ↳ The file was updated.".to_string()),
            "{out:?}"
        );
        assert!(out.contains(&"▶ fix the bug please".to_string()), "{out:?}");
        // Sidechain + bookkeeping + thinking are dropped.
        assert!(!out.iter().any(|l| l.contains("subagent noise")), "{out:?}");
        assert!(!out.iter().any(|l| l.contains("hmm")), "{out:?}");
    }

    #[test]
    fn distill_transcript_summaries_and_clip() {
        // Bash summary is the first command line; long text is ellipsised.
        let long = "x".repeat(300);
        let raw = format!(
            concat!(
                r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","name":"Bash","input":{{"command":"cargo test\nsecond line"}}}}]}}}}"#,
                "\n",
                r#"{{"type":"assistant","message":{{"content":[{{"type":"text","text":"{}"}}]}}}}"#,
                "\n",
            ),
            long
        );
        let out = distill_transcript(&raw);
        assert!(out.contains(&"⚙ Bash: cargo test".to_string()), "{out:?}");
        let prose = out.last().unwrap();
        assert!(
            prose.chars().count() <= 200,
            "len={}",
            prose.chars().count()
        );
        assert!(prose.ends_with('…'), "{prose}");
    }

    #[test]
    fn session_id_guard_rejects_injection() {
        assert!(is_session_id("86c7dfb7-2733-49ef-9231-bea82c3e7d3f"));
        assert!(!is_session_id("a; rm -rf /"));
        assert!(!is_session_id("a/../b"));
        assert!(!is_session_id(""));
    }

    #[test]
    fn last_assistant_message_picks_final_prose() {
        // Two assistant prose turns with a tool-only turn last: the tool turn must
        // not clobber the final textual answer; multi-line prose is split.
        let raw = concat!(
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"first answer"}]}}"#,
            "\n",
            r#"{"type":"user","message":{"content":[{"type":"tool_result","content":"ok"}]}}"#,
            "\n",
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"final line 1\nfinal line 2"}]}}"#,
            "\n",
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"echo hi"}}]}}"#,
            "\n",
        );
        assert_eq!(
            last_assistant_message(raw),
            vec!["final line 1".to_string(), "final line 2".to_string()]
        );
    }

    #[test]
    fn last_assistant_message_ignores_sidechain_and_handles_string_content() {
        // Sidechain (subagent) turns are skipped; a plain-string message body works.
        let raw = concat!(
            r#"{"type":"assistant","message":{"content":"the answer"}}"#,
            "\n",
            r#"{"type":"assistant","isSidechain":true,"message":{"content":[{"type":"text","text":"subagent noise"}]}}"#,
            "\n",
        );
        assert_eq!(last_assistant_message(raw), vec!["the answer".to_string()]);
    }

    #[test]
    fn last_assistant_message_empty_when_no_prose() {
        let raw = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Read","input":{}}]}}"#;
        assert!(last_assistant_message(raw).is_empty());
    }

    #[test]
    fn distill_codex_transcript_extracts_activity() {
        let raw = concat!(
            r#"{"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"fix the logs"}]}}"#,
            "\n",
            r#"{"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"I will inspect the log path."}]}}"#,
            "\n",
            r#"{"type":"response_item","payload":{"type":"function_call","name":"exec_command","arguments":"{\"cmd\":\"rg logs\"}"}}"#,
            "\n",
            r#"{"type":"response_item","payload":{"type":"function_call_output","output":"Chunk ID: abc\nOutput:\nfound it"}}"#,
            "\n",
            r#"{"type":"event_msg","payload":{"type":"token_count","info":{}}}"#,
            "\n",
        );
        let out = distill_codex_transcript(raw);
        assert!(out.contains(&"user: fix the logs".to_string()), "{out:?}");
        assert!(
            out.contains(&"I will inspect the log path.".to_string()),
            "{out:?}"
        );
        assert!(
            out.contains(&"tool exec_command: rg logs".to_string()),
            "{out:?}"
        );
        assert!(out.contains(&"  -> Chunk ID: abc".to_string()), "{out:?}");
        assert!(!out.iter().any(|l| l.contains("token_count")), "{out:?}");
    }

    #[test]
    fn last_codex_assistant_message_picks_final_text() {
        let raw = concat!(
            r#"{"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"first"}]}}"#,
            "\n",
            r#"{"type":"response_item","payload":{"type":"function_call","name":"exec_command","arguments":"{}"}}"#,
            "\n",
            r#"{"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"final 1\nfinal 2"}]}}"#,
            "\n",
        );
        assert_eq!(
            last_codex_assistant_message(raw),
            vec!["final 1".to_string(), "final 2".to_string()]
        );
    }
}
