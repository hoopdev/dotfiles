//! Key handler for the dev Zellij plugin.
//!
//! zellij-tile 0.44 uses `KeyWithModifier { bare_key: BareKey, .. }` instead
//! of the old `Key` enum, so we match on `key.bare_key`.

use crate::{DevPlugin, View, ACTION_CTX, LANES};
use std::collections::BTreeMap;
use std::path::PathBuf;
use zellij_tile::prelude::*;

/// Context map tagging a board-triggered mutation so its `RunCommandResult`
/// chains a refresh (see `lib.rs`).
fn action_ctx() -> BTreeMap<String, String> {
    let mut ctx = BTreeMap::new();
    ctx.insert("source".to_string(), ACTION_CTX.to_string());
    ctx
}

pub fn handle(plugin: &mut DevPlugin, key: KeyWithModifier) -> bool {
    // Tab toggles Board ⇄ Inbox regardless of the current view.
    if key.bare_key == BareKey::Tab {
        plugin.view = match plugin.view {
            View::Board => View::Inbox,
            View::Inbox => View::Board,
        };
        return true;
    }
    match plugin.view {
        View::Board => handle_board(plugin, key),
        View::Inbox => handle_inbox(plugin, key),
    }
}

fn handle_board(plugin: &mut DevPlugin, key: KeyWithModifier) -> bool {
    match key.bare_key {
        BareKey::Char('h') | BareKey::Left => {
            if plugin.selected_col > 0 {
                plugin.selected_col -= 1;
                plugin.selected_row = 0;
                true
            } else {
                false
            }
        }
        BareKey::Char('l') | BareKey::Right => {
            if plugin.selected_col + 1 < LANES.len() {
                plugin.selected_col += 1;
                plugin.selected_row = 0;
                true
            } else {
                false
            }
        }
        BareKey::Char('j') | BareKey::Down => {
            let phase = LANES[plugin.selected_col].0;
            let count = plugin.tasks.iter().filter(|t| t.phase == phase).count();
            if plugin.selected_row + 1 < count {
                plugin.selected_row += 1;
                true
            } else {
                false
            }
        }
        BareKey::Char('k') | BareKey::Up => {
            if plugin.selected_row > 0 {
                plugin.selected_row -= 1;
                true
            } else {
                false
            }
        }
        BareKey::Enter => {
            if let Some(task) = selected_task(plugin) {
                let id = task.id.clone();
                let tab_name = format!("task {id}");
                if plugin.open_tab_names.iter().any(|n| n == &tab_name) {
                    // Already attached — focus that tab instead of duplicating.
                    go_to_tab_name(&tab_name);
                } else {
                    // Open the interactive agent in its own named tab.
                    // `run_command` would background it — you could never type
                    // into the agent.
                    let cmd = CommandToRun {
                        path: PathBuf::from("dev"),
                        args: vec!["task".into(), "attach".into(), id],
                        cwd: None,
                    };
                    let (tab_id, _) = open_command_pane_in_new_tab(cmd, BTreeMap::new());
                    if let Some(t) = tab_id {
                        rename_tab_with_id(t as u64, tab_name);
                    }
                }
            }
            false
        }
        BareKey::Char('a') => {
            if let Some(task) = selected_task(plugin) {
                let id = task.id.clone();
                run_command(&["dev", "task", "approve", &id], action_ctx());
            }
            false
        }
        BareKey::Char('d') => {
            if let Some(task) = selected_task(plugin) {
                let id = task.id.clone();
                run_command(&["dev", "task", "dispatch", &id], action_ctx());
            }
            false
        }
        BareKey::Char('r') => {
            // Async refresh: data arrives via the RunCommandResult event.
            plugin.fetch();
            false
        }
        _ => false,
    }
}

fn handle_inbox(plugin: &mut DevPlugin, key: KeyWithModifier) -> bool {
    match key.bare_key {
        BareKey::Char('j') | BareKey::Down => {
            if plugin.inbox_row + 1 < plugin.questions.len() {
                plugin.inbox_row += 1;
                true
            } else {
                false
            }
        }
        BareKey::Char('k') | BareKey::Up => {
            if plugin.inbox_row > 0 {
                plugin.inbox_row -= 1;
                true
            } else {
                false
            }
        }
        // Answer the selected question with option N (1-based). The option id is
        // sent as the answer string; the mutation chains a refresh so the
        // question leaves the inbox once resolved.
        BareKey::Char(c @ '1'..='9') => {
            let opt_idx = (c as usize) - ('1' as usize);
            if let Some(q) = plugin.questions.get(plugin.inbox_row) {
                if let Some(opt) = q.options.get(opt_idx) {
                    let qid = q.id.clone();
                    let ans = opt.id.clone();
                    run_command(&["dev", "task", "answer", &qid, &ans], action_ctx());
                }
            }
            false
        }
        BareKey::Char('r') => {
            plugin.fetch();
            false
        }
        _ => false,
    }
}

fn selected_task(plugin: &DevPlugin) -> Option<&dev_core::DevTask> {
    let phase = LANES[plugin.selected_col].0;
    plugin
        .tasks
        .iter()
        .filter(|t| t.phase.as_str() == phase)
        .nth(plugin.selected_row)
}
