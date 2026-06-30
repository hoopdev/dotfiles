use ratatui::prelude::*;

/// One entry from `dev tools --json`.
#[derive(Clone)]
pub struct Tool {
    pub name: String,
    pub dispatchable: bool,
    #[allow(dead_code)] // used in Phase 5 (review action)
    pub review: bool,
}

#[derive(Clone, Default)]
pub struct Agent {
    pub tool: String,
    pub status: String,
    pub kind: String,
    pub pid: String,
    pub cwd: String,
    /// Claude session ID — used for `--resume` on attach and display.
    pub session_id: Option<String>,
    /// Human-readable agent name (from `claude agents --json` `.name` field).
    pub agent_name: Option<String>,
    /// Current model, if known (from run meta or future config fetch).
    pub model: Option<String>,
}

#[derive(Clone)]
pub struct Env {
    pub name: String,
    pub group: String, // "local" | remote env name ("coder", "bf-e", ...)
    pub host: String,
    pub shell: String,
    pub os: String,
    pub path: String,
    pub base_status: String,
    pub agents: Vec<Agent>,
    pub expanded: bool,
}

#[derive(Clone)]
pub struct GroupInfo {
    pub key: String,   // "local" | "coder" | "bf-e" ...
    pub label: String, // "LOCAL" | "REMOTE · coder" ...
}

#[derive(Clone, PartialEq)]
pub enum Item {
    GroupHeader(usize), // index into App.groups
    EnvRow(usize),
    AgentRow(usize, usize),
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolPurpose {
    #[default]
    Start,
    Dispatch,
    Review,
}

#[derive(Clone, Default)]
pub struct GitState {
    pub branch: String,
    pub head: String,
    pub changes: i64,
}

/// Active main tab in the TUI.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Agents,
    TaskBoard,
    Inbox,
}

impl Tab {
    pub fn next(self) -> Self {
        match self {
            Tab::Agents => Tab::TaskBoard,
            Tab::TaskBoard => Tab::Inbox,
            Tab::Inbox => Tab::Agents,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Tab::Agents => "Agents",
            Tab::TaskBoard => "Tasks",
            Tab::Inbox => "Inbox",
        }
    }
}

pub fn status_rank(s: &str) -> u8 {
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

pub fn status_style(s: &str) -> Style {
    match s {
        "waiting" => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        "error" | "unreachable" => Style::default().fg(Color::Red),
        "busy" | "running" => Style::default().fg(Color::Green),
        "stopped" => Style::default().fg(Color::DarkGray),
        _ => Style::default(),
    }
}

pub fn env_dominant_status(env: &Env) -> &str {
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

pub fn env_status_label(env: &Env) -> String {
    if env.agents.is_empty() {
        let s = &env.base_status;
        if s.len() > 13 {
            format!("{}...", &s[..12])
        } else {
            s.clone()
        }
    } else {
        let n = env.agents.len();
        let dom = env_dominant_status(env);
        format!("{n} {dom}")
    }
}

pub fn remote_meta_label(os: &str, shell: &str) -> String {
    match (os.is_empty() || os == "unknown", shell.is_empty()) {
        (false, false) => format!("{os}/{shell}"),
        (false, true) => os.to_string(),
        (true, false) => shell.to_string(),
        (true, true) => String::new(),
    }
}
