use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{sh_quote, App, Focus, InspectorView, Mode};
use crate::model::{env_dominant_status, status_rank, Item, ToolPurpose};
use crate::render::ACTION_MENU_ITEMS;
use crate::terminal::{run_dev, run_dev_pane, run_shell_pane, Term};

impl App {
    pub(crate) fn handle_key(&mut self, key: KeyEvent, term: &mut Term) -> bool {
        // Tab / Shift-Tab cycle cockpit focus (Normal mode only; not while typing
        // an inbox answer).
        if self.mode == Mode::Normal && !(self.focus == Focus::Inbox && self.inbox_answering) {
            match key.code {
                KeyCode::Tab => {
                    self.focus = self.focus.next();
                    self.on_focus_changed();
                    return false;
                }
                KeyCode::BackTab => {
                    self.focus = self.focus.prev();
                    self.on_focus_changed();
                    return false;
                }
                _ => {}
            }
        }
        match self.mode {
            Mode::Normal => return self.key_normal(key, term),
            Mode::Help => self.mode = Mode::Normal,
            Mode::Filter => self.key_filter(key),
            Mode::Dispatch => self.key_dispatch(key),
            Mode::ConfirmKill => self.key_confirm(key),
            Mode::LogView => self.key_logview(key),
            Mode::ActionMenu => self.key_action_menu(key, term),
            Mode::ToolPick => self.key_tool_pick(key, term),
            Mode::BatchMenu => self.key_batch_menu(key, term),
            Mode::ResultView => self.key_result_view(key),
            Mode::ModelPicker => self.key_model_picker(key, term),
            Mode::TaskView => self.key_task_view(key),
            Mode::Followup => self.key_followup(key),
            Mode::UsageView => self.key_usage_view(key),
            Mode::BoardModal => return self.key_board_modal(key, term),
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
            // Continuation affordances — extract the run's result, or follow up.
            KeyCode::Char('o') => {
                let t = self.log_target.clone();
                self.stop_tail();
                self.request_output(t);
            }
            KeyCode::Char('f') => {
                let t = self.log_target.clone();
                self.begin_followup(t);
            }
            _ => {}
        }
    }

