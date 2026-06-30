//! dev top — live TUI over the `dev` CLI's machine-readable surface.
//!
//! Fleet rows group by LOCAL / REMOTE·<env>. Space collapses groups or
//! expands agent sub-rows. Enter opens an action menu (attach, dispatch,
//! start tool, logs, diff, kill). Dispatch and tool-start flow through a
//! tool-picker first. All commands delegate to `dev`.

use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead, BufReader, Stdout, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use serde_json::Value;

const SPIN: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
/// Fallback tool list used when `dev tools --json` is unavailable.
const DEFAULT_TOOLS: [&str; 4] = ["claude", "codex", "opencode", "agy"];

// ── data ─────────────────────────────────────────────────────────────────────

/// One entry from `dev tools --json`.
#[derive(Clone)]
struct Tool {
    name: String,
    dispatchable: bool,
    #[allow(dead_code)] // used in Phase 5 (review action)
    review: bool,
}

#[derive(Clone, Default)]
struct Agent {
    tool: String,
    status: String,
    kind: String,
    pid: String,
    cwd: String,
    /// Claude session ID — used for `--resume` on attach and display.
    session_id: Option<String>,
    /// Human-readable agent name (from `claude agents --json` `.name` field).
    agent_name: Option<String>,
    /// Current model, if known (from run meta or future config fetch).
    model: Option<String>,
}

#[derive(Clone)]
struct Env {
    name: String,
    group: String, // "local" | remote env name ("coder", "bf-e", …)
    path: String,
    base_status: String,
    agents: Vec<Agent>,
    expanded: bool,
}

#[derive(Clone)]
struct GroupInfo {
    key: String,   // "local" | "coder" | "bf-e" …
    label: String, // "LOCAL" | "REMOTE · coder" …
}

