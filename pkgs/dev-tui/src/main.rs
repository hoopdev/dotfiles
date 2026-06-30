//! dev top — a live TUI over the `dev` CLI's machine-readable surface.
//!
//! Architecture: `dev … --json` is the contract. This TUI is just a *client* of
//! it — it polls `dev ps --json` for fleet state, `dev status --json` for git,
//! `dev logs … --json` for the selected agent's tail, and shells out to `dev
//! attach / logs / kill / dispatch / diff` for actions (the same commands an LLM
//! orchestrator or a human at the shell would run). It holds no logic of its own
//! about local/remote, Coder, or 1Password; all of that lives in `dev`.
//!
//! All `dev` calls run on a background worker thread and arrive over a channel,
//! so a slow SSH fan-out never freezes the UI — a spinner shows work in flight.

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
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState, Wrap};
use serde_json::Value;

const SPIN: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

// ── data ────────────────────────────────────────────────────────────────────

#[derive(Clone, Default)]
struct Agent {
    target: String,
    location: String,
    tool: String,
    status: String,
    kind: String,
    pid: String,
    cwd: String,
}

#[derive(Clone, Default)]
struct GitState {
    branch: String,
    head: String,
    changes: i64,
}

/// Worker requests (main → worker) and results (worker → main).
enum Req {
    Refresh,
    Git,
    Logs(String),
}

enum Msg {
    Agents(Vec<Agent>),
    Git(HashMap<String, GitState>),
    Logs { target: String, lines: Vec<String> },
}

/// Sort key: agents that need a human (waiting) rise to the top, then errors,
/// then active, then idle, then stopped/unreachable.
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

fn loc_short(loc: &str) -> String {
    match loc {
        "local" => "loc".into(),
        "remote" => "rem".into(),
        other => other.chars().take(3).collect(),
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
        _ => "-".to_string(),
    }
}

// ── fetchers (run on the worker thread) ───────────────────────────────────────

