use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use ratatui::widgets::ListState;

use crate::data::{fetch_codex_usage, strip_ansi, Msg, Req};
use crate::model::{Env, GitState, GroupInfo, Item, Tool, ToolPurpose};
use crate::terminal::{run_dev_pane, Term};

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

/// Ordering weight for a task phase in the cockpit's compact list — lower sorts
/// first, so human-gated / attention-needing phases float to the top.
pub(crate) fn task_attention_rank(phase: &str) -> u8 {
    match phase {
        "needs_fix" => 0,    // failed review — human must dispatch a fix
        "review" => 1,       // awaiting review decision/dispatch
        "mergeable" => 2,    // one action (merge/PR) from done
        "planned" => 3,      // human gate: approve the plan
        "needs_spec" => 4,   // human gate: usually mirrored as an Inbox question
        "approved" => 5,     // ready to dispatch impl
        "implementing" => 6, // in-flight, no action needed (still visible)
        _ => 7,              // unknown (merged is filtered out before sorting)
    }
}

pub(crate) fn priority_rank(priority: &str) -> u8 {
    match priority {
        "high" => 0,
        "med" | "medium" => 1,
        "low" => 2,
        _ => 3,
    }
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
    /// Full 7-lane kanban board, opened on demand with `b`.
    BoardModal,
}

/// Which left-column panel of the cockpit currently has focus.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Focus {
    Fleet,
    Inbox,
    Tasks,
}

impl Focus {
    pub(crate) fn next(self) -> Self {
        match self {
            Focus::Fleet => Focus::Inbox,
            Focus::Inbox => Focus::Tasks,
            Focus::Tasks => Focus::Fleet,
        }
    }
    pub(crate) fn prev(self) -> Self {
        match self {
            Focus::Fleet => Focus::Tasks,
            Focus::Tasks => Focus::Inbox,
            Focus::Inbox => Focus::Fleet,
        }
    }
}