#[derive(Clone, PartialEq)]
enum Item {
    GroupHeader(usize), // index into App.groups
    EnvRow(usize),
    AgentRow(usize, usize),
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum ToolPurpose {
    #[default]
    Start,
    Dispatch,
}

#[derive(Clone, Default)]
struct GitState {
    branch: String,
    head: String,
    changes: i64,
}

fn status_rank(s: &str) -> u8 {
    match s {
        "waiting" => 0,
        "error" => 1,
        "busy" | "running" => 2,
        "idle" => 3,
        "stopped" => 5,
        "unreachable" => 6,
        _ => 4,
    }
}

fn status_style(s: &str) -> Style {
    match s {
        "waiting" => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        "error" | "unreachable" => Style::default().fg(Color::Red),
        "busy" | "running" => Style::default().fg(Color::Green),
        "stopped" => Style::default().fg(Color::DarkGray),
        _ => Style::default(),
    }
}

fn env_dominant_status(env: &Env) -> &str {
    if env.agents.is_empty() {
        &env.base_status
    } else {
        env.agents
            .iter()
            .min_by_key(|a| status_rank(&a.status))
            .map(|a| a.status.as_str())
            .unwrap_or("?")
    }
}

fn env_status_label(env: &Env) -> String {
    if env.agents.is_empty() {
        let s = &env.base_status;
        if s.len() > 13 {
            format!("{}…", &s[..12])
        } else {
            s.clone()
        }
    } else {
        let n = env.agents.len();
        let dom = env_dominant_status(env);
        format!("{n} {dom}")
    }
}

fn truncate(s: &str, w: usize) -> String {
    if w == 0 {
        return String::new();
    }
    if s.chars().count() <= w {
        return s.to_string();
    }
    let mut t: String = s.chars().take(w.saturating_sub(1)).collect();
    t.push('…');
    t
}

/// Wrap a logical line into chunks of `w` chars each.
fn wrap_line(s: &str, w: usize) -> Vec<String> {
    if w == 0 {
        return vec![String::new()];
    }
    if s.is_empty() {
        return vec![String::new()];
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= w {
        return vec![s.to_string()];
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let end = (i + w).min(chars.len());
        out.push(chars[i..end].iter().collect());
        i = end;
    }
    out
}

fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
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

// ── worker ────────────────────────────────────────────────────────────────────

enum Req {
    Refresh,
    Git,
    Logs(String),
    Tools,
    Review { target: String, tool: String },
    Usage,
    AgyUsage,
}

enum Msg {
    State(Vec<Env>),
    Git(HashMap<String, GitState>),
    Logs { target: String, lines: Vec<String> },
    Tools(Vec<Tool>),
    /// Generic text result (review, worktree list, session list, etc.)
    Result { title: String, lines: Vec<String> },
    Usage(Option<ClaudeUsage>),
    AgyUsage(Option<AgyUsage>),
    CodexUsage(Option<CodexUsage>),
}

/// Codex rate-limit data via JSON-RPC to `codex app-server`.
#[derive(Clone)]
struct CodexUsage {
    /// Primary window (5h) used percent.
    primary_used_pct: Option<f64>,
    #[allow(dead_code)]
    primary_resets_at: Option<i64>,
    /// Secondary window (7d) used percent.
    secondary_used_pct: Option<f64>,
    #[allow(dead_code)]
    secondary_resets_at: Option<i64>,
    #[allow(dead_code)]
    plan_type: Option<String>,
}

/// agy (Antigravity) quota from its local language-server Connect-RPC API.
#[derive(Clone)]
struct AgyUsage {
    /// Remaining prompt credits (absolute).
    available_credits: Option<u64>,
    /// Monthly credit cap.
    monthly_credits: Option<u64>,
    /// Per-model: (display label, used_pct 0-100).
    models: Vec<(String, u32)>,
}

#[derive(Clone)]
struct ClaudeUsage {
    five_hour_pct: Option<u32>,
    #[allow(dead_code)] // displayed by `dev usage`, available for future detail pane
    five_hour_resets_at: Option<i64>,
    seven_day_pct: Option<u32>,
    #[allow(dead_code)]
    seven_day_resets_at: Option<i64>,
    #[allow(dead_code)]
    updated_at: Option<f64>,
}

fn fetch_envs_base() -> Vec<Env> {
    let out = match Command::new("dev").args(["ls", "--json"]).stdin(Stdio::null()).output() {
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
    let out = match Command::new("dev").args(["ps", "--json"]).stdin(Stdio::null()).output() {
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
    // `dev status --json` can hang when remote hosts are unreachable; cap at 10 s.
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut m = HashMap::new();
        let out = match Command::new("dev")
            .args(["status", "--json"])
            .stdin(Stdio::null())
            .output()
        {
            Ok(o) => o,
            Err(_) => { let _ = tx.send(m); return; }
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
    let out = match Command::new("dev").args(["logs", target, "--json"]).stdin(Stdio::null()).output() {
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

fn fetch_review(target: &str, tool: &str) -> Vec<String> {
    let args = if tool.is_empty() {
        vec!["review".to_string(), target.to_string()]
    } else {
        vec!["review".to_string(), target.to_string(), "--tool".to_string(), tool.to_string()]
    };
    let out = Command::new("dev").args(&args).stdin(Stdio::null()).output();
    match out {
        Ok(o) => {
            let text = String::from_utf8_lossy(&o.stdout);
            let err = String::from_utf8_lossy(&o.stderr);
            let combined = format!("{}{}", text, err);
            combined.lines().map(|l| l.to_string()).collect()
        }
        Err(e) => vec![format!("dev review failed: {e}")],
    }
}

fn fetch_tools() -> Vec<Tool> {
    let out = match Command::new("dev").args(["tools", "--json"]).stdin(Stdio::null()).output() {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    if let Ok(Value::Array(arr)) = serde_json::from_slice::<Value>(&out.stdout) {
        return arr
            .iter()
            .filter_map(|t| {
                let name = sget(t, "name")?;
                let dispatchable = t.get("dispatchable").and_then(|v| v.as_bool()).unwrap_or(false);
                let review = t.get("review").and_then(|v| v.as_bool()).unwrap_or(false);
                Some(Tool { name, dispatchable, review })
            })
            .collect();
    }
    Vec::new()
}

fn fetch_usage() -> Option<ClaudeUsage> {
    let home = std::env::var("HOME").ok()?;
    let cache = std::env::var("XDG_CACHE_HOME")
        .unwrap_or_else(|_| format!("{home}/.cache"));
    let path = format!("{cache}/claude/usage.json");
    let data = std::fs::read(&path).ok()?;
    let v: Value = serde_json::from_slice(&data).ok()?;
    Some(ClaudeUsage {
        five_hour_pct: v.pointer("/five_hour/used_percentage")
            .and_then(|x| x.as_u64()).map(|x| x as u32),
        five_hour_resets_at: v.pointer("/five_hour/resets_at")
            .and_then(|x| x.as_i64()),
        seven_day_pct: v.pointer("/seven_day/used_percentage")
            .and_then(|x| x.as_u64()).map(|x| x as u32),
        seven_day_resets_at: v.pointer("/seven_day/resets_at")
            .and_then(|x| x.as_i64()),
        updated_at: v.get("updated_at").and_then(|x| x.as_f64()),
    })
}

/// Query `codex -s read-only -a untrusted app-server` via JSON-RPC 2.0 to get
/// rate-limit data.  Blocks up to 12 seconds — call from a dedicated thread.
fn fetch_codex_usage() -> Option<CodexUsage> {
    let mut child = Command::new("codex")
        .args(["-s", "read-only", "-a", "untrusted", "app-server"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let mut stdin = child.stdin.take()?;
    let stdout = child.stdout.take()?;

    // Pipe stdout lines to a channel so we can apply per-phase timeouts.
    let (line_tx, line_rx) = mpsc::channel::<String>();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => { if line_tx.send(l).is_err() { break; } }
                Err(_) => break,
            }
        }
    });

    // Phase 1 — send initialize, wait for server-ready response (up to 8 s).
    let _ = writeln!(stdin, r#"{{"id":1,"method":"initialize","params":{{"clientInfo":{{"name":"dev-tui","version":"0.1.0"}}}}}}"#);

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

    // Phase 2 — initialized notification + rate-limit request, wait 3 s.
    let _ = writeln!(stdin, r#"{{"method":"initialized","params":{{}}}}"#);
    let _ = writeln!(stdin, r#"{{"id":2,"method":"account/rateLimits/read","params":{{}}}}"#);

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
                                primary_used_pct: rl.pointer("/primary/usedPercent")
                                    .and_then(|x| x.as_f64()),
                                primary_resets_at: rl.pointer("/primary/resetsAt")
                                    .and_then(|x| x.as_i64()),
                                secondary_used_pct: rl.pointer("/secondary/usedPercent")
                                    .and_then(|x| x.as_f64()),
                                secondary_resets_at: rl.pointer("/secondary/resetsAt")
                                    .and_then(|x| x.as_i64()),
                                plan_type: rl.get("planType")
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

/// Find the running agy (antigravity) language-server PID and optional CSRF token.
/// Returns (pid_string, csrf_token).
fn agy_find_process() -> Option<(String, String)> {
    // agy embeds the language server in the CLI process itself; search by process name.
    let out = Command::new("pgrep").args(["-x", "agy"]).stdin(Stdio::null()).output().ok()?;
    let pids: Vec<String> = String::from_utf8(out.stdout)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(String::from)
        .collect();
    for pid in pids {
        let args_out = match Command::new("ps").args(["-p", &pid, "-o", "args="]).stdin(Stdio::null()).output() {
            Ok(o) => o,
            Err(_) => continue,
        };
        let args = String::from_utf8(args_out.stdout).unwrap_or_default();
        let csrf = args.split_whitespace()
            .find(|a| a.starts_with("--csrf_token="))
            .map(|a| a["--csrf_token=".len()..].to_string())
            .unwrap_or_default();
        return Some((pid, csrf));
    }
    None
}

/// Return TCP ports the given process is listening on (via lsof).
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
        // column 9 (0-indexed 8) is "ADDRESS:PORT"
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

/// POST `{}` to the Antigravity language-server Connect-RPC endpoint on a given port.
/// Returns parsed JSON body on HTTP 200, None otherwise.
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
    // Must be 200
    if !text.starts_with("HTTP/1.1 200") && !text.starts_with("HTTP/1.0 200") {
        return None;
    }
    let body = text.split("\r\n\r\n").nth(1)?;
    serde_json::from_str(body).ok()
}

fn fetch_agy_usage() -> Option<AgyUsage> {
    let (pid, csrf) = agy_find_process()?;
    for port in agy_listening_ports(&pid) {
        let Some(v) = agy_query_port(port, &csrf) else { continue };
        let status = v.get("userStatus")?;
        let available = status.pointer("/planStatus/availablePromptCredits")
            .and_then(|x| x.as_u64());
        let monthly = status.pointer("/planStatus/planInfo/monthlyPromptCredits")
            .and_then(|x| x.as_u64());
        let models = status.pointer("/cascadeModelConfigData/clientModelConfigs")
            .and_then(|x| x.as_array())
            .map(|arr| arr.iter().filter_map(|m| {
                let label = sget(m, "label")?;
                let frac = m.pointer("/quotaInfo/remainingFraction").and_then(|x| x.as_f64())?;
                // remainingFraction=1.0 means nothing used; convert to used%
                let used_pct = ((1.0 - frac) * 100.0).round() as u32;
                Some((label, used_pct))
            }).collect())
            .unwrap_or_default();
        return Some(AgyUsage { available_credits: available, monthly_credits: monthly, models });
    }
    None
}

fn worker(req_rx: Receiver<Req>, msg_tx: Sender<Msg>) {
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
                Req::Review { target, tool } => {
                    let lines = fetch_review(&target, &tool);
                    let title = if tool.is_empty() {
                        format!("review: {target}")
                    } else {
                        format!("review: {target} ({tool})")
                    };
                    Msg::Result { title, lines }
                }
                Req::Usage => Msg::Usage(fetch_usage()),
                Req::AgyUsage => Msg::AgyUsage(fetch_agy_usage()),
            };
            let _ = tx.send(msg);
        });
    }
}

// ── terminal drop-out ─────────────────────────────────────────────────────────

type Term = Terminal<CrosstermBackend<Stdout>>;

fn run_dev(args: &[&str], term: &mut Term) {
    let _ = disable_raw_mode();
    let _ = execute!(term.backend_mut(), LeaveAlternateScreen);
    let _ = Command::new("dev").args(args).status();
    let _ = enable_raw_mode();
    let _ = execute!(term.backend_mut(), EnterAlternateScreen);
    let _ = term.clear();
}

fn run_shell(cmd: &str, term: &mut Term) {
    let _ = disable_raw_mode();
    let _ = execute!(term.backend_mut(), LeaveAlternateScreen);
    let _ = Command::new("sh").arg("-c").arg(cmd).status();
    let _ = enable_raw_mode();
    let _ = execute!(term.backend_mut(), EnterAlternateScreen);
    let _ = term.clear();
}

// ── app ───────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Normal,
    Help,
    Filter,
    Dispatch,
    ConfirmKill,
    LogView,
    ActionMenu,
    ToolPick,
    BatchMenu,
    /// Inline text result viewer (review output, session list, etc.)
    ResultView,
    /// Model picker: select model then optionally effort for next dispatch.
    ModelPicker,
}

struct App {
    envs: Vec<Env>,
    git: HashMap<String, GitState>,
    groups: Vec<GroupInfo>,
    group_collapsed: HashMap<String, bool>,
    view: Vec<Item>,
    list_state: ListState,

    last_refresh: Instant,
    refreshing: bool,
    interval: Duration,
    last_git: Instant,
    git_inflight: bool,
    git_interval: Duration,

    filter: String,
    active_only: bool,
    mode: Mode,

    log_target: String,
    log_lines: Vec<String>,
    log_wanted: Option<(String, Instant)>,
    log_inflight: Option<String>,
    last_log: Instant,
    log_scroll: usize,
    log_follow: bool,

    dispatch_target: String,
    dispatch_input: String,
    dispatch_tool: String,
    /// Non-empty when dispatching to multiple marked targets (batch mode).
    /// Replaced by CLI fan-out in Phase 3; for now dispatches serially.
    dispatch_targets: Vec<String>,

    menu_index: usize,
    tool_index: usize,
    tool_purpose: ToolPurpose,
    tool_prev_mode: Mode,

    /// Backend registry from `dev tools --json`. Empty until first fetch.
    tools: Vec<Tool>,
    /// Env names marked for batch operations.
    marked: HashSet<String>,
    batch_menu_index: usize,

    /// ResultView: text displayed in the overlay (review, session list, …)
    result_title: String,
    result_lines: Vec<String>,
    result_scroll: usize,
    /// true while a background Req::Review (etc.) is in flight.
    result_inflight: bool,

    /// Dispatch model override (set by ModelPicker, applied to next dispatch).
    dispatch_model: String,
    dispatch_effort: String,
    /// Index within the model picker list.
    model_pick_index: usize,

    flash: Option<(String, Instant)>,
    spinner: u32,

    /// Claude rate-limit usage cached from ~/.cache/claude/usage.json.
    claude_usage: Option<ClaudeUsage>,
    last_usage: Instant,
    agy_usage: Option<AgyUsage>,
    last_agy_usage: Instant,
    codex_usage: Option<CodexUsage>,
    last_codex_usage: Instant,
    codex_usage_inflight: bool,

    req_tx: Sender<Req>,
    msg_rx: Receiver<Msg>,
    /// Cloned sender — allows spawning background usage threads that bypass the worker.
    msg_tx: Sender<Msg>,
}

impl App {
    fn new(req_tx: Sender<Req>, msg_rx: Receiver<Msg>, msg_tx: Sender<Msg>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        App {
            envs: Vec::new(),
            git: HashMap::new(),
            groups: Vec::new(),
            group_collapsed: HashMap::new(),
            view: Vec::new(),
            list_state,
            last_refresh: Instant::now(),
            refreshing: false,
            interval: Duration::from_secs(10),
            last_git: Instant::now(),
            git_inflight: false,
            git_interval: Duration::from_secs(120),
            filter: String::new(),
            active_only: false,
            mode: Mode::Normal,
            log_target: String::new(),
            log_lines: Vec::new(),
            log_wanted: None,
            log_inflight: None,
            last_log: Instant::now(),
            log_scroll: 0,
            log_follow: true,
            dispatch_target: String::new(),
            dispatch_input: String::new(),
            dispatch_tool: String::new(),
            dispatch_targets: Vec::new(),
            menu_index: 0,
            tool_index: 0,
            tool_purpose: ToolPurpose::Start,
            tool_prev_mode: Mode::Normal,
            tools: Vec::new(),
            marked: HashSet::new(),
            batch_menu_index: 0,
            result_title: String::new(),
            result_lines: Vec::new(),
            result_scroll: 0,
            result_inflight: false,
            dispatch_model: String::new(),
            dispatch_effort: String::new(),
            model_pick_index: 0,
            flash: None,
            spinner: 0,
            claude_usage: None,
            last_usage: Instant::now(),
            agy_usage: None,
            last_agy_usage: Instant::now(),
            codex_usage: None,
            last_codex_usage: Instant::now(),
            codex_usage_inflight: false,
            req_tx,
            msg_rx,
            msg_tx,
        }
    }

    fn request_codex_usage(&mut self) {
        if self.codex_usage_inflight { return; }
        self.codex_usage_inflight = true;
        self.last_codex_usage = Instant::now();
        let tx = self.msg_tx.clone();
        thread::spawn(move || {
            let result = fetch_codex_usage();
            let _ = tx.send(Msg::CodexUsage(result));
        });
    }

    fn request_refresh(&mut self) {
        if self.refreshing {
            return;
        }
        self.refreshing = true;
        let _ = self.req_tx.send(Req::Refresh);
    }

    fn request_git(&mut self) {
        if self.git_inflight {
            return;
        }
        self.git_inflight = true;
        let _ = self.req_tx.send(Req::Git);
    }

    fn set_flash(&mut self, msg: &str) {
        self.flash = Some((msg.to_string(), Instant::now()));
    }

    fn selected_key(&self) -> Option<String> {
        let s = self.list_state.selected()?;
        match self.view.get(s)? {
            Item::GroupHeader(gi) => Some(format!("group:{}", self.groups[*gi].key)),
            Item::EnvRow(i) => Some(self.envs[*i].name.clone()),
            Item::AgentRow(i, j) => {
                Some(format!("{}:{}", self.envs[*i].name, self.envs[*i].agents[*j].pid))
            }
        }
    }

    fn restore_selection(&mut self, key: Option<String>) {
        let key = match key {
            Some(k) => k,
            None => return,
        };
        for (idx, item) in self.view.iter().enumerate() {
            let k = match item {
                Item::GroupHeader(gi) => format!("group:{}", self.groups[*gi].key),
                Item::EnvRow(i) => self.envs[*i].name.clone(),
                Item::AgentRow(i, j) => {
                    format!("{}:{}", self.envs[*i].name, self.envs[*i].agents[*j].pid)
                }
            };
            if k == key {
                self.list_state.select(Some(idx));
                return;
            }
        }
        self.clamp_selection();
    }

    fn apply(&mut self, msg: Msg) {
        match msg {
            Msg::State(new_envs) => {
                let key = self.selected_key();
                let expansions: HashMap<String, bool> =
                    self.envs.iter().map(|e| (e.name.clone(), e.expanded)).collect();
                self.envs = new_envs;
                for env in &mut self.envs {
                    if let Some(&exp) = expansions.get(&env.name) {
                        env.expanded = exp;
                    }
                }
                self.refreshing = false;
                self.last_refresh = Instant::now();
                self.rebuild_view();
                self.restore_selection(key);
            }
            Msg::Git(g) => {
                self.git = g;
                self.git_inflight = false;
                self.last_git = Instant::now();
            }
            Msg::Logs { target, lines } => {
                if self.log_inflight.as_deref() == Some(target.as_str()) {
                    self.log_inflight = None;
                    self.log_target = target;
                    self.log_lines = lines;
                    self.last_log = Instant::now();
                }
            }
            Msg::Tools(t) => {
                if !t.is_empty() {
                    self.tools = t;
                }
            }
            Msg::Result { title, lines } => {
                self.result_title = title;
                self.result_lines = lines;
                self.result_scroll = 0;
                self.result_inflight = false;
                self.mode = Mode::ResultView;
            }
            Msg::Usage(u) => {
                self.claude_usage = u;
                self.last_usage = Instant::now();
            }
            Msg::AgyUsage(u) => {
                self.agy_usage = u;
                self.last_agy_usage = Instant::now();
            }
            Msg::CodexUsage(u) => {
                self.codex_usage = u;
                self.codex_usage_inflight = false;
            }
        }
    }

    /// Return tool names for the tool picker, filtered by purpose.
    /// Falls back to DEFAULT_TOOLS when the registry hasn't loaded yet.
    fn tools_for_picker(&self, purpose: ToolPurpose) -> Vec<String> {
        if self.tools.is_empty() {
            return DEFAULT_TOOLS.iter().map(|s| s.to_string()).collect();
        }
        self.tools
            .iter()
            .filter(|t| match purpose {
                ToolPurpose::Start => true,
                ToolPurpose::Dispatch => t.dispatchable,
            })
            .map(|t| t.name.clone())
            .collect()
    }

    /// Model names for the model picker given the selected tool.
    /// Returns (display_label, model_id) pairs.
    fn models_for_picker(&self, tool: &str) -> Vec<(String, String)> {
        match tool {
            "claude" => vec![
                ("sonnet  (claude-sonnet-4-6)".into(), "claude-sonnet-4-6".into()),
                ("opus    (claude-opus-4-8)".into(), "claude-opus-4-8".into()),
                ("haiku   (claude-haiku-4-5)".into(), "claude-haiku-4-5-20251001".into()),
                ("fable   (claude-fable-5)".into(), "claude-fable-5".into()),
            ],
            "codex" => vec![
                ("gpt-4o".into(), "gpt-4o".into()),
                ("gpt-4.1".into(), "gpt-4.1".into()),
                ("gpt-4.5".into(), "gpt-4.5".into()),
                ("gpt-5.5".into(), "gpt-5.5".into()),
                ("o4-mini".into(), "o4-mini".into()),
            ],
            "opencode" => vec![
                ("(current default)".into(), String::new()),
                ("lite-llm/Qwen3.6-27B".into(), "lite-llm/unsloth/Qwen3.6-27B-NVFP4".into()),
                ("vllm-oiwa/Gemma4-31B".into(), "vllm-oiwa/nvidia/Gemma-4-31B-IT-NVFP4".into()),
            ],
            _ => vec![],
        }
    }

    fn rebuild_view(&mut self) {
        // Collect unique groups in first-appearance order
        let mut seen: Vec<String> = Vec::new();
        for env in &self.envs {
            if !seen.contains(&env.group) {
                seen.push(env.group.clone());
            }
        }
        self.groups = seen
            .iter()
            .map(|key| {
                let label = if key == "local" {
                    "LOCAL".to_string()
                } else {
                    format!("REMOTE · {}", key)
                };
                GroupInfo { key: key.clone(), label }
            })
            .collect();

        let f = self.filter.to_lowercase();
        let mut items = Vec::new();

        for (gi, group) in self.groups.iter().enumerate() {
            let collapsed = self.group_collapsed.get(&group.key).copied().unwrap_or(false);
            items.push(Item::GroupHeader(gi));
            if collapsed {
                continue;
            }
            for (i, env) in self.envs.iter().enumerate() {
                if env.group != group.key {
                    continue;
                }
                if self.active_only && env.agents.is_empty() {
                    continue;
                }
                if !f.is_empty() {
                    let hay =
                        format!("{} {} {}", env.name, env.group, env_status_label(env))
                            .to_lowercase();
                    let agent_match = env.agents.iter().any(|a| {
                        format!("{} {}", a.tool, a.status).to_lowercase().contains(&f)
                    });
                    if !hay.contains(&f) && !agent_match {
                        continue;
                    }
                }
                items.push(Item::EnvRow(i));
                if env.expanded {
                    for j in 0..env.agents.len() {
                        items.push(Item::AgentRow(i, j));
                    }
                }
            }
        }

        self.view = items;
        // Prune marks for envs that have disappeared from the fleet.
        let env_names: HashSet<&String> = self.envs.iter().map(|e| &e.name).collect();
        self.marked.retain(|n| env_names.contains(n));
        self.clamp_selection();
    }

    fn clamp_selection(&mut self) {
        if self.view.is_empty() {
            self.list_state.select(None);
        } else {
            let s = self.list_state.selected().unwrap_or(0).min(self.view.len() - 1);
            self.list_state.select(Some(s));
        }
    }

    fn selected_item_cloned(&self) -> Option<Item> {
        let s = self.list_state.selected()?;
        self.view.get(s).cloned()
    }

    fn selected_env_name(&self) -> Option<String> {
        match self.selected_item_cloned()? {
            Item::GroupHeader(_) => None,
            Item::EnvRow(i) => Some(self.envs[i].name.clone()),
            Item::AgentRow(i, _) => Some(self.envs[i].name.clone()),
        }
    }

    /// Session ID of the selected Claude agent (for --resume on attach).
    fn selected_agent_session_id(&self) -> Option<String> {
        match self.selected_item_cloned()? {
            Item::AgentRow(i, j) => {
                self.envs[i].agents.get(j).and_then(|a| a.session_id.clone())
            }
            Item::EnvRow(i) => {
                self.envs[i].agents.first().and_then(|a| a.session_id.clone())
            }
            _ => None,
        }
    }

    /// Agent name label of selected agent (claude session name, if available).
    #[allow(dead_code)]
    fn selected_agent_name(&self) -> Option<String> {
        match self.selected_item_cloned()? {
            Item::AgentRow(i, j) => {
                self.envs[i].agents.get(j).and_then(|a| a.agent_name.clone())
            }
            Item::EnvRow(i) => {
                self.envs[i].agents.first().and_then(|a| a.agent_name.clone())
            }
            _ => None,
        }
    }

    /// Tool of the selected agent (or first agent of the selected env).
    fn selected_tool(&self) -> String {
        match self.selected_item_cloned() {
            Some(Item::EnvRow(i)) => {
                self.envs[i].agents.first().map(|a| a.tool.clone()).unwrap_or_default()
            }
            Some(Item::AgentRow(i, j)) => {
                self.envs[i].agents.get(j).map(|a| a.tool.clone()).unwrap_or_default()
            }
            _ => String::new(),
        }
    }

    fn move_sel(&mut self, delta: isize) {
        if self.view.is_empty() {
            return;
        }
        let len = self.view.len() as isize;
        let cur = self.list_state.selected().unwrap_or(0) as isize;
        self.list_state.select(Some((cur + delta).clamp(0, len - 1) as usize));
    }

    fn toggle_expand(&mut self) {
        let idx = match self.list_state.selected() {
            Some(i) => i,
            None => return,
        };
        match self.view.get(idx).cloned() {
            Some(Item::GroupHeader(gi)) => {
                let key = self.groups[gi].key.clone();
                let entry = self.group_collapsed.entry(key).or_insert(false);
                *entry = !*entry;
                self.rebuild_view();
                if let Some(pos) = self.view.iter().position(|it| *it == Item::GroupHeader(gi)) {
                    self.list_state.select(Some(pos));
                }
            }
            Some(Item::EnvRow(i)) => {
                if self.envs[i].agents.is_empty() {
                    self.set_flash("no agents");
                    return;
                }
                self.envs[i].expanded = !self.envs[i].expanded;
                self.rebuild_view();
                if let Some(pos) = self.view.iter().position(|it| *it == Item::EnvRow(i)) {
                    self.list_state.select(Some(pos));
                }
            }
            Some(Item::AgentRow(i, _)) => {
                self.envs[i].expanded = false;
                self.rebuild_view();
                if let Some(pos) = self.view.iter().position(|it| *it == Item::EnvRow(i)) {
                    self.list_state.select(Some(pos));
                }
            }
            None => {}
        }
    }

    fn after_action(&mut self) {
        self.request_refresh();
        self.request_git();
        self.log_target.clear();
        self.log_wanted = None;
    }

    fn maybe_request_logs(&mut self) {
        let (target, tool) = match self.selected_item_cloned() {
            Some(Item::EnvRow(i)) => {
                let e = &self.envs[i];
                let tool = e.agents.first().map(|a| a.tool.clone()).unwrap_or_default();
                (e.name.clone(), tool)
            }
            Some(Item::AgentRow(i, j)) => {
                let a = &self.envs[i].agents[j];
                (self.envs[i].name.clone(), a.tool.clone())
            }
            _ => return, // GroupHeader or no selection
        };
        if self.log_inflight.is_some() {
            return;
        }
        if tool == "claude" {
            if self.log_target != target {
                self.log_target = target;
                self.log_lines =
                    vec!["(claude logs to its own store — press 'a' to attach)".into()];
                self.log_wanted = None;
            }
            return;
        }
        if tool.is_empty() || tool == "-" {
            if self.log_target != target {
                self.log_target = target;
                self.log_lines = Vec::new();
                self.log_wanted = None;
            }
            return;
        }
        let is_new = self.log_target != target;
        let is_stale =
            self.log_target == target && self.last_log.elapsed() >= Duration::from_secs(4);
        if is_new {
            match &self.log_wanted {
                Some((t, since)) if *t == target => {
                    if since.elapsed() >= Duration::from_millis(250) {
                        self.send_log_req(target);
                    }
                }
                _ => self.log_wanted = Some((target, Instant::now())),
            }
        } else if is_stale {
            self.send_log_req(target);
        }
    }

    fn send_log_req(&mut self, target: String) {
        self.log_inflight = Some(target.clone());
        self.log_wanted = None;
        let _ = self.req_tx.send(Req::Logs(target));
    }

    /// Open the in-TUI log view for a target, forcing an immediate fetch.
    fn open_log_view(&mut self, target: String, tool: String) {
        self.log_follow = true;
        self.log_scroll = 0;
        self.log_wanted = None;
        if tool == "claude" {
            self.log_target = target;
            self.log_lines =
                vec!["(claude logs to its own store — press 'a' to attach)".into()];
        } else {
            // Force fetch unless already in-flight for this exact target
            if self.log_inflight.as_deref() != Some(target.as_str()) {
                self.log_target = target.clone();
                self.log_lines = vec!["(loading…)".into()];
                self.send_log_req(target);
            }
        }
        self.mode = Mode::LogView;
    }

    // ── input ────────────────────────────────────────────────────────────────

    fn handle_key(&mut self, key: KeyEvent, term: &mut Term) -> bool {
        match self.mode {
            Mode::Normal => return self.key_normal(key, term),
            Mode::Help => self.mode = Mode::Normal,
            Mode::Filter => self.key_filter(key),
            Mode::Dispatch => self.key_dispatch(key, term),
            Mode::ConfirmKill => self.key_confirm(key),
            Mode::LogView => self.key_logview(key),
            Mode::ActionMenu => self.key_action_menu(key, term),
            Mode::ToolPick => self.key_tool_pick(key, term),
            Mode::BatchMenu => self.key_batch_menu(key, term),
            Mode::ResultView => self.key_result_view(key),
            Mode::ModelPicker => self.key_model_picker(key),
        }
        false
    }

    fn key_logview(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Char('j') | KeyCode::Down => {
                self.log_follow = false;
                self.log_scroll = self.log_scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.log_follow = false;
                self.log_scroll = self.log_scroll.saturating_sub(1);
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.log_follow = false;
                self.log_scroll = 0;
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.log_follow = true;
            }
            KeyCode::PageDown => {
                self.log_follow = false;
                self.log_scroll = self.log_scroll.saturating_add(20);
            }
            KeyCode::PageUp => {
                self.log_follow = false;
                self.log_scroll = self.log_scroll.saturating_sub(20);
            }
            _ => {}
        }
    }

    fn key_normal(&mut self, key: KeyEvent, term: &mut Term) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return true;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Char('j') | KeyCode::Down => self.move_sel(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_sel(-1),
            KeyCode::Char('g') | KeyCode::Home => self.list_state.select(Some(0)),
            KeyCode::Char('G') | KeyCode::End if !self.view.is_empty() => {
                self.list_state.select(Some(self.view.len() - 1));
            }
            KeyCode::Char(' ') => self.toggle_expand(),
            KeyCode::Char('r') => {
                self.request_refresh();
                self.request_git();
                self.set_flash("refreshing…");
            }
            KeyCode::Char('/') => self.mode = Mode::Filter,
            KeyCode::Char('w') => {
                self.active_only = !self.active_only;
                self.rebuild_view();
                self.set_flash(if self.active_only {
                    "filter: active only"
                } else {
                    "filter: all"
                });
            }
            KeyCode::Char('?') => self.mode = Mode::Help,
            KeyCode::Enter => {
                match self.selected_item_cloned() {
                    Some(Item::GroupHeader(_)) => self.toggle_expand(),
                    Some(Item::EnvRow(_)) | Some(Item::AgentRow(_, _)) => {
                        self.menu_index = 0;
                        self.mode = Mode::ActionMenu;
                    }
                    None => {}
                }
            }
            KeyCode::Char('l') => {
                if let Some(t) = self.selected_env_name() {
                    run_dev(&["logs", &t, "-f"], term);
                    self.after_action();
                }
            }
            KeyCode::Char('a') => {
                if let Some(t) = self.selected_env_name() {
                    run_dev(&["attach", &t], term);
                    self.after_action();
                }
            }
            KeyCode::Char('D') => {
                if let Some(t) = self.selected_env_name() {
                    run_shell(&format!("dev diff {} | less -R", sh_quote(&t)), term);
                    self.after_action();
                }
            }
            KeyCode::Char('x') => match self.selected_item_cloned() {
                Some(Item::EnvRow(i)) => {
                    if self.envs[i].agents.is_empty() {
                        self.set_flash("no agents to kill");
                    } else {
                        self.mode = Mode::ConfirmKill;
                    }
                }
                Some(Item::AgentRow(_, _)) => self.mode = Mode::ConfirmKill,
                Some(Item::GroupHeader(_)) | None => {}
            },
            KeyCode::Char('d') => {
                if let Some(t) = self.selected_env_name() {
                    self.dispatch_target = t;
                    self.dispatch_tool = String::new();
                    self.tool_index = 0;
                    self.tool_purpose = ToolPurpose::Dispatch;
                    self.tool_prev_mode = Mode::Normal;
                    self.mode = Mode::ToolPick;
                }
            }
            // ── batch / triage ──────────────────────────────────────────────
            KeyCode::Char('m') => {
                match self.selected_item_cloned() {
                    Some(Item::EnvRow(i)) => {
                        let name = self.envs[i].name.clone();
                        if self.marked.contains(&name) {
                            self.marked.remove(&name);
                        } else {
                            self.marked.insert(name);
                        }
                    }
                    _ => self.set_flash("m: select an env row to mark"),
                }
            }
            KeyCode::Char('M') => {
                self.marked.clear();
                self.set_flash("marks cleared");
            }
            KeyCode::Char('b') => {
                if self.marked.is_empty() {
                    self.set_flash("no marks — press 'm' to mark envs first");
                } else {
                    self.batch_menu_index = 0;
                    self.mode = Mode::BatchMenu;
                }
            }
            KeyCode::Char('n') => self.jump_attention(1),
            KeyCode::Char('N') => self.jump_attention(-1),
            _ => {}
        }
        false
    }

    /// Jump to the next (dir=1) or previous (dir=-1) row with status
    /// waiting or error, wrapping around.
    fn jump_attention(&mut self, dir: isize) {
        if self.view.is_empty() {
            return;
        }
        let len = self.view.len();
        let start = self.list_state.selected().unwrap_or(0);
        let mut i = (start as isize + dir).rem_euclid(len as isize) as usize;
        for _ in 0..len {
            let status = match self.view.get(i) {
                Some(Item::EnvRow(ei)) => env_dominant_status(&self.envs[*ei]).to_owned(),
                Some(Item::AgentRow(ei, aj)) => self.envs[*ei].agents[*aj].status.clone(),
                _ => String::new(),
            };
            if matches!(status_rank(&status), 0 | 1) {
                self.list_state.select(Some(i));
                return;
            }
            i = (i as isize + dir).rem_euclid(len as isize) as usize;
        }
        self.set_flash("no waiting/error agents");
    }

    fn key_filter(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.filter.clear();
                self.mode = Mode::Normal;
                self.rebuild_view();
            }
            KeyCode::Enter => self.mode = Mode::Normal,
            KeyCode::Backspace => {
                self.filter.pop();
                self.rebuild_view();
            }
            KeyCode::Char(c) => {
                self.filter.push(c);
                self.rebuild_view();
            }
            _ => {}
        }
    }

