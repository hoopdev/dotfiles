use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use crate::app::{truncate, App};

use super::theme::panel_block;

/// Cockpit Inbox panel — compact 1-line-per-question list (left column).
pub(super) fn render_inbox_panel(f: &mut Frame, area: Rect, app: &App, focused: bool) {
    let questions = &app.dev_questions;
    let title = if questions.is_empty() {
        " inbox ".to_string()
    } else {
        format!(" inbox ⚠{} ", questions.len())
    };
    let block = panel_block(&title, focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if questions.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                "no blocking questions",
                Style::default().fg(Color::DarkGray),
            )),
            inner,
        );
        return;
    }

    let sel = app.inbox_sel.min(questions.len().saturating_sub(1));
    let height = inner.height as usize;
    let offset = sel.saturating_sub(height.saturating_sub(1));
    let w = inner.width as usize;
    let mut items: Vec<ListItem<'static>> = Vec::new();
    for (i, q) in questions.iter().enumerate().skip(offset).take(height) {
        let selected = i == sel && focused;
        let sev_style = match q.severity.as_str() {
            "blocking" => Style::default().fg(Color::Red),
            "nonblocking" => Style::default().fg(Color::Yellow),
            _ => Style::default().fg(Color::DarkGray),
        };
        let marker = if selected { "▶ " } else { "  " };
        let text_w = w.saturating_sub(marker.len() + 2 + q.id.len() + 1);
        let text = truncate(&q.question, text_w.max(6));
        let line = Line::from(vec![
            Span::raw(marker),
            Span::styled("● ", sev_style),
            Span::styled(format!("{} ", q.id), Style::default().fg(Color::Cyan)),
            Span::raw(text),
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

/// Inbox inspector — selected question detail + inline answer input (right pane).
pub(super) fn render_inbox_detail(f: &mut Frame, area: Rect, app: &App) {
    let questions = &app.dev_questions;
    let sel = if questions.is_empty() {
        0
    } else {
        app.inbox_sel.min(questions.len().saturating_sub(1))
    };
    if let Some(q) = questions.get(sel) {
        let mut detail_lines: Vec<Line<'static>> = vec![Line::from(Span::styled(
            format!("Q: {}", q.question.clone()),
            Style::default().add_modifier(Modifier::BOLD),
        ))];
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
            detail_lines.push(Line::from(Span::raw(format!(
                "  [{}] {} — {}",
                opt.id, opt.label, opt.impact
            ))));
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
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Question Detail "),
            )
            .wrap(Wrap { trim: false });
        f.render_widget(detail, area);
    } else {
        let empty = Paragraph::new("No question selected.").block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Question Detail "),
        );
        f.render_widget(empty, area);
    }
}
