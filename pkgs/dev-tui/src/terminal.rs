use std::io::Stdout;
use std::process::{Command, Stdio};

use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::*;

pub type Term = Terminal<CrosstermBackend<Stdout>>;

/// Suspend the TUI, run `dev <args>` inline in this terminal, then restore.
pub fn run_dev(args: &[&str], term: &mut Term) {
    let _ = disable_raw_mode();
    let _ = execute!(term.backend_mut(), LeaveAlternateScreen);
    let _ = Command::new("dev").args(args).status();
    let _ = enable_raw_mode();
    let _ = execute!(term.backend_mut(), EnterAlternateScreen);
    let _ = term.clear();
}

/// Suspend the TUI, run a shell pipeline inline, then restore.
pub fn run_shell(cmd: &str, term: &mut Term) {
    let _ = disable_raw_mode();
    let _ = execute!(term.backend_mut(), LeaveAlternateScreen);
    let _ = Command::new("sh").arg("-c").arg(cmd).status();
    let _ = enable_raw_mode();
    let _ = execute!(term.backend_mut(), EnterAlternateScreen);
    let _ = term.clear();
}

/// True when running inside a Zellij session.
fn in_zellij() -> bool {
    std::env::var_os("ZELLIJ").is_some()
}

/// Open `argv` in a fresh, tiled Zellij pane named `name` that closes when the
/// command exits. `zellij run` dispatches the action to the session over its
/// socket and returns immediately, so it neither blocks nor touches this pane's
/// terminal — the fleet TUI keeps drawing (and picks up its new size on the next
/// frame) while the freshly opened pane takes focus. Returns whether the pane
/// was opened.
fn zellij_pane(name: &str, argv: &[&str]) -> bool {
    Command::new("zellij")
        .args(["run", "--close-on-exit", "--name", name, "--"])
        .args(argv)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Launch an interactive `dev <args>` session (an agent attach, a follow-log, …).
/// Inside Zellij this opens a new tiled pane (`name`) so the fleet TUI stays live
/// beside it; otherwise it falls back to suspending the TUI and running inline.
pub fn run_dev_pane(name: &str, args: &[&str], term: &mut Term) {
    if in_zellij() {
        let mut argv = Vec::with_capacity(args.len() + 1);
        argv.push("dev");
        argv.extend_from_slice(args);
        if zellij_pane(name, &argv) {
            return;
        }
        // Pane couldn't be opened (no server?) — fall back to inline.
    }
    run_dev(args, term);
}

/// Interactive shell pipeline (e.g. a `… | less` pager): a new tiled Zellij pane
/// inside a session, an inline suspend otherwise.
pub fn run_shell_pane(name: &str, cmd: &str, term: &mut Term) {
    if in_zellij() && zellij_pane(name, &["sh", "-c", cmd]) {
        return;
    }
    run_shell(cmd, term);
}