    fn key_dispatch(&mut self, key: KeyEvent, term: &mut Term) {
        match key.code {
            KeyCode::Esc => {
                self.dispatch_targets.clear();
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                let task = self.dispatch_input.trim().to_string();
                let tool = self.dispatch_tool.clone();
                let model = self.dispatch_model.clone();
                let effort = self.dispatch_effort.clone();
                self.mode = Mode::Normal;
                if !task.is_empty() {
                    // Build extra flags for model/effort.
                    let mut extra: Vec<String> = Vec::new();
                    if !model.is_empty() {
                        extra.push("--model".into());
                        extra.push(model.clone());
                    }
                    if !effort.is_empty() {
                        extra.push("--effort".into());
                        extra.push(effort.clone());
                    }
                    if !self.dispatch_targets.is_empty() {
                        let targets = std::mem::take(&mut self.dispatch_targets);
                        for t in &targets {
                            let mut args = vec!["dispatch", t];
                            if !tool.is_empty() {
                                args.extend_from_slice(&["--tool", &tool]);
                            }
                            let extra_refs: Vec<&str> = extra.iter().map(|s| s.as_str()).collect();
                            args.extend_from_slice(&extra_refs);
                            args.push(&task);
                            run_dev(&args, term);
                        }
                        self.set_flash(&format!("dispatched → {} envs", targets.len()));
                    } else {
                        let target = self.dispatch_target.clone();
                        let mut args = vec!["dispatch", &target];
                        if !tool.is_empty() {
                            args.extend_from_slice(&["--tool", &tool]);
                        }
                        let extra_refs: Vec<&str> = extra.iter().map(|s| s.as_str()).collect();
                        args.extend_from_slice(&extra_refs);
                        args.push(&task);
                        run_dev(&args, term);
                        self.set_flash(&format!("dispatched → {target}"));
                    }
                    self.after_action();
                }
            }
            KeyCode::Backspace => {
                self.dispatch_input.pop();
            }
            KeyCode::Char(c) => self.dispatch_input.push(c),
            _ => {}
        }
    }

