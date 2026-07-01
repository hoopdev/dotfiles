//! Zellij plugin — dev task board.
//!
//! Data path: the plugin does **not** read the host filesystem (a WASM plugin
//! can't touch `~/.dev/*` without the broad `FullHdAccess` permission, and
//! `$HOME` isn't reliably set in the WASI sandbox). Instead it shells out via
//! `run_command(["dev","snapshot","--json"])` and parses the `RunCommandResult`
//! event into a [`BoardSnapshot`]. The board therefore renders exactly what the
//! `dev` CLI computes — one source of truth — and needs only `RunCommands`.

use dev_core::{BoardSnapshot, DevQuestion, DevTask};
use std::collections::BTreeMap;
use zellij_tile::prelude::*;

mod color;
mod keys;
mod render;

/// Context marker echoed back on our snapshot `RunCommandResult` so we ignore
/// results from any other command the plugin may run.
const SNAPSHOT_CTX: &str = "dev-snapshot";

/// Context marker for board-triggered mutations (approve / dispatch). When such
/// a command finishes we chain a fresh snapshot fetch so the task moves lanes
/// immediately instead of waiting for the fallback timer.
pub const ACTION_CTX: &str = "dev-action";

/// Fallback poll cadence. Primary refresh is event-driven (a `dev` mutation
/// pipes `refresh` to us — see the `pipe` handler); this timer only covers
/// changes made outside the board.
const REFRESH_SECS: f64 = 8.0;

/// Which screen the plugin is showing. `Tab` toggles between them.
#[derive(Default, Clone, Copy, PartialEq)]
pub enum View {
    #[default]
    Board,
    Inbox,
}

#[derive(Default)]
struct DevPlugin {
    tasks: Vec<DevTask>,
    questions: Vec<DevQuestion>,
    selected_col: usize, // lane index 0-5
    selected_row: usize, // task index within lane
    inbox_row: usize,    // selected question in the Inbox view
    view: View,
    loading: bool,
    /// Last fetch error, surfaced in the header instead of silently blanking.
    error: Option<String>,
    /// Names of currently-open Zellij tabs (from `TabUpdate`). Used to focus an
    /// existing `task <id>` tab instead of opening a duplicate on attach.
    open_tab_names: Vec<String>,
}

pub const LANES: &[(&str, &str)] = &[
    ("needs_spec", "Needs Spec"),
    ("planned", "Planned"),
    ("implementing", "Running"),
    ("review", "Review"),
    ("needs_fix", "Needs Fix"),
    ("mergeable", "Mergeable"),
];

impl DevPlugin {
    /// Kick off an async `dev snapshot --json`; the result lands as a
    /// `RunCommandResult` event tagged with [`SNAPSHOT_CTX`].
    fn fetch(&self) {
        let mut ctx = BTreeMap::new();
        ctx.insert("source".to_string(), SNAPSHOT_CTX.to_string());
        run_command(&["dev", "snapshot", "--json"], ctx);
    }

    /// Keep the selection inside the bounds of the freshly-loaded data so a task
    /// vanishing (e.g. it merged) can't strand the cursor off the board.
    fn clamp_selection(&mut self) {
        if self.selected_col >= LANES.len() {
            self.selected_col = LANES.len().saturating_sub(1);
        }
        let phase = LANES[self.selected_col].0;
        let count = self.tasks.iter().filter(|t| t.phase == phase).count();
        if self.selected_row >= count {
            self.selected_row = count.saturating_sub(1);
        }
        if self.inbox_row >= self.questions.len() {
            self.inbox_row = self.questions.len().saturating_sub(1);
        }
    }

    fn apply_snapshot(&mut self, stdout: &[u8]) {
        match serde_json::from_slice::<BoardSnapshot>(stdout) {
            Ok(snap) => {
                self.tasks = snap.tasks;
                self.questions = snap.questions;
                self.error = None;
                self.clamp_selection();
            }
            Err(e) => self.error = Some(format!("snapshot parse: {e}")),
        }
    }
}

impl ZellijPlugin for DevPlugin {
    fn load(&mut self, _config: BTreeMap<String, String>) {
        self.loading = true;
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::RunCommands,
        ]);
        subscribe(&[
            EventType::Timer,
            EventType::Key,
            EventType::RunCommandResult,
            EventType::TabUpdate,
        ]);
        set_timeout(REFRESH_SECS);
        self.fetch();
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::Timer(_) => {
                self.fetch();
                set_timeout(REFRESH_SECS);
                false
            }
            Event::RunCommandResult(exit, stdout, _stderr, ctx) => {
                match ctx.get("source").map(String::as_str) {
                    Some(SNAPSHOT_CTX) => {
                        self.loading = false;
                        if exit == Some(0) {
                            self.apply_snapshot(&stdout);
                        } else {
                            self.error = Some(format!("dev snapshot exited: {exit:?}"));
                        }
                        true
                    }
                    Some(ACTION_CTX) => {
                        if exit != Some(0) {
                            self.error = Some(format!("action exited: {exit:?}"));
                        }
                        self.fetch(); // pull fresh state now that the mutation landed
                        false
                    }
                    _ => false,
                }
            }
            Event::TabUpdate(tabs) => {
                self.open_tab_names = tabs.into_iter().map(|t| t.name).collect();
                false
            }
            Event::Key(key) => keys::handle(self, key), // key: KeyWithModifier
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        render::draw(self, rows, cols);
    }
}

register_plugin!(DevPlugin);
