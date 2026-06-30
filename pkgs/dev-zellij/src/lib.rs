//! Zellij plugin for dev task orchestration.
//! Phase 0 skeleton: full implementation in Phase 5.

use zellij_tile::prelude::*;
use std::collections::BTreeMap;

#[derive(Default)]
struct DevPlugin;

impl ZellijPlugin for DevPlugin {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {}
    fn update(&mut self, _event: Event) -> bool { false }
    fn render(&mut self, _rows: usize, _cols: usize) {
        println!("dev-zellij: loading...");
    }
}

register_plugin!(DevPlugin);
