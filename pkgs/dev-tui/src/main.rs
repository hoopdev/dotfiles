//! dev top — a live TUI over the `dev` CLI's machine-readable surface.
//!
//! Architecture: `dev … --json` is the contract. This TUI is just a *client* of
//! it — it polls `dev ps --json` for fleet state and shells out to `dev attach /
//! logs / kill / dispatch` for actions (the same commands an LLM orchestrator or
//! a human at the shell would run). It holds no logic of its own about
//! local/remote, Coder, or 1Password; all of that lives in `dev`.

use std::io::{self, Stdout, Write};
use std::process::Command;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use serde_json::Value;

struct Agent {
    target: String,
    location: String,
    tool: String,
    status: String,
    pid: String,
}

/// Sort key: agents that need a human (waiting) rise to the top, then errors,
/// then active, then idle, then stopped/unreachable.
fn status_rank(s: &str) -> u8 {
    match s {
        "waiting" => 0,
        "error" => 1,
        "busy" | "running" => 2,
        "idle" => 3,
        "stopped" => 5,
        "unreachable" => 6,
        _ => 4,
    }
}

fn status_style(s: &str) -> Style {
    match s {
        "waiting" => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        "error" | "unreachable" => Style::default().fg(Color::Red),
        "busy" | "running" => Style::default().fg(Color::Green),
        "stopped" => Style::default().fg(Color::DarkGray),
        _ => Style::default(),
    }
}

fn pid_to_string(v: Option<&Value>) -> String {
    match v {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        _ => "-".to_string(),
    }
}

fn fetch_agents() -> Vec<Agent> {
    let mut out = Vec::new();
    let cmd = Command::new("dev").args(["ps", "--json"]).output();
    if let Ok(o) = cmd {
        if let Ok(Value::Array(arr)) = serde_json::from_slice::<Value>(&o.stdout) {
            for a in &arr {
                out.push(Agent {
                    target: a.get("target").and_then(|x| x.as_str()).unwrap_or("?").to_string(),
                    location: a.get("location").and_then(|x| x.as_str()).unwrap_or("?").to_string(),
                    tool: a.get("tool").and_then(|x| x.as_str()).unwrap_or("-").to_string(),
                    status: a.get("status").and_then(|x| x.as_str()).unwrap_or("?").to_string(),
                    pid: pid_to_string(a.get("pid")),
                });
            }
        }
    }
    out.sort_by(|a, b| {
        status_rank(&a.status)
            .cmp(&status_rank(&b.status))
            .then(a.target.cmp(&b.target))
    });
    out
}

type Term = Terminal<CrosstermBackend<Stdout>>;

/// Drop out of the alternate screen, run `dev <args>` with the real terminal,
/// then restore the TUI. Used for interactive actions (attach/logs) and kill.
fn run_dev(args: &[&str], term: &mut Term) {
    let _ = disable_raw_mode();
    let _ = execute!(term.backend_mut(), LeaveAlternateScreen);
    let _ = Command::new("dev").args(args).status();
    let _ = enable_raw_mode();
    let _ = execute!(term.backend_mut(), EnterAlternateScreen);
    let _ = term.clear();
}

fn prompt_dispatch(target: &str, term: &mut Term) {
    let _ = disable_raw_mode();
    let _ = execute!(term.backend_mut(), LeaveAlternateScreen);
    print!("dispatch task for {target} (empty cancels): ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line);
    let task = line.trim();
    if !task.is_empty() {
        let _ = Command::new("dev").args(["dispatch", target, task]).status();
    }
    let _ = enable_raw_mode();
    let _ = execute!(term.backend_mut(), EnterAlternateScreen);
    let _ = term.clear();
}

fn ui(f: &mut Frame, agents: &[Agent], state: &mut TableState, since: Duration) {
    let chunks =
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).split(f.area());

    let rows: Vec<Row> = agents
        .iter()
        .map(|a| {
            Row::new(vec![
                Cell::from(a.target.clone()),
                Cell::from(a.location.clone()),
                Cell::from(a.tool.clone()),
                Cell::from(a.status.clone()).style(status_style(&a.status)),
                Cell::from(a.pid.clone()),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(24),
        Constraint::Length(8),
        Constraint::Length(10),
        Constraint::Length(13),
        Constraint::Length(8),
    ];
    let title = format!(
        " dev top — {} agents · refreshed {}s ago ",
        agents.len(),
        since.as_secs()
    );
    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["TARGET", "LOC", "TOOL", "STATUS", "PID"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(Block::default().borders(Borders::ALL).title(title))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("▶ ");
    f.render_stateful_widget(table, chunks[0], state);

    let help = Paragraph::new(
        "j/k:nav  enter:logs  a:attach  x:kill  d:dispatch  r:refresh  q:quit",
    )
    .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[1]);
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let mut agents = fetch_agents();
    let mut state = TableState::default();
    state.select(if agents.is_empty() { None } else { Some(0) });
    let interval = Duration::from_secs(3);
    let mut last = Instant::now();

    let res = (|| -> io::Result<()> {
        loop {
            terminal.draw(|f| ui(f, &agents, &mut state, last.elapsed()))?;

            if event::poll(Duration::from_millis(250))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    let sel = state.selected().unwrap_or(0);
                    let target = agents.get(sel).map(|a| a.target.clone());
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char('r') => {
                            agents = fetch_agents();
                            last = Instant::now();
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if !agents.is_empty() {
                                state.select(Some((sel + 1).min(agents.len() - 1)));
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            state.select(Some(sel.saturating_sub(1)));
                        }
                        KeyCode::Enter => {
                            if let Some(t) = target {
                                run_dev(&["logs", &t, "-f"], &mut terminal);
                            }
                        }
                        KeyCode::Char('a') => {
                            if let Some(t) = target {
                                run_dev(&["attach", &t], &mut terminal);
                                agents = fetch_agents();
                                last = Instant::now();
                            }
                        }
                        KeyCode::Char('x') => {
                            if let Some(t) = target {
                                run_dev(&["kill", &t], &mut terminal);
                                agents = fetch_agents();
                                last = Instant::now();
                            }
                        }
                        KeyCode::Char('d') => {
                            if let Some(t) = target {
                                prompt_dispatch(&t, &mut terminal);
                                agents = fetch_agents();
                                last = Instant::now();
                            }
                        }
                        _ => {}
                    }
                }
            }

            if last.elapsed() >= interval {
                agents = fetch_agents();
                // keep selection in range
                if let Some(s) = state.selected() {
                    if !agents.is_empty() && s >= agents.len() {
                        state.select(Some(agents.len() - 1));
                    }
                }
                last = Instant::now();
            }
        }
        Ok(())
    })();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    res
}
