//! Key handler for the dev Zellij plugin.
//!
//! zellij-tile 0.41 uses `KeyWithModifier { bare_key: BareKey, .. }` instead
//! of the old `Key` enum, so we match on `key.bare_key`.

use zellij_tile::prelude::*;
use std::collections::BTreeMap;
use crate::{DevPlugin, LANES};

pub fn handle(plugin: &mut DevPlugin, key: KeyWithModifier) -> bool {
    match key.bare_key {
        BareKey::Char('h') | BareKey::Left => {
            if plugin.selected_col > 0 {
                plugin.selected_col -= 1;
                plugin.selected_row = 0;
                true
            } else { false }
        }
        BareKey::Char('l') | BareKey::Right => {
            if plugin.selected_col + 1 < LANES.len() {
                plugin.selected_col += 1;
                plugin.selected_row = 0;
                true
            } else { false }
        }
        BareKey::Char('j') | BareKey::Down => {
            let phase = LANES[plugin.selected_col].0;
            let count = plugin.tasks.iter().filter(|t| t.phase == phase).count();
            if plugin.selected_row + 1 < count {
                plugin.selected_row += 1;
                true
            } else { false }
        }
        BareKey::Char('k') | BareKey::Up => {
            if plugin.selected_row > 0 {
                plugin.selected_row -= 1;
                true
            } else { false }
        }
        BareKey::Enter => {
            if let Some(task) = selected_task(plugin) {
                let id = task.id.clone();
                run_command(&["dev", "task", "attach", &id], BTreeMap::new());
            }
            false
        }
        BareKey::Char('a') => {
            if let Some(task) = selected_task(plugin) {
                let id = task.id.clone();
                run_command(&["dev", "task", "approve", &id], BTreeMap::new());
            }
            false
        }
        BareKey::Char('d') => {
            if let Some(task) = selected_task(plugin) {
                let id = task.id.clone();
                run_command(&["dev", "task", "dispatch", &id], BTreeMap::new());
            }
            false
        }
        BareKey::Char('r') => {
            let (tasks, questions) = dev_core::load_dev_tasks();
            plugin.tasks = tasks;
            plugin.questions = questions;
            true
        }
        _ => false,
    }
}

fn selected_task(plugin: &DevPlugin) -> Option<&dev_core::DevTask> {
    let phase = LANES[plugin.selected_col].0;
    plugin.tasks.iter()
        .filter(|t| t.phase.as_str() == phase)
        .nth(plugin.selected_row)
}
