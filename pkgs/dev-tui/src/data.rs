use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::model::{Agent, Env, GitState, Tool};
use crate::usage::{fetch_claude_usage, ClaudeUsage};

pub enum Req {
    Refresh,
    Git,
    /// Recent activity for `target`. `session` pins a claude session (from `ps`)
    /// so the worker reads that agent's transcript without rediscovering it.
    Logs {
        target: String,
        tool: String,
        session: Option<String>,
    },
    Tools,
    Usage,
    AgyUsage,
    DevTasks,
    TaskDetail(String),
    /// `dev git diff <target>` snapshot for the Fleet inspector Diff view.
    Diff(String),
    /// An agent run's final result (`dev agent output`), for the ResultView overlay.
    /// `full` fetches the whole run log instead of the tail / last message.
    Output {
        target: String,
        full: bool,
    },
}

#[allow(dead_code)]
pub enum Msg {
    State(Vec<Env>),
    Git(HashMap<String, GitState>),
    Logs {
        target: String,
        lines: Vec<String>,
    },
    Tools(Vec<Tool>),
    /// Generic text result (review, worktree list, session list, etc.)
    Result {
        title: String,
        lines: Vec<String>,
    },
    Usage(Option<ClaudeUsage>),
    AgyUsage(Option<AgyUsage>),
    CodexUsage(Option<CodexUsage>),
    /// Real-time log line from `dev logs -f` tail.
    LogLine {
        target: String,
        line: String,
    },
    /// Error message to display as flash.
    Error(String),
    DevTasks(Vec<crate::task::DevTask>, Vec<crate::task::DevQuestion>),
    TaskDetail(Option<crate::task::TaskDetail>),
    /// `dev git diff <target>` snapshot for the Fleet inspector Diff view.
    Diff {
        target: String,
        lines: Vec<String>,
    },
}

/// Codex rate-limit data via JSON-RPC to `codex app-server`.
#[derive(Clone)]
pub struct CodexUsage {
    /// Primary window (5h) used percent.
    pub primary_used_pct: Option<f64>,
    #[allow(dead_code)]
    pub primary_resets_at: Option<i64>,
    /// Secondary window (7d) used percent.
    pub secondary_used_pct: Option<f64>,
    #[allow(dead_code)]
    pub secondary_resets_at: Option<i64>,
    #[allow(dead_code)]
    pub plan_type: Option<String>,
}

/// agy (Antigravity) quota from its local language-server Connect-RPC API.
#[derive(Clone)]
pub struct AgyUsage {
    /// Remaining prompt credits (absolute).
    pub available_credits: Option<u64>,
    /// Monthly credit cap.
    pub monthly_credits: Option<u64>,
    /// Per-model: (display label, used_pct 0-100).
    pub models: Vec<(String, u32)>,
}

