//! `dev tui` / `dev board` / `dev dash` — launch the fleet UIs.

use std::os::unix::process::CommandExt;
use std::process::Command;

/// Exec the ratatui TUI binary (installed alongside `dev`).
pub fn tui(args: Vec<String>) {
    let err = Command::new("dev-tui").args(&args).exec();
    eprintln!("dev tui: {err} (is dev-tui installed?)");
    std::process::exit(1);
}

/// Zellij task-board plugin: floating pane inside Zellij, else a new session.
pub fn board() {
    let home = std::env::var("HOME").unwrap_or_default();
    let plugin = format!("file:{home}/.config/zellij/plugins/dev.wasm");
    if std::env::var_os("ZELLIJ").is_some() {
        let err = Command::new("zellij")
            .args(["plugin", "--floating", "--", &plugin])
            .exec();
        eprintln!("dev board: {err}");
        std::process::exit(1);
    }
    let ok = Command::new("zellij")
        .args(["--layout", "plugin", "--", &plugin])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        let _ = Command::new("zellij")
            .args(["plugin", "--", &plugin])
            .status();
    }
}

/// `dash` is superseded by the ratatui TUI.
pub fn dash() {
    tui(Vec::new());
}
