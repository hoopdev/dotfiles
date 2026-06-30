use std::collections::{HashMap, HashSet};
use std::time::Duration;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Sparkline, Wrap};

use crate::app::{truncate, wrap_line, App, Mode, SPIN};
use crate::model::{
    env_dominant_status, env_status_label, remote_meta_label, status_style, Env,
    GitState, GroupInfo, Item, Tab, ToolPurpose,
};

fn centered_rect(px: u16, py: u16, area: Rect) -> Rect {
    let v = Layout::vertical([
        Constraint::Percentage((100 - py) / 2),
        Constraint::Percentage(py),
        Constraint::Percentage((100 - py) / 2),
    ])
    .split(area);
    Layout::horizontal([
        Constraint::Percentage((100 - px) / 2),
        Constraint::Percentage(px),
        Constraint::Percentage((100 - px) / 2),
    ])
    .split(v[1])[1]
}

pub(crate) fn ui(f: &mut Frame, app: &mut App) {
    let v = Layout::vertical([
        Constraint::Length(1),       // summary
        Constraint::Length(1),       // tab bar
        Constraint::Percentage(95),  // main content
        Constraint::Length(1),       // bottom
    ])
    .split(f.area());
    render_summary(f, v[0], app);
    render_tab_bar(f, v[1], app);
    match app.active_tab {
        Tab::Agents => {
            if app.mode == Mode::LogView {
                render_log_view(f, v[2], app);
            } else {
                let h = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
                    .split(v[2]);
                render_fleet(f, h[0], app);
                render_detail(f, h[1], app);
            }
        }
        Tab::TaskBoard => render_task_board(f, v[2], app),
        Tab::Inbox => render_inbox(f, v[2], app),
    }
    render_bottom(f, v[3], app);
    match app.mode {
        Mode::Help => render_help(f),
        Mode::Dispatch => render_dispatch(f, app),
        Mode::ConfirmKill => render_confirm(f, app),
        Mode::ActionMenu => render_action_menu(f, app),
        Mode::ToolPick => render_tool_pick(f, app),
        Mode::BatchMenu => render_batch_menu(f, app),
        Mode::ResultView => render_result_view(f, app),
        Mode::ModelPicker => render_model_picker(f, app),
        Mode::TaskView => render_task_view(f, app),
        Mode::UsageView => render_usage_view(f, app),
        _ => {}
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
            let k = if a.status == "busy" { "running" } else { a.status.as_str() };
            *counts.entry(k).or_insert(0) += 1;
            total_agents += 1;
        }
    }
    let usage_spans: Vec<Span<'static>> = if let Some(u) = &app.claude_usage {
        let pct_style = |pct: u32| -> Style {
            if pct >= 85 { Style::default().fg(Color::Red) }
            else if pct >= 70 { Style::default().fg(Color::Yellow) }
            else { Style::default().fg(Color::Green) }
        };
        let mut s: Vec<Span<'static>> = vec![
            Span::styled("│ claude ", Style::default().fg(Color::DarkGray)),
        ];
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
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
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
            if pct >= 85 { Style::default().fg(Color::Red) }
            else if pct >= 70 { Style::default().fg(Color::Yellow) }
            else { Style::default().fg(Color::Green) }
        };
        spans.push(Span::styled("│ agy ", Style::default().fg(Color::DarkGray)));
        if let (Some(avail), Some(monthly)) = (u.available_credits, u.monthly_credits) {
            let used_pct = ((monthly.saturating_sub(avail)) * 100 / monthly.max(1)) as u32;
            spans.push(Span::styled(format!("{used_pct}%"), pct_style(used_pct)));
            spans.push(Span::styled(format!(" ({avail}cr) "), Style::default().fg(Color::DarkGray)));
        } else if !u.models.is_empty() {
            for (label, pct) in u.models.iter().take(2) {
                let short = label.split_whitespace().next().unwrap_or(label);
                spans.push(Span::styled(format!("{short}:"), Style::default().fg(Color::DarkGray)));
                spans.push(Span::styled(format!("{pct}% "), pct_style(*pct)));
            }
        }
    }

    // codex usage
    if let Some(u) = &app.codex_usage {
        let pct_style = |pct: f64| -> Style {
            if pct >= 85.0 { Style::default().fg(Color::Red) }
            else if pct >= 70.0 { Style::default().fg(Color::Yellow) }
            else { Style::default().fg(Color::Green) }
        };
        spans.push(Span::styled("│ codex ", Style::default().fg(Color::DarkGray)));
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
                    Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
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
                    Span::styled("✓ ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
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
                    Span::styled(format!(" {:<9}", pid_s), Style::default().fg(Color::DarkGray)),
                    Span::styled("● ", status_style(&a.status)),
                    Span::styled(a.status.clone(), status_style(&a.status)),
                ]))
            }
        })
        .collect()
}

