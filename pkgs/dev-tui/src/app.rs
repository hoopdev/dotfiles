use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use ratatui::widgets::ListState;

use crate::data::{fetch_codex_usage, Msg, Req};
use crate::model::{Env, GitState, GroupInfo, Item, Tab, Tool, ToolPurpose};

pub(crate) const SPIN: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
/// Fallback tool list used when `dev tools --json` is unavailable.
pub(crate) const DEFAULT_TOOLS: [&str; 4] = ["claude", "codex", "opencode", "agy"];

// ── text helpers ─────────────────────────────────────────────────────────────

pub(crate) fn truncate(s: &str, w: usize) -> String {
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
pub(crate) fn wrap_line(s: &str, w: usize) -> Vec<String> {
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

pub(crate) fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

// ── app ───────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
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
    /// Task dispatch history view.
    TaskView,
    /// Usage dashboard with sparklines.
    UsageView,
}

pub(crate) struct App {
    pub(crate) envs: Vec<Env>,
    pub(crate) git: HashMap<String, GitState>,
    pub(crate) groups: Vec<GroupInfo>,
    pub(crate) group_collapsed: HashMap<String, bool>,
    pub(crate) view: Vec<Item>,
    pub(crate) list_state: ListState,

    pub(crate) last_refresh: Instant,
    pub(crate) refreshing: bool,
    pub(crate) interval: Duration,
    pub(crate) last_git: Instant,
    pub(crate) git_inflight: bool,
    pub(crate) git_interval: Duration,

    pub(crate) filter: String,
    pub(crate) active_only: bool,
    pub(crate) mode: Mode,

    pub(crate) log_target: String,
    pub(crate) log_lines: Vec<String>,
    pub(crate) log_wanted: Option<(String, Instant)>,
    pub(crate) log_inflight: Option<String>,
    pub(crate) last_log: Instant,
    pub(crate) log_scroll: usize,
    pub(crate) log_follow: bool,

    pub(crate) dispatch_target: String,
    pub(crate) dispatch_input: String,
    pub(crate) dispatch_tool: String,
    /// Non-empty when dispatching to multiple marked targets (batch mode).
    /// Replaced by CLI fan-out in Phase 3; for now dispatches serially.
    pub(crate) dispatch_targets: Vec<String>,

    pub(crate) menu_index: usize,
    pub(crate) tool_index: usize,
    pub(crate) tool_purpose: ToolPurpose,
    pub(crate) tool_prev_mode: Mode,

    /// Backend registry from `dev tools --json`. Empty until first fetch.
    pub(crate) tools: Vec<Tool>,
    /// Env names marked for batch operations.
    pub(crate) marked: HashSet<String>,
    pub(crate) batch_menu_index: usize,

    /// ResultView: text displayed in the overlay (review, session list, …)
    pub(crate) result_title: String,
    pub(crate) result_lines: Vec<String>,
    pub(crate) result_scroll: usize,
    /// true while a background Req::Review (etc.) is in flight.
    pub(crate) result_inflight: bool,

    /// Dispatch model override (set by ModelPicker, applied to next dispatch).
    pub(crate) dispatch_model: String,
    pub(crate) dispatch_effort: String,
    /// Index within the model picker list.
    pub(crate) model_pick_index: usize,

    pub(crate) flash: Option<(String, Instant)>,
    pub(crate) spinner: u32,

    /// Claude rate-limit usage cached from ~/.cache/claude/usage.json.
    pub(crate) claude_usage: Option<crate::usage::ClaudeUsage>,
    pub(crate) last_usage: Instant,
    pub(crate) agy_usage: Option<crate::data::AgyUsage>,
    pub(crate) last_agy_usage: Instant,
    pub(crate) codex_usage: Option<crate::data::CodexUsage>,
    pub(crate) last_codex_usage: Instant,
    pub(crate) codex_usage_inflight: bool,

    pub(crate) req_tx: Sender<Req>,
    pub(crate) msg_rx: Receiver<Msg>,
    /// Cloned sender — allows spawning background usage threads that bypass the worker.
    pub(crate) msg_tx: Sender<Msg>,

    // ── new fields ───────────────────────────────────────────────────────
    pub(crate) tasks: Vec<crate::task::Task>,
    pub(crate) task_scroll: usize,
    pub(crate) tail_pid: Option<u32>,
    pub(crate) usage_history: crate::usage::UsageHistory,

    // ── Phase 2: task board & inbox ──────────────────────────────────────
    pub(crate) active_tab: Tab,
    pub(crate) dev_tasks: Vec<crate::task::DevTask>,
    pub(crate) dev_questions: Vec<crate::task::DevQuestion>,
    pub(crate) board_col: usize,
    pub(crate) board_sel: usize,
    pub(crate) inbox_sel: usize,
    pub(crate) inbox_answer: String,
    pub(crate) inbox_answering: bool,
    pub(crate) last_tasks: Instant,
    pub(crate) tasks_inflight: bool,
}

impl App {
    pub(crate) fn new(req_tx: Sender<Req>, msg_rx: Receiver<Msg>, msg_tx: Sender<Msg>) -> Self {
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
            tasks: crate::task::load_tasks(),
            task_scroll: 0,
            tail_pid: None,
            usage_history: crate::usage::UsageHistory::load(),
            active_tab: Tab::Agents,
            dev_tasks: Vec::new(),
            dev_questions: Vec::new(),
            board_col: 0,
            board_sel: 0,
            inbox_sel: 0,
            inbox_answer: String::new(),
            inbox_answering: false,
            last_tasks: Instant::now(),
            tasks_inflight: false,
        }
    }

    pub(crate) fn request_codex_usage(&mut self) {
        if self.codex_usage_inflight { return; }
        self.codex_usage_inflight = true;
        self.last_codex_usage = Instant::now();
        let tx = self.msg_tx.clone();
        thread::spawn(move || {
            let result = fetch_codex_usage();
            let _ = tx.send(Msg::CodexUsage(result));
        });
    }

    pub(crate) fn request_dev_tasks(&mut self) {
        if self.tasks_inflight { return; }
        self.tasks_inflight = true;
        self.last_tasks = Instant::now();
        let _ = self.req_tx.send(Req::DevTasks);
    }

    pub(crate) fn request_refresh(&mut self) {
        if self.refreshing {
            return;
        }
        self.refreshing = true;
        let _ = self.req_tx.send(Req::Refresh);
    }

    pub(crate) fn request_git(&mut self) {
        if self.git_inflight {
            return;
        }
        self.git_inflight = true;
        let _ = self.req_tx.send(Req::Git);
    }

    pub(crate) fn set_flash(&mut self, msg: &str) {
        self.flash = Some((msg.to_string(), Instant::now()));
    }

    /// Start monitoring a remote env if not already monitored.
    #[allow(dead_code)]
    pub(crate) fn start_remote_monitor(&mut self, _env_name: String) {
        // No-op: removed system monitoring, use gotop/nvtop instead.
    }


    pub(crate) fn selected_key(&self) -> Option<String> {
        let s = self.list_state.selected()?;
        match self.view.get(s)? {
            Item::GroupHeader(gi) => Some(format!("group:{}", self.groups[*gi].key)),
            Item::EnvRow(i) => Some(self.envs[*i].name.clone()),
            Item::AgentRow(i, j) => {
                Some(format!("{}:{}", self.envs[*i].name, self.envs[*i].agents[*j].pid))
            }
        }
    }

    pub(crate) fn restore_selection(&mut self, key: Option<String>) {
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

    pub(crate) fn apply(&mut self, msg: Msg) {
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
                crate::task::reconcile_tasks(&mut self.tasks, &self.envs);
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
            Msg::LogLine { target, line } => {
                if self.mode == Mode::LogView && self.log_target == target {
                    self.log_lines.push(line);
                    if self.log_lines.len() > 5000 {
                        self.log_lines.drain(..1000);
                    }
                }
            }
            Msg::Error(msg) => {
                self.set_flash(&format!("⚠ {}", msg));
            }
            Msg::DevTasks(tasks, questions) => {
                self.dev_tasks = tasks;
                self.dev_questions = questions;
                self.tasks_inflight = false;
                self.last_tasks = Instant::now();
                // Clamp selections
                let lane_count = self.tasks_for_lane(self.board_col).len();
                if lane_count == 0 { self.board_sel = 0; }
                else { self.board_sel = self.board_sel.min(lane_count - 1); }
                let qcount = self.dev_questions.len();
                if qcount == 0 { self.inbox_sel = 0; }
                else { self.inbox_sel = self.inbox_sel.min(qcount - 1); }
            }
        }
    }

    /// The board lanes and their corresponding phase names.
    pub(crate) const BOARD_LANES: &'static [(&'static str, &'static str)] = &[
        ("Needs Spec", "needs_spec"),
        ("Planned",    "planned"),
        ("Running",    "implementing"),
        ("Review",     "review"),
        ("Needs Fix",  "needs_fix"),
        ("Mergeable",  "mergeable"),
    ];

    pub(crate) fn tasks_for_lane(&self, col: usize) -> Vec<&crate::task::DevTask> {
        let phase = Self::BOARD_LANES.get(col).map(|(_, p)| *p).unwrap_or("");
        self.dev_tasks.iter().filter(|t| t.phase.as_str() == phase).collect()
    }

    /// Return tool names for the tool picker, filtered by purpose.
    /// Falls back to DEFAULT_TOOLS when the registry hasn't loaded yet.
    pub(crate) fn tools_for_picker(&self, purpose: ToolPurpose) -> Vec<String> {
        if self.tools.is_empty() {
            return DEFAULT_TOOLS.iter().map(|s| s.to_string()).collect();
        }
        self.tools
            .iter()
            .filter(|t| match purpose {
                ToolPurpose::Start => true,
                ToolPurpose::Dispatch => t.dispatchable,
                ToolPurpose::Review => t.review,
            })
            .map(|t| t.name.clone())
            .collect()
    }

    /// Model names for the model picker given the selected tool.
    /// Returns (display_label, model_id) pairs.
    pub(crate) fn models_for_picker(&self, tool: &str) -> Vec<(String, String)> {
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
            "agy" => vec![
                ("Gemini 3.5 Flash (Medium)".into(), "Gemini 3.5 Flash (Medium)".into()),
                ("Gemini 3.5 Flash (High)".into(), "Gemini 3.5 Flash (High)".into()),
                ("Gemini 3.5 Flash (Low)".into(), "Gemini 3.5 Flash (Low)".into()),
                ("Gemini 3.1 Pro (Low)".into(), "Gemini 3.1 Pro (Low)".into()),
                ("Gemini 3.1 Pro (High)".into(), "Gemini 3.1 Pro (High)".into()),
                ("Claude Sonnet 4.6 (Thinking)".into(), "Claude Sonnet 4.6 (Thinking)".into()),
                ("Claude Opus 4.6 (Thinking)".into(), "Claude Opus 4.6 (Thinking)".into()),
                ("GPT-OSS 120B (Medium)".into(), "GPT-OSS 120B (Medium)".into()),
            ],
            _ => vec![],
        }
    }

    pub(crate) fn rebuild_view(&mut self) {
        use crate::model::{env_status_label, remote_meta_label};

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
                    let meta = self
                        .envs
                        .iter()
                        .find(|e| e.group == *key)
                        .map(|e| remote_meta_label(&e.os, &e.shell))
                        .unwrap_or_default();
                    if meta.is_empty() {
                        format!("REMOTE · {}", key)
                    } else {
                        format!("REMOTE · {} · {}", key, meta)
                    }
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

    pub(crate) fn clamp_selection(&mut self) {
        if self.view.is_empty() {
            self.list_state.select(None);
        } else {
            let s = self.list_state.selected().unwrap_or(0).min(self.view.len() - 1);
            self.list_state.select(Some(s));
        }
    }

    pub(crate) fn selected_item_cloned(&self) -> Option<Item> {
        let s = self.list_state.selected()?;
        self.view.get(s).cloned()
    }

    pub(crate) fn selected_env_name(&self) -> Option<String> {
        match self.selected_item_cloned()? {
            Item::GroupHeader(_) => None,
            Item::EnvRow(i) => Some(self.envs[i].name.clone()),
            Item::AgentRow(i, _) => Some(self.envs[i].name.clone()),
        }
    }

    /// Session ID of the selected Claude agent (for --resume on attach).
    pub(crate) fn selected_agent_session_id(&self) -> Option<String> {
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
    pub(crate) fn selected_agent_name(&self) -> Option<String> {
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
    pub(crate) fn selected_tool(&self) -> String {
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

    pub(crate) fn move_sel(&mut self, delta: isize) {
        if self.view.is_empty() {
            return;
        }
        let len = self.view.len() as isize;
        let cur = self.list_state.selected().unwrap_or(0) as isize;
        self.list_state.select(Some((cur + delta).clamp(0, len - 1) as usize));
        
        // Start monitoring the newly selected env if it's remote
        if let Some(env_name) = self.selected_env_name() {
            if env_name != "local" {
                self.start_remote_monitor(env_name);
            }
        }
    }

    pub(crate) fn toggle_expand(&mut self) {
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

    pub(crate) fn after_action(&mut self) {
        self.request_refresh();
        self.request_git();
        self.log_target.clear();
        self.log_wanted = None;
    }

    pub(crate) fn maybe_request_logs(&mut self) {
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

    pub(crate) fn send_log_req(&mut self, target: String) {
        self.log_inflight = Some(target.clone());
        self.log_wanted = None;
        let _ = self.req_tx.send(Req::Logs(target));
    }

    /// Open the in-TUI log view for a target, with real-time tailing for non-claude targets.
    pub(crate) fn open_log_view(&mut self, target: String, tool: String) {
        self.log_follow = true;
        self.log_scroll = 0;
        self.log_wanted = None;
        if tool == "claude" {
            self.log_target = target;
            self.log_lines =
                vec!["(claude logs to its own store — press 'a' to attach)".into()];
        } else {
            self.log_target = target.clone();
            self.log_lines = vec!["(connecting…)".into()];
            self.start_tail(&target);
        }
        self.mode = Mode::LogView;
    }

    // ── new methods ──────────────────────────────────────────────────────

    pub(crate) fn start_tail(&mut self, target: &str) {
        self.stop_tail();
        let mut child = match std::process::Command::new("dev")
            .args(["logs", target, "-f"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .stdin(std::process::Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => return,
        };
        self.tail_pid = Some(child.id());
        let stdout = match child.stdout.take() {
            Some(s) => s,
            None => return,
        };
        let tx = self.msg_tx.clone();
        let tgt = target.to_string();
        std::thread::spawn(move || {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines().flatten() {
                if tx.send(crate::data::Msg::LogLine { target: tgt.clone(), line }).is_err() {
                    break;
                }
            }
            let _ = child.wait();
        });
    }

    pub(crate) fn stop_tail(&mut self) {
        if let Some(pid) = self.tail_pid.take() {
            let _ = std::process::Command::new("kill").arg(pid.to_string()).status();
        }
    }

    pub(crate) fn record_task(&mut self, target: &str, tool: &str, model: &str, task_text: &str) {
        let task = crate::task::Task::new(
            target.to_string(), tool.to_string(), model.to_string(), task_text.to_string(),
        );
        crate::task::save_task(&task);
        self.tasks.push(task);
    }

    pub(crate) fn record_usage_sample(&mut self) {
        let sample = crate::usage::UsageSample {
            timestamp: crate::task::unix_now(),
            claude_5h: self.claude_usage.as_ref().and_then(|u| u.five_hour_pct),
            claude_7d: self.claude_usage.as_ref().and_then(|u| u.seven_day_pct),
            codex_5h: self.codex_usage.as_ref().and_then(|u| u.primary_used_pct.map(|p| p.round() as u32)),
            codex_7d: self.codex_usage.as_ref().and_then(|u| u.secondary_used_pct.map(|p| p.round() as u32)),
            agy_pct: self.agy_usage.as_ref().and_then(|u| {
                match (u.available_credits, u.monthly_credits) {
                    (Some(avail), Some(monthly)) => Some(((monthly.saturating_sub(avail)) * 100 / monthly.max(1)) as u32),
                    _ => None,
                }
            }),
        };
        self.usage_history.push(sample);
    }
}
