use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders};

/// A bordered panel whose border turns cyan when focused, dim otherwise —
/// the "one pane is live" cue borrowed from Claude Squad.
pub(super) fn panel_block(title: &str, focused: bool) -> Block<'static> {
    let border = if focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .title(title.to_string())
}

pub(super) fn diff_line_style(l: &str) -> Style {
    if l.starts_with("+++") || l.starts_with("---") {
        Style::default().fg(Color::DarkGray)
    } else if l.starts_with('+') {
        Style::default().fg(Color::Green)
    } else if l.starts_with('-') {
        Style::default().fg(Color::Red)
    } else if l.starts_with("@@") {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

pub(super) fn phase_style(phase: &str) -> Style {
    match phase {
        "needs_spec" => Style::default().fg(Color::Red),
        "planned" => Style::default().fg(Color::Blue),
        "approved" => Style::default().fg(Color::Green),
        "implementing" => Style::default().fg(Color::Yellow),
        "review" => Style::default().fg(Color::Magenta),
        "needs_fix" => Style::default().fg(Color::Red),
        "mergeable" => Style::default().fg(Color::Green),
        "merged" => Style::default().fg(Color::DarkGray),
        _ => Style::default().fg(Color::DarkGray),
    }
}
