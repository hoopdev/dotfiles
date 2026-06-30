//! Zellij plugin — dev task board.

use zellij_tile::prelude::*;
use std::collections::BTreeMap;
use dev_core::{DevTask, DevQuestion, load_dev_tasks};

mod render;
mod keys;

#[derive(Default)]
struct DevPlugin {
    tasks: Vec<DevTask>,
    questions: Vec<DevQuestion>,
    selected_col: usize,  // lane index 0-5
    selected_row: usize,  // task index within lane
    loading: bool,
}

pub const LANES: &[(&str, &str)] = &[
    ("needs_spec",   "Needs Spec"),
    ("planned",      "Planned"),
    ("implementing", "Running"),
    ("review",       "Review"),
    ("needs_fix",    "Needs Fix"),
    ("mergeable",    "Mergeable"),
];

impl ZellijPlugin for DevPlugin {
    fn load(&mut self, _config: BTreeMap<String, String>) {
        self.loading = true;
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::RunCommands,
        ]);
        subscribe(&[EventType::Timer, EventType::Key]);
        set_timeout(5.0);
        // Immediately load tasks
        let (tasks, questions) = load_dev_tasks();
        self.tasks = tasks;
        self.questions = questions;
        self.loading = false;
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::Timer(_) => {
                let (tasks, questions) = load_dev_tasks();
                self.tasks = tasks;
                self.questions = questions;
                set_timeout(5.0);
                true
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
