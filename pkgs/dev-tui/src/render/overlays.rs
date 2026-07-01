use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::{truncate, wrap_line, App, SPIN};
use crate::model::{Item, ToolPurpose};

use super::layout::centered_rect;

fn kv(k: &str, v: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {k:<12}"), Style::default().fg(Color::Cyan)),
        Span::raw(v.to_string()),
    ])
}

pub(super) fn render_help(f: &mut Frame) {
    let area = centered_rect(60, 80, f.area());
    f.render_widget(Clear, area);
    let lines = vec![
        Line::from(Span::styled(
            "dev top — keys",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  navigation",
            Style::default().fg(Color::Blue),
        )),
        kv("j/k ↑/↓", "move selection (focused panel)"),
        kv("g / G", "first / last"),
        kv("tab", "cycle focus — Fleet / Inbox / Tasks"),
        kv("space", "expand/collapse group or env"),
        Line::from(""),
        Line::from(Span::styled(
            "  fleet actions (enter = menu)",
            Style::default().fg(Color::Blue),
        )),
        kv(
            "enter",
            "action menu (new/attach/code · dispatch/review · logs/diff/kill)",
        ),
        kv("l", "follow logs (dev logs -f, outside TUI)"),
        kv("a", "attach (dev attach)"),
        kv("c", "open in VS Code (dev code)"),
        kv("d", "dispatch — select backend → enter task"),
        kv("v", "toggle inspector detail ↔ diff"),
        kv("x", "kill agents (dev kill / kill <pid>)"),
        kv("D", "view diff (dev diff | less)"),
        Line::from(""),
        Line::from(Span::styled(
            "  batch (env rows)",
            Style::default().fg(Color::Blue),
        )),
        kv("m", "toggle mark on selected env"),
        kv("M", "clear all marks"),
        kv("B", "batch menu (dispatch/diff/kill/clear)"),
        kv("n / N", "jump to next/prev waiting or error"),
        Line::from(""),
        Line::from(Span::styled(
            "  tasks / inbox",
            Style::default().fg(Color::Blue),
        )),
        kv("b", "open full kanban board (b/esc close)"),
        kv("enter", "Inbox: answer · Tasks: open board"),
        Line::from(""),
        Line::from(Span::styled("  view", Style::default().fg(Color::Blue))),
        kv("/", "filter by text (fleet)"),
        kv("w", "toggle active-only"),
        kv("r", "refresh now"),
        kv("T", "task history view"),
        kv("u", "usage dashboard"),
        kv("?", "toggle this help"),
        kv("q / esc", "quit"),
        Line::from(""),
        Line::from(Span::styled(
            "press any key to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    f.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" help ")),
        area,
    );
}

/// Single source of truth for the per-env action menu.
/// `key_action_menu` and `render_action_menu` both derive N from `.len()`.
// Grouped by concern; `key_action_menu` matches these indices in the same
// order, and `render_action_menu` inserts separators at the group boundaries
// (after index 2 and 4). Keep all three in sync when editing.
//
// `new`, `dispatch` and `review` all route through the backend picker, then the
// model picker (per-backend model list) before running — there is no standalone
// model item; model selection lives in the setup flow for each.
pub(crate) const ACTION_MENU_ITEMS: [(&str, &str); 8] = [
    // session — connect to or open a workspace
    ("new  (backend → model)", "dev agent attach --fresh"),
    ("attach  (reconnect / resume)", "dev agent attach"),
    ("open in VS Code", "dev code"),
    // dispatch — run agent work (backend → model → run)
    ("dispatch  (backend → model → task)", "dev dispatch"),
    ("review  (backend → model)", "dev review"),
    // inspect
    ("logs", "show logs in TUI"),
    ("diff", "dev diff | less"),
    // destructive
    ("kill", "dev kill / kill <pid>"),
];

/// Indices after which `render_action_menu` draws a blank separator line so the
/// [session | dispatch | inspect] groups read as distinct blocks.
const ACTION_MENU_GROUP_BREAKS: [usize; 2] = [2, 4];

pub(super) fn render_action_menu(f: &mut Frame, app: &App) {
    let target = app.selected_env_name().unwrap_or_else(|| "?".into());
    let area = centered_rect(52, 68, f.area());
    f.render_widget(Clear, area);

    let mut lines = vec![Line::from("")];
    for (i, (lbl, hint)) in ACTION_MENU_ITEMS.iter().enumerate() {
        let selected = i == app.menu_index;
        let prefix = if selected { "▶ " } else { "  " };
        let sty = if selected {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{}{}", prefix, lbl), sty),
            Span::styled(format!("   {}", hint), Style::default().fg(Color::DarkGray)),
        ]));
        if ACTION_MENU_GROUP_BREAKS.contains(&i) {
            lines.push(Line::from(""));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " j/k move · enter execute · esc cancel",
        Style::default().fg(Color::DarkGray),
    )));

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", target)),
        ),
        area,
    );
}