fn render_fleet(f: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default().borders(Borders::ALL).title(" fleet ");
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
    let list = List::new(items)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("▶ ");
    f.render_stateful_widget(list, inner, &mut app.list_state);
}

fn render_detail(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default().borders(Borders::ALL).title(" detail ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);
    let label = Style::default().fg(Color::DarkGray);
    let section = Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD);
    let w = inner.width as usize;

    let mut lines: Vec<Line<'static>> = Vec::new();

    let sel = app.list_state.selected().and_then(|s| app.view.get(s)).cloned();

    let log_env_name = match &sel {
        Some(Item::EnvRow(i)) => app.envs[*i].name.clone(),
        Some(Item::AgentRow(i, _)) => app.envs[*i].name.clone(),
        _ => String::new(),
    };

    match sel {
        None | Some(Item::GroupHeader(_)) => {
            f.render_widget(
                Paragraph::new(Span::styled("no selection", dim)),
                inner,
            );
            return;
        }
        Some(Item::EnvRow(i)) => {
            let env = &app.envs[i];
            let dom = env_dominant_status(env);
            lines.push(Line::from(Span::styled(
                env.name.clone(),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
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
                    Span::raw(if g.branch.is_empty() { "-".into() } else { g.branch.clone() }),
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
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
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

fn render_bottom(f: &mut Frame, area: Rect, app: &App) {
    let line = if app.mode == Mode::Filter {
        Line::from(vec![
            Span::styled(" filter ", Style::default().bg(Color::Cyan).fg(Color::Black)),
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
        Mode::LogView => " j/k scroll · g top · G follow · PgDn/PgUp · esc/q back",
        Mode::ActionMenu => " j/k move · enter execute · esc cancel",
        Mode::ToolPick => " j/k select tool · enter confirm · esc back",
        Mode::BatchMenu => " j/k move · enter execute · esc cancel",
        Mode::TaskView => " j/k scroll · g top · G end · PgDn/PgUp · esc/q back",
        Mode::UsageView => " r refresh · esc/q back",
        _ => {
            let sel = app.list_state.selected().and_then(|s| app.view.get(s)).cloned();
            match sel {
                Some(Item::GroupHeader(_)) => {
                    " space expand/collapse · j/k move · / filter · w active · r refresh · ? help · q quit"
                }
                Some(Item::AgentRow(_, _)) => {
                    " enter menu · space collapse · l logs-f · a attach · x kill pid · n/N next · r refresh · ? help · q quit"
                }
                _ => " enter menu · m mark · b batch · T tasks · u usage · n/N next · l logs-f · a attach · d dispatch · x kill · D diff · / filter · w active · r refresh · ? help · q quit",
            }
        }
    };
    Line::from(Span::styled(hint, Style::default().fg(Color::DarkGray)))
}

fn kv(k: &str, v: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {k:<12}"), Style::default().fg(Color::Cyan)),
        Span::raw(v.to_string()),
    ])
}

fn render_help(f: &mut Frame) {
    let area = centered_rect(60, 80, f.area());
    f.render_widget(Clear, area);
    let lines = vec![
        Line::from(Span::styled(
            "dev top — keys",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("  navigation", Style::default().fg(Color::Blue))),
        kv("j/k ↑/↓", "move selection"),
        kv("g / G", "first / last"),
        kv("space", "expand/collapse group or env"),
        Line::from(""),
        Line::from(Span::styled("  actions (enter = menu)", Style::default().fg(Color::Blue))),
        kv("enter", "action menu (attach/code/dispatch/start/logs/diff/kill)"),
        kv("l", "follow logs (dev logs -f, outside TUI)"),
        kv("a", "attach (dev attach)"),
        kv("c", "open in VS Code (dev code)"),
        kv("d", "dispatch — select tool → enter task"),
        kv("x", "kill agents (dev kill / kill <pid>)"),
        kv("D", "view diff (dev diff | less)"),
        Line::from(""),
        Line::from(Span::styled("  batch (env rows)", Style::default().fg(Color::Blue))),
        kv("m", "toggle mark on selected env"),
        kv("M", "clear all marks"),
        kv("b", "batch menu (dispatch/diff/kill/clear)"),
        kv("n / N", "jump to next/prev waiting or error"),
        Line::from(""),
        Line::from(Span::styled("  view", Style::default().fg(Color::Blue))),
        kv("/", "filter by text"),
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
pub(crate) const ACTION_MENU_ITEMS: [(&str, &str); 9] = [
    ("attach", "dev attach"),
    ("open in VS Code", "dev code"),
    ("dispatch  (tool → task)", "dev dispatch --tool"),
    ("start tool  (interactive)", "dev <tool>"),
    ("review  (code review)", "dev review"),
    ("model picker  (next dispatch)", "set --model flag"),
    ("logs", "show logs in TUI"),
    ("diff", "dev diff | less"),
    ("kill", "dev kill / kill <pid>"),
];

fn render_action_menu(f: &mut Frame, app: &App) {
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
            Span::styled(
                format!("   {}", hint),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
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

fn render_tool_pick(f: &mut Frame, app: &App) {
    let purpose_label = match app.tool_purpose {
        ToolPurpose::Start => "start tool",
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
        lines.push(Line::from(Span::styled(
            format!("{}{}", prefix, tool),
            sty,
        )));
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

fn render_dispatch(f: &mut Frame, app: &App) {
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
        "(tool default)".to_string()
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
            Span::styled("tool:   ", Style::default().fg(Color::DarkGray)),
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
        Line::from(""),
        Line::from(Span::styled(
            "enter: run    esc: cancel",
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

fn render_log_view(f: &mut Frame, area: Rect, app: &App) {
    let title = if app.log_target.is_empty() {
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
    let rows: Vec<String> = app
        .log_lines
        .iter()
        .flat_map(|l| wrap_line(l, w))
        .collect();
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

fn render_confirm(f: &mut Frame, app: &App) {
    let area = centered_rect(52, 24, f.area());
    f.render_widget(Clear, area);
    let sel = app.list_state.selected().and_then(|s| app.view.get(s)).cloned();
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

fn render_batch_menu(f: &mut Frame, app: &App) {
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

fn render_result_view(f: &mut Frame, app: &App) {
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

    let rows: Vec<String> = app.result_lines.iter().flat_map(|l| wrap_line(l, w)).collect();
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
        format!("q/esc: close   j/k: scroll   {}/{} lines", top + 1, total.max(1))
    };
    lines.push(Line::from(Span::styled(hint, Style::default().fg(Color::DarkGray))));
    f.render_widget(Paragraph::new(lines), inner);
}

fn render_model_picker(f: &mut Frame, app: &App) {
    let tool = if app.dispatch_tool.is_empty() { "claude" } else { &app.dispatch_tool };
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
        " j/k: move  enter: select  c: clear  esc: back",
        Style::default().fg(Color::DarkGray),
    )));

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" model picker — {} ", tool)),
        ),
        area,
    );
}

// ── new view renders ──────────────────────────────────────────────────────────

fn render_task_view(f: &mut Frame, app: &App) {
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
            Span::styled(format!("{:<16}", target_str), Style::default().fg(Color::Cyan)),
            Span::styled(format!(" {:<8}", tool_str), dim),
            Span::styled(format!(" {:>8} ", elapsed), Style::default().fg(Color::Yellow)),
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

fn render_usage_view(f: &mut Frame, app: &App) {
    let area = centered_rect(80, 80, f.area());
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" usage dashboard ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);

    // Split inner area into sections for each service
    let sections = Layout::vertical([
        Constraint::Length(5),  // Claude
        Constraint::Length(5),  // Codex
        Constraint::Length(5),  // Agy
        Constraint::Min(1),    // Current values + hint
    ])
    .split(inner);

    // Claude sparkline
    {
        let data = app.usage_history.sparkline_data("claude_5h");
        let label_line = Line::from(vec![
            Span::styled(" claude 5h ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            if let Some(u) = &app.claude_usage {
                if let Some(p) = u.five_hour_pct {
                    Span::styled(format!("{}%", p), Style::default().fg(if p >= 85 { Color::Red } else if p >= 70 { Color::Yellow } else { Color::Green }))
                } else {
                    Span::styled("-", dim)
                }
            } else {
                Span::styled("-", dim)
            },
        ]);
        let spark_block = Block::default().title(label_line);
        let spark_inner = spark_block.inner(sections[0]);
        f.render_widget(spark_block, sections[0]);
        if !data.is_empty() {
            f.render_widget(
                Sparkline::default()
                    .data(&data)
                    .max(100)
                    .style(Style::default().fg(Color::Green)),
                spark_inner,
            );
        } else {
            f.render_widget(Paragraph::new(Span::styled("(no data)", dim)), spark_inner);
        }
    }

    // Codex sparkline
    {
        let data = app.usage_history.sparkline_data("codex_5h");
        let label_line = Line::from(vec![
            Span::styled(" codex 5h ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            if let Some(u) = &app.codex_usage {
                if let Some(p) = u.primary_used_pct {
                    Span::styled(format!("{:.0}%", p), Style::default().fg(if p >= 85.0 { Color::Red } else if p >= 70.0 { Color::Yellow } else { Color::Green }))
                } else {
                    Span::styled("-", dim)
                }
            } else {
                Span::styled("-", dim)
            },
        ]);
        let spark_block = Block::default().title(label_line);
        let spark_inner = spark_block.inner(sections[1]);
        f.render_widget(spark_block, sections[1]);
        if !data.is_empty() {
            f.render_widget(
                Sparkline::default()
                    .data(&data)
                    .max(100)
                    .style(Style::default().fg(Color::Blue)),
                spark_inner,
            );
        } else {
            f.render_widget(Paragraph::new(Span::styled("(no data)", dim)), spark_inner);
        }
    }

    // Agy sparkline
    {
        let data = app.usage_history.sparkline_data("agy_pct");
        let label_line = Line::from(vec![
            Span::styled(" agy ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            if let Some(u) = &app.agy_usage {
                if let (Some(avail), Some(monthly)) = (u.available_credits, u.monthly_credits) {
                    let pct = ((monthly.saturating_sub(avail)) * 100 / monthly.max(1)) as u32;
                    Span::styled(format!("{}%", pct), Style::default().fg(if pct >= 85 { Color::Red } else if pct >= 70 { Color::Yellow } else { Color::Green }))
                } else {
                    Span::styled("-", dim)
                }
            } else {
                Span::styled("-", dim)
            },
        ]);
        let spark_block = Block::default().title(label_line);
        let spark_inner = spark_block.inner(sections[2]);
        f.render_widget(spark_block, sections[2]);
        if !data.is_empty() {
            f.render_widget(
                Sparkline::default()
                    .data(&data)
                    .max(100)
                    .style(Style::default().fg(Color::Magenta)),
                spark_inner,
            );
        } else {
            f.render_widget(Paragraph::new(Span::styled("(no data)", dim)), spark_inner);
        }
    }

    // Current values + hint
    {
        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::from(""));

        // Claude 7d
        if let Some(u) = &app.claude_usage {
            let mut spans: Vec<Span<'static>> = vec![
                Span::styled("  claude ", Style::default().fg(Color::Cyan)),
            ];
            if let Some(p) = u.five_hour_pct {
                spans.push(Span::styled(format!("5h:{p}%"), dim));
                spans.push(Span::raw("  "));
            }
            if let Some(p) = u.seven_day_pct {
                spans.push(Span::styled(format!("7d:{p}%"), dim));
            }
            lines.push(Line::from(spans));
        }
        // Codex
        if let Some(u) = &app.codex_usage {
            let mut spans: Vec<Span<'static>> = vec![
                Span::styled("  codex  ", Style::default().fg(Color::Cyan)),
            ];
            if let Some(p) = u.primary_used_pct {
                spans.push(Span::styled(format!("5h:{p:.0}%"), dim));
                spans.push(Span::raw("  "));
            }
            if let Some(p) = u.secondary_used_pct {
                spans.push(Span::styled(format!("7d:{p:.0}%"), dim));
            }
            lines.push(Line::from(spans));
        }
        // Agy
        if let Some(u) = &app.agy_usage {
            let mut spans: Vec<Span<'static>> = vec![
                Span::styled("  agy    ", Style::default().fg(Color::Cyan)),
            ];
            if let (Some(avail), Some(monthly)) = (u.available_credits, u.monthly_credits) {
                let pct = ((monthly.saturating_sub(avail)) * 100 / monthly.max(1)) as u32;
                spans.push(Span::styled(format!("{pct}%"), dim));
                spans.push(Span::styled(format!("  ({avail}/{monthly} cr)"), dim));
            }
            lines.push(Line::from(spans));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {} samples · r: refresh · esc/q: back", app.usage_history.samples.len()),
            dim,
        )));

        f.render_widget(Paragraph::new(lines), sections[3]);
    }
}

// ── Phase 2 renders ──────────────────────────────────────────────────────────

fn render_tab_bar(f: &mut Frame, area: Rect, app: &App) {
    let tabs = [Tab::Agents, Tab::TaskBoard, Tab::Inbox];
    let blocking = app.dev_questions.iter().filter(|q| q.severity == "blocking").count();
    let mut spans: Vec<Span<'static>> = Vec::new();
    for tab in tabs {
        let active = app.active_tab == tab;
        let label: String = match tab {
            Tab::Inbox if blocking > 0 => format!(" {} ({}) ", tab.label(), blocking),
            _ => format!(" {} ", tab.label()),
        };
        let style = if active {
            Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(label, style));
        spans.push(Span::raw("  "));
    }
    spans.push(Span::styled(" tab:switch ", Style::default().fg(Color::DarkGray)));
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_task_board(f: &mut Frame, area: Rect, app: &App) {
    // Split: lane selector header (1 line) + task list
    let v = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area);

    // Lane selector header
    let col = app.board_col;
    let mut lane_spans: Vec<Span<'static>> = Vec::new();
    for (i, (name, phase)) in App::BOARD_LANES.iter().enumerate() {
        let count = app.dev_tasks.iter().filter(|t| t.phase.as_str() == *phase).count();
        let label = format!(" {name}({count}) ");
        let style = if i == col {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
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
    let sel = if lane_tasks.is_empty() { 0 } else { app.board_sel.min(lane_tasks.len().saturating_sub(1)) };

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
            let title = truncate(&task.title, 40);
            let tool = task.assigned_tool.as_deref().unwrap_or("-").to_string();
            let q_count = app.dev_questions.iter().filter(|q| q.task_id == task.id).count();
            let project_id = task.project_id.clone();
            let line1 = if selected {
                Line::from(vec![
                    Span::styled(format!("{id_short}  "), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::styled(title, Style::default().add_modifier(Modifier::BOLD)),
                ])
            } else {
                Line::from(vec![
                    Span::styled(format!("{id_short}  "), Style::default().fg(Color::DarkGray)),
                    Span::raw(title),
                ])
            };
            let line2 = Line::from(vec![
                Span::styled(
                    format!("  {}  {}  q:{}", project_id, tool, q_count),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
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

    // Key hints at bottom right
    let hints = "h/l:lane  j/k:select  A:approve  p:plan  ?:help";
    let hint_area = Rect {
        x: area.x,
        y: area.bottom().saturating_sub(1),
        width: area.width,
        height: 1,
    };
    f.render_widget(
        Paragraph::new(hints).style(Style::default().fg(Color::DarkGray)).alignment(Alignment::Right),
        hint_area,
    );
}

fn render_inbox(f: &mut Frame, area: Rect, app: &App) {
    let v = Layout::vertical([Constraint::Min(0), Constraint::Length(6)]).split(area);

    // Question list (top)
    let questions = &app.dev_questions;
    let sel = if questions.is_empty() { 0 } else { app.inbox_sel.min(questions.len().saturating_sub(1)) };
    let height = v[0].height as usize;
    let offset = sel.saturating_sub(height / 2);

    if questions.is_empty() {
        let msg = Paragraph::new("No open blocking questions.")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title("Inbox — Needs Spec"));
        f.render_widget(msg, v[0]);
    } else {
        let mut items: Vec<ListItem<'static>> = Vec::new();
        for (i, q) in questions.iter().enumerate().skip(offset).take(height) {
            let selected = i == sel;
            let sev_style = match q.severity.as_str() {
                "blocking" => Style::default().fg(Color::Red),
                "nonblocking" => Style::default().fg(Color::Yellow),
                _ => Style::default().fg(Color::DarkGray),
            };
            let severity = q.severity.clone();
            let qid = q.id.clone();
            let task_id = q.task_id.clone();
            let project_id = q.project_id.clone();
            let question_text = truncate(&q.question, 70);
            let line1 = Line::from(vec![
                Span::styled(format!("[{}] ", severity), sev_style),
                Span::styled(qid, Style::default().fg(Color::Cyan)),
                Span::raw(format!("  {}", task_id)),
                Span::styled(format!("  {}", project_id), Style::default().fg(Color::DarkGray)),
            ]);
            let line2 = Line::from(vec![
                Span::raw(format!("  {}", question_text)),
            ]);
            let style = if selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            items.push(ListItem::new(vec![line1, line2]).style(style));
        }
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(format!("Inbox — {} blocking question(s)", questions.len())));
        f.render_widget(list, v[0]);
    }

    // Selected question detail + answer input (bottom)
    if let Some(q) = questions.get(sel) {
        let mut detail_lines: Vec<Line<'static>> = vec![
            Line::from(Span::styled(format!("Q: {}", q.question.clone()), Style::default().add_modifier(Modifier::BOLD))),
        ];
        if let Some(rec) = &q.agent_recommendation {
            detail_lines.push(Line::from(Span::styled(
                format!("recommendation: {rec}"),
                Style::default().fg(Color::Cyan),
            )));
        }
        if !q.context.is_empty() {
            detail_lines.push(Line::from(Span::styled(
                format!("context: {}", truncate(&q.context, 60)),
                Style::default().fg(Color::DarkGray),
            )));
        }
        for opt in &q.options {
            detail_lines.push(Line::from(Span::raw(format!("  [{}] {} — {}", opt.id, opt.label, opt.impact))));
        }
        if app.inbox_answering {
            detail_lines.push(Line::from(vec![
                Span::styled("answer> ", Style::default().fg(Color::Yellow)),
                Span::raw(app.inbox_answer.clone()),
                Span::styled("█", Style::default().fg(Color::Yellow)),
            ]));
        } else {
            detail_lines.push(Line::from(Span::styled(
                "Enter:answer  Esc:cancel",
                Style::default().fg(Color::DarkGray),
            )));
        }
        let detail = Paragraph::new(detail_lines)
            .block(Block::default().borders(Borders::ALL).title("Question Detail"))
            .wrap(Wrap { trim: false });
        f.render_widget(detail, v[1]);
    } else {
        let empty = Paragraph::new("No question selected.")
            .block(Block::default().borders(Borders::ALL).title("Question Detail"));
        f.render_widget(empty, v[1]);
    }
}

// ── End of file ────────────────────────────────────────────────────────────────

