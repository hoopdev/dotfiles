use std::collections::{HashMap, HashSet};

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::{truncate, wrap_line, App};
use crate::model::{
    env_dominant_status, env_status_label, remote_meta_label, status_style, Env, GitState,
    GroupInfo, Item,
};

use super::theme::{diff_line_style, panel_block};

/// Fleet inspector Diff view — cached `dev git diff <env>` with scroll + color.
pub(super) fn render_fleet_diff(f: &mut Frame, area: Rect, app: &App) {
    let target = app.selected_env_name().unwrap_or_default();
    let title = if target.is_empty() {
        " diff  (v: back) ".to_string()
    } else {
        format!(" diff: {}  (v: back) ", target)
    };
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);
    let h = inner.height as usize;
    let w = inner.width as usize;
    if target.is_empty() {
        f.render_widget(Paragraph::new(Span::styled("no selection", dim)), inner);
        return;
    }
    if app.fleet_diff_target != target {
        f.render_widget(
            Paragraph::new(Span::styled("(loading… press v to refresh)", dim)),
            inner,
        );
        return;
    }
    let rows: Vec<String> = app
        .fleet_diff_lines
        .iter()
        .flat_map(|l| wrap_line(l, w))
        .collect();
    let total = rows.len();
    let top = app.fleet_diff_scroll.min(total.saturating_sub(h));
    let lines: Vec<Line> = rows[top..(top + h).min(total)]
        .iter()
        .map(|l| Line::from(Span::styled(l.clone(), diff_line_style(l))))
        .collect();
    f.render_widget(Paragraph::new(lines), inner);
}

fn make_fleet_items(
    view: &[Item],
    envs: &[Env],
    git: &HashMap<String, GitState>,
    groups: &[GroupInfo],
    group_collapsed: &HashMap<String, bool>,
    marked: &HashSet<String>,
    inner_w: u16,
) -> Vec<ListItem<'static>> {
    // layout after the "▶ " highlight symbol (2 chars):
    // expand(2) + dot(2) + name(20) + status(18) + delta(6)
    let w = inner_w as usize;
    let fixed = 2 + 2 + 20 + 18 + 6;
    let name_w = w.saturating_sub(fixed + 2 /* highlight sym */).min(20);

    view.iter()
        .map(|item| match item {
            Item::GroupHeader(gi) => {
                let g = &groups[*gi];
                let collapsed = group_collapsed.get(&g.key).copied().unwrap_or(false);
                let arrow = if collapsed { "▸" } else { "▾" };
                let label = if collapsed {
                    let count = envs.iter().filter(|e| e.group == g.key).count();
                    format!("{} {}  ({})", arrow, g.label, count)
                } else {
                    format!("{} {}", arrow, g.label)
                };
                ListItem::new(Line::from(Span::styled(
                    label,
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                )))
            }
            Item::EnvRow(i) => {
                let env = &envs[*i];
                let dom = env_dominant_status(env);
                let label = env_status_label(env);
                let expand = if env.agents.is_empty() {
                    "  "
                } else if env.expanded {
                    "▾ "
                } else {
                    "▸ "
                };
                let g = git.get(&env.name);
                let (delta, dstyle) = match g {
                    Some(gs) if gs.changes > 0 => (
                        format!("Δ{:<4}", gs.changes),
                        Style::default().fg(Color::Yellow),
                    ),
                    Some(gs) if gs.changes == 0 => {
                        ("·     ".into(), Style::default().fg(Color::DarkGray))
                    }
                    Some(_) => ("      ".into(), Style::default()),
                    None => ("…     ".into(), Style::default().fg(Color::DarkGray)),
                };
                let name = truncate(&env.name, name_w.max(6));
                let name_padded = format!("{:<width$}", name, width = name_w.max(6));
                let status_padded = format!("{:<17}", label);
                let is_marked = marked.contains(&env.name);
                let dot_span = if is_marked {
                    Span::styled(
                        "✓ ",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled("● ", status_style(dom))
                };
                ListItem::new(Line::from(vec![
                    Span::raw(expand),
                    dot_span,
                    Span::raw(name_padded),
                    Span::styled(status_padded, status_style(dom)),
                    Span::styled(delta, dstyle),
                ]))
            }
            Item::AgentRow(i, j) => {
                let env = &envs[*i];
                let a = &env.agents[*j];
                let is_last = *j + 1 == env.agents.len();
                let connector = if is_last { "└ " } else { "├ " };
                let tool = truncate(&a.tool, 7);
                let pid_s = if a.pid.is_empty() {
                    "-".into()
                } else {
                    format!("#{}", a.pid)
                };
                let pid_s = truncate(&pid_s, 9);
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  {connector}"),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(format!("{:<7}", tool), Style::default().fg(Color::Cyan)),
                    Span::styled(
                        format!(" {:<9}", pid_s),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled("● ", status_style(&a.status)),
                    Span::styled(a.status.clone(), status_style(&a.status)),
                ]))
            }
        })
        .collect()
}

