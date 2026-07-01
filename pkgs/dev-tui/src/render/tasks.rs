use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use crate::app::{truncate, App};

use super::layout::centered_rect;
use super::theme::{panel_block, phase_style};

/// Full 7-lane kanban, opened on demand with `b` — a fullscreen overlay.
pub(super) fn render_board_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(94, 90, f.area());
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" board — full kanban  (b/esc close) ");
    let inner = block.inner(area);
    f.render_widget(block, area);
    let h =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).split(inner);
    render_board_left(f, h[0], app);
    render_board_right(f, h[1], app);
}

fn render_board_left(f: &mut Frame, area: Rect, app: &App) {
    // Split: lane selector header (1 line) + task list
    let v = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area);

    // Lane selector header
    let col = app.board_col;
    let mut lane_spans: Vec<Span<'static>> = Vec::new();
    for (i, (name, phase)) in App::BOARD_LANES.iter().enumerate() {
        let count = app
            .dev_tasks
            .iter()
            .filter(|t| t.phase.as_str() == *phase)
            .count();
        let label = format!(" {name}({count}) ");
        let style = if i == col {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        lane_spans.push(Span::styled(label, style));
        lane_spans.push(Span::raw(" "));
    }
    f.render_widget(Paragraph::new(Line::from(lane_spans)), v[0]);

    // Task list for current lane
    let lane_tasks = app.tasks_for_lane(col);
    let height = v[1].height as usize;
    let sel = if lane_tasks.is_empty() {
        0
    } else {
        app.board_sel.min(lane_tasks.len().saturating_sub(1))
    };

    if lane_tasks.is_empty() {
        let msg = Paragraph::new("(no tasks in this lane)")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::NONE));
        f.render_widget(msg, v[1]);
    } else {
        let offset = sel.saturating_sub(height.saturating_sub(4) / 2);
        let mut items: Vec<ListItem<'static>> = Vec::new();
        for (i, task) in lane_tasks.iter().enumerate().skip(offset).take(height) {
            let selected = i == sel;
            let id_short = task.id.clone();
            let title = truncate(&task.title, 32);
            let tool = task.assigned_tool.as_deref().unwrap_or("-").to_string();
            let q_count = app
                .dev_questions
                .iter()
                .filter(|q| q.task_id == task.id)
                .count();
            let project_id = task.project_id.clone();
            let line1 = if selected {
                Line::from(vec![
                    Span::styled(
                        format!("{id_short}  "),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(title, Style::default().add_modifier(Modifier::BOLD)),
                ])
            } else {
                Line::from(vec![
                    Span::styled(
                        format!("{id_short}  "),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw(title),
                ])
            };
            let line2 = Line::from(vec![Span::styled(
                format!("  {}  {}  q:{}", project_id, tool, q_count),
                Style::default().fg(Color::DarkGray),
            )]);
            let style = if selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            items.push(ListItem::new(vec![line1, line2]).style(style));
        }
        let list = List::new(items).block(Block::default().borders(Borders::NONE));
        f.render_widget(list, v[1]);
    }

    // Key hints at bottom
    let hints = "←→:lane  j/k:sel  A:approve  p:plan  ?:help";
    let hint_area = Rect {
        x: area.x,
        y: area.bottom().saturating_sub(1),
        width: area.width,
        height: 1,
    };
    f.render_widget(
        Paragraph::new(hints)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Right),
        hint_area,
    );
}

fn render_board_right(f: &mut Frame, area: Rect, app: &App) {
    let lanes = app.tasks_for_lane(app.board_col);
    let sel = app.board_sel.min(lanes.len().saturating_sub(1));
    match lanes.get(sel) {
        Some(t) => render_task_detail_body(f, area, app, t),
        None => {
            let empty = Paragraph::new("(no task selected)").block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Task Detail "),
            );
            f.render_widget(empty, area);
        }
    }
}

