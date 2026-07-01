use std::collections::HashMap;
use std::time::Duration;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, Focus, InspectorView, Mode, SPIN};
use crate::model::Item;

mod fleet;
mod inbox;
mod layout;
mod overlays;
mod tasks;
mod theme;
mod usage;

pub(crate) use overlays::ACTION_MENU_ITEMS;

pub(crate) fn ui(f: &mut Frame, app: &mut App) {
    let v = Layout::vertical([
        Constraint::Length(1), // summary
        Constraint::Length(1), // focus bar
        Constraint::Min(0),    // cockpit
        Constraint::Length(1), // bottom
    ])
    .split(f.area());
    render_summary(f, v[0], app);
    render_focus_bar(f, v[1], app);
    if app.mode == Mode::LogView {
        overlays::render_log_view(f, v[2], app);
    } else {
        render_cockpit(f, v[2], app);
    }
    render_bottom(f, v[3], app);
    match app.mode {
        Mode::Help => overlays::render_help(f),
        Mode::Dispatch => overlays::render_dispatch(f, app),
        Mode::ConfirmKill => overlays::render_confirm(f, app),
        Mode::ActionMenu => overlays::render_action_menu(f, app),
        Mode::ToolPick => overlays::render_tool_pick(f, app),
        Mode::BatchMenu => overlays::render_batch_menu(f, app),
        Mode::ResultView => overlays::render_result_view(f, app),
        Mode::Followup => overlays::render_followup(f, app),
        Mode::ModelPicker => overlays::render_model_picker(f, app),
        Mode::TaskView => overlays::render_task_view(f, app),
        Mode::UsageView => usage::render_usage_view(f, app),
        Mode::BoardModal => tasks::render_board_modal(f, app),
        _ => {}
    }
}

/// The unified cockpit: left column (Fleet / Inbox / Tasks vertical stack) +
/// a universal inspector on the right that follows the focused panel.
fn render_cockpit(f: &mut Frame, area: Rect, app: &mut App) {
    let cols =
        Layout::horizontal([Constraint::Percentage(38), Constraint::Percentage(62)]).split(area);
    render_left_column(f, cols[0], app);
    render_inspector(f, cols[1], app);
}

/// Content-aware vertical stack: Inbox/Tasks size to their content (capped) so
/// Fleet reclaims the empty space when there are few questions/tasks.
fn render_left_column(f: &mut Frame, area: Rect, app: &mut App) {
    let q = app
        .dev_questions
        .iter()
        .filter(|x| x.severity == "blocking")
        .count();
    let t = app.active_tasks().len();
    let inbox_h = if q == 0 { 3 } else { (q.min(5) as u16) + 2 };
    let tasks_h = if t == 0 { 3 } else { (t.min(6) as u16) + 2 };
    let rows = Layout::vertical([
        Constraint::Min(6), // Fleet grows to fill
        Constraint::Length(inbox_h),
        Constraint::Length(tasks_h),
    ])
    .split(area);
    let focus = app.focus;
    fleet::render_fleet(f, rows[0], app, focus == Focus::Fleet);
    inbox::render_inbox_panel(f, rows[1], app, focus == Focus::Inbox);
    tasks::render_tasks_panel(f, rows[2], app, focus == Focus::Tasks);
}

/// Right pane: whatever the focused left panel + its selection points at.
fn render_inspector(f: &mut Frame, area: Rect, app: &App) {
    match app.focus {
        Focus::Fleet => match app.fleet_view {
            InspectorView::Detail => fleet::render_detail(f, area, app),
            InspectorView::Diff => fleet::render_fleet_diff(f, area, app),
        },
        Focus::Inbox => inbox::render_inbox_detail(f, area, app),
        Focus::Tasks => {
            let tasks = app.active_tasks();
            match tasks.get(app.tasks_sel) {
                Some(t) => tasks::render_task_detail_body(f, area, app, t),
                None => {
                    let block = Block::default().borders(Borders::ALL).title(" task ");
                    f.render_widget(
                        Paragraph::new(Span::styled(
                            "(no task selected)",
                            Style::default().fg(Color::DarkGray),
                        ))
                        .block(block),
                        area,
                    );
                }
            }
        }
    }
}