    fn key_confirm(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => {
                match self.selected_item_cloned() {
                    Some(Item::EnvRow(i)) => {
                        let name = self.envs[i].name.clone();
                        let _ = Command::new("dev").args(["kill", &name]).spawn();
                        self.set_flash(&format!("killing {name}…"));
                    }
                    Some(Item::AgentRow(i, j)) => {
                        let pid = self.envs[i].agents[j].pid.clone();
                        if !pid.is_empty() {
                            let _ = Command::new("kill").arg(&pid).spawn();
                            self.set_flash(&format!("killing pid {pid}…"));
                        }
                    }
                    Some(Item::GroupHeader(_)) | None => {}
                }
                self.mode = Mode::Normal;
                self.after_action();
            }
            _ => self.mode = Mode::Normal,
        }
    }

    fn key_action_menu(&mut self, key: KeyEvent, term: &mut Term) {
        const N: usize = ACTION_MENU_ITEMS.len();
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Char('j') | KeyCode::Down => {
                self.menu_index = (self.menu_index + 1) % N;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.menu_index = if self.menu_index == 0 { N - 1 } else { self.menu_index - 1 };
            }
            KeyCode::Enter => {
                let target = match self.selected_env_name() {
                    Some(t) => t,
                    None => {
                        self.mode = Mode::Normal;
                        return;
                    }
                };
                let tool = self.selected_tool();
                match self.menu_index {
                    0 => {
                        // attach — use session_id for claude resume
                        self.mode = Mode::Normal;
                        let session = self.selected_agent_session_id();
                        if let Some(sid) = session {
                            run_dev(&["attach", &target, "--session", &sid], term);
                        } else {
                            run_dev(&["attach", &target], term);
                        }
                        self.after_action();
                    }
                    1 => {
                        // dispatch → tool picker
                        self.dispatch_target = target;
                        self.dispatch_tool = String::new();
                        self.tool_index = 0;
                        self.tool_purpose = ToolPurpose::Dispatch;
                        self.tool_prev_mode = Mode::ActionMenu;
                        self.mode = Mode::ToolPick;
                    }
                    2 => {
                        // start tool (interactive)
                        self.dispatch_target = target;
                        self.tool_index = 0;
                        self.tool_purpose = ToolPurpose::Start;
                        self.tool_prev_mode = Mode::ActionMenu;
                        self.mode = Mode::ToolPick;
                    }
                    3 => {
                        // review — background fetch → ResultView
                        self.mode = Mode::Normal;
                        self.result_title = format!("review: {target} (loading…)");
                        self.result_lines = vec!["running dev review, please wait…".into()];
                        self.result_scroll = 0;
                        self.result_inflight = true;
                        self.mode = Mode::ResultView;
                        let _ = self.req_tx.send(Req::Review {
                            target: target.clone(),
                            tool: tool.clone(),
                        });
                    }
                    4 => {
                        // model picker for next dispatch
                        self.dispatch_target = target.clone();
                        self.model_pick_index = 0;
                        self.mode = Mode::ModelPicker;
                    }
                    5 => {
                        // logs (TUI内)
                        self.open_log_view(target, tool);
                    }
                    6 => {
                        // diff
                        self.mode = Mode::Normal;
                        run_shell(&format!("dev diff {} | less -R", sh_quote(&target)), term);
                        self.after_action();
                    }
                    7 => {
                        // kill
                        self.mode = Mode::ConfirmKill;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn key_tool_pick(&mut self, key: KeyEvent, term: &mut Term) {
        let tools = self.tools_for_picker(self.tool_purpose);
        let n = tools.len().max(1);
        match key.code {
            KeyCode::Esc => self.mode = self.tool_prev_mode,
            KeyCode::Char('j') | KeyCode::Down => {
                self.tool_index = (self.tool_index + 1) % n;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.tool_index = if self.tool_index == 0 { n - 1 } else { self.tool_index - 1 };
            }
            KeyCode::Enter => {
                let tool = tools.get(self.tool_index).cloned().unwrap_or_default();
                let target = self.dispatch_target.clone();
                match self.tool_purpose {
                    ToolPurpose::Start => {
                        self.mode = Mode::Normal;
                        run_dev(&[&tool, &target], term);
                        self.after_action();
                    }
                    ToolPurpose::Dispatch => {
                        self.dispatch_tool = tool;
                        self.dispatch_input.clear();
                        self.mode = Mode::Dispatch;
                    }
                }
            }
            _ => {}
        }
    }

    fn key_result_view(&mut self, key: KeyEvent) {
        let page = 20usize;
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc if !self.result_inflight => {
                self.mode = Mode::Normal;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.result_scroll = self.result_scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.result_scroll = self.result_scroll.saturating_sub(1);
            }
            KeyCode::PageDown => {
                self.result_scroll = self.result_scroll.saturating_add(page);
            }
            KeyCode::PageUp => {
                self.result_scroll = self.result_scroll.saturating_sub(page);
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.result_scroll = 0;
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.result_scroll = self.result_lines.len().saturating_sub(1);
            }
            _ => {}
        }
    }

    fn key_model_picker(&mut self, key: KeyEvent) {
        let tool = self.dispatch_tool.clone();
        let tool = if tool.is_empty() { "claude".to_string() } else { tool };
        let models = self.models_for_picker(&tool);
        let n = models.len().max(1);
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::ActionMenu;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.model_pick_index = (self.model_pick_index + 1) % n;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.model_pick_index =
                    if self.model_pick_index == 0 { n - 1 } else { self.model_pick_index - 1 };
            }
            KeyCode::Enter => {
                if let Some((_, model_id)) = models.get(self.model_pick_index) {
                    self.dispatch_model = model_id.clone();
                    self.set_flash(&format!("model → {}", if model_id.is_empty() { "default" } else { model_id }));
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Delete | KeyCode::Char('c') => {
                // Clear model override.
                self.dispatch_model.clear();
                self.dispatch_effort.clear();
                self.set_flash("model cleared (next dispatch uses tool default)");
                self.mode = Mode::Normal;
            }
            _ => {}
        }
    }
}

// ── rendering ─────────────────────────────────────────────────────────────────

fn centered_rect(px: u16, py: u16, area: Rect) -> Rect {
    let v = Layout::vertical([
        Constraint::Percentage((100 - py) / 2),
        Constraint::Percentage(py),
        Constraint::Percentage((100 - py) / 2),
    ])
    .split(area);
    Layout::horizontal([
        Constraint::Percentage((100 - px) / 2),
        Constraint::Percentage(px),
        Constraint::Percentage((100 - px) / 2),
    ])
    .split(v[1])[1]
}

fn ui(f: &mut Frame, app: &mut App) {
    let v = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .split(f.area());
    render_summary(f, v[0], app);
    if app.mode == Mode::LogView {
        render_log_view(f, v[1], app);
    } else {
        let h = Layout::horizontal([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(v[1]);
        render_fleet(f, h[0], app);
        render_detail(f, h[1], app);
    }
    render_bottom(f, v[2], app);
    match app.mode {
        Mode::Help => render_help(f),
        Mode::Dispatch => render_dispatch(f, app),
        Mode::ConfirmKill => render_confirm(f, app),
        Mode::ActionMenu => render_action_menu(f, app),
        Mode::ToolPick => render_tool_pick(f, app),
        Mode::BatchMenu => render_batch_menu(f, app),
        Mode::ResultView => render_result_view(f, app),
        Mode::ModelPicker => render_model_picker(f, app),
        _ => {}
    }
}

fn render_summary(f: &mut Frame, area: Rect, app: &App) {
    let order: [(&str, Color); 5] = [
        ("waiting", Color::Yellow),
        ("error", Color::Red),
        ("running", Color::Green),
        ("idle", Color::Gray),
        ("stopped", Color::DarkGray),
    ];
    let mut counts: HashMap<&str, usize> = HashMap::new();
    let mut total_agents = 0usize;
    for env in &app.envs {
        for a in &env.agents {
            let k = if a.status == "busy" { "running" } else { a.status.as_str() };
            *counts.entry(k).or_insert(0) += 1;
            total_agents += 1;
        }
    }
    let usage_spans: Vec<Span<'static>> = if let Some(u) = &app.claude_usage {
        let pct_style = |pct: u32| -> Style {
            if pct >= 85 { Style::default().fg(Color::Red) }
            else if pct >= 70 { Style::default().fg(Color::Yellow) }
            else { Style::default().fg(Color::Green) }
        };
        let mut s: Vec<Span<'static>> = vec![
            Span::styled("│ claude ", Style::default().fg(Color::DarkGray)),
        ];
        if let Some(p) = u.five_hour_pct {
            s.push(Span::styled("5h:", Style::default().fg(Color::DarkGray)));
            s.push(Span::styled(format!("{p}%"), pct_style(p)));
            s.push(Span::raw(" "));
        }
        if let Some(p) = u.seven_day_pct {
            s.push(Span::styled("7d:", Style::default().fg(Color::DarkGray)));
            s.push(Span::styled(format!("{p}%"), pct_style(p)));
            s.push(Span::raw(" "));
        }
        s
    } else {
        vec![]
    };

    let mut spans: Vec<Span<'static>> = vec![
        Span::styled(
            " dev tui ",
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
    ];
    for (name, col) in order {
        if let Some(&c) = counts.get(name) {
            if c > 0 {
                spans.push(Span::styled(
                    format!("● {c} {name}  "),
                    Style::default().fg(col),
                ));
            }
        }
    }
    let n_projects = app.envs.len();
    let n_envs = app.groups.len();
    spans.push(Span::styled(
        format!("· {n_envs} envs {n_projects} projects {total_agents} agents "),
        Style::default().fg(Color::DarkGray),
    ));
    if !app.marked.is_empty() {
        let n = app.marked.len();
        spans.push(Span::styled(
            format!("✓{n} marked "),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));
    }
    if app.refreshing || app.git_inflight {
        spans.push(Span::styled(
            format!(" {} sync", SPIN[(app.spinner as usize) % SPIN.len()]),
            Style::default().fg(Color::Cyan),
        ));
    } else {
        spans.push(Span::styled(
            format!(" ⟳ {}s", app.last_refresh.elapsed().as_secs()),
            Style::default().fg(Color::DarkGray),
        ));
    }
    if app.active_only {
        spans.push(Span::styled(" [active]", Style::default().fg(Color::Cyan)));
    }
    spans.extend(usage_spans);

    // agy usage
    if let Some(u) = &app.agy_usage {
        let pct_style = |pct: u32| -> Style {
            if pct >= 85 { Style::default().fg(Color::Red) }
            else if pct >= 70 { Style::default().fg(Color::Yellow) }
            else { Style::default().fg(Color::Green) }
        };
        spans.push(Span::styled("│ agy ", Style::default().fg(Color::DarkGray)));
        if let (Some(avail), Some(monthly)) = (u.available_credits, u.monthly_credits) {
            let used_pct = ((monthly.saturating_sub(avail)) * 100 / monthly.max(1)) as u32;
            spans.push(Span::styled(format!("{used_pct}%"), pct_style(used_pct)));
            spans.push(Span::styled(format!(" ({avail}cr) "), Style::default().fg(Color::DarkGray)));
        } else if !u.models.is_empty() {
            for (label, pct) in u.models.iter().take(2) {
                let short = label.split_whitespace().next().unwrap_or(label);
                spans.push(Span::styled(format!("{short}:"), Style::default().fg(Color::DarkGray)));
                spans.push(Span::styled(format!("{pct}% "), pct_style(*pct)));
            }
        }
    }

    // codex usage
    if let Some(u) = &app.codex_usage {
        let pct_style = |pct: f64| -> Style {
            if pct >= 85.0 { Style::default().fg(Color::Red) }
            else if pct >= 70.0 { Style::default().fg(Color::Yellow) }
            else { Style::default().fg(Color::Green) }
        };
        spans.push(Span::styled("│ codex ", Style::default().fg(Color::DarkGray)));
        if let Some(p) = u.primary_used_pct {
            spans.push(Span::styled("5h:", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(format!("{p:.0}%"), pct_style(p)));
            spans.push(Span::raw(" "));
        }
        if let Some(p) = u.secondary_used_pct {
            spans.push(Span::styled("7d:", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(format!("{p:.0}%"), pct_style(p)));
            spans.push(Span::raw(" "));
        }
    }

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn make_fleet_items(
    view: &[Item],
    envs: &[Env],
    git: &HashMap<String, GitState>,
    groups: &[GroupInfo],
    group_collapsed: &HashMap<String, bool>,
    marked: &HashSet<String>,
    inner_w: u16,
) -> Vec<ListItem<'static>> {
    // layout after the "▶ " highlight symbol (2 chars):
    // expand(2) + dot(2) + name(flexible) + status(14) + delta(5)
    let w = inner_w as usize;
    let fixed = 2 + 2 + 14 + 5;
    let name_w = w.saturating_sub(fixed + 2 /* highlight sym */);

    view.iter()
        .map(|item| match item {
            Item::GroupHeader(gi) => {
                let g = &groups[*gi];
                let collapsed = group_collapsed.get(&g.key).copied().unwrap_or(false);
                let arrow = if collapsed { "▸" } else { "▾" };
                let label = if collapsed {
                    let count = envs.iter().filter(|e| e.group == g.key).count();
                    format!("{} {}  ({})", arrow, g.label, count)
                } else {
                    format!("{} {}", arrow, g.label)
                };
                ListItem::new(Line::from(Span::styled(
                    label,
                    Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
                )))
            }
            Item::EnvRow(i) => {
                let env = &envs[*i];
                let dom = env_dominant_status(env);
                let label = env_status_label(env);
                let expand = if env.agents.is_empty() {
                    "  "
                } else if env.expanded {
                    "▾ "
                } else {
                    "▸ "
                };
                let g = git.get(&env.name);
                let (delta, dstyle) = match g {
                    Some(gs) if gs.changes > 0 => (
                        format!("Δ{:<3}", gs.changes),
                        Style::default().fg(Color::Yellow),
                    ),
                    Some(gs) if gs.changes == 0 => {
                        ("·    ".into(), Style::default().fg(Color::DarkGray))
                    }
                    Some(_) => (" ".into(), Style::default()),
                    None => ("…    ".into(), Style::default().fg(Color::DarkGray)),
                };
                let name = truncate(&env.name, name_w.max(6));
                let name_padded = format!("{:<width$}", name, width = name_w.max(6));
                let status_padded = format!("{:<13}", label);
                let is_marked = marked.contains(&env.name);
                let dot_span = if is_marked {
                    Span::styled("✓ ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                } else {
                    Span::styled("● ", status_style(dom))
                };
                ListItem::new(Line::from(vec![
                    Span::raw(expand),
                    dot_span,
                    Span::raw(name_padded),
                    Span::styled(status_padded, status_style(dom)),
                    Span::styled(delta, dstyle),
                ]))
            }
            Item::AgentRow(i, j) => {
                let env = &envs[*i];
                let a = &env.agents[*j];
                let is_last = *j + 1 == env.agents.len();
                let connector = if is_last { "└ " } else { "├ " };
                let tool = truncate(&a.tool, 7);
                let pid_s = if a.pid.is_empty() {
                    "-".into()
                } else {
                    format!("#{}", a.pid)
                };
                let pid_s = truncate(&pid_s, 9);
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  {connector}"),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(format!("{:<7}", tool), Style::default().fg(Color::Cyan)),
                    Span::styled(format!(" {:<9}", pid_s), Style::default().fg(Color::DarkGray)),
                    Span::styled("● ", status_style(&a.status)),
                    Span::styled(a.status.clone(), status_style(&a.status)),
                ]))
            }
        })
        .collect()
}

fn render_fleet(f: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default().borders(Borders::ALL).title(" fleet ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let items = make_fleet_items(
        &app.view,
        &app.envs,
        &app.git,
        &app.groups,
        &app.group_collapsed,
        &app.marked,
        inner.width,
    );
    let list = List::new(items)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("▶ ");
    f.render_stateful_widget(list, inner, &mut app.list_state);
}

fn render_detail(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default().borders(Borders::ALL).title(" detail ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);
    let label = Style::default().fg(Color::DarkGray);
    let section = Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD);
    let w = inner.width as usize;

    let mut lines: Vec<Line<'static>> = Vec::new();

    let sel = app.list_state.selected().and_then(|s| app.view.get(s)).cloned();

    let log_env_name = match &sel {
        Some(Item::EnvRow(i)) => app.envs[*i].name.clone(),
        Some(Item::AgentRow(i, _)) => app.envs[*i].name.clone(),
        _ => String::new(),
    };

    match sel {
        None | Some(Item::GroupHeader(_)) => {
            f.render_widget(
                Paragraph::new(Span::styled("no selection", dim)),
                inner,
            );
            return;
        }
        Some(Item::EnvRow(i)) => {
            let env = &app.envs[i];
            let dom = env_dominant_status(env);
            lines.push(Line::from(Span::styled(
                env.name.clone(),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(vec![
                Span::styled("● ", status_style(dom)),
                Span::styled(env_status_label(env), status_style(dom)),
                Span::styled(format!("  {}", env.group), dim),
            ]));
            if !env.path.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("path ", label),
                    Span::styled(truncate(&env.path, w.saturating_sub(5)), dim),
                ]));
            }

            if !env.agents.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled("agents", section)));
                for a in &env.agents {
                    let pid_s = if a.pid.is_empty() {
                        "-".into()
                    } else {
                        format!("#{}", a.pid)
                    };
                    lines.push(Line::from(vec![
                        Span::styled("  ● ", status_style(&a.status)),
                        Span::styled(format!("{:<7}", a.tool), Style::default().fg(Color::Cyan)),
                        Span::styled(format!(" {:<9}", pid_s), dim),
                        Span::styled(a.status.clone(), status_style(&a.status)),
                    ]));
                    if !a.kind.is_empty() {
                        lines.push(Line::from(vec![
                            Span::raw("         "),
                            Span::styled(a.kind.clone(), dim),
                        ]));
                    }
                }
            }

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("git", section)));
            if let Some(g) = app.git.get(&env.name) {
                let chstyle = if g.changes > 0 {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Green)
                };
                lines.push(Line::from(vec![
                    Span::styled("branch ", label),
                    Span::raw(if g.branch.is_empty() { "-".into() } else { g.branch.clone() }),
                    Span::raw("  "),
                    Span::styled(format!("Δ{}", g.changes), chstyle),
                ]));
                if !g.head.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("head   ", label),
                        Span::styled(truncate(&g.head, w.saturating_sub(7)), dim),
                    ]));
                }
            } else {
                lines.push(Line::from(Span::styled("(loading…)", dim)));
            }
        }
        Some(Item::AgentRow(i, j)) => {
            let env = &app.envs[i];
            let a = &env.agents[j];
            // Header: use session name when available (claude), else tool + pid.
            let header_str = if let Some(nm) = &a.agent_name {
                format!("{} · {}", a.tool, nm)
            } else {
                format!("{} #{}", a.tool, a.pid)
            };
            lines.push(Line::from(Span::styled(
                header_str,
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(vec![
                Span::styled("● ", status_style(&a.status)),
                Span::styled(a.status.clone(), status_style(&a.status)),
                if !a.kind.is_empty() {
                    Span::styled(format!("  {}", a.kind), dim)
                } else {
                    Span::raw("")
                },
            ]));
            lines.push(Line::from(vec![
                Span::styled("env  ", label),
                Span::raw(env.name.clone()),
                Span::styled(format!("  pid #{}", a.pid), dim),
            ]));
            if !a.cwd.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("cwd  ", label),
                    Span::styled(truncate(&a.cwd, w.saturating_sub(5)), dim),
                ]));
            }
            if let Some(sid) = &a.session_id {
                if !sid.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("sid  ", label),
                        Span::styled(truncate(sid, w.saturating_sub(5)), dim),
                    ]));
                }
            }
            if let Some(model) = &a.model {
                if !model.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("mdl  ", label),
                        Span::styled(model.clone(), Style::default().fg(Color::Cyan)),
                    ]));
                }
            }
        }
    }

    // Log tail
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("recent log", section)));
    let header = lines.len();
    let avail = (inner.height as usize).saturating_sub(header).max(1);
    if app.log_target == log_env_name {
        if app.log_lines.is_empty() {
            lines.push(Line::from(Span::styled("(no logs)", dim)));
        } else {
            let start = app.log_lines.len().saturating_sub(avail);
            for l in &app.log_lines[start..] {
                lines.push(Line::from(Span::styled(truncate(l, w), dim)));
            }
        }
    } else {
        lines.push(Line::from(Span::styled("(loading…)", dim)));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn render_bottom(f: &mut Frame, area: Rect, app: &App) {
    let line = if app.mode == Mode::Filter {
        Line::from(vec![
            Span::styled(" filter ", Style::default().bg(Color::Cyan).fg(Color::Black)),
            Span::raw(format!(" /{}", app.filter)),
            Span::styled("▏", Style::default().fg(Color::Cyan)),
        ])
    } else if let Some((msg, t)) = &app.flash {
        if t.elapsed() < Duration::from_secs(3) {
            Line::from(Span::styled(
                format!(" {msg}"),
                Style::default().fg(Color::Green),
            ))
        } else {
            help_line(app)
        }
    } else {
        help_line(app)
    };
    f.render_widget(Paragraph::new(line), area);
}

fn help_line(app: &App) -> Line<'static> {
    let hint: &'static str = match app.mode {
        Mode::LogView => " j/k scroll · g top · G follow · PgDn/PgUp · esc/q back",
        Mode::ActionMenu => " j/k 移動 · enter 実行 · esc キャンセル",
        Mode::ToolPick => " j/k ツール選択 · enter 確定 · esc 戻る",
        Mode::BatchMenu => " j/k 移動 · enter 実行 · esc キャンセル",
        _ => {
            let sel = app.list_state.selected().and_then(|s| app.view.get(s)).cloned();
            match sel {
                Some(Item::GroupHeader(_)) => {
                    " space 開閉 · j/k 移動 · / filter · w active · r refresh · ? help · q quit"
                }
                Some(Item::AgentRow(_, _)) => {
                    " enter メニュー · space 畳む · l logs-f · a attach · x kill pid · n/N 要注目 · r refresh · ? help · q quit"
                }
                _ => " enter メニュー · m mark · b batch · n/N 要注目 · l logs-f · a attach · d dispatch · x kill · D diff · / filter · w active · r refresh · ? help · q quit",
            }
        }
    };
    Line::from(Span::styled(hint, Style::default().fg(Color::DarkGray)))
}