/// Right-pane inspector view for the Fleet panel (Claude-Squad-style toggle).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum InspectorView {
    Detail,
    Diff,
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
    /// true when the user dismissed the result view before it completed.
    pub(crate) result_cancelled: bool,

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
    /// When the active log tail started — drives the LogView spinner + elapsed
    /// header so a live view (review or logs) never looks frozen while waiting
    /// for the first agent output. `None` when no tail is running.
    pub(crate) tail_started: Option<Instant>,
    pub(crate) usage_history: crate::usage::UsageHistory,

    // ── Phase 2: task board & inbox ──────────────────────────────────────
    pub(crate) dev_tasks: Vec<crate::task::DevTask>,
    pub(crate) dev_questions: Vec<crate::task::DevQuestion>,
    pub(crate) board_col: usize,
    pub(crate) board_sel: usize,
    pub(crate) inbox_sel: usize,
    pub(crate) inbox_answer: String,
    pub(crate) inbox_answering: bool,
    pub(crate) last_tasks: Instant,
    pub(crate) tasks_inflight: bool,

    // ── Phase 3: task detail panel ───────────────────────────────────────
    pub(crate) task_detail: Option<crate::task::TaskDetail>,
    /// Which task ID the current detail is for (avoids redundant fetches).
    pub(crate) detail_task_id: String,

    // ── Cockpit: unified single-screen layout ────────────────────────────
    /// Which left-column panel (Fleet / Inbox / Tasks) is focused.
    pub(crate) focus: Focus,
    /// Selection into the compact `active_tasks()` list (Tasks panel).
    pub(crate) tasks_sel: usize,
    /// Fleet inspector view (Detail ↔ Diff), toggled with `v`.
    pub(crate) fleet_view: InspectorView,
    /// Cached `dev git diff <env>` output for the Fleet Diff inspector.
    pub(crate) fleet_diff_lines: Vec<String>,
    /// Env the cached diff belongs to (avoids showing a stale diff).
    pub(crate) fleet_diff_target: String,
    pub(crate) fleet_diff_scroll: usize,
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
            result_cancelled: false,
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
            tail_started: None,
            usage_history: crate::usage::UsageHistory::load(),
            dev_tasks: Vec::new(),
            dev_questions: Vec::new(),
            board_col: 0,
            board_sel: 0,
            inbox_sel: 0,
            inbox_answer: String::new(),
            inbox_answering: false,
            last_tasks: Instant::now(),
            tasks_inflight: false,
            task_detail: None,
            detail_task_id: String::new(),
            focus: Focus::Fleet,
            tasks_sel: 0,
            fleet_view: InspectorView::Detail,
            fleet_diff_lines: Vec::new(),
            fleet_diff_target: String::new(),
            fleet_diff_scroll: 0,
        }
    }

    pub(crate) fn request_codex_usage(&mut self) {
        if self.codex_usage_inflight {
            return;
        }
        self.codex_usage_inflight = true;
        self.last_codex_usage = Instant::now();
        let tx = self.msg_tx.clone();
        thread::spawn(move || {
            let result = fetch_codex_usage();
            let _ = tx.send(Msg::CodexUsage(result));
        });
    }

    pub(crate) fn request_dev_tasks(&mut self) {
        if self.tasks_inflight {
            return;
        }
        self.tasks_inflight = true;
        self.last_tasks = Instant::now();
        let _ = self.req_tx.send(Req::DevTasks);
    }

    /// Request fresh detail for the currently selected task. The id is sourced
    /// from the board modal's lane when it's open, otherwise from the cockpit
    /// Tasks panel — both write the same shared `task_detail` cache (only one
    /// context is visible at a time).
    pub(crate) fn refresh_task_detail(&mut self) {
        let id = if self.mode == Mode::BoardModal {
            self.tasks_for_lane(self.board_col)
                .get(self.board_sel)
                .map(|t| t.id.clone())
        } else {
            self.selected_active_task_id()
        };
        if let Some(id) = id {
            if id != self.detail_task_id {
                self.detail_task_id = id.clone();
                self.task_detail = None;
            }
            let _ = self.req_tx.send(Req::TaskDetail(id));
        }
    }

    /// Tasks in the compact cockpit list: everything except `merged`, sorted
    /// attention-first (needs_fix → review → mergeable → planned → …) so the
    /// items that need a human float to the top.
    pub(crate) fn active_tasks(&self) -> Vec<&crate::task::DevTask> {
        let mut v: Vec<&crate::task::DevTask> = self
            .dev_tasks
            .iter()
            .filter(|t| t.phase != "merged")
            .collect();
        v.sort_by(|a, b| {
            task_attention_rank(&a.phase)
                .cmp(&task_attention_rank(&b.phase))
                .then(priority_rank(&a.priority).cmp(&priority_rank(&b.priority)))
                .then(a.id.cmp(&b.id))
        });
        v
    }

    /// Id of the task selected in the cockpit Tasks panel.
    pub(crate) fn selected_active_task_id(&self) -> Option<String> {
        self.active_tasks()
            .get(self.tasks_sel)
            .map(|t| t.id.clone())
    }

    /// Open the full board modal positioned on the given task (its lane + row).
    pub(crate) fn open_board_on(&mut self, task_id: &str) {
        if let Some(task) = self.dev_tasks.iter().find(|t| t.id == task_id) {
            let phase = task.phase.clone();
            if let Some(col) = Self::BOARD_LANES.iter().position(|(_, p)| *p == phase) {
                self.board_col = col;
                self.board_sel = self
                    .tasks_for_lane(col)
                    .iter()
                    .position(|t| t.id == task_id)
                    .unwrap_or(0);
            }
        }
        self.mode = Mode::BoardModal;
        self.refresh_task_detail();
    }

    /// Called after `focus` changes: refetch task detail when landing on Tasks.
    pub(crate) fn on_focus_changed(&mut self) {
        if self.focus == Focus::Tasks {
            self.refresh_task_detail();
        }
    }

    /// Request `dev git diff <env>` output for the Fleet Diff inspector.
    pub(crate) fn request_fleet_diff(&mut self, target: String) {
        self.fleet_diff_target = target.clone();
        self.fleet_diff_scroll = 0;
        self.fleet_diff_lines = vec!["(loading diff…)".into()];
        let _ = self.req_tx.send(Req::Diff(target));
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
            Item::AgentRow(i, j) => Some(format!(
                "{}:{}",
                self.envs[*i].name, self.envs[*i].agents[*j].pid
            )),
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
                let expansions: HashMap<String, bool> = self
                    .envs
                    .iter()
                    .map(|e| (e.name.clone(), e.expanded))
                    .collect();
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
                self.result_inflight = false;
                if !self.result_cancelled {
                    self.result_title = title;
                    self.result_lines = lines;
                    self.result_scroll = 0;
                    self.mode = Mode::ResultView;
                }
                self.result_cancelled = false;
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
                // Preserve the cockpit Tasks selection across the (derived,
                // re-sorted) list by id, not index — mirrors restore_selection.
                let prev_active_id = self.selected_active_task_id();
                self.dev_tasks = tasks;
                self.dev_questions = questions;
                self.tasks_inflight = false;
                self.last_tasks = Instant::now();
                // Clamp board-modal selection
                let lane_count = self.tasks_for_lane(self.board_col).len();
                if lane_count == 0 {
                    self.board_sel = 0;
                } else {
                    self.board_sel = self.board_sel.min(lane_count - 1);
                }
                // Restore cockpit Tasks selection by id (fallback: clamp)
                let active = self.active_tasks();
                if let Some(pos) = prev_active_id
                    .as_ref()
                    .and_then(|id| active.iter().position(|t| &t.id == id))
                {
                    self.tasks_sel = pos;
                } else {
                    self.tasks_sel = self.tasks_sel.min(active.len().saturating_sub(1));
                }
                let qcount = self.dev_questions.len();
                if qcount == 0 {
                    self.inbox_sel = 0;
                } else {
                    self.inbox_sel = self.inbox_sel.min(qcount - 1);
                }
                // Refetch detail for whichever task context is currently visible.
                self.refresh_task_detail();
            }
            Msg::TaskDetail(detail) => {
                self.task_detail = detail;
            }
            Msg::Diff { target, lines } => {
                // Only accept if it still matches the env we're inspecting.
                if self.fleet_diff_target == target {
                    self.fleet_diff_lines = lines;
                    self.fleet_diff_scroll = 0;
                }
            }
        }
    }

    /// The board lanes and their corresponding phase names.
    pub(crate) const BOARD_LANES: &'static [(&'static str, &'static str)] = &[
        ("Needs Spec", "needs_spec"),
        ("Planned", "planned"),
        ("Approved", "approved"),
        ("Running", "implementing"),
        ("Review", "review"),
        ("Needs Fix", "needs_fix"),
        ("Mergeable", "mergeable"),
    ];

    pub(crate) fn tasks_for_lane(&self, col: usize) -> Vec<&crate::task::DevTask> {
        let phase = Self::BOARD_LANES.get(col).map(|(_, p)| *p).unwrap_or("");
        self.dev_tasks
            .iter()
            .filter(|t| t.phase.as_str() == phase)
            .collect()
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

    /// Models for the picker given the selected backend, from the single
    /// dev-core registry (was a hardcoded, drift-prone copy). Returns
    /// (display_label, model_id) pairs.
    pub(crate) fn models_for_picker(&self, backend: &str) -> Vec<(String, String)> {
        let Some(spec) = dev_core::agent::backend(backend) else {
            return Vec::new();
        };
        spec.models
            .iter()
            .map(|m| {
                let display = if m.id.is_empty() || m.label == m.id {
                    m.label.to_string()
                } else {
                    format!("{}  ({})", m.label, m.id)
                };
                (display, m.id.to_string())
            })
            .collect()
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
                GroupInfo {
                    key: key.clone(),
                    label,
                }
            })
            .collect();

        let f = self.filter.to_lowercase();
        let mut items = Vec::new();

        for (gi, group) in self.groups.iter().enumerate() {
            let collapsed = self
                .group_collapsed
                .get(&group.key)
                .copied()
                .unwrap_or(false);
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
                    let hay = format!("{} {} {}", env.name, env.group, env_status_label(env))
                        .to_lowercase();
                    let agent_match = env.agents.iter().any(|a| {
                        format!("{} {}", a.tool, a.status)
                            .to_lowercase()
                            .contains(&f)
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
            let s = self
                .list_state
                .selected()
                .unwrap_or(0)
                .min(self.view.len() - 1);
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
    #[allow(dead_code)]
    pub(crate) fn selected_agent_session_id(&self) -> Option<String> {
        match self.selected_item_cloned()? {
            Item::AgentRow(i, j) => self.envs[i]
                .agents
                .get(j)
                .and_then(|a| a.session_id.clone()),
            Item::EnvRow(i) => self.envs[i]
                .agents
                .first()
                .and_then(|a| a.session_id.clone()),
            _ => None,
        }
    }

    /// Agent name label of selected agent (claude session name, if available).
    #[allow(dead_code)]
    pub(crate) fn selected_agent_name(&self) -> Option<String> {
        match self.selected_item_cloned()? {
            Item::AgentRow(i, j) => self.envs[i]
                .agents
                .get(j)
                .and_then(|a| a.agent_name.clone()),
            Item::EnvRow(i) => self.envs[i]
                .agents
                .first()
                .and_then(|a| a.agent_name.clone()),
            _ => None,
        }
    }

    /// Tool of the selected agent (or first agent of the selected env).
    pub(crate) fn selected_tool(&self) -> String {
        match self.selected_item_cloned() {
            Some(Item::EnvRow(i)) => self.envs[i]
                .agents
                .first()
                .map(|a| a.tool.clone())
                .unwrap_or_default(),
            Some(Item::AgentRow(i, j)) => self.envs[i]
                .agents
                .get(j)
                .map(|a| a.tool.clone())
                .unwrap_or_default(),
            _ => String::new(),
        }
    }

    pub(crate) fn move_sel(&mut self, delta: isize) {
        if self.view.is_empty() {
            return;
        }
        let len = self.view.len() as isize;
        let cur = self.list_state.selected().unwrap_or(0) as isize;
        self.list_state
            .select(Some((cur + delta).clamp(0, len - 1) as usize));

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
        let (target, tool, session) = match self.selected_item_cloned() {
            Some(Item::EnvRow(i)) => {
                let e = &self.envs[i];
                match e.agents.first() {
                    Some(a) => (e.name.clone(), a.tool.clone(), a.session_id.clone()),
                    None => (e.name.clone(), String::new(), None),
                }
            }
            Some(Item::AgentRow(i, j)) => {
                let a = &self.envs[i].agents[j];
                (
                    self.envs[i].name.clone(),
                    a.tool.clone(),
                    a.session_id.clone(),
                )
            }
            _ => return, // GroupHeader or no selection
        };
        if self.log_inflight.is_some() {
            return;
        }
        // Nothing running (stopped/unreachable env) → no activity to fetch.
        if tool.is_empty() || tool == "-" {
            if self.log_target != target {
                self.log_target = target;
                self.log_lines = Vec::new();
                self.log_wanted = None;
            }
            return;
        }
        // Every backend (claude included) now has readable activity: claude via
        // its transcript, others via the run log. Fetch on select + on a staleness
        // timer so a live agent's output keeps flowing into the detail pane.
        let is_new = self.log_target != target;
        let is_stale =
            self.log_target == target && self.last_log.elapsed() >= Duration::from_secs(4);
        if is_new {
            match &self.log_wanted {
                Some((t, since)) if *t == target => {
                    if since.elapsed() >= Duration::from_millis(250) {
                        self.send_log_req(target, session);
                    }
                }
                _ => self.log_wanted = Some((target, Instant::now())),
            }
        } else if is_stale {
            self.send_log_req(target, session);
        }
    }

    pub(crate) fn send_log_req(&mut self, target: String, session: Option<String>) {
        self.log_inflight = Some(target.clone());
        self.log_wanted = None;
        let _ = self.req_tx.send(Req::Logs { target, session });
    }

    /// Open the in-TUI log view for a target. Non-claude backends stream their run
    /// log via `tail -f`; claude has no streamable log, so its transcript is polled
    /// by `maybe_request_logs` (which keeps running in LogView mode). We backdate
    /// `last_log` so the first poll fires immediately instead of after the 4 s gate.
    pub(crate) fn open_log_view(&mut self, target: String, tool: String) {
        self.log_follow = true;
        self.log_scroll = 0;
        self.log_wanted = None;
        if tool == "claude" {
            if self.log_target != target {
                self.log_target = target;
                self.log_lines = vec!["(loading transcript…)".into()];
            }
            self.last_log = Instant::now()
                .checked_sub(Duration::from_secs(5))
                .unwrap_or_else(Instant::now);
        } else {
            self.log_target = target.clone();
            self.log_lines = vec!["(connecting…)".into()];
            self.start_tail(&target);
        }
        self.mode = Mode::LogView;
    }

    // ── new methods ──────────────────────────────────────────────────────

    /// Dispatch a code review as a normal background agent, then attach to it.
    ///
    /// `dev agent review` seeds a background agent with a review prompt (see
    /// `_dev_review` → `_dev_dispatch` in coder.nix), so the review is tracked in
    /// `dev agent ps` and is attach/resume-able like any other agent. We block
    /// briefly to record the run, then drop straight into the interactive session
    /// via `dev agent attach` (which resumes the recorded session per-tool). This
    /// shows the findings live and lets you follow up ("fix issue #2") in the same
    /// session — instead of parking in the log view, which for claude only ever
    /// shows the "press 'a' to attach" hint anyway.
    pub(crate) fn start_review_process(&mut self, target: &str, tool: &str, term: &mut Term) {
        self.stop_tail();

        let mut cmd = std::process::Command::new("dev");
        cmd.args(["agent", "review", target]);
        if !tool.is_empty() {
            cmd.args(["--backend", tool]);
        }
        let output = cmd.stdin(std::process::Stdio::null()).output();

        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let summary = stdout
                    .lines()
                    .map(str::trim)
                    .find(|l| !l.is_empty())
                    .unwrap_or("review dispatched")
                    .to_string();
                self.set_flash(&summary);
                // Attach straight to the just-dispatched review agent instead of
                // opening the (for claude, empty) log view.
                self.mode = Mode::Normal;
                run_dev_pane(
                    &format!("review:{target}"),
                    &["agent", "attach", target],
                    term,
                );
                self.after_action();
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                let msg = stderr
                    .lines()
                    .map(str::trim)
                    .find(|l| !l.is_empty())
                    .unwrap_or("review dispatch failed")
                    .to_string();
                self.set_flash(&format!("⚠ {msg}"));
                self.mode = Mode::Normal;
            }
            Err(e) => {
                self.set_flash(&format!("⚠ review failed: {e}"));
                self.mode = Mode::Normal;
            }
        }
    }

    pub(crate) fn start_tail(&mut self, target: &str) {
        self.stop_tail();
        let mut child = match std::process::Command::new("dev")
            .args(["agent", "logs", target, "-f"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .stdin(std::process::Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => return,
        };
        self.tail_pid = Some(child.id());
        self.tail_started = Some(Instant::now());
        let stdout = match child.stdout.take() {
            Some(s) => s,
            None => return,
        };
        let tx = self.msg_tx.clone();
        let tgt = target.to_string();
        std::thread::spawn(move || {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                // Strip ANSI so tool output (e.g. codex exec) renders cleanly in
                // the plain-text LogView instead of showing escape-code garbage.
                let line = strip_ansi(&line);
                if tx
                    .send(crate::data::Msg::LogLine {
                        target: tgt.clone(),
                        line,
                    })
                    .is_err()
                {
                    break;
                }
            }
            let _ = child.wait();
        });
    }

    pub(crate) fn stop_tail(&mut self) {
        self.tail_started = None;
        if let Some(pid) = self.tail_pid.take() {
            let _ = std::process::Command::new("kill")
                .arg(pid.to_string())
                .status();
        }
    }

    pub(crate) fn record_task(&mut self, target: &str, tool: &str, model: &str, task_text: &str) {
        let task = crate::task::Task::new(
            target.to_string(),
            tool.to_string(),
            model.to_string(),
            task_text.to_string(),
        );
        crate::task::save_task(&task);
        self.tasks.push(task);
    }

    pub(crate) fn record_usage_sample(&mut self) {
        let sample = crate::usage::UsageSample {
            timestamp: crate::task::unix_now(),
            claude_5h: self.claude_usage.as_ref().and_then(|u| u.five_hour_pct),
            claude_7d: self.claude_usage.as_ref().and_then(|u| u.seven_day_pct),
            codex_5h: self
                .codex_usage
                .as_ref()
                .and_then(|u| u.primary_used_pct.map(|p| p.round() as u32)),
            codex_7d: self
                .codex_usage
                .as_ref()
                .and_then(|u| u.secondary_used_pct.map(|p| p.round() as u32)),
            agy_pct: self.agy_usage.as_ref().and_then(|u| {
                match (u.available_credits, u.monthly_credits) {
                    (Some(avail), Some(monthly)) => {
                        Some(((monthly.saturating_sub(avail)) * 100 / monthly.max(1)) as u32)
                    }
                    _ => None,
                }
            }),
        };
        self.usage_history.push(sample);
    }
}
