//! dev top — live TUI over the `dev` CLI's machine-readable surface.

use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::*;

mod app;
mod data;
mod input;
mod model;
mod render;
mod task;
mod terminal;
mod usage;
mod watcher;

use app::App;
use data::{worker, Msg, Req};

fn run(term: &mut terminal::Term, app: &mut App) -> io::Result<()> {
    loop {
        let mut msgs = Vec::new();
        while let Ok(m) = app.msg_rx.try_recv() {
            msgs.push(m);
        }
        for m in msgs {
            app.apply(m);
        }

        if !app.refreshing && app.last_refresh.elapsed() >= app.interval {
            app.request_refresh();
        }
        if !app.git_inflight && app.last_git.elapsed() >= app.git_interval {
            app.request_git();
        }
        if app.last_usage.elapsed() >= Duration::from_secs(180) {
            app.last_usage = std::time::Instant::now();
            let _ = app.req_tx.send(Req::Usage);
            app.record_usage_sample();
            app.usage_history.save();
        }
        if app.last_agy_usage.elapsed() >= Duration::from_secs(180) {
            app.last_agy_usage = std::time::Instant::now();
            let _ = app.req_tx.send(Req::AgyUsage);
        }
        if !app.codex_usage_inflight && app.last_codex_usage.elapsed() >= Duration::from_secs(180) {
            app.request_codex_usage();
        }
        if !app.tasks_inflight && app.last_tasks.elapsed() >= Duration::from_secs(300) {
            app.request_dev_tasks();
        }
        app.maybe_request_logs();
        if app.refreshing || app.git_inflight {
            app.spinner = app.spinner.wrapping_add(1);
        }

        term.draw(|f| render::ui(f, app))?;

        if event::poll(Duration::from_millis(120))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && app.handle_key(key, term) {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let (req_tx, req_rx) = mpsc::channel::<Req>();
    let (msg_tx, msg_rx) = mpsc::channel::<Msg>();
    let msg_tx_for_app = msg_tx.clone();
    thread::spawn(move || worker(req_rx, msg_tx));

    let mut app = App::new(req_tx, msg_rx, msg_tx_for_app);

    app.request_refresh();
    app.request_git();
    let _ = app.req_tx.send(Req::Tools);
    let _ = app.req_tx.send(Req::Usage);
    let _ = app.req_tx.send(Req::AgyUsage);
    app.request_codex_usage();
    app.request_dev_tasks();

    let _task_watcher = watcher::start_task_watcher(app.req_tx.clone());

    let res = run(&mut terminal, &mut app);
    app.stop_tail();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    res
}