fn kv(k: &str, v: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {k:<12}"), Style::default().fg(Color::Cyan)),
        Span::raw(v.to_string()),
    ])
}

fn render_help(f: &mut Frame) {
    let area = centered_rect(60, 80, f.area());
    f.render_widget(Clear, area);
    let lines = vec![
        Line::from(Span::styled(
            "dev top — keys",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("  navigation", Style::default().fg(Color::Blue))),
        kv("j/k ↑/↓", "move selection"),
        kv("g / G", "first / last"),
        kv("space", "expand/collapse group or env"),
        Line::from(""),
        Line::from(Span::styled("  actions (enter = menu)", Style::default().fg(Color::Blue))),
        kv("enter", "action menu (attach/dispatch/start/logs/diff/kill)"),
        kv("l", "follow logs (dev logs -f, TUI外)"),
        kv("a", "attach (dev attach)"),
        kv("d", "dispatch — tool選択 → task入力"),
        kv("x", "kill agents (dev kill / kill <pid>)"),
        kv("D", "view diff (dev diff | less)"),
        Line::from(""),
        Line::from(Span::styled("  batch (env rows)", Style::default().fg(Color::Blue))),
        kv("m", "toggle mark on selected env"),
        kv("M", "clear all marks"),
        kv("b", "batch menu (dispatch/diff/kill/clear)"),
        kv("n / N", "jump to next/prev waiting or error"),
        Line::from(""),
        Line::from(Span::styled("  view", Style::default().fg(Color::Blue))),
        kv("/", "filter by text"),
        kv("w", "toggle active-only"),
        kv("r", "refresh now"),
        kv("?", "toggle this help"),
        kv("q / esc", "quit"),
        Line::from(""),
        Line::from(Span::styled(
            "press any key to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    f.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" help ")),
        area,
    );
}

/// Single source of truth for the per-env action menu.
/// `key_action_menu` and `render_action_menu` both derive N from `.len()`.
const ACTION_MENU_ITEMS: [(&str, &str); 8] = [
    ("attach", "dev attach"),
    ("dispatch  (tool → task)", "dev dispatch --tool"),
    ("start tool  (interactive)", "dev <tool>"),
    ("review  (code review)", "dev review"),
    ("model picker  (next dispatch)", "set --model flag"),
    ("logs", "TUI内ログ表示"),
    ("diff", "dev diff | less"),
    ("kill", "dev kill / kill <pid>"),
];

fn render_action_menu(f: &mut Frame, app: &App) {
    let target = app.selected_env_name().unwrap_or_else(|| "?".into());
    let area = centered_rect(52, 68, f.area());
    f.render_widget(Clear, area);

    let mut lines = vec![Line::from("")];
    for (i, (lbl, hint)) in ACTION_MENU_ITEMS.iter().enumerate() {
        let selected = i == app.menu_index;
        let prefix = if selected { "▶ " } else { "  " };
        let sty = if selected {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{}{}", prefix, lbl), sty),
            Span::styled(
                format!("   {}", hint),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " j/k 移動 · enter 実行 · esc キャンセル",
        Style::default().fg(Color::DarkGray),
    )));

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", target)),
        ),
        area,
    );
}

fn render_tool_pick(f: &mut Frame, app: &App) {
    let purpose_label = match app.tool_purpose {
        ToolPurpose::Start => "start tool",
        ToolPurpose::Dispatch => "dispatch",
    };
    let tools = app.tools_for_picker(app.tool_purpose);
    let area = centered_rect(44, 52, f.area());
    f.render_widget(Clear, area);

    let mut lines = vec![Line::from("")];
    for (i, tool) in tools.iter().enumerate() {
        let selected = i == app.tool_index;
        let prefix = if selected { "▶ " } else { "  " };
        let sty = if selected {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(
            format!("{}{}", prefix, tool),
            sty,
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " j/k 移動 · enter 確定 · esc 戻る",
        Style::default().fg(Color::DarkGray),
    )));

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} → {} ", purpose_label, app.dispatch_target)),
        ),
        area,
    );
}

