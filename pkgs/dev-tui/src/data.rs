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
    Logs(String),
    Tools,
    Usage,
    AgyUsage,
    DevTasks,
    TaskDetail(String),
}

#[allow(dead_code)]
pub enum Msg {
    State(Vec<Env>),
    Git(HashMap<String, GitState>),
    Logs { target: String, lines: Vec<String> },
    Tools(Vec<Tool>),
    /// Generic text result (review, worktree list, session list, etc.)
    Result { title: String, lines: Vec<String> },
    Usage(Option<ClaudeUsage>),
    AgyUsage(Option<AgyUsage>),
    CodexUsage(Option<CodexUsage>),
    /// Real-time log line from `dev logs -f` tail.
    LogLine { target: String, line: String },
    /// Error message to display as flash.
    Error(String),
    DevTasks(Vec<crate::task::DevTask>, Vec<crate::task::DevQuestion>),
    TaskDetail(Option<crate::task::TaskDetail>),
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
                Req::Logs(t) => {
                    let lines = fetch_logs(&t);
                    Msg::Logs { target: t, lines }
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
            };
            let _ = tx.send(msg);
        });
    }
}

fn fetch_envs_base() -> Vec<Env> {
    let out = match Command::new("dev")
        .args(["ls", "--json"])
        .stdin(Stdio::null())
        .output()
    {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    let mut envs = Vec::new();
    if let Ok(val) = serde_json::from_slice::<Value>(&out.stdout) {
        if let Some(Value::Array(locals)) = val.get("local") {
            for l in locals {
                envs.push(Env {
                    name: sget(l, "name").unwrap_or_default(),
                    group: "local".into(),
                    host: String::new(),
                    shell: String::new(),
                    os: String::new(),
                    path: sget(l, "path").unwrap_or_default(),
                    base_status: "stopped".into(),
                    agents: Vec::new(),
                    expanded: false,
                });
            }
        }
        if let Some(Value::Array(remotes)) = val.get("remote") {
            for r in remotes {
                let env_name = sget(r, "env").unwrap_or_else(|| "remote".into());
                envs.push(Env {
                    name: sget(r, "name").unwrap_or_default(),
                    group: env_name,
                    host: sget(r, "host").unwrap_or_default(),
                    shell: sget(r, "shell").unwrap_or_default(),
                    os: sget(r, "os").unwrap_or_default(),
                    path: sget(r, "path").unwrap_or_default(),
                    base_status: "stopped".into(),
                    agents: Vec::new(),
                    expanded: false,
                });
            }
        }
    }
    envs
}

fn fetch_state() -> Vec<Env> {
    let mut envs = fetch_envs_base();
    let out = match Command::new("dev")
        .args(["agent", "ps", "--json"])
        .stdin(Stdio::null())
        .output()
    {
        Ok(o) => o,
        Err(_) => return envs,
    };
    if let Ok(Value::Array(arr)) = serde_json::from_slice::<Value>(&out.stdout) {
        for a in &arr {
            let target = match sget(a, "target") {
                Some(t) => t,
                None => continue,
            };
            let env = match envs.iter_mut().find(|e| e.name == target) {
                Some(e) => e,
                None => continue,
            };
            let has_pid = matches!(a.get("pid"), Some(Value::Number(_)) | Some(Value::String(_)));
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
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut m = HashMap::new();
        let out = match Command::new("dev")
            .args(["git", "status", "--json"])
            .stdin(Stdio::null())
            .output()
        {
            Ok(o) => o,
            Err(_) => {
                let _ = tx.send(m);
                return;
            }
        };
        if let Ok(Value::Array(arr)) = serde_json::from_slice::<Value>(&out.stdout) {
            for g in &arr {
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
        }
        let _ = tx.send(m);
    });
    rx.recv_timeout(Duration::from_secs(10)).unwrap_or_default()
}

fn fetch_logs(target: &str) -> Vec<String> {
    let out = match Command::new("dev")
        .args(["agent", "logs", target, "--json"])
        .stdin(Stdio::null())
        .output()
    {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    if let Ok(v) = serde_json::from_slice::<Value>(&out.stdout) {
        if let Some(Value::Array(lines)) = v.get("lines") {
            return lines
                .iter()
                .filter_map(|l| l.as_str().map(|s| s.to_string()))
                .collect();
        }
    }
    Vec::new()
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
                    if ch.is_ascii_alphabetic() { break; }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn fetch_tools() -> Vec<Tool> {
    let out = match Command::new("dev")
        .args(["tools", "--json"])
        .stdin(Stdio::null())
        .output()
    {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    if let Ok(Value::Array(arr)) = serde_json::from_slice::<Value>(&out.stdout) {
        return arr
            .iter()
            .filter_map(|t| {
                let name = sget(t, "name")?;
                let dispatchable = t
                    .get("dispatchable")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let review = t.get("review").and_then(|v| v.as_bool()).unwrap_or(false);
                Some(Tool {
                    name,
                    dispatchable,
                    review,
                })
            })
            .collect();
    }
    Vec::new()
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