/// Render a single task's detail (brief/plan/handoff/review + action hints) into
/// `area`. Shared by the board modal's right pane and the cockpit Tasks inspector.
pub(super) fn render_task_detail_body(f: &mut Frame, area: Rect, app: &App, task: &crate::task::DevTask) {
    let mut lines: Vec<Line<'static>> = vec![
        Line::from(vec![
            Span::styled(
                task.id.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(task.phase.clone(), phase_style(&task.phase)),
        ]),
        Line::from(vec![Span::styled(
            task.title.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled(
                format!("{} ", task.project_id),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                task.assigned_tool.as_deref().unwrap_or("-").to_string(),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(Span::raw("")),
    ];

    if let Some(detail) = &app.task_detail {
        if detail.task_id == task.id {
            if !detail.brief.is_empty() {
                lines.push(Line::from(Span::styled(
                    "Brief:",
                    Style::default().fg(Color::Yellow),
                )));
                for l in detail.brief.lines().take(4) {
                    lines.push(Line::from(Span::raw(format!("  {l}"))));
                }
                lines.push(Line::from(Span::raw("")));
            }
            if !detail.plan_summary.is_empty() {
                lines.push(Line::from(Span::styled(
                    "Plan:",
                    Style::default().fg(Color::Green),
                )));
                for l in detail.plan_summary.lines().take(8) {
                    lines.push(Line::from(Span::raw(format!("  {l}"))));
                }
                lines.push(Line::from(Span::raw("")));
            }
            if !detail.handoff.is_empty() {
                lines.push(Line::from(Span::styled(
                    "Handoff:",
                    Style::default().fg(Color::Magenta),
                )));
                for l in detail.handoff.lines().take(4) {
                    lines.push(Line::from(Span::raw(format!("  {l}"))));
                }
                lines.push(Line::from(Span::raw("")));
            }
            if !detail.review_summary.is_empty() {
                lines.push(Line::from(Span::styled(
                    "Review:",
                    Style::default().fg(Color::Red),
                )));
                for l in detail.review_summary.lines().take(3) {
                    lines.push(Line::from(Span::raw(format!("  {l}"))));
                }
            }
        } else {
            lines.push(Line::from(Span::styled(
                "(loading...)",
                Style::default().fg(Color::DarkGray),
            )));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "(loading...)",
            Style::default().fg(Color::DarkGray),
        )));
    }

    // Action hints
    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(Span::styled(
        "i:impl  a:attach  l:logs  h:harvest  d:diff  t:test  r:review  f:fix  m:pr",
        Style::default().fg(Color::DarkGray),
    )));

    let detail_widget = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Task Detail "),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(detail_widget, area);
}

/// Cockpit Tasks panel — compact 1-line-per-task list over `active_tasks()`.
pub(super) fn render_tasks_panel(f: &mut Frame, area: Rect, app: &App, focused: bool) {
    let tasks = app.active_tasks();
    let title = if tasks.is_empty() {
        " tasks ".to_string()
    } else {
        format!(" tasks {} ", tasks.len())
    };
    let block = panel_block(&title, focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if tasks.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                "no active tasks",
                Style::default().fg(Color::DarkGray),
            )),
            inner,
        );
        return;
    }

    let sel = app.tasks_sel.min(tasks.len().saturating_sub(1));
    let height = inner.height as usize;
    let offset = sel.saturating_sub(height.saturating_sub(1));
    let w = inner.width as usize;
    let mut items: Vec<ListItem<'static>> = Vec::new();
    for (i, t) in tasks.iter().enumerate().skip(offset).take(height) {
        let selected = i == sel && focused;
        let marker = if selected { "▶ " } else { "  " };
        let q_count = app
            .dev_questions
            .iter()
            .filter(|q| q.task_id == t.id)
            .count();
        let qsuffix = if q_count > 0 {
            format!(" q:{}", q_count)
        } else {
            String::new()
        };
        let title_w = w.saturating_sub(marker.len() + 2 + t.id.len() + 1 + qsuffix.len());
        let title_txt = truncate(&t.title, title_w.max(6));
        let line = Line::from(vec![
            Span::raw(marker),
            Span::styled("● ", phase_style(&t.phase)),
            Span::styled(format!("{} ", t.id), Style::default().fg(Color::DarkGray)),
            Span::raw(title_txt),
            Span::styled(qsuffix, Style::default().fg(Color::Yellow)),
        ]);
        let style = if selected {
            Style::default().bg(Color::DarkGray)
        } else {
            Style::default()
        };
        items.push(ListItem::new(line).style(style));
    }
    f.render_widget(List::new(items), inner);
}