fn render_dispatch(f: &mut Frame, app: &App) {
    let area = centered_rect(64, 32, f.area());
    f.render_widget(Clear, area);
    let tool_display = if app.dispatch_tool.is_empty() {
        "claude  (default)".to_string()
    } else {
        app.dispatch_tool.clone()
    };
    let target_display = if !app.dispatch_targets.is_empty() {
        format!("{} envs", app.dispatch_targets.len())
    } else {
        app.dispatch_target.clone()
    };
    let model_display = if app.dispatch_model.is_empty() {
        "(tool default)".to_string()
    } else {
        app.dispatch_model.clone()
    };
    let effort_display = if app.dispatch_effort.is_empty() {
        String::new()
    } else {
        format!("  effort: {}", app.dispatch_effort)
    };
    let lines = vec![
        Line::from(vec![
            Span::styled("dispatch → ", Style::default().fg(Color::Cyan)),
            Span::styled(
                target_display,
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("tool:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(tool_display, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("model:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(model_display, Style::default().fg(Color::Yellow)),
            Span::styled(effort_display, Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("task:   ", Style::default().fg(Color::DarkGray)),
            Span::raw(app.dispatch_input.clone()),
            Span::styled("▏", Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "enter: run    esc: cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" dispatch "))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_log_view(f: &mut Frame, area: Rect, app: &App) {
    let title = if app.log_target.is_empty() {
        " log ".to_string()
    } else {
        format!(" log: {} ", app.log_target)
    };
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);
    let h = inner.height as usize;
    let w = inner.width as usize;

    if app.log_lines.is_empty() {
        f.render_widget(Paragraph::new(Span::styled("(no logs)", dim)), inner);
        return;
    }

    // Wrap all logical lines at inner width
    let rows: Vec<String> = app
        .log_lines
        .iter()
        .flat_map(|l| wrap_line(l, w))
        .collect();
    let total = rows.len();
    let max_top = total.saturating_sub(h);
    let top = if app.log_follow {
        max_top
    } else {
        app.log_scroll.min(max_top)
    };

    let lines: Vec<Line> = rows[top..(top + h).min(total)]
        .iter()
        .map(|l| Line::from(Span::styled(l.clone(), dim)))
        .collect();

    f.render_widget(Paragraph::new(lines), inner);
}

fn render_confirm(f: &mut Frame, app: &App) {
    let area = centered_rect(52, 24, f.area());
    f.render_widget(Clear, area);
    let sel = app.list_state.selected().and_then(|s| app.view.get(s)).cloned();
    let (action, target) = match sel {
        Some(Item::EnvRow(i)) => ("kill all agents in", app.envs[i].name.clone()),
        Some(Item::AgentRow(i, j)) => ("kill pid", app.envs[i].agents[j].pid.clone()),
        _ => return,
    };
    let lines = vec![
        Line::from(vec![
            Span::styled(format!("{action} "), Style::default().fg(Color::Red)),
            Span::styled(target, Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" ?"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "y / enter: yes      n / esc: no",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" confirm ")
                .border_style(Style::default().fg(Color::Red)),
        ),
        area,
    );
}

// ── batch menu ────────────────────────────────────────────────────────────────

impl App {
    fn key_batch_menu(&mut self, key: KeyEvent, term: &mut Term) {
        const N: usize = 4;
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Char('j') | KeyCode::Down => {
                self.batch_menu_index = (self.batch_menu_index + 1) % N;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.batch_menu_index =
                    if self.batch_menu_index == 0 { N - 1 } else { self.batch_menu_index - 1 };
            }
            KeyCode::Enter => match self.batch_menu_index {
                0 => {
                    // dispatch → ToolPick → Dispatch (batch mode)
                    let mut targets: Vec<String> = self.marked.iter().cloned().collect();
                    targets.sort();
                    self.dispatch_targets = targets;
                    self.dispatch_tool = String::new();
                    self.dispatch_input.clear();
                    self.tool_index = 0;
                    self.tool_purpose = ToolPurpose::Dispatch;
                    self.tool_prev_mode = Mode::BatchMenu;
                    self.mode = Mode::ToolPick;
                }
                1 => {
                    // diff all marked — build a pager pipeline
                    let mut targets: Vec<String> = self.marked.iter().cloned().collect();
                    targets.sort();
                    let parts: Vec<String> = targets
                        .iter()
                        .map(|t| format!("echo '=== {} ==='; dev diff {}", t, sh_quote(t)))
                        .collect();
                    let cmd = format!("{{ {}; }} 2>&1 | less -R", parts.join("; "));
                    self.mode = Mode::Normal;
                    run_shell(&cmd, term);
                    self.after_action();
                }
                2 => {
                    // kill all marked (fire-and-forget, no second confirm)
                    let targets: Vec<String> = self.marked.iter().cloned().collect();
                    self.mode = Mode::Normal;
                    for t in &targets {
                        let _ = Command::new("dev").args(["kill", t]).spawn();
                    }
                    self.set_flash(&format!("killing {} envs…", targets.len()));
                    self.after_action();
                }
                3 => {
                    // clear all marks
                    self.marked.clear();
                    self.mode = Mode::Normal;
                    self.set_flash("marks cleared");
                }
                _ => {}
            },
            _ => {}
        }
    }
}

fn render_batch_menu(f: &mut Frame, app: &App) {
    let n = app.marked.len();
    let area = centered_rect(54, 60, f.area());
    f.render_widget(Clear, area);

    let actions: [(&str, &str); 4] = [
        ("dispatch  (tool → task)", "dev dispatch each"),
        ("diff", "dev diff | less per env"),
        ("kill", "dev kill each (no confirm)"),
        ("clear marks", "deselect all"),
    ];

    let mut lines = vec![Line::from("")];
    for (i, (lbl, hint)) in actions.iter().enumerate() {
        let selected = i == app.batch_menu_index;
        let prefix = if selected { "▶ " } else { "  " };
        let sty = if selected {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{}{}", prefix, lbl), sty),
            Span::styled(format!("   {}", hint), Style::default().fg(Color::DarkGray)),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " j/k 移動 · enter 実行 · esc キャンセル",
        Style::default().fg(Color::DarkGray),
    )));

    // List marked envs at the bottom for confirmation
    let mut env_list: Vec<String> = app.marked.iter().cloned().collect();
    env_list.sort();
    lines.push(Line::from(""));
    for e in env_list.iter().take(6) {
        lines.push(Line::from(Span::styled(
            format!("  ✓ {}", e),
            Style::default().fg(Color::Cyan),
        )));
    }
    if env_list.len() > 6 {
        lines.push(Line::from(Span::styled(
            format!("  … {} more", env_list.len() - 6),
            Style::default().fg(Color::DarkGray),
        )));
    }

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" batch — {} envs marked ", n)),
        ),
        area,
    );
}

fn render_result_view(f: &mut Frame, app: &App) {
    let title = if app.result_inflight {
        format!(" {} … ", app.result_title)
    } else {
        format!(" {} ", app.result_title)
    };
    let area = centered_rect(88, 86, f.area());
    f.render_widget(Clear, area);
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);
    let h = inner.height as usize;
    let w = inner.width as usize;

    let rows: Vec<String> = app.result_lines.iter().flat_map(|l| wrap_line(l, w)).collect();
    let total = rows.len();
    let max_top = total.saturating_sub(h.saturating_sub(1));
    let top = app.result_scroll.min(max_top);
    let mut lines: Vec<Line> = rows[top..(top + h.saturating_sub(1)).min(total)]
        .iter()
        .map(|l| Line::from(Span::styled(l.clone(), dim)))
        .collect();

    let hint = if app.result_inflight {
        "(running…)".to_string()
    } else {
        format!("q/esc: close   j/k: scroll   {}/{} lines", top + 1, total.max(1))
    };
    lines.push(Line::from(Span::styled(hint, Style::default().fg(Color::DarkGray))));
    f.render_widget(Paragraph::new(lines), inner);
}

