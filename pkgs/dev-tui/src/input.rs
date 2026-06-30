use std::process::Command;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{sh_quote, App, Mode};
use crate::data::Req;
use crate::model::{env_dominant_status, status_rank, Item, Tab, ToolPurpose};
use crate::render::ACTION_MENU_ITEMS;
use crate::terminal::{run_dev, run_shell, Term};

impl App {
    pub(crate) fn handle_key(&mut self, key: KeyEvent, term: &mut Term) -> bool {
        // Tab always switches main tab (except when typing in inbox answer)
        if key.code == KeyCode::Tab
            && !(self.mode == Mode::Normal
                && self.active_tab == Tab::Inbox
                && self.inbox_answering)
        {
            self.active_tab = self.active_tab.next();
            return false;
        }
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
            Mode::TaskView => self.key_task_view(key),
            Mode::UsageView => self.key_usage_view(key),
        }
        false
    }

    fn key_logview(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.stop_tail();
                self.mode = Mode::Normal;
            }
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
        if self.active_tab == Tab::TaskBoard {
            return self.key_task_board(key, term);
        }
        if self.active_tab == Tab::Inbox {
            return self.key_inbox(key, term);
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
                    run_dev(&["agent", "logs", &t, "-f"], term);
                    self.after_action();
                }
            }
            KeyCode::Char('a') => {
                match self.selected_item_cloned() {
                    Some(Item::AgentRow(i, j)) => {
                        // Agent row: attach directly to this agent's tool
                        let agent = &self.envs[i].agents[j];
                        let target = self.envs[i].name.clone();
                        match agent.tool.as_str() {
                            "claude" => {
                                if let Some(sid) = &agent.session_id {
                                    run_dev(&["claude", &target, "--resume", sid], term);
                                } else {
                                    run_dev(&["claude", &target], term);
                                }
                            }
                            "codex" => {
                                run_dev(&["codex", &target, "resume", "--last"], term);
                            }
                            "opencode" => {
                                run_dev(&["opencode", &target, "--continue"], term);
                            }
                            "agy" => {
                                run_dev(&["agy", &target], term);
                            }
                            _ => {
                                self.set_flash("cannot attach to this tool");
                            }
                        }
                        self.after_action();
                    }
                    Some(Item::EnvRow(_)) => {
                        // Env row: use dev agent attach (dispatched run or default claude)
                        if let Some(t) = self.selected_env_name() {
                            run_dev(&["agent", "attach", &t], term);
                            self.after_action();
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Char('c') => {
                if let Some(t) = self.selected_env_name() {
                    run_dev(&["code", &t], term);
                    self.after_action();
                }
            }
            KeyCode::Char('D') => {
                if let Some(t) = self.selected_env_name() {
                    run_shell(&format!("dev git diff {} | less -R", sh_quote(&t)), term);
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
            KeyCode::Char('T') => {
                self.task_scroll = 0;
                self.mode = Mode::TaskView;
            }
            KeyCode::Char('u') => {
                self.mode = Mode::UsageView;
            }
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
                            let mut args = vec!["agent", "dispatch", t];
                            if !tool.is_empty() {
                                args.extend_from_slice(&["--tool", &tool]);
                            }
                            let extra_refs: Vec<&str> = extra.iter().map(|s| s.as_str()).collect();
                            args.extend_from_slice(&extra_refs);
                            args.push(&task);
                            run_dev(&args, term);
                            self.record_task(t, &tool, &model, &task);
                        }
                        self.set_flash(&format!("dispatched → {} envs", targets.len()));
                    } else {
                        let target = self.dispatch_target.clone();
                        let mut args = vec!["agent", "dispatch", &target];
                        if !tool.is_empty() {
                            args.extend_from_slice(&["--tool", &tool]);
                        }
                        let extra_refs: Vec<&str> = extra.iter().map(|s| s.as_str()).collect();
                        args.extend_from_slice(&extra_refs);
                        args.push(&task);
                        run_dev(&args, term);
                        self.set_flash(&format!("dispatched → {target}"));
                        self.record_task(&target, &tool, &model, &task);
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
                        let _ = Command::new("dev").args(["agent", "kill", &name]).spawn();
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
                        // attach — directly to agent's tool or env-level attach
                        self.mode = Mode::Normal;
                        match self.selected_item_cloned() {
                            Some(Item::AgentRow(i, j)) => {
                                let agent = &self.envs[i].agents[j];
                                match agent.tool.as_str() {
                                    "claude" => {
                                        if let Some(sid) = &agent.session_id {
                                            run_dev(&["claude", &target, "--resume", sid], term);
                                        } else {
                                            run_dev(&["claude", &target], term);
                                        }
                                    }
                                    "codex" => {
                                        run_dev(&["codex", &target, "resume", "--last"], term);
                                    }
                                    "opencode" => {
                                        run_dev(&["opencode", &target, "--continue"], term);
                                    }
                                    "agy" => {
                                        run_dev(&["agy", &target], term);
                                    }
                                    _ => {
                                        self.set_flash("cannot attach to this tool");
                                    }
                                }
                            }
                            _ => {
                                // Env row or other: use dev agent attach
                                run_dev(&["agent", "attach", &target], term);
                            }
                        }
                        self.after_action();
                    }
                    1 => {
                        // open in VS Code
                        self.mode = Mode::Normal;
                        run_dev(&["code", &target], term);
                        self.after_action();
                    }
                    2 => {
                        // dispatch → tool picker
                        self.dispatch_target = target;
                        self.dispatch_tool = String::new();
                        self.tool_index = 0;
                        self.tool_purpose = ToolPurpose::Dispatch;
                        self.tool_prev_mode = Mode::ActionMenu;
                        self.mode = Mode::ToolPick;
                    }
                    3 => {
                        // start tool (interactive)
                        self.dispatch_target = target;
                        self.tool_index = 0;
                        self.tool_purpose = ToolPurpose::Start;
                        self.tool_prev_mode = Mode::ActionMenu;
                        self.mode = Mode::ToolPick;
                    }
                    4 => {
                        // review — tool picker or direct execution
                        match self.selected_item_cloned() {
                            Some(Item::AgentRow(i, j)) => {
                                // Agent row: review with that agent's tool
                                self.mode = Mode::Normal;
                                let agent_tool = self.envs[i].agents[j].tool.clone();
                                self.result_title = format!("review: {target} ({agent_tool}) (loading…)");
                                self.result_lines = vec!["running dev review, please wait…".into()];
                                self.result_scroll = 0;
                                self.result_inflight = true;
                                self.mode = Mode::ResultView;
                                let _ = self.req_tx.send(Req::Review {
                                    target: target.clone(),
                                    tool: agent_tool,
                                });
                            }
                            _ => {
                                // Env row: show tool picker for review
                                self.dispatch_target = target;
                                self.tool_index = 0;
                                self.tool_purpose = ToolPurpose::Review;
                                self.tool_prev_mode = Mode::ActionMenu;
                                self.mode = Mode::ToolPick;
                            }
                        }
                    }
                    5 => {
                        // model picker for next dispatch
                        self.dispatch_target = target.clone();
                        self.model_pick_index = 0;
                        self.mode = Mode::ModelPicker;
                    }
                    6 => {
                        // logs (TUI内)
                        self.open_log_view(target, tool);
                    }
                    7 => {
                        // diff
                        self.mode = Mode::Normal;
                        run_shell(&format!("dev git diff {} | less -R", sh_quote(&target)), term);
                        self.after_action();
                    }
                    8 => {
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
                    ToolPurpose::Review => {
                        self.mode = Mode::Normal;
                        self.result_title = format!("review: {target} ({tool}) (loading…)");
                        self.result_lines = vec!["running dev review, please wait…".into()];
                        self.result_scroll = 0;
                        self.result_inflight = true;
                        self.mode = Mode::ResultView;
                        let _ = self.req_tx.send(Req::Review {
                            target: target.clone(),
                            tool: tool.clone(),
                        });
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
                        let _ = Command::new("dev").args(["agent", "kill", t]).spawn();
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

    fn key_task_view(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Char('j') | KeyCode::Down => self.task_scroll = self.task_scroll.saturating_add(1),
            KeyCode::Char('k') | KeyCode::Up => self.task_scroll = self.task_scroll.saturating_sub(1),
            KeyCode::Char('g') | KeyCode::Home => self.task_scroll = 0,
            KeyCode::Char('G') | KeyCode::End => self.task_scroll = self.tasks.len().saturating_sub(1),
            KeyCode::PageDown => self.task_scroll = self.task_scroll.saturating_add(20),
            KeyCode::PageUp => self.task_scroll = self.task_scroll.saturating_sub(20),
            _ => {}
        }
    }

    fn key_usage_view(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Char('r') => {
                self.last_usage = Instant::now() - Duration::from_secs(999);
                self.last_agy_usage = Instant::now() - Duration::from_secs(999);
                self.request_codex_usage();
                self.set_flash("usage refreshing…");
            }
            _ => {}
        }
    }

    fn key_task_board(&mut self, key: KeyEvent, term: &mut Term) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return true;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Char('j') | KeyCode::Down => {
                let count = self.tasks_for_lane(self.board_col).len();
                if count > 0 { self.board_sel = (self.board_sel + 1).min(count - 1); }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.board_sel = self.board_sel.saturating_sub(1);
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.board_col = self.board_col.saturating_sub(1);
                self.board_sel = 0;
            }
            KeyCode::Char('l') | KeyCode::Right => {
                let max_col = App::BOARD_LANES.len() - 1;
                self.board_col = (self.board_col + 1).min(max_col);
                self.board_sel = 0;
            }
            KeyCode::Char('r') => {
                self.request_dev_tasks();
                self.set_flash("tasks refreshing…");
            }
            KeyCode::Char('A') => {
                let id = {
                    let tasks = self.tasks_for_lane(self.board_col);
                    tasks.get(self.board_sel).map(|t| t.id.clone())
                };
                if let Some(id) = id {
                    run_dev(&["task", "approve", &id], term);
                    self.request_dev_tasks();
                    self.set_flash(&format!("approved {id}"));
                    self.after_action();
                }
            }
            KeyCode::Char('p') => {
                let id = {
                    let tasks = self.tasks_for_lane(self.board_col);
                    tasks.get(self.board_sel).map(|t| t.id.clone())
                };
                if let Some(id) = id {
                    run_dev(&["task", "plan", &id], term);
                    self.request_dev_tasks();
                    self.set_flash(&format!("planning agent dispatched for {id}"));
                    self.after_action();
                }
            }
            KeyCode::Char('x') => {
                let id = {
                    let tasks = self.tasks_for_lane(self.board_col);
                    tasks.get(self.board_sel).map(|t| t.id.clone())
                };
                if let Some(id) = id {
                    run_dev(&["task", "reject", &id], term);
                    self.request_dev_tasks();
                    self.set_flash(&format!("rejected {id}"));
                    self.after_action();
                }
            }
            KeyCode::Char('?') => self.mode = Mode::Help,
            _ => {}
        }
        false
    }

    fn key_inbox(&mut self, key: KeyEvent, term: &mut Term) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return true;
        }
        if self.inbox_answering {
            match key.code {
                KeyCode::Esc => {
                    self.inbox_answering = false;
                    self.inbox_answer.clear();
                }
                KeyCode::Enter => {
                    let answer = self.inbox_answer.trim().to_string();
                    if !answer.is_empty() {
                        let qid = self.dev_questions.get(self.inbox_sel).map(|q| q.id.clone());
                        if let Some(qid) = qid {
                            run_dev(&["task", "answer", &qid, &answer], term);
                            self.set_flash(&format!("answered {qid}"));
                            self.after_action();
                        }
                    }
                    self.inbox_answering = false;
                    self.inbox_answer.clear();
                    self.request_dev_tasks();
                }
                KeyCode::Backspace => { self.inbox_answer.pop(); }
                KeyCode::Char(c) => self.inbox_answer.push(c),
                _ => {}
            }
            return false;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Char('j') | KeyCode::Down => {
                let count = self.dev_questions.len();
                if count > 0 { self.inbox_sel = (self.inbox_sel + 1).min(count - 1); }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.inbox_sel = self.inbox_sel.saturating_sub(1);
            }
            KeyCode::Enter => {
                if !self.dev_questions.is_empty() {
                    self.inbox_answering = true;
                    self.inbox_answer.clear();
                }
            }
            KeyCode::Char('r') => {
                self.request_dev_tasks();
                self.set_flash("questions refreshing…");
            }
            KeyCode::Char('?') => self.mode = Mode::Help,
            _ => {}
        }
        false
    }
}
