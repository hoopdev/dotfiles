//! dev top — live TUI over the `dev` CLI's machine-readable surface.
//!
//! Fleet rows group by LOCAL / REMOTE·<env>. Space collapses groups or
//! expands agent sub-rows. Enter opens an action menu (attach, dispatch,
//! start tool, logs, diff, kill). Dispatch and tool-start flow through a
//! tool-picker first. All commands delegate to `dev`.

use std::collections::HashMap;
use std::io::{self, Stdout};
use std::process::Command;
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
const TOOLS: [&str; 4] = ["claude", "codex", "opencode", "agy"];

// ── data ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Default)]
struct Agent {
    tool: String,
    status: String,
    kind: String,
    pid: String,
    cwd: String,
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
}

enum Msg {
    State(Vec<Env>),
    Git(HashMap<String, GitState>),
    Logs { target: String, lines: Vec<String> },
}

fn fetch_envs_base() -> Vec<Env> {
    let out = match Command::new("dev").args(["ls", "--json"]).output() {
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
    let out = match Command::new("dev").args(["ps", "--json"]).output() {
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
                env.agents.push(Agent {
                    tool: sget(a, "tool").unwrap_or_else(|| "-".into()),
                    status,
                    kind: sget(a, "kind").unwrap_or_default(),
                    pid: pid_to_string(a.get("pid")),
                    cwd: sget(a, "cwd").unwrap_or_default(),
                });
            } else {
                env.base_status = status;
            }
        }
    }
    envs
}