fn render_model_picker(f: &mut Frame, app: &App) {
    let tool = if app.dispatch_tool.is_empty() { "claude" } else { &app.dispatch_tool };
    let models = app.models_for_picker(tool);
    let area = centered_rect(56, 62, f.area());
    f.render_widget(Clear, area);

    let current = if app.dispatch_model.is_empty() {
        "(default)".to_string()
    } else {
        app.dispatch_model.clone()
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("current: ", Style::default().fg(Color::DarkGray)),
            Span::styled(current, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
    ];
    for (i, (label, _)) in models.iter().enumerate() {
        let selected = i == app.model_pick_index;
        let sty = if selected {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(
            format!("{}  {}", if selected { "▶" } else { " " }, label),
            sty,
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " j/k: 移動  enter: 選択  c: クリア  esc: 戻る",
        Style::default().fg(Color::DarkGray),
    )));

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" model picker — {} ", tool)),
        ),
        area,
    );
}

// ── main loop ─────────────────────────────────────────────────────────────────

fn run(term: &mut Term, app: &mut App) -> io::Result<()> {
    loop {
        let mut msgs = Vec::new();
        while let Ok(m) = app.msg_rx.try_recv() {
            msgs.push(m);
        }
        for m in msgs {
            app.apply(m);
        }

        if !app.refreshing && app.last_refresh.elapsed() >= app.interval {
            app.request_refresh();
        }
        if !app.git_inflight && app.last_git.elapsed() >= app.git_interval {
            app.request_git();
        }
        if app.last_usage.elapsed() >= Duration::from_secs(120) {
            app.last_usage = Instant::now();
            let _ = app.req_tx.send(Req::Usage);
        }
        if app.last_agy_usage.elapsed() >= Duration::from_secs(60) {
            app.last_agy_usage = Instant::now();
            let _ = app.req_tx.send(Req::AgyUsage);
        }
        if !app.codex_usage_inflight && app.last_codex_usage.elapsed() >= Duration::from_secs(120) {
            app.request_codex_usage();
        }
        app.maybe_request_logs();
        if app.refreshing || app.git_inflight {
            app.spinner = app.spinner.wrapping_add(1);
        }

        term.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(120))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && app.handle_key(key, term) {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let (req_tx, req_rx) = mpsc::channel::<Req>();
    let (msg_tx, msg_rx) = mpsc::channel::<Msg>();
    let msg_tx_for_app = msg_tx.clone();
    thread::spawn(move || worker(req_rx, msg_tx));

    let mut app = App::new(req_tx, msg_rx, msg_tx_for_app);
    app.request_refresh();
    app.request_git();
    let _ = app.req_tx.send(Req::Tools);
    let _ = app.req_tx.send(Req::Usage);
    let _ = app.req_tx.send(Req::AgyUsage);
    app.request_codex_usage();

    let res = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    res
}
