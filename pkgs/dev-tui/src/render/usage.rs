use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Sparkline};

use crate::app::App;

use super::layout::centered_rect;

pub(super) fn render_usage_view(f: &mut Frame, app: &App) {
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
        Constraint::Length(5), // Claude
        Constraint::Length(5), // Codex
        Constraint::Length(5), // Agy
        Constraint::Min(1),    // Current values + hint
    ])
    .split(inner);

    // Claude sparkline
    {
        let data = app.usage_history.sparkline_data("claude_5h");
        let label_line = Line::from(vec![
            Span::styled(
                " claude 5h ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            if let Some(u) = &app.claude_usage {
                if let Some(p) = u.five_hour_pct {
                    Span::styled(
                        format!("{}%", p),
                        Style::default().fg(if p >= 85 {
                            Color::Red
                        } else if p >= 70 {
                            Color::Yellow
                        } else {
                            Color::Green
                        }),
                    )
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
            Span::styled(
                " codex 5h ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            if let Some(u) = &app.codex_usage {
                if let Some(p) = u.primary_used_pct {
                    Span::styled(
                        format!("{:.0}%", p),
                        Style::default().fg(if p >= 85.0 {
                            Color::Red
                        } else if p >= 70.0 {
                            Color::Yellow
                        } else {
                            Color::Green
                        }),
                    )
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
            Span::styled(
                " agy ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            if let Some(u) = &app.agy_usage {
                if let (Some(avail), Some(monthly)) = (u.available_credits, u.monthly_credits) {
                    let pct = ((monthly.saturating_sub(avail)) * 100 / monthly.max(1)) as u32;
                    Span::styled(
                        format!("{}%", pct),
                        Style::default().fg(if pct >= 85 {
                            Color::Red
                        } else if pct >= 70 {
                            Color::Yellow
                        } else {
                            Color::Green
                        }),
                    )
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
            let mut spans: Vec<Span<'static>> =
                vec![Span::styled("  claude ", Style::default().fg(Color::Cyan))];
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
            let mut spans: Vec<Span<'static>> =
                vec![Span::styled("  codex  ", Style::default().fg(Color::Cyan))];
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
            let mut spans: Vec<Span<'static>> =
                vec![Span::styled("  agy    ", Style::default().fg(Color::Cyan))];
            if let (Some(avail), Some(monthly)) = (u.available_credits, u.monthly_credits) {
                let pct = ((monthly.saturating_sub(avail)) * 100 / monthly.max(1)) as u32;
                spans.push(Span::styled(format!("{pct}%"), dim));
                spans.push(Span::styled(format!("  ({avail}/{monthly} cr)"), dim));
            }
            lines.push(Line::from(spans));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!(
                "  {} samples · r: refresh · esc/q: back",
                app.usage_history.samples.len()
            ),
            dim,
        )));

        f.render_widget(Paragraph::new(lines), sections[3]);
    }
}