    fn key_normal(&mut self, key: KeyEvent, term: &mut Term) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return true;
        }
        // While typing an inbox answer, every key goes to the answer editor.
        if self.focus == Focus::Inbox && self.inbox_answering {
            return self.key_inbox(key, term);
        }
        // Globals — active regardless of which panel has focus.
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Char('?') => {
                self.mode = Mode::Help;
                return false;
            }
            KeyCode::Char('r') => {
                self.request_refresh();
                self.request_git();
                self.request_dev_tasks();
                self.set_flash("refreshing…");
                return false;
            }
            KeyCode::Char('u') => {
                self.mode = Mode::UsageView;
                return false;
            }
            KeyCode::Char('T') => {
                self.task_scroll = 0;
                self.mode = Mode::TaskView;
                return false;
            }
            KeyCode::Char('b') => {
                self.mode = Mode::BoardModal;
                self.refresh_task_detail();
                return false;
            }
            _ => {}
        }
        match self.focus {
            Focus::Fleet => self.key_fleet(key, term),
            Focus::Inbox => self.key_inbox(key, term),
            Focus::Tasks => self.key_tasks(key, term),
        }
    }

    fn key_fleet(&mut self, key: KeyEvent, term: &mut Term) -> bool {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => self.move_sel(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_sel(-1),
            KeyCode::Char('g') | KeyCode::Home => self.list_state.select(Some(0)),
            KeyCode::Char('G') | KeyCode::End if !self.view.is_empty() => {
                self.list_state.select(Some(self.view.len() - 1));
            }
            KeyCode::Char(' ') => self.toggle_expand(),
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
            KeyCode::Char('v') => {
                // Toggle the Fleet inspector between detail+log and git diff.
                self.fleet_view = match self.fleet_view {
                    InspectorView::Detail => InspectorView::Diff,
                    InspectorView::Diff => InspectorView::Detail,
                };
                if self.fleet_view == InspectorView::Diff {
                    if let Some(t) = self.selected_env_name() {
                        self.request_fleet_diff(t);
                    }
                }
            }
            KeyCode::Enter => match self.selected_item_cloned() {
                Some(Item::GroupHeader(_)) => self.toggle_expand(),
                Some(Item::EnvRow(_)) | Some(Item::AgentRow(_, _)) => {
                    self.menu_index = 0;
                    self.mode = Mode::ActionMenu;
                }
                None => {}
            },
            KeyCode::Char('l') => {
                if let Some(t) = self.selected_env_name() {
                    run_dev_pane(&format!("logs:{t}"), &["agent", "logs", &t, "-f"], term);
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
                                    run_dev_pane(
                                        &format!("claude:{target}"),
                                        &["claude", &target, "--resume", sid],
                                        term,
                                    );
                                } else {
                                    run_dev_pane(
                                        &format!("claude:{target}"),
                                        &["claude", &target],
                                        term,
                                    );
                                }
                            }
                            "codex" => {
                                run_dev_pane(
                                    &format!("codex:{target}"),
                                    &["codex", &target, "resume", "--last"],
                                    term,
                                );
                            }
                            "opencode" => {
                                run_dev_pane(
                                    &format!("opencode:{target}"),
                                    &["opencode", &target, "--continue"],
                                    term,
                                );
                            }
                            "agy" => {
                                run_dev_pane(&format!("agy:{target}"), &["agy", &target], term);
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
                            run_dev_pane(&format!("attach:{t}"), &["agent", "attach", &t], term);
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
                    run_shell_pane(
                        &format!("diff:{t}"),
                        &format!("dev git diff {} | less -R", sh_quote(&t)),
                        term,
                    );
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
                    self.dispatch_model.clear();
                    self.dispatch_effort.clear();
                    self.tool_index = 0;
                    self.tool_purpose = ToolPurpose::Dispatch;
                    self.tool_prev_mode = Mode::Normal;
                    self.mode = Mode::ToolPick;
                }
            }
            // ── batch / triage ──────────────────────────────────────────────
            KeyCode::Char('m') => match self.selected_item_cloned() {
                Some(Item::EnvRow(i)) => {
                    let name = self.envs[i].name.clone();
                    if self.marked.contains(&name) {
                        self.marked.remove(&name);
                    } else {
                        self.marked.insert(name);
                    }
                }
                _ => self.set_flash("m: select an env row to mark"),
            },
            KeyCode::Char('M') => {
                self.marked.clear();
                self.set_flash("marks cleared");
            }
            KeyCode::Char('B') => {
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

    /// Cockpit Tasks panel — navigation + Enter opens the full board on the
    /// selected task. Task verbs live only in the board modal (key_board_modal).
    fn key_tasks(&mut self, key: KeyEvent, _term: &mut Term) -> bool {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                let n = self.active_tasks().len();
                if n > 0 {
                    self.tasks_sel = (self.tasks_sel + 1).min(n - 1);
                }
                self.refresh_task_detail();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.tasks_sel = self.tasks_sel.saturating_sub(1);
                self.refresh_task_detail();
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.tasks_sel = 0;
                self.refresh_task_detail();
            }
            KeyCode::Char('G') | KeyCode::End => {
                let n = self.active_tasks().len();
                if n > 0 {
                    self.tasks_sel = n - 1;
                }
                self.refresh_task_detail();
            }
            KeyCode::Enter => {
                if let Some(id) = self.selected_active_task_id() {
                    self.open_board_on(&id);
                }
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

    fn key_dispatch(&mut self, key: KeyEvent) {
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
                    // `--json` marks these as non-interactive `dev` calls so the
                    // CLI never pops an fzf model picker into the pane/terminal.
                    let supervise = self.dispatch_supervise;
                    if !self.dispatch_targets.is_empty() {
                        let targets = std::mem::take(&mut self.dispatch_targets);
                        for t in &targets {
                            let mut args = vec!["agent", "dispatch", t, "--json"];
                            if !tool.is_empty() {
                                args.extend_from_slice(&["--backend", &tool]);
                            }
                            if supervise {
                                args.push("--supervise");
                            }
                            let extra_refs: Vec<&str> = extra.iter().map(|s| s.as_str()).collect();
                            args.extend_from_slice(&extra_refs);
                            args.push(&task);
                            // Fire-and-forget: detach with output to /dev/null so it
                            // neither blocks nor scribbles over the live TUI (and the
                            // null stdout keeps `dev` non-interactive too). One pane
                            // per marked target would be too noisy for a batch.
                            let _ = Command::new("dev")
                                .args(&args)
                                .stdout(Stdio::null())
                                .stderr(Stdio::null())
                                .spawn();
                            self.record_task(t, &tool, &model, &task);
                        }
                        self.set_flash(&format!("dispatched → {} envs", targets.len()));
                    } else {
                        let target = self.dispatch_target.clone();
                        let mut args = vec!["agent", "dispatch", &target, "--json"];
                        if !tool.is_empty() {
                            args.extend_from_slice(&["--backend", &tool]);
                        }
                        if supervise {
                            args.push("--supervise");
                        }
                        let extra_refs: Vec<&str> = extra.iter().map(|s| s.as_str()).collect();
                        args.extend_from_slice(&extra_refs);
                        args.push(&task);
                        // `dev agent dispatch` is a fire-and-forget background launch
                        // that prints its JSON and returns at once — running it in a
                        // `--close-on-exit` Zellij pane just flashes a pane open and
                        // shut. Spawn it detached (like the batch path above) so the
                        // agent starts and the fleet TUI stays live, no flickering pane.
                        let _ = Command::new("dev")
                            .args(&args)
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .spawn();
                        self.set_flash(&format!("dispatched → {target}"));
                        self.record_task(&target, &tool, &model, &task);
                    }
                    self.after_action();
                }
            }
            KeyCode::Backspace => {
                self.dispatch_input.pop();
            }
            // Tab toggles supervised dispatch (task-tracked → agent can ask into Inbox).
            KeyCode::Tab => {
                self.dispatch_supervise = !self.dispatch_supervise;
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
                self.menu_index = if self.menu_index == 0 {
                    N - 1
                } else {
                    self.menu_index - 1
                };
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
                // Arms are ordered to match `ACTION_MENU_ITEMS` in render.rs:
                // session (0-2) · dispatch (3-4) · inspect (5-6) · destructive (7).
                // `new`/`dispatch`/`review` all go backend picker → model picker →
                // run, so each clears any stale model override before entering.
                match self.menu_index {
                    // --- session ---
                    0 => {
                        // new (fresh interactive session) → tool picker → model
                        self.dispatch_target = target;
                        self.dispatch_tool = String::new();
                        self.dispatch_model.clear();
                        self.dispatch_effort.clear();
                        self.tool_index = 0;
                        self.tool_purpose = ToolPurpose::Start;
                        self.tool_prev_mode = Mode::ActionMenu;
                        self.mode = Mode::ToolPick;
                    }
                    1 => {
                        // attach — reconnect via the unified `dev agent attach`,
                        // which computes the per-backend resume command from the
                        // core registry (no hardcoded per-tool arms here). On an
                        // agent row, pin the backend to that row's; otherwise let
                        // the CLI reconnect / resume the newest.
                        self.mode = Mode::Normal;
                        let mut args = vec!["agent", "attach", target.as_str()];
                        let backend = match self.selected_item_cloned() {
                            Some(Item::AgentRow(i, j)) => self.envs[i].agents[j].tool.clone(),
                            _ => String::new(),
                        };
                        if !backend.is_empty() {
                            args.extend_from_slice(&["--backend", &backend]);
                        }
                        run_dev_pane(&format!("attach:{target}"), &args, term);
                        self.after_action();
                    }
                    2 => {
                        // open in VS Code
                        self.mode = Mode::Normal;
                        run_dev(&["code", &target], term);
                        self.after_action();
                    }
                    // --- dispatch ---
                    3 => {
                        // dispatch → tool picker → model picker → task input
                        self.dispatch_target = target;
                        self.dispatch_tool = String::new();
                        self.dispatch_model.clear();
                        self.dispatch_effort.clear();
                        self.tool_index = 0;
                        self.tool_purpose = ToolPurpose::Dispatch;
                        self.tool_prev_mode = Mode::ActionMenu;
                        self.mode = Mode::ToolPick;
                    }
                    4 => {
                        // review → tool picker → model picker → run. On an agent
                        // row the backend is fixed to that agent's tool, so skip
                        // the tool picker and go straight to the model picker.
                        self.dispatch_target = target;
                        self.dispatch_model.clear();
                        self.dispatch_effort.clear();
                        self.tool_index = 0;
                        self.tool_purpose = ToolPurpose::Review;
                        self.tool_prev_mode = Mode::ActionMenu;
                        match self.selected_item_cloned() {
                            Some(Item::AgentRow(i, j)) => {
                                self.dispatch_tool = self.envs[i].agents[j].tool.clone();
                                self.model_pick_index = 0;
                                self.mode = Mode::ModelPicker;
                            }
                            _ => {
                                self.dispatch_tool = String::new();
                                self.mode = Mode::ToolPick;
                            }
                        }
                    }
                    // --- inspect ---
                    5 => {
                        // logs (TUI内)
                        self.open_log_view(target, tool);
                    }
                    6 => {
                        // diff
                        self.mode = Mode::Normal;
                        run_shell_pane(
                            &format!("diff:{target}"),
                            &format!("dev git diff {} | less -R", sh_quote(&target)),
                            term,
                        );
                        self.after_action();
                    }
                    // --- destructive ---
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
                self.tool_index = if self.tool_index == 0 {
                    n - 1
                } else {
                    self.tool_index - 1
                };
            }
            KeyCode::Enter => {
                // Backend chosen → pick a model next (the backend's own list).
                // If the backend exposes no models, run the action straight away.
                self.dispatch_tool = tools.get(self.tool_index).cloned().unwrap_or_default();
                self.model_pick_index = 0;
                if self.models_for_picker(&self.dispatch_tool).is_empty() {
                    self.run_tool_action(term);
                } else {
                    self.mode = Mode::ModelPicker;
                }
            }
            _ => {}
        }
    }

    /// Run the action set up by the tool + model pickers, per `tool_purpose`.
    /// Reads `dispatch_target`/`dispatch_tool`/`dispatch_model` (and, for
    /// dispatch, `dispatch_targets`/`dispatch_effort`). Called once a model has
    /// been chosen (or skipped) — the single exit for new/dispatch/review.
    fn run_tool_action(&mut self, term: &mut Term) {
        let tool = self.dispatch_tool.clone();
        let target = self.dispatch_target.clone();
        let model = self.dispatch_model.clone();
        match self.tool_purpose {
            ToolPurpose::Start => {
                // new = the merged `dev agent attach --fresh`. The model rides in
                // as trailing `extra`, which the CLI appends verbatim to the
                // interactive launch (`--model` is accepted by every backend).
                self.mode = Mode::Normal;
                let mut args: Vec<String> = vec![
                    "agent".into(),
                    "attach".into(),
                    target.clone(),
                    "--backend".into(),
                    tool.clone(),
                    "--fresh".into(),
                ];
                if !model.is_empty() {
                    args.push("--model".into());
                    args.push(model.clone());
                }
                let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
                run_dev_pane(&format!("{tool}:{target}"), &arg_refs, term);
                self.after_action();
            }
            ToolPurpose::Dispatch => {
                // Task text is entered on the next screen; key_dispatch applies
                // dispatch_tool/model/effort when it fires.
                self.dispatch_input.clear();
                self.dispatch_supervise = false;
                self.mode = Mode::Dispatch;
            }
            ToolPurpose::Review => {
                // Dispatch the review agent (with model/effort) and follow it.
                self.start_review_process(&target, &tool, term);
            }
        }
    }

    fn key_result_view(&mut self, key: KeyEvent) {
        let page = 20usize;
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.result_inflight {
                    self.result_cancelled = true;
                }
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
            // Follow up on the run this output belongs to.
            KeyCode::Char('f') => {
                let t = self.result_target.clone();
                self.begin_followup(t);
            }
            _ => {}
        }
    }

    /// Follow-up instruction editor — Enter dispatches `dev agent followup`
    /// (detached, like the batch dispatch path), Esc cancels.
    fn key_followup(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                let text = self.followup_input.trim().to_string();
                let target = self.followup_target.clone();
                self.mode = Mode::Normal;
                if text.is_empty() || target.is_empty() {
                    return;
                }
                // Fire-and-forget: `dev agent followup` re-dispatches a background
                // agent and returns at once (like the dispatch path), so detach it
                // and keep the TUI live.
                let _ = Command::new("dev")
                    .args(["agent", "followup", &target, &text])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();
                self.set_flash(&format!("followup → {target}"));
                self.after_action();
            }
            KeyCode::Backspace => {
                self.followup_input.pop();
            }
            KeyCode::Char(c) => self.followup_input.push(c),
            _ => {}
        }
    }

    fn key_model_picker(&mut self, key: KeyEvent, term: &mut Term) {
        let tool = self.dispatch_tool.clone();
        let tool = if tool.is_empty() {
            "claude".to_string()
        } else {
            tool
        };
        let models = self.models_for_picker(&tool);
        let n = models.len().max(1);
        match key.code {
            KeyCode::Esc => {
                // Back to the backend picker to re-choose the tool.
                self.mode = Mode::ToolPick;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.model_pick_index = (self.model_pick_index + 1) % n;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.model_pick_index = if self.model_pick_index == 0 {
                    n - 1
                } else {
                    self.model_pick_index - 1
                };
            }
            KeyCode::Enter => {
                // Selected model → run the configured action.
                if let Some((_, model_id)) = models.get(self.model_pick_index) {
                    self.dispatch_model = model_id.clone();
                }
                self.run_tool_action(term);
            }
            KeyCode::Delete | KeyCode::Char('c') => {
                // Skip the override — run with the backend's own default model.
                self.dispatch_model.clear();
                self.dispatch_effort.clear();
                self.run_tool_action(term);
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
                self.batch_menu_index = if self.batch_menu_index == 0 {
                    N - 1
                } else {
                    self.batch_menu_index - 1
                };
            }
            KeyCode::Enter => match self.batch_menu_index {
                0 => {
                    // dispatch → ToolPick → ModelPicker → Dispatch (batch mode)
                    let mut targets: Vec<String> = self.marked.iter().cloned().collect();
                    targets.sort();
                    self.dispatch_targets = targets;
                    self.dispatch_tool = String::new();
                    self.dispatch_model.clear();
                    self.dispatch_effort.clear();
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
                        .map(|t| format!("echo '=== {} ==='; dev git diff {}", t, sh_quote(t)))
                        .collect();
                    let cmd = format!("{{ {}; }} 2>&1 | less -R", parts.join("; "));
                    self.mode = Mode::Normal;
                    run_shell_pane("diff:marked", &cmd, term);
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
            KeyCode::Char('j') | KeyCode::Down => {
                self.task_scroll = self.task_scroll.saturating_add(1)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.task_scroll = self.task_scroll.saturating_sub(1)
            }
            KeyCode::Char('g') | KeyCode::Home => self.task_scroll = 0,
            KeyCode::Char('G') | KeyCode::End => {
                self.task_scroll = self.tasks.len().saturating_sub(1)
            }
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

    fn key_board_modal(&mut self, key: KeyEvent, term: &mut Term) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return true;
        }
        match key.code {
            // Close the modal back to the cockpit (do NOT quit the app).
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('b') => {
                self.mode = Mode::Normal;
                return false;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let count = self.tasks_for_lane(self.board_col).len();
                if count > 0 {
                    self.board_sel = (self.board_sel + 1).min(count - 1);
                }
                self.refresh_task_detail();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.board_sel = self.board_sel.saturating_sub(1);
                self.refresh_task_detail();
            }
            // Lane navigation: arrow keys only (h/l now bound to harvest/logs)
            KeyCode::Left => {
                self.board_col = self.board_col.saturating_sub(1);
                self.board_sel = 0;
                self.refresh_task_detail();
            }
            KeyCode::Right => {
                let max_col = App::BOARD_LANES.len() - 1;
                self.board_col = (self.board_col + 1).min(max_col);
                self.board_sel = 0;
                self.refresh_task_detail();
            }
            // R (capital) = refresh task list
            KeyCode::Char('R') => {
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
            // ── Phase 3 action keys ─────────────────────────────────────────
            KeyCode::Char('i') => {
                let id = self
                    .tasks_for_lane(self.board_col)
                    .get(self.board_sel)
                    .map(|t| t.id.clone());
                if let Some(id) = id {
                    run_dev(&["task", "dispatch", &id], term);
                    self.request_dev_tasks();
                    self.set_flash(&format!("dispatching impl for {id}"));
                    self.after_action();
                }
            }
            KeyCode::Char('a') => {
                let id = self
                    .tasks_for_lane(self.board_col)
                    .get(self.board_sel)
                    .map(|t| t.id.clone());
                if let Some(id) = id {
                    run_dev_pane(&format!("task:{id}"), &["task", "attach", &id], term);
                    self.after_action();
                }
            }
            KeyCode::Char('l') => {
                let id = self
                    .tasks_for_lane(self.board_col)
                    .get(self.board_sel)
                    .map(|t| t.id.clone());
                if let Some(id) = id {
                    run_dev_pane(
                        &format!("task-logs:{id}"),
                        &["task", "logs", &id, "-f"],
                        term,
                    );
                    self.after_action();
                }
            }
            KeyCode::Char('h') => {
                let id = self
                    .tasks_for_lane(self.board_col)
                    .get(self.board_sel)
                    .map(|t| t.id.clone());
                if let Some(id) = id {
                    run_dev(&["task", "harvest", &id], term);
                    self.request_dev_tasks();
                    self.set_flash(&format!("harvested {id}"));
                    self.after_action();
                }
            }
            KeyCode::Char('d') => {
                let id = self
                    .tasks_for_lane(self.board_col)
                    .get(self.board_sel)
                    .map(|t| t.id.clone());
                if let Some(id) = id {
                    // A one-shot `dev task diff` printed via `run_dev` is wiped the
                    // instant the TUI re-enters the alt-screen (its output is never
                    // readable). Page it through `less` in a pane, like the fleet-side
                    // diff actions (`D` / action-menu / batch) all do.
                    run_shell_pane(
                        &format!("task-diff:{id}"),
                        &format!("dev task diff {} --stat | less -R", sh_quote(&id)),
                        term,
                    );
                    self.after_action();
                }
            }
            KeyCode::Char('t') => {
                let id = self
                    .tasks_for_lane(self.board_col)
                    .get(self.board_sel)
                    .map(|t| t.id.clone());
                if let Some(id) = id {
                    run_dev(&["task", "test", &id], term);
                    self.request_dev_tasks();
                    self.set_flash(&format!("test run for {id}"));
                    self.after_action();
                }
            }
            KeyCode::Char('r') => {
                // r = review (R = refresh)
                let id = self
                    .tasks_for_lane(self.board_col)
                    .get(self.board_sel)
                    .map(|t| t.id.clone());
                if let Some(id) = id {
                    run_dev(&["task", "review", &id], term);
                    self.request_dev_tasks();
                    self.set_flash(&format!("reviewing {id}"));
                    self.after_action();
                }
            }
            KeyCode::Char('f') => {
                let id = self
                    .tasks_for_lane(self.board_col)
                    .get(self.board_sel)
                    .map(|t| t.id.clone());
                if let Some(id) = id {
                    run_dev(&["task", "fix", &id], term);
                    self.request_dev_tasks();
                    self.set_flash(&format!("fix dispatched for {id}"));
                    self.after_action();
                }
            }
            KeyCode::Char('m') => {
                let id = self
                    .tasks_for_lane(self.board_col)
                    .get(self.board_sel)
                    .map(|t| t.id.clone());
                if let Some(id) = id {
                    run_dev(&["task", "pr", &id], term);
                    self.request_dev_tasks();
                    self.set_flash(&format!("PR created for {id}"));
                    self.after_action();
                }
            }
            KeyCode::Char('n') => {
                self.set_flash("use: dev task new <project> --title <t>");
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
                KeyCode::Backspace => {
                    self.inbox_answer.pop();
                }
                KeyCode::Char(c) => self.inbox_answer.push(c),
                _ => {}
            }
            return false;
        }
        // Non-answering: navigation + Enter to answer. Globals (q/r/?/b/…) are
        // handled by key_normal before dispatching here.
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                let count = self.dev_questions.len();
                if count > 0 {
                    self.inbox_sel = (self.inbox_sel + 1).min(count - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.inbox_sel = self.inbox_sel.saturating_sub(1);
            }
            KeyCode::Enter if !self.dev_questions.is_empty() => {
                self.inbox_answering = true;
                self.inbox_answer.clear();
            }
            _ => {}
        }
        false
    }
}