fn fetch_agents() -> Vec<Agent> {
    let out = match Command::new("dev").args(["ps", "--json"]).output() {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    let mut v = Vec::new();
    if let Ok(Value::Array(arr)) = serde_json::from_slice::<Value>(&out.stdout) {
        for a in &arr {
            v.push(Agent {
                target: sget(a, "target").unwrap_or_else(|| "?".into()),
                location: sget(a, "location").unwrap_or_else(|| "?".into()),
                tool: sget(a, "tool").unwrap_or_else(|| "-".into()),
                status: sget(a, "status").unwrap_or_else(|| "?".into()),
                kind: sget(a, "kind").unwrap_or_default(),
                pid: pid_to_string(a.get("pid")),
                cwd: sget(a, "cwd").unwrap_or_default(),
            });
        }
    }
    v
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
            Req::Refresh => Msg::Agents(fetch_agents()),
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

// ── terminal drop-out actions ─────────────────────────────────────────────────

type Term = Terminal<CrosstermBackend<Stdout>>;

/// Drop out of the alternate screen, run `dev <args>` with the real terminal,
/// then restore the TUI. Used for interactive actions (attach/logs) and kill.
fn run_dev(args: &[&str], term: &mut Term) {
    let _ = disable_raw_mode();
    let _ = execute!(term.backend_mut(), LeaveAlternateScreen);
    let _ = Command::new("dev").args(args).status();
    let _ = enable_raw_mode();
    let _ = execute!(term.backend_mut(), EnterAlternateScreen);
    let _ = term.clear();
}

/// Same drop-out dance, but run a shell pipeline (used for `dev diff | less`).
fn run_shell(cmd: &str, term: &mut Term) {
    let _ = disable_raw_mode();
    let _ = execute!(term.backend_mut(), LeaveAlternateScreen);
    let _ = Command::new("sh").arg("-c").arg(cmd).status();
    let _ = enable_raw_mode();
    let _ = execute!(term.backend_mut(), EnterAlternateScreen);
    let _ = term.clear();
}

// ── app state ─────────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
enum SortMode {
    Smart,
    Name,
    Tool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Normal,
    Help,
    Filter,
    Dispatch,
    ConfirmKill,
}

struct App {
    agents: Vec<Agent>,
    git: HashMap<String, GitState>,
    view: Vec<usize>,
    state: TableState,

    last_refresh: Instant,
    refreshing: bool,
    interval: Duration,
    last_git: Instant,
    git_inflight: bool,
    git_interval: Duration,

    filter: String,
    active_only: bool,
    sort: SortMode,
    mode: Mode,

    log_target: String,
    log_lines: Vec<String>,
    log_wanted: Option<(String, Instant)>,
    log_inflight: Option<String>,
    last_log: Instant,

    dispatch_target: String,
    dispatch_input: String,

    flash: Option<(String, Instant)>,
    spinner: u32,

    req_tx: Sender<Req>,
    msg_rx: Receiver<Msg>,
}

impl App {
    fn new(req_tx: Sender<Req>, msg_rx: Receiver<Msg>) -> Self {
        let mut state = TableState::default();
        state.select(Some(0));
        App {
            agents: Vec::new(),
            git: HashMap::new(),
            view: Vec::new(),
            state,
            last_refresh: Instant::now(),
            refreshing: false,
            interval: Duration::from_secs(3),
            last_git: Instant::now(),
            git_inflight: false,
            git_interval: Duration::from_secs(12),
            filter: String::new(),
            active_only: false,
            sort: SortMode::Smart,
            mode: Mode::Normal,
            log_target: String::new(),
            log_lines: Vec::new(),
            log_wanted: None,
            log_inflight: None,
            last_log: Instant::now(),
            dispatch_target: String::new(),
            dispatch_input: String::new(),
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

    fn apply(&mut self, msg: Msg) {
        match msg {
            Msg::Agents(a) => {
                self.agents = a;
                self.refreshing = false;
                self.last_refresh = Instant::now();
                self.rebuild_view();
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
        let f = self.filter.to_lowercase();
        let mut idx: Vec<usize> = (0..self.agents.len())
            .filter(|&i| {
                let a = &self.agents[i];
                if self.active_only
                    && !matches!(a.status.as_str(), "waiting" | "error" | "busy" | "running")
                {
                    return false;
                }
                if !f.is_empty() {
                    let hay = format!("{} {} {} {}", a.target, a.location, a.tool, a.status)
                        .to_lowercase();
                    if !hay.contains(&f) {
                        return false;
                    }
                }
                true
            })
            .collect();
        idx.sort_by(|&x, &y| {
            let a = &self.agents[x];
            let b = &self.agents[y];
            match self.sort {
                SortMode::Smart => status_rank(&a.status)
                    .cmp(&status_rank(&b.status))
                    .then(a.target.cmp(&b.target)),
                SortMode::Name => a.target.cmp(&b.target),
                SortMode::Tool => a.tool.cmp(&b.tool).then(a.target.cmp(&b.target)),
            }
        });
        self.view = idx;
        self.clamp_selection();
    }

    fn clamp_selection(&mut self) {
        if self.view.is_empty() {
            self.state.select(None);
        } else {
            let s = self.state.selected().unwrap_or(0).min(self.view.len() - 1);
            self.state.select(Some(s));
        }
    }

    fn selected_agent(&self) -> Option<&Agent> {
        self.state
            .selected()
            .and_then(|s| self.view.get(s))
            .map(|&i| &self.agents[i])
    }

    fn sel_target(&self) -> Option<String> {
        self.selected_agent().map(|a| a.target.clone())
    }

    fn move_sel(&mut self, delta: isize) {
        if self.view.is_empty() {
            return;
        }
        let len = self.view.len() as isize;
        let cur = self.state.selected().unwrap_or(0) as isize;
        self.state.select(Some((cur + delta).clamp(0, len - 1) as usize));
    }

    fn cycle_sort(&mut self) {
        self.sort = match self.sort {
            SortMode::Smart => SortMode::Name,
            SortMode::Name => SortMode::Tool,
            SortMode::Tool => SortMode::Smart,
        };
        let label = match self.sort {
            SortMode::Smart => "smart",
            SortMode::Name => "name",
            SortMode::Tool => "tool",
        };
        self.rebuild_view();
        self.set_flash(&format!("sort: {label}"));
    }

    /// State may have changed after a drop-out action — refresh everything and
    /// force the log pane to refetch for the current selection.
    fn after_action(&mut self) {
        self.request_refresh();
        self.request_git();
        self.log_target.clear();
        self.log_wanted = None;
    }

    /// Lazily fetch the selected agent's log tail: debounce a new selection,
    /// re-poll a settled one every few seconds, and skip claude (no file log).
    fn maybe_request_logs(&mut self) {
        let target = match self.sel_target() {
            Some(t) => t,
            None => return,
        };
        let tool = self
            .selected_agent()
            .map(|a| a.tool.clone())
            .unwrap_or_default();
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

    // ── input ────────────────────────────────────────────────────────────────

    /// Returns true to quit.
    fn handle_key(&mut self, key: KeyEvent, term: &mut Term) -> bool {
        match self.mode {
            Mode::Normal => return self.key_normal(key, term),
            Mode::Help => self.mode = Mode::Normal,
            Mode::Filter => self.key_filter(key),
            Mode::Dispatch => self.key_dispatch(key, term),
            Mode::ConfirmKill => self.key_confirm(key, term),
        }
        false
    }

    fn key_normal(&mut self, key: KeyEvent, term: &mut Term) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return true;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Char('j') | KeyCode::Down => self.move_sel(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_sel(-1),
            KeyCode::Char('g') | KeyCode::Home => self.state.select(Some(0)),
            KeyCode::Char('G') | KeyCode::End => {
                if !self.view.is_empty() {
                    self.state.select(Some(self.view.len() - 1));
                }
            }
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
                    "filter: all agents"
                });
            }
            KeyCode::Char('s') => self.cycle_sort(),
            KeyCode::Char('?') => self.mode = Mode::Help,
            KeyCode::Enter => {
                if let Some(t) = self.sel_target() {
                    run_dev(&["logs", &t, "-f"], term);
                    self.after_action();
                }
            }
            KeyCode::Char('a') => {
                if let Some(t) = self.sel_target() {
                    run_dev(&["attach", &t], term);
                    self.after_action();
                }
            }
            KeyCode::Char('D') => {
                if let Some(t) = self.sel_target() {
                    run_shell(&format!("dev diff {} | less -R", sh_quote(&t)), term);
                    self.after_action();
                }
            }
            KeyCode::Char('x') => {
                if self.sel_target().is_some() {
                    self.mode = Mode::ConfirmKill;
                }
            }
            KeyCode::Char('d') => {
                if let Some(t) = self.sel_target() {
                    self.dispatch_target = t;
                    self.dispatch_input.clear();
                    self.mode = Mode::Dispatch;
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
                self.mode = Mode::Normal;
                if !task.is_empty() {
                    run_dev(&["dispatch", &target, &task], term);
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

    fn key_confirm(&mut self, key: KeyEvent, term: &mut Term) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => {
                if let Some(t) = self.sel_target() {
                    run_dev(&["kill", &t], term);
                    self.set_flash(&format!("killed {t}"));
                }
                self.mode = Mode::Normal;
                self.after_action();
            }
            _ => self.mode = Mode::Normal,
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
    let h = Layout::horizontal([Constraint::Percentage(58), Constraint::Percentage(42)]).split(v[1]);
    render_table(f, h[0], app);
    render_detail(f, h[1], app);
    render_bottom(f, v[2], app);
    match app.mode {
        Mode::Help => render_help(f),
        Mode::Dispatch => render_dispatch(f, app),
        Mode::ConfirmKill => render_confirm(f, app),
        _ => {}
    }
}

fn render_summary(f: &mut Frame, area: Rect, app: &App) {
    let order: [(&str, Color); 6] = [
        ("waiting", Color::Yellow),
        ("error", Color::Red),
        ("busy", Color::Green),
        ("idle", Color::Gray),
        ("stopped", Color::DarkGray),
        ("unreachable", Color::Red),
    ];
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for a in &app.agents {
        let k = if a.status == "running" { "busy" } else { a.status.as_str() };
        *counts.entry(k).or_insert(0) += 1;
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
                spans.push(Span::styled(format!("● {c} {name}  "), Style::default().fg(col)));
            }
        }
    }
    spans.push(Span::styled(
        format!("· {} total ", app.agents.len()),
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
    let mut tags = String::new();
    if app.active_only {
        tags.push_str(" [active]");
    }
    match app.sort {
        SortMode::Name => tags.push_str(" [name]"),
        SortMode::Tool => tags.push_str(" [tool]"),
        SortMode::Smart => {}
    }
    if !tags.is_empty() {
        spans.push(Span::styled(tags, Style::default().fg(Color::Cyan)));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_table(f: &mut Frame, area: Rect, app: &mut App) {
    let rows: Vec<Row> = app
        .view
        .iter()
        .map(|&i| {
            let a = &app.agents[i];
            let changes = app.git.get(&a.target).map(|g| g.changes).unwrap_or(-1);
            let (delta, dstyle) = if changes > 0 {
                (format!("Δ{changes}"), Style::default().fg(Color::Yellow))
            } else if changes == 0 {
                ("·".to_string(), Style::default().fg(Color::DarkGray))
            } else {
                (" ".to_string(), Style::default())
            };
            Row::new(vec![
                Cell::from("●").style(status_style(&a.status)),
                Cell::from(a.target.clone()),
                Cell::from(loc_short(&a.location)),
                Cell::from(a.tool.clone()),
                Cell::from(a.status.clone()).style(status_style(&a.status)),
                Cell::from(delta).style(dstyle),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(1),
        Constraint::Min(14),
        Constraint::Length(4),
        Constraint::Length(8),
        Constraint::Length(10),
        Constraint::Length(5),
    ];
    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["", "TARGET", "LOC", "TOOL", "STATUS", "Δ"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(Block::default().borders(Borders::ALL).title(" fleet "))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("▶ ");
    f.render_stateful_widget(table, area, &mut app.state);
}

fn render_detail(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default().borders(Borders::ALL).title(" detail ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let a = match app.selected_agent() {
        Some(a) => a,
        None => {
            f.render_widget(
                Paragraph::new(Span::styled(
                    "no agent selected",
                    Style::default().fg(Color::DarkGray),
                )),
                inner,
            );
            return;
        }
    };

    let label = Style::default().fg(Color::DarkGray);
    let dim = Style::default().fg(Color::DarkGray);
    let section = Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD);
    let mut lines: Vec<Line<'static>> = Vec::new();

    lines.push(Line::from(Span::styled(
        a.target.clone(),
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    )));
    let mut sl = vec![
        Span::styled("● ", status_style(&a.status)),
        Span::styled(a.status.clone(), status_style(&a.status)),
    ];
    if !a.kind.is_empty() {
        sl.push(Span::styled(format!("  {}", a.kind), dim));
    }
    lines.push(Line::from(sl));
    lines.push(Line::from(vec![
        Span::styled("tool ", label),
        Span::raw(a.tool.clone()),
        Span::styled("   pid ", label),
        Span::raw(if a.pid.is_empty() { "-".into() } else { a.pid.clone() }),
        Span::styled("   loc ", label),
        Span::raw(a.location.clone()),
    ]));
    if !a.cwd.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("cwd  ", label),
            Span::styled(truncate(&a.cwd, inner.width as usize), dim),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("git", section)));
    if let Some(g) = app.git.get(&a.target) {
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
                Span::styled(truncate(&g.head, inner.width as usize), dim),
            ]));
        }
    } else {
        lines.push(Line::from(Span::styled("(loading…)", dim)));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("recent log", section)));
    let header = lines.len();
    let avail = (inner.height as usize).saturating_sub(header).max(1);
    let w = inner.width as usize;
    if app.log_target == a.target {
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
            Span::styled(
                " filter ",
                Style::default().bg(Color::Cyan).fg(Color::Black),
            ),
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
            help_line()
        }
    } else {
        help_line()
    };
    f.render_widget(Paragraph::new(line), area);
}

fn help_line() -> Line<'static> {
    Line::from(Span::styled(
        " j/k nav · enter logs · a attach · d dispatch · x kill · D diff · / filter · w active · s sort · r refresh · ? help · q quit",
        Style::default().fg(Color::DarkGray),
    ))
}

fn kv(k: &str, v: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {k:<12}"), Style::default().fg(Color::Cyan)),
        Span::raw(v.to_string()),
    ])
}

fn render_help(f: &mut Frame) {
    let area = centered_rect(58, 75, f.area());
    f.render_widget(Clear, area);
    let lines = vec![
        Line::from(Span::styled(
            "dev top — keys",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        kv("j/k ↑/↓", "move selection"),
        kv("g / G", "first / last"),
        kv("enter", "follow logs (dev logs -f)"),
        kv("a", "attach (dev attach)"),
        kv("d", "dispatch a task (dev dispatch)"),
        kv("x", "kill agent (dev kill)"),
        kv("D", "view diff (dev diff | less)"),
        kv("/", "filter by text"),
        kv("w", "toggle active-only"),
        kv("s", "cycle sort (smart/name/tool)"),
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

fn render_dispatch(f: &mut Frame, app: &App) {
    let area = centered_rect(64, 28, f.area());
    f.render_widget(Clear, area);
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

fn render_confirm(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 24, f.area());
    f.render_widget(Clear, area);
    let target = app.sel_target().unwrap_or_default();
    let lines = vec![
        Line::from(vec![
            Span::styled("kill ", Style::default().fg(Color::Red)),
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