pub fn fetch_codex_usage() -> Option<CodexUsage> {
    let mut child = Command::new("codex")
        .args(["-s", "read-only", "-a", "untrusted", "app-server"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let mut stdin = child.stdin.take()?;
    let stdout = child.stdout.take()?;

    let (line_tx, line_rx) = mpsc::channel::<String>();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    if line_tx.send(l).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let _ = writeln!(
        stdin,
        r#"{{"id":1,"method":"initialize","params":{{"clientInfo":{{"name":"dev-tui","version":"0.1.0"}}}}}}"#
    );

    let deadline = Instant::now() + Duration::from_secs(8);
    let mut got_init = false;
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match line_rx.recv_timeout(remaining.min(Duration::from_millis(200))) {
            Ok(line) => {
                if let Ok(v) = serde_json::from_str::<Value>(&line) {
                    if v.get("id").and_then(|x| x.as_u64()) == Some(1) {
                        got_init = true;
                        break;
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
        }
    }

    if !got_init {
        let _ = child.kill();
        return None;
    }

    let _ = writeln!(stdin, r#"{{"method":"initialized","params":{{}}}}"#);
    let _ = writeln!(
        stdin,
        r#"{{"id":2,"method":"account/rateLimits/read","params":{{}}}}"#
    );

    let deadline = Instant::now() + Duration::from_secs(3);
    let mut result = None;
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match line_rx.recv_timeout(remaining.min(Duration::from_millis(200))) {
            Ok(line) => {
                if let Ok(v) = serde_json::from_str::<Value>(&line) {
                    if v.get("id").and_then(|x| x.as_u64()) == Some(2) {
                        if let Some(rl) = v.pointer("/result/rateLimits") {
                            result = Some(CodexUsage {
                                primary_used_pct: rl
                                    .pointer("/primary/usedPercent")
                                    .and_then(|x| x.as_f64()),
                                primary_resets_at: rl
                                    .pointer("/primary/resetsAt")
                                    .and_then(|x| x.as_i64()),
                                secondary_used_pct: rl
                                    .pointer("/secondary/usedPercent")
                                    .and_then(|x| x.as_f64()),
                                secondary_resets_at: rl
                                    .pointer("/secondary/resetsAt")
                                    .and_then(|x| x.as_i64()),
                                plan_type: rl
                                    .get("planType")
                                    .and_then(|x| x.as_str())
                                    .map(String::from),
                            });
                        }
                        break;
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
        }
    }

    let _ = child.kill();
    result
}

pub fn worker(req_rx: Receiver<Req>, msg_tx: Sender<Msg>) {
    while let Ok(req) = req_rx.recv() {
        let tx = msg_tx.clone();
        thread::spawn(move || {
            let msg = match req {
                Req::Refresh => Msg::State(fetch_state()),
                Req::Git => Msg::Git(fetch_git()),
                Req::Logs {
                    target,
                    tool,
                    session,
                } => {
                    let lines = fetch_logs(&target, &tool, session.as_deref());
                    Msg::Logs { target, lines }
                }
                Req::Tools => Msg::Tools(fetch_tools()),
                Req::Usage => Msg::Usage(fetch_claude_usage()),
                Req::AgyUsage => Msg::AgyUsage(fetch_agy_usage()),
                Req::DevTasks => {
                    let (tasks, questions) = crate::task::load_dev_tasks();
                    Msg::DevTasks(tasks, questions)
                }
                Req::TaskDetail(task_id) => {
                    let detail = crate::task::load_task_detail(&task_id);
                    Msg::TaskDetail(detail)
                }
                Req::Diff(target) => {
                    let lines = fetch_diff(&target);
                    Msg::Diff { target, lines }
                }
                Req::Output { target, full } => {
                    // In-process (was `dev agent output --json`): the run's final
                    // result, richer than the scrolling log tail.
                    let cfg = dev_core::config::Config::load_or_default();
                    let mut lines = dev_core::agent::final_output(&cfg, &target, full);
                    if lines.is_empty() {
                        lines.push("(no output yet)".to_string());
                    }
                    Msg::Result {
                        title: format!("output: {target}"),
                        lines,
                    }
                }
            };
            let _ = tx.send(msg);
        });
    }
}

fn fetch_envs_base() -> Vec<Env> {
    // In-process: read config.toml directly via dev-core (was `dev ls --json`).
    let cfg = dev_core::config::Config::load_or_default();
    let mut envs = Vec::new();
    for l in &cfg.local {
        envs.push(Env {
            name: l.name.clone(),
            group: "local".into(),
            host: String::new(),
            shell: String::new(),
            os: String::new(),
            path: l.path.clone(),
            base_status: "stopped".into(),
            agents: Vec::new(),
            expanded: false,
        });
    }
    for r in &cfg.remote {
        let e = cfg.env(&r.env);
        envs.push(Env {
            name: r.name.clone(),
            group: r.env.clone(),
            host: e.map(|e| e.host.clone()).unwrap_or_default(),
            shell: e.map(|e| e.shell.clone()).unwrap_or_default(),
            os: cfg.env_os(&r.env, &r.path),
            path: r.path.clone(),
            base_status: "stopped".into(),
            agents: Vec::new(),
            expanded: false,
        });
    }
    envs
}

fn fetch_state() -> Vec<Env> {
    let mut envs = fetch_envs_base();
    // In-process agent discovery (was `dev agent ps --json`). Runs on the worker
    // thread; dev-core parallelizes across projects internally.
    let cfg = dev_core::config::Config::load_or_default();
    let arr = dev_core::agent::ps(&cfg);
    {
        for a in &arr {
            let target = match sget(a, "target") {
                Some(t) => t,
                None => continue,
            };
            let env = match envs.iter_mut().find(|e| e.name == target) {
                Some(e) => e,
                None => continue,
            };
            let has_pid = matches!(
                a.get("pid"),
                Some(Value::Number(_)) | Some(Value::String(_))
            );
            let status = sget(a, "status").unwrap_or_else(|| "?".into());
            if has_pid {
                let session_id = sget(a, "session_id").filter(|s| !s.is_empty());
                let agent_name = sget(a, "name").filter(|s| !s.is_empty());
                env.agents.push(Agent {
                    tool: sget(a, "tool").unwrap_or_else(|| "-".into()),
                    status,
                    kind: sget(a, "kind").unwrap_or_default(),
                    pid: pid_to_string(a.get("pid")),
                    cwd: sget(a, "cwd").unwrap_or_default(),
                    session_id,
                    agent_name,
                    model: None,
                });
            } else {
                env.base_status = status;
            }
        }
    }
    envs
}

fn fetch_git() -> HashMap<String, GitState> {
    // In-process: dev-core parallelizes status across targets internally
    // (was `dev git status --json`). This already runs on the worker thread.
    let cfg = dev_core::config::Config::load_or_default();
    let targets = cfg.list_projects();
    let rows = dev_core::git::status_all(&cfg, &targets);
    let mut m = HashMap::new();
    for g in &rows {
        if let Some(t) = sget(g, "target") {
            m.insert(
                t,
                GitState {
                    branch: sget(g, "branch").unwrap_or_default(),
                    head: sget(g, "head").unwrap_or_default(),
                    changes: g.get("changes").and_then(|x| x.as_i64()).unwrap_or(0),
                },
            );
        }
    }
    m
}

fn fetch_logs(target: &str, tool: &str, session: Option<&str>) -> Vec<String> {
    // In-process recent activity: claude → distilled transcript, other backends →
    // run-log tail (see `dev_core::agent::recent_activity`). Strip ANSI so an
    // agent's default (formatted, colorized) output renders cleanly in the ratatui
    // detail pane instead of showing escape-code garbage. The live-tail path
    // already strips ANSI in `App::start_tail`.
    let cfg = dev_core::config::Config::load_or_default();
    dev_core::agent::recent_activity_for_tool(&cfg, target, tool, session)
        .iter()
        .map(|l| strip_ansi(l))
        .collect()
}

/// Snapshot of `dev git diff <target>` for the Fleet inspector Diff view.
/// Shells `dev` (rather than git2 in-process) so it works for remote envs too,
/// matching what the `D` key shows in the external pager.
fn fetch_diff(target: &str) -> Vec<String> {
    let out = Command::new("dev")
        .args(["git", "diff", target])
        .stdin(Stdio::null())
        .output();
    match out {
        Ok(o) => {
            let text = if o.stdout.is_empty() && !o.status.success() {
                String::from_utf8_lossy(&o.stderr).into_owned()
            } else {
                String::from_utf8_lossy(&o.stdout).into_owned()
            };
            let lines: Vec<String> = text.lines().map(strip_ansi).collect();
            if lines.is_empty() {
                vec!["(no changes)".into()]
            } else {
                lines
            }
        }
        Err(e) => vec![format!("(diff failed: {e})")],
    }
}

pub(crate) fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                // consume until a letter terminator
                for ch in chars.by_ref() {
                    if ch.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn fetch_tools() -> Vec<Tool> {
    // In-process: the backend registry lives in dev-core (was `dev backends --json`).
    dev_core::agent::BACKENDS
        .iter()
        .map(|t| Tool {
            name: t.name.to_string(),
            dispatchable: t.dispatchable,
            review: t.review,
        })
        .collect()
}

fn fetch_agy_usage() -> Option<AgyUsage> {
    let (pid, csrf) = agy_find_process()?;
    for port in agy_listening_ports(&pid) {
        let Some(v) = agy_query_port(port, &csrf) else {
            continue;
        };
        let status = v.get("userStatus")?;
        let available = status
            .pointer("/planStatus/availablePromptCredits")
            .and_then(|x| x.as_u64());
        let monthly = status
            .pointer("/planStatus/planInfo/monthlyPromptCredits")
            .and_then(|x| x.as_u64());
        let models = status
            .pointer("/cascadeModelConfigData/clientModelConfigs")
            .and_then(|x| x.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| {
                        let label = sget(m, "label")?;
                        let frac = m
                            .pointer("/quotaInfo/remainingFraction")
                            .and_then(|x| x.as_f64())?;
                        Some((label, ((1.0 - frac) * 100.0).round() as u32))
                    })
                    .collect()
            })
            .unwrap_or_default();
        return Some(AgyUsage {
            available_credits: available,
            monthly_credits: monthly,
            models,
        });
    }
    None
}

fn agy_find_process() -> Option<(String, String)> {
    let out = Command::new("pgrep")
        .args(["-x", "agy"])
        .stdin(Stdio::null())
        .output()
        .ok()?;
    let pids: Vec<String> = String::from_utf8(out.stdout)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(String::from)
        .collect();
    for pid in pids {
        let args_out = match Command::new("ps")
            .args(["-p", &pid, "-o", "args="])
            .stdin(Stdio::null())
            .output()
        {
            Ok(o) => o,
            Err(_) => continue,
        };
        let args = String::from_utf8(args_out.stdout).unwrap_or_default();
        let csrf = args
            .split_whitespace()
            .find(|a| a.starts_with("--csrf_token="))
            .map(|a| a["--csrf_token=".len()..].to_string())
            .unwrap_or_default();
        return Some((pid, csrf));
    }
    None
}

fn agy_listening_ports(pid: &str) -> Vec<u16> {
    let out = match Command::new("lsof")
        .args(["-nP", "-iTCP", "-sTCP:LISTEN", "-a", "-p", pid])
        .output()
    {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    let text = String::from_utf8(out.stdout).unwrap_or_default();
    let mut ports = Vec::new();
    for line in text.lines().skip(1) {
        if let Some(addr) = line.split_whitespace().nth(8) {
            if let Some(port_str) = addr.rsplit(':').next() {
                if let Ok(p) = port_str.parse::<u16>() {
                    ports.push(p);
                }
            }
        }
    }
    ports.sort_unstable();
    ports.dedup();
    ports
}

fn agy_query_port(port: u16, csrf: &str) -> Option<Value> {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    const PATH: &str = "/exa.language_server_pb.LanguageServerService/GetUserStatus";
    let csrf_hdr = if csrf.is_empty() {
        String::new()
    } else {
        format!("X-Codeium-Csrf-Token: {csrf}\r\n")
    };
    let req = format!(
        "POST {PATH} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\
         Content-Type: application/json\r\nConnect-Protocol-Version: 1\r\n\
         {csrf_hdr}Content-Length: 2\r\nConnection: close\r\n\r\n{{}}"
    );
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(3))).ok()?;
    stream.write_all(req.as_bytes()).ok()?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).ok()?;
    let text = String::from_utf8_lossy(&raw);
    if !text.starts_with("HTTP/1.1 200") && !text.starts_with("HTTP/1.0 200") {
        return None;
    }
    let body = text.split("\r\n\r\n").nth(1)?;
    serde_json::from_str(body).ok()
}

fn sget(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.as_str()).map(|s| s.to_string())
}

fn pid_to_string(v: Option<&Value>) -> String {
    match v {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}