pub(super) fn render_fleet(f: &mut Frame, area: Rect, app: &mut App, focused: bool) {
    let block = panel_block(" fleet ", focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let items = make_fleet_items(
        &app.view,
        &app.envs,
        &app.git,
        &app.groups,
        &app.group_collapsed,
        &app.marked,
        inner.width,
    );
    // Keep a 2-char highlight symbol in both states (make_fleet_items reserves
    // its width); only the focused panel shows the reverse-video cursor.
    let list = if focused {
        List::new(items)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("▶ ")
    } else {
        List::new(items).highlight_symbol("  ")
    };
    f.render_stateful_widget(list, inner, &mut app.list_state);
}

pub(super) fn render_detail(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default().borders(Borders::ALL).title(" detail ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);
    let label = Style::default().fg(Color::DarkGray);
    let section = Style::default()
        .fg(Color::Blue)
        .add_modifier(Modifier::BOLD);
    let w = inner.width as usize;

    let mut lines: Vec<Line<'static>> = Vec::new();

    let sel = app
        .list_state
        .selected()
        .and_then(|s| app.view.get(s))
        .cloned();

    let log_env_name = match &sel {
        Some(Item::EnvRow(i)) => app.envs[*i].name.clone(),
        Some(Item::AgentRow(i, _)) => app.envs[*i].name.clone(),
        _ => String::new(),
    };

    match sel {
        None | Some(Item::GroupHeader(_)) => {
            f.render_widget(Paragraph::new(Span::styled("no selection", dim)), inner);
            return;
        }
        Some(Item::EnvRow(i)) => {
            let env = &app.envs[i];
            let dom = env_dominant_status(env);
            lines.push(Line::from(Span::styled(
                env.name.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(vec![
                Span::styled("● ", status_style(dom)),
                Span::styled(env_status_label(env), status_style(dom)),
                Span::styled(format!("  {}", env.group), dim),
            ]));
            if env.group != "local" {
                let meta = remote_meta_label(&env.os, &env.shell);
                if !meta.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("remote ", label),
                        Span::styled(meta, Style::default().fg(Color::Cyan)),
                    ]));
                }
                if !env.host.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("host ", label),
                        Span::styled(truncate(&env.host, w.saturating_sub(5)), dim),
                    ]));
                }
            }
            if !env.path.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("path ", label),
                    Span::styled(truncate(&env.path, w.saturating_sub(5)), dim),
                ]));
            }

            if !env.agents.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled("agents", section)));
                for a in &env.agents {
                    let pid_s = if a.pid.is_empty() {
                        "-".into()
                    } else {
                        format!("#{}", a.pid)
                    };
                    lines.push(Line::from(vec![
                        Span::styled("  ● ", status_style(&a.status)),
                        Span::styled(format!("{:<7}", a.tool), Style::default().fg(Color::Cyan)),
                        Span::styled(format!(" {:<9}", pid_s), dim),
                        Span::styled(a.status.clone(), status_style(&a.status)),
                    ]));
                    if !a.kind.is_empty() {
                        lines.push(Line::from(vec![
                            Span::raw("         "),
                            Span::styled(a.kind.clone(), dim),
                        ]));
                    }
                }
            }

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("git", section)));
            if let Some(g) = app.git.get(&env.name) {
                let chstyle = if g.changes > 0 {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Green)
                };
                lines.push(Line::from(vec![
                    Span::styled("branch ", label),
                    Span::raw(if g.branch.is_empty() {
                        "-".into()
                    } else {
                        g.branch.clone()
                    }),
                    Span::raw("  "),
                    Span::styled(format!("Δ{}", g.changes), chstyle),
                ]));
                if !g.head.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("head   ", label),
                        Span::styled(truncate(&g.head, w.saturating_sub(7)), dim),
                    ]));
                }
            } else {
                lines.push(Line::from(Span::styled("(loading…)", dim)));
            }
        }
        Some(Item::AgentRow(i, j)) => {
            let env = &app.envs[i];
            let a = &env.agents[j];
            // Header: use session name when available (claude), else tool + pid.
            let header_str = if let Some(nm) = &a.agent_name {
                format!("{} · {}", a.tool, nm)
            } else {
                format!("{} #{}", a.tool, a.pid)
            };
            lines.push(Line::from(Span::styled(
                header_str,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(vec![
                Span::styled("● ", status_style(&a.status)),
                Span::styled(a.status.clone(), status_style(&a.status)),
                if !a.kind.is_empty() {
                    Span::styled(format!("  {}", a.kind), dim)
                } else {
                    Span::raw("")
                },
            ]));
            lines.push(Line::from(vec![
                Span::styled("env  ", label),
                Span::raw(env.name.clone()),
                Span::styled(format!("  pid #{}", a.pid), dim),
            ]));
            if !a.cwd.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("cwd  ", label),
                    Span::styled(truncate(&a.cwd, w.saturating_sub(5)), dim),
                ]));
            }
            if let Some(sid) = &a.session_id {
                if !sid.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("sid  ", label),
                        Span::styled(truncate(sid, w.saturating_sub(5)), dim),
                    ]));
                }
            }
            if let Some(model) = &a.model {
                if !model.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("mdl  ", label),
                        Span::styled(model.clone(), Style::default().fg(Color::Cyan)),
                    ]));
                }
            }
        }
    }

    // Log tail
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("recent log", section)));
    let header = lines.len();
    let avail = (inner.height as usize).saturating_sub(header).max(1);
    if app.log_target == log_env_name {
        if app.log_lines.is_empty() {
            lines.push(Line::from(Span::styled("(no logs)", dim)));
        } else {
            let start = app.log_lines.len().saturating_sub(avail);
            for l in &app.log_lines[start..] {
                lines.push(Line::from(Span::styled(truncate(l, w), dim)));
            }
        }
    } else {
        lines.push(Line::from(Span::styled("(loading…)", dim)));
    }

    f.render_widget(Paragraph::new(lines), inner);
}
