//! Base16-safe ANSI color helpers for the dev board.
//!
//! Uses only STANDARD SGR foreground codes (30-37 normal, 90-97 bright) plus
//! reset — never 24-bit RGB — so the rendered colors resolve through the user's
//! terminal palette (Stylix / base16) instead of clashing with it.
//!
//! CRITICAL width invariant: the escape sequences these helpers emit are
//! zero-width. Callers MUST truncate/pad text to its visible width *first*, then
//! wrap the result with [`paint`]. Never truncate a string that already carries
//! SGR codes — the byte count would include the invisible escapes and alignment
//! would drift.

/// Standard ANSI SGR foreground colors. Rendered through the terminal theme, so
/// the exact hue is whatever the user's base16 palette maps these slots to.
#[derive(Clone, Copy, PartialEq)]
pub enum Fg {
    /// Terminal default foreground (no color param emitted).
    Default,
    BrightBlack,
    Red,
    Green,
    Yellow,
    Blue,
    Cyan,
}

impl Fg {
    /// SGR parameter for this color, or `None` for the terminal default.
    fn param(self) -> Option<&'static str> {
        match self {
            Fg::Default => None,
            Fg::BrightBlack => Some("90"),
            Fg::Red => Some("31"),
            Fg::Green => Some("32"),
            Fg::Yellow => Some("33"),
            Fg::Blue => Some("34"),
            Fg::Cyan => Some("36"),
        }
    }
}

/// Lane / task-cell color keyed on the task phase.
pub fn phase_fg(phase: &str) -> Fg {
    match phase {
        "needs_spec" => Fg::BrightBlack,
        "planned" => Fg::Blue,
        "implementing" => Fg::Cyan,
        "review" => Fg::Yellow,
        "needs_fix" => Fg::Red,
        "mergeable" => Fg::Green,
        _ => Fg::Default,
    }
}

/// Inbox question color keyed on severity.
pub fn severity_fg(severity: &str) -> Fg {
    match severity {
        "blocking" => Fg::Red,
        "warning" => Fg::Yellow,
        _ => Fg::Default,
    }
}

/// Wrap already-width-fitted `text` in one SGR sequence combining an optional
/// foreground color with an optional reverse-video (selection) highlight, always
/// resetting afterward. When neither applies the text is returned untouched, so
/// no stray escapes are emitted.
pub fn paint(text: &str, fg: Fg, reverse: bool) -> String {
    let mut params: Vec<&str> = Vec::new();
    if reverse {
        params.push("7");
    }
    if let Some(c) = fg.param() {
        params.push(c);
    }
    if params.is_empty() {
        text.to_string()
    } else {
        format!("\x1b[{}m{}\x1b[0m", params.join(";"), text)
    }
}