fn fetch_git() -> HashMap<String, GitState> {
    let mut m = HashMap::new();
    let out = match Command::new("dev").args(["status", "--json"]).output() {
        Ok(o) => o,
        Err(_) => return m,
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
    m
}

fn fetch_logs(target: &str) -> Vec<String> {
    let out = match Command::new("dev").args(["logs", target, "--json"]).output() {
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

fn worker(req_rx: Receiver<Req>, msg_tx: Sender<Msg>) {
    while let Ok(req) = req_rx.recv() {
        let msg = match req {
            Req::Refresh => Msg::State(fetch_state()),
            Req::Git => Msg::Git(fetch_git()),
            Req::Logs(t) => {
                let lines = fetch_logs(&t);
                Msg::Logs { target: t, lines }
            }
        };
        if msg_tx.send(msg).is_err() {
            break;
        }
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

    menu_index: usize,
    tool_index: usize,
    tool_purpose: ToolPurpose,
    tool_prev_mode: Mode,

    flash: Option<(String, Instant)>,
    spinner: u32,

    req_tx: Sender<Req>,
    msg_rx: Receiver<Msg>,
}

impl App {
    fn new(req_tx: Sender<Req>, msg_rx: Receiver<Msg>) -> Self {
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
            interval: Duration::from_secs(3),
            last_git: Instant::now(),
            git_inflight: false,
            git_interval: Duration::from_secs(12),
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
            menu_index: 0,
            tool_index: 0,
            tool_purpose: ToolPurpose::Start,
            tool_prev_mode: Mode::Normal,
            flash: None,
            spinner: 0,
            req_tx,
            msg_rx,
        }
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
            KeyCode::Char('G') | KeyCode::End => {
                if !self.view.is_empty() {
                    self.list_state.select(Some(self.view.len() - 1));
                }
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
            _ => {}
        }
        false
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
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Enter => {
                let task = self.dispatch_input.trim().to_string();
                let target = self.dispatch_target.clone();
                let tool = self.dispatch_tool.clone();
                self.mode = Mode::Normal;
                if !task.is_empty() {
                    if tool.is_empty() {
                        run_dev(&["dispatch", &target, &task], term);
                    } else {
                        run_dev(&["dispatch", &target, "--tool", &tool, &task], term);
                    }
                    self.set_flash(&format!("dispatched → {target}"));
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
        const N: usize = 6;
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
                        // attach
                        self.mode = Mode::Normal;
                        run_dev(&["attach", &target], term);
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
                        // logs (TUI内)
                        self.open_log_view(target, tool);
                    }
                    4 => {
                        // diff
                        self.mode = Mode::Normal;
                        run_shell(&format!("dev diff {} | less -R", sh_quote(&target)), term);
                        self.after_action();
                    }
                    5 => {
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
        let n = TOOLS.len();
        match key.code {
            KeyCode::Esc => self.mode = self.tool_prev_mode,
            KeyCode::Char('j') | KeyCode::Down => {
                self.tool_index = (self.tool_index + 1) % n;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.tool_index = if self.tool_index == 0 { n - 1 } else { self.tool_index - 1 };
            }
            KeyCode::Enter => {
                let tool = TOOLS[self.tool_index];
                let target = self.dispatch_target.clone();
                match self.tool_purpose {
                    ToolPurpose::Start => {
                        self.mode = Mode::Normal;
                        run_dev(&[tool, &target], term);
                        self.after_action();
                    }
                    ToolPurpose::Dispatch => {
                        self.dispatch_tool = tool.to_string();
                        self.dispatch_input.clear();
                        self.mode = Mode::Dispatch;
                    }
                }
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
    let mut spans: Vec<Span<'static>> = vec![
        Span::styled(
            " dev top ",
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
    let n_envs = app.envs.len();
    spans.push(Span::styled(
        format!("· {n_envs} envs {total_agents} agents "),
        Style::default().fg(Color::DarkGray),
    ));
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
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn make_fleet_items(
    view: &[Item],
    envs: &[Env],
    git: &HashMap<String, GitState>,
    groups: &[GroupInfo],
    group_collapsed: &HashMap<String, bool>,
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
                ListItem::new(Line::from(vec![
                    Span::raw(expand),
                    Span::styled("● ", status_style(dom)),
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
            lines.push(Line::from(Span::styled(
                format!("{} #{}", a.tool, a.pid),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(vec![
                Span::styled("● ", status_style(&a.status)),
                Span::styled(a.status.clone(), status_style(&a.status)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("env  ", label),
                Span::raw(env.name.clone()),
            ]));
            if !a.cwd.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("cwd  ", label),
                    Span::styled(truncate(&a.cwd, w.saturating_sub(5)), dim),
                ]));
            }
            if !a.kind.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("kind ", label),
                    Span::raw(a.kind.clone()),
                ]));
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
        _ => {
            let sel = app.list_state.selected().and_then(|s| app.view.get(s)).cloned();
            match sel {
                Some(Item::GroupHeader(_)) => {
                    " space 開閉 · j/k 移動 · / filter · w active · r refresh · ? help · q quit"
                }
                Some(Item::AgentRow(_, _)) => {
                    " enter メニュー · space 畳む · l logs-f · a attach · x kill pid · r refresh · ? help · q quit"
                }
                _ => " enter メニュー · space 開閉 · l logs-f · a attach · d dispatch · x kill · D diff · / filter · w active · r refresh · ? help · q quit",
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

fn render_action_menu(f: &mut Frame, app: &App) {
    let target = app.selected_env_name().unwrap_or_else(|| "?".into());
    let area = centered_rect(52, 68, f.area());
    f.render_widget(Clear, area);

    // (label, hint)
    let actions: [(&str, &str); 6] = [
        ("attach", "dev attach"),
        ("dispatch  (tool → task)", "dev dispatch --tool"),
        ("start tool  (interactive)", "dev <tool>"),
        ("logs", "TUI内ログ表示"),
        ("diff", "dev diff | less"),
        ("kill", "dev kill / kill <pid>"),
    ];

    let mut lines = vec![Line::from("")];
    for (i, (lbl, hint)) in actions.iter().enumerate() {
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
    let area = centered_rect(44, 52, f.area());
    f.render_widget(Clear, area);

    let mut lines = vec![Line::from("")];
    for (i, tool) in TOOLS.iter().enumerate() {
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
    let lines = vec![
        Line::from(vec![
            Span::styled("dispatch → ", Style::default().fg(Color::Cyan)),
            Span::styled(
                app.dispatch_target.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("tool: ", Style::default().fg(Color::DarkGray)),
            Span::styled(tool_display, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("task: ", Style::default().fg(Color::DarkGray)),
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
    thread::spawn(move || worker(req_rx, msg_tx));

    let mut app = App::new(req_tx, msg_rx);
    app.request_refresh();
    app.request_git();

    let res = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    res
}