pub(super) fn render_tool_pick(f: &mut Frame, app: &App) {
    let purpose_label = match app.tool_purpose {
        ToolPurpose::Start => "new",
        ToolPurpose::Dispatch => "dispatch",
        ToolPurpose::Review => "review",
    };
    let tools = app.tools_for_picker(app.tool_purpose);
    let area = centered_rect(44, 52, f.area());
    f.render_widget(Clear, area);

    let mut lines = vec![Line::from("")];
    for (i, tool) in tools.iter().enumerate() {
        let selected = i == app.tool_index;
        let prefix = if selected { "▶ " } else { "  " };
        let sty = if selected {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(format!("{}{}", prefix, tool), sty)));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " j/k move · enter confirm · esc back",
        Style::default().fg(Color::DarkGray),
    )));

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} → {} ", purpose_label, app.dispatch_target)),
        ),
        area,
    );
}

pub(super) fn render_followup(f: &mut Frame, app: &App) {
    let area = centered_rect(64, 28, f.area());
    f.render_widget(Clear, area);
    let lines = vec![
        Line::from(vec![
            Span::styled("followup → ", Style::default().fg(Color::Cyan)),
            Span::styled(
                app.followup_target.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "continues the run in the background, seeded with its prior result",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("say:    ", Style::default().fg(Color::DarkGray)),
            Span::raw(app.followup_input.clone()),
            Span::styled("▏", Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "enter: send    esc: cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" followup "))
            .wrap(Wrap { trim: false }),
        area,
    );
}

pub(super) fn render_dispatch(f: &mut Frame, app: &App) {
    let area = centered_rect(64, 32, f.area());
    f.render_widget(Clear, area);
    let tool_display = if app.dispatch_tool.is_empty() {
        "claude  (default)".to_string()
    } else {
        app.dispatch_tool.clone()
    };
    let target_display = if !app.dispatch_targets.is_empty() {
        format!("{} envs", app.dispatch_targets.len())
    } else {
        app.dispatch_target.clone()
    };
    let model_display = if app.dispatch_model.is_empty() {
        "(backend default)".to_string()
    } else {
        app.dispatch_model.clone()
    };
    let effort_display = if app.dispatch_effort.is_empty() {
        String::new()
    } else {
        format!("  effort: {}", app.dispatch_effort)
    };
    let lines = vec![
        Line::from(vec![
            Span::styled("dispatch → ", Style::default().fg(Color::Cyan)),
            Span::styled(
                target_display,
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("backend:", Style::default().fg(Color::DarkGray)),
            Span::styled(tool_display, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("model:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(model_display, Style::default().fg(Color::Yellow)),
            Span::styled(effort_display, Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("task:   ", Style::default().fg(Color::DarkGray)),
            Span::raw(app.dispatch_input.clone()),
            Span::styled("▏", Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("track:  ", Style::default().fg(Color::DarkGray)),
            if app.dispatch_supervise {
                Span::styled(
                    "supervised (agent can ask → Inbox)",
                    Style::default().fg(Color::Green),
                )
            } else {
                Span::styled("off (quick dispatch)", Style::default().fg(Color::DarkGray))
            },
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "enter: run    tab: toggle track    esc: cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" dispatch "))
            .wrap(Wrap { trim: false }),
        area,
    );
}

pub(super) fn render_log_view(f: &mut Frame, area: Rect, app: &App) {
    let title = if app.tail_pid.is_some() {
        // Live tail (review or logs): spinner + elapsed so it never looks frozen.
        let spin = SPIN[(app.spinner as usize) % SPIN.len()];
        let elapsed = app.tail_started.map(|t| t.elapsed().as_secs()).unwrap_or(0);
        format!(" {} live: {} · {}s ", spin, app.log_target, elapsed)
    } else if app.log_target.is_empty() {
        " log ".to_string()
    } else {
        format!(" log: {} ", app.log_target)
    };
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);
    let h = inner.height as usize;
    let w = inner.width as usize;

    if app.log_lines.is_empty() {
        f.render_widget(Paragraph::new(Span::styled("(no logs)", dim)), inner);
        return;
    }

    // Wrap all logical lines at inner width
    let rows: Vec<String> = app.log_lines.iter().flat_map(|l| wrap_line(l, w)).collect();
    let total = rows.len();
    let max_top = total.saturating_sub(h);
    let top = if app.log_follow {
        max_top
    } else {
        app.log_scroll.min(max_top)
    };

    let lines: Vec<Line> = rows[top..(top + h).min(total)]
        .iter()
        .map(|l| Line::from(Span::styled(l.clone(), dim)))
        .collect();

    f.render_widget(Paragraph::new(lines), inner);
}

pub(super) fn render_confirm(f: &mut Frame, app: &App) {
    let area = centered_rect(52, 24, f.area());
    f.render_widget(Clear, area);
    let sel = app
        .list_state
        .selected()
        .and_then(|s| app.view.get(s))
        .cloned();
    let (action, target) = match sel {
        Some(Item::EnvRow(i)) => ("kill all agents in", app.envs[i].name.clone()),
        Some(Item::AgentRow(i, j)) => ("kill pid", app.envs[i].agents[j].pid.clone()),
        _ => return,
    };
    let lines = vec![
        Line::from(vec![
            Span::styled(format!("{action} "), Style::default().fg(Color::Red)),
            Span::styled(target, Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" ?"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "y / enter: yes      n / esc: no",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" confirm ")
                .border_style(Style::default().fg(Color::Red)),
        ),
        area,
    );
}

pub(super) fn render_batch_menu(f: &mut Frame, app: &App) {
    let n = app.marked.len();
    let area = centered_rect(54, 60, f.area());
    f.render_widget(Clear, area);

    let actions: [(&str, &str); 4] = [
        ("dispatch  (tool → task)", "dev dispatch each"),
        ("diff", "dev diff | less per env"),
        ("kill", "dev kill each (no confirm)"),
        ("clear marks", "deselect all"),
    ];

    let mut lines = vec![Line::from("")];
    for (i, (lbl, hint)) in actions.iter().enumerate() {
        let selected = i == app.batch_menu_index;
        let prefix = if selected { "▶ " } else { "  " };
        let sty = if selected {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{}{}", prefix, lbl), sty),
            Span::styled(format!("   {}", hint), Style::default().fg(Color::DarkGray)),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " j/k move · enter execute · esc cancel",
        Style::default().fg(Color::DarkGray),
    )));

    // List marked envs at the bottom for confirmation
    let mut env_list: Vec<String> = app.marked.iter().cloned().collect();
    env_list.sort();
    lines.push(Line::from(""));
    for e in env_list.iter().take(6) {
        lines.push(Line::from(Span::styled(
            format!("  ✓ {}", e),
            Style::default().fg(Color::Cyan),
        )));
    }
    if env_list.len() > 6 {
        lines.push(Line::from(Span::styled(
            format!("  … {} more", env_list.len() - 6),
            Style::default().fg(Color::DarkGray),
        )));
    }

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" batch — {} envs marked ", n)),
        ),
        area,
    );
}

pub(super) fn render_result_view(f: &mut Frame, app: &App) {
    let title = if app.result_inflight {
        format!(" {} … ", app.result_title)
    } else {
        format!(" {} ", app.result_title)
    };
    let area = centered_rect(88, 86, f.area());
    f.render_widget(Clear, area);
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);
    let h = inner.height as usize;
    let w = inner.width as usize;

    let rows: Vec<String> = app
        .result_lines
        .iter()
        .flat_map(|l| wrap_line(l, w))
        .collect();
    let total = rows.len();
    let max_top = total.saturating_sub(h.saturating_sub(1));
    let top = app.result_scroll.min(max_top);
    let mut lines: Vec<Line> = rows[top..(top + h.saturating_sub(1)).min(total)]
        .iter()
        .map(|l| Line::from(Span::styled(l.clone(), dim)))
        .collect();

    let hint = if app.result_inflight {
        "(running…)".to_string()
    } else {
        let followup = if app.result_target.is_empty() {
            ""
        } else {
            "   f: followup"
        };
        format!(
            "q/esc: close   j/k: scroll{}   {}/{} lines",
            followup,
            top + 1,
            total.max(1)
        )
    };
    lines.push(Line::from(Span::styled(
        hint,
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(Paragraph::new(lines), inner);
}

pub(super) fn render_model_picker(f: &mut Frame, app: &App) {
    let tool = if app.dispatch_tool.is_empty() {
        "claude"
    } else {
        &app.dispatch_tool
    };
    let models = app.models_for_picker(tool);
    let area = centered_rect(56, 62, f.area());
    f.render_widget(Clear, area);

    let current = if app.dispatch_model.is_empty() {
        "(default)".to_string()
    } else {
        app.dispatch_model.clone()
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("current: ", Style::default().fg(Color::DarkGray)),
            Span::styled(current, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
    ];
    for (i, (label, _)) in models.iter().enumerate() {
        let selected = i == app.model_pick_index;
        let sty = if selected {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(
            format!("{}  {}", if selected { "▶" } else { " " }, label),
            sty,
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " j/k move · enter select · c backend default · esc back",
        Style::default().fg(Color::DarkGray),
    )));

    let purpose = match app.tool_purpose {
        ToolPurpose::Start => "new",
        ToolPurpose::Dispatch => "dispatch",
        ToolPurpose::Review => "review",
    };
    f.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(format!(
            " {} → {} · model ({}) ",
            purpose, app.dispatch_target, tool
        ))),
        area,
    );
}

pub(super) fn render_task_view(f: &mut Frame, app: &App) {
    let area = centered_rect(90, 88, f.area());
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" task history ({}) ", app.tasks.len()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);
    let h = inner.height as usize;
    let w = inner.width as usize;

    if app.tasks.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled("no tasks dispatched yet", dim)),
            inner,
        );
        return;
    }

    // Show tasks in reverse chronological order (newest first).
    let tasks_rev: Vec<&crate::task::Task> = app.tasks.iter().rev().collect();
    let total = tasks_rev.len();
    let max_top = total.saturating_sub(h.saturating_sub(1));
    let top = app.task_scroll.min(max_top);

    let mut lines: Vec<Line<'static>> = Vec::new();
    for task in tasks_rev.iter().skip(top).take(h.saturating_sub(1)) {
        let icon = match task.status {
            crate::task::TaskStatus::Pending => "○",
            crate::task::TaskStatus::Running => "⏳",
            crate::task::TaskStatus::Success => "✓",
            crate::task::TaskStatus::Error => "✗",
            crate::task::TaskStatus::Killed => "⊘",
        };
        let icon_style = match task.status {
            crate::task::TaskStatus::Pending => Style::default().fg(Color::DarkGray),
            crate::task::TaskStatus::Running => Style::default().fg(Color::Yellow),
            crate::task::TaskStatus::Success => Style::default().fg(Color::Green),
            crate::task::TaskStatus::Error => Style::default().fg(Color::Red),
            crate::task::TaskStatus::Killed => Style::default().fg(Color::DarkGray),
        };
        let elapsed = if let Some(fin) = task.finished_at {
            crate::task::format_elapsed(fin - task.started_at)
        } else {
            let now = crate::task::unix_now();
            format!("{}…", crate::task::format_elapsed(now - task.started_at))
        };
        let task_text = truncate(&task.task_text, w.saturating_sub(40));
        let target_str = truncate(&task.target, 16);
        let tool_str = truncate(&task.tool, 8);

        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", icon), icon_style),
            Span::styled(
                format!("{:<16}", target_str),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(format!(" {:<8}", tool_str), dim),
            Span::styled(
                format!(" {:>8} ", elapsed),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(task_text, Style::default()),
        ]));
    }

    let hint = format!(
        "j/k: scroll  g/G: top/end  {}/{} tasks  esc: back",
        (top + 1).min(total),
        total,
    );
    lines.push(Line::from(Span::styled(hint, dim)));

    f.render_widget(Paragraph::new(lines), inner);
}