fn render_summary(f: &mut Frame, area: Rect, app: &App) {
    let order: [(&str, Color); 5] = [
        ("waiting", Color::Yellow),
        ("error", Color::Red),
        ("running", Color::Green),
        ("idle", Color::Gray),
        ("stopped", Color::DarkGray),
    ];
    let mut counts: HashMap<&str, usize> = HashMap::new();
    let mut total_agents = 0usize;
    for env in &app.envs {
        for a in &env.agents {
            let k = if a.status == "busy" {
                "running"
            } else {
                a.status.as_str()
            };
            *counts.entry(k).or_insert(0) += 1;
            total_agents += 1;
        }
    }
    let usage_spans: Vec<Span<'static>> = if let Some(u) = &app.claude_usage {
        let pct_style = |pct: u32| -> Style {
            if pct >= 85 {
                Style::default().fg(Color::Red)
            } else if pct >= 70 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Green)
            }
        };
        let mut s: Vec<Span<'static>> = vec![Span::styled(
            "│ claude ",
            Style::default().fg(Color::DarkGray),
        )];
        if let Some(p) = u.five_hour_pct {
            s.push(Span::styled("5h:", Style::default().fg(Color::DarkGray)));
            s.push(Span::styled(format!("{p}%"), pct_style(p)));
            s.push(Span::raw(" "));
        }
        if let Some(p) = u.seven_day_pct {
            s.push(Span::styled("7d:", Style::default().fg(Color::DarkGray)));
            s.push(Span::styled(format!("{p}%"), pct_style(p)));
            s.push(Span::raw(" "));
        }
        s
    } else {
        vec![]
    };

    let mut spans: Vec<Span<'static>> = vec![
        Span::styled(
            " dev tui ",
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
    ];
    for (name, col) in order {
        if let Some(&c) = counts.get(name) {
            if c > 0 {
                spans.push(Span::styled(
                    format!("● {c} {name}  "),
                    Style::default().fg(col),
                ));
            }
        }
    }
    let n_projects = app.envs.len();
    let n_envs = app.groups.len();
    spans.push(Span::styled(
        format!("· {n_envs} envs {n_projects} projects {total_agents} agents "),
        Style::default().fg(Color::DarkGray),
    ));
    if !app.marked.is_empty() {
        let n = app.marked.len();
        spans.push(Span::styled(
            format!("✓{n} marked "),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if app.refreshing || app.git_inflight {
        spans.push(Span::styled(
            format!(" {} sync", SPIN[(app.spinner as usize) % SPIN.len()]),
            Style::default().fg(Color::Cyan),
        ));
    } else {
        spans.push(Span::styled(
            format!(" ⟳ {}s", app.last_refresh.elapsed().as_secs()),
            Style::default().fg(Color::DarkGray),
        ));
    }
    if app.active_only {
        spans.push(Span::styled(" [active]", Style::default().fg(Color::Cyan)));
    }
    spans.extend(usage_spans);

    // agy usage
    if let Some(u) = &app.agy_usage {
        let pct_style = |pct: u32| -> Style {
            if pct >= 85 {
                Style::default().fg(Color::Red)
            } else if pct >= 70 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Green)
            }
        };
        spans.push(Span::styled("│ agy ", Style::default().fg(Color::DarkGray)));
        if let (Some(avail), Some(monthly)) = (u.available_credits, u.monthly_credits) {
            let used_pct = ((monthly.saturating_sub(avail)) * 100 / monthly.max(1)) as u32;
            spans.push(Span::styled(format!("{used_pct}%"), pct_style(used_pct)));
            spans.push(Span::styled(
                format!(" ({avail}cr) "),
                Style::default().fg(Color::DarkGray),
            ));
        } else if !u.models.is_empty() {
            for (label, pct) in u.models.iter().take(2) {
                let short = label.split_whitespace().next().unwrap_or(label);
                spans.push(Span::styled(
                    format!("{short}:"),
                    Style::default().fg(Color::DarkGray),
                ));
                spans.push(Span::styled(format!("{pct}% "), pct_style(*pct)));
            }
        }
    }

    // codex usage
    if let Some(u) = &app.codex_usage {
        let pct_style = |pct: f64| -> Style {
            if pct >= 85.0 {
                Style::default().fg(Color::Red)
            } else if pct >= 70.0 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Green)
            }
        };
        spans.push(Span::styled(
            "│ codex ",
            Style::default().fg(Color::DarkGray),
        ));
        if let Some(p) = u.primary_used_pct {
            spans.push(Span::styled("5h:", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(format!("{p:.0}%"), pct_style(p)));
            spans.push(Span::raw(" "));
        }
        if let Some(p) = u.secondary_used_pct {
            spans.push(Span::styled("7d:", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(format!("{p:.0}%"), pct_style(p)));
            spans.push(Span::raw(" "));
        }
    }

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_bottom(f: &mut Frame, area: Rect, app: &App) {
    let line = if app.mode == Mode::Filter {
        Line::from(vec![
            Span::styled(
                " filter ",
                Style::default().bg(Color::Cyan).fg(Color::Black),
            ),
            Span::raw(format!(" /{}", app.filter)),
            Span::styled("▏", Style::default().fg(Color::Cyan)),
        ])
    } else if let Some((msg, t)) = &app.flash {
        if t.elapsed() < Duration::from_secs(3) {
            Line::from(Span::styled(
                format!(" {msg}"),
                Style::default().fg(Color::Green),
            ))
        } else {
            help_line(app)
        }
    } else {
        help_line(app)
    };
    f.render_widget(Paragraph::new(line), area);
}

fn help_line(app: &App) -> Line<'static> {
    let hint: &'static str = match app.mode {
        Mode::LogView => " j/k scroll · G follow · o output · f followup · esc/q back",
        Mode::Followup => " type instruction · enter send · esc cancel",
        Mode::ActionMenu => " j/k move · enter execute · esc cancel",
        Mode::ToolPick => " j/k select backend · enter confirm · esc back",
        Mode::BatchMenu => " j/k move · enter execute · esc cancel",
        Mode::TaskView => " j/k scroll · g top · G end · PgDn/PgUp · esc/q back",
        Mode::UsageView => " r refresh · esc/q back",
        Mode::BoardModal => " ←→ lane · j/k sel · i impl · a attach · l logs · r review · f fix · A approve · p plan · b/esc close",
        _ => match app.focus {
            Focus::Inbox => {
                " j/k move · enter answer · tab focus · b board · r refresh · ? help · q quit"
            }
            Focus::Tasks => {
                " j/k move · enter open board · tab focus · b board · u usage · r refresh · ? help · q quit"
            }
            Focus::Fleet => {
                let sel = app
                    .list_state
                    .selected()
                    .and_then(|s| app.view.get(s))
                    .cloned();
                match sel {
                    Some(Item::GroupHeader(_)) => {
                        " space expand/collapse · j/k move · tab focus · / filter · w active · r refresh · ? help · q quit"
                    }
                    Some(Item::AgentRow(_, _)) => {
                        " enter menu · space collapse · l logs-f · a attach · x kill · v diff · tab focus · b board · r refresh · ? help · q quit"
                    }
                    _ => " enter menu · m mark · B batch · v diff · b board · l logs · a attach · d dispatch · x kill · D diff · n/N next · T tasks · u usage · / filter · tab focus · ? help · q quit",
                }
            }
        },
    };
    Line::from(Span::styled(hint, Style::default().fg(Color::DarkGray)))
}

fn render_focus_bar(f: &mut Frame, area: Rect, app: &App) {
    let blocking = app
        .dev_questions
        .iter()
        .filter(|q| q.severity == "blocking")
        .count();
    let n_tasks = app.active_tasks().len();
    let segs: [(Focus, String); 3] = [
        (Focus::Fleet, " Fleet ".to_string()),
        (
            Focus::Inbox,
            if blocking > 0 {
                format!(" Inbox ⚠{} ", blocking)
            } else {
                " Inbox ".to_string()
            },
        ),
        (
            Focus::Tasks,
            if n_tasks > 0 {
                format!(" Tasks {} ", n_tasks)
            } else {
                " Tasks ".to_string()
            },
        ),
    ];
    let mut spans: Vec<Span<'static>> = Vec::new();
    for (focus, label) in segs {
        let active = app.focus == focus;
        let style = if active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(label, style));
        spans.push(Span::raw("  "));
    }
    spans.push(Span::styled(
        "tab:focus  b:board",
        Style::default().fg(Color::DarkGray),
    ));
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}
