//! ASCII task board renderer for the Zellij plugin.

use crate::color::{self, Fg};
use crate::{DevPlugin, View, LANES};
use dev_core::DevTask;

pub fn draw(plugin: &DevPlugin, rows: usize, cols: usize) {
    if plugin.loading {
        println!("dev task board — loading...");
        return;
    }
    match plugin.view {
        View::Board => draw_board(plugin, rows, cols),
        View::Inbox => draw_inbox(plugin, rows, cols),
    }
}

/// Char-safe truncation. Byte slicing (`&s[..n]`) panics when `n` lands inside a
/// multibyte char — a real risk for Japanese task titles.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect()
    }
}

// Fixed board chrome, in printed rows:
//   header block  = header + separator
//   lane block    = lane labels + separator
//   footer block  = separator + hints
const HEADER_ROWS: usize = 2;
const LANE_HEADER_ROWS: usize = 2;
const FOOTER_ROWS: usize = 2;
/// Detail panel: a top separator plus three content lines.
const DETAIL_ROWS: usize = 4;

fn draw_board(plugin: &DevPlugin, rows: usize, cols: usize) {
    let blocking = plugin
        .questions
        .iter()
        .filter(|q| q.severity == "blocking")
        .count();

    // ── header bar ────────────────────────────────────────────────────────────
    let total = plugin.tasks.len();
    let implementing = plugin
        .tasks
        .iter()
        .filter(|t| t.phase == "implementing")
        .count();
    let review = plugin.tasks.iter().filter(|t| t.phase == "review").count();
    let mergeable = plugin
        .tasks
        .iter()
        .filter(|t| t.phase == "mergeable")
        .count();
    let header = format!(
        " dev tasks  total:{total}  running:{implementing}  review:{review}  mergeable:{mergeable}{}{}",
        if blocking > 0 { format!("  ! {blocking} blocking") } else { String::new() },
        match &plugin.error {
            Some(e) => format!("  [err: {e}]"),
            None => String::new(),
        },
    );
    // Full-width reverse-video title bar (pad first so the highlight isn't ragged).
    let bar = format!("{:<width$}", truncate(&header, cols), width = cols);
    println!("{}", color::paint(&bar, Fg::Default, true));
    println!("{}", "-".repeat(cols));

    // ── lane widths ───────────────────────────────────────────────────────────
    let n_lanes = LANES.len();
    let lane_w = (cols.saturating_sub(n_lanes + 1)) / n_lanes;

    // Build per-lane task lists (used for headers, cells and the detail panel).
    let lane_tasks: Vec<Vec<&DevTask>> = LANES
        .iter()
        .map(|(phase, _)| {
            plugin
                .tasks
                .iter()
                .filter(|t| t.phase.as_str() == *phase)
                .collect()
        })
        .collect();

    // lane headers — colored by phase, count appended, selection reversed.
    let mut header_row = String::new();
    for (i, (phase, label)) in LANES.iter().enumerate() {
        let count = lane_tasks.get(i).map(Vec::len).unwrap_or(0);
        let selected = i == plugin.selected_col;
        let cell = format_lane_header(label, count, lane_w, color::phase_fg(phase), selected);
        if i > 0 {
            header_row.push('|');
        }
        header_row.push_str(&cell);
    }
    println!("{header_row}");
    println!(
        "{}",
        LANES
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let sep = if i > 0 { "+" } else { "" };
                format!("{}{}", sep, "-".repeat(lane_w))
            })
            .collect::<String>()
    );

    // ── row budget ────────────────────────────────────────────────────────────
    // Reserve the detail panel only when a task is selected AND there is still
    // room for at least one task row above it; otherwise skip it entirely.
    let selected_task = lane_tasks
        .get(plugin.selected_col)
        .and_then(|l| l.get(plugin.selected_row))
        .copied();
    let chrome = HEADER_ROWS + LANE_HEADER_ROWS + FOOTER_ROWS;
    let detail_rows = match selected_task {
        Some(_) if rows > chrome + DETAIL_ROWS => DETAIL_ROWS,
        _ => 0,
    };
    let task_rows = rows.saturating_sub(chrome + detail_rows).max(1);

    // ── task rows (with per-lane scrolling for the selected lane) ─────────────
    for row in 0..task_rows {
        let mut line = String::new();
        for (col, tasks) in lane_tasks.iter().enumerate() {
            if col > 0 {
                line.push('|');
            }
            let offset = lane_offset(col, plugin.selected_col, plugin.selected_row, task_rows);
            let idx = offset + row;
            if let Some(task) = tasks.get(idx) {
                let selected = col == plugin.selected_col && idx == plugin.selected_row;
                let cell = format_task_cell(task, lane_w, selected);
                line.push_str(&cell);
            } else {
                line.push_str(&" ".repeat(lane_w));
            }
        }
        println!("{line}");
    }

    // ── task detail panel ─────────────────────────────────────────────────────
    if detail_rows > 0 {
        if let Some(task) = selected_task {
            draw_task_detail(task, cols);
        }
    }

    // ── footer / key hints ────────────────────────────────────────────────────
    println!("{}", "-".repeat(cols));
    let footer = " h/l:lane j/k:task Enter:attach a:approve d:dispatch Tab:inbox r:reload";
    println!("{}", truncate(footer, cols));
}

/// Scroll offset for a lane's visible window. Only the *selected* lane scrolls —
/// it keeps `selected_row` on screen by pinning it to the bottom edge once it
/// would fall past `visible`. Other lanes always show from the top. Panic-safe:
/// `saturating_sub` guards empty/short lanes and `visible == 0`.
fn lane_offset(col: usize, selected_col: usize, selected_row: usize, visible: usize) -> usize {
    if col == selected_col {
        selected_row.saturating_sub(visible.saturating_sub(1))
    } else {
        0
    }
}

/// Bottom detail panel for the selected task — only existing [`DevTask`] fields.
fn draw_task_detail(task: &DevTask, cols: usize) {
    println!("{}", "-".repeat(cols));
    let fg = color::phase_fg(&task.phase);
    let line1 = format!(" {}  [{}]  prio:{}", task.id, task.phase, task.priority);
    // Color the id/phase line by phase; truncate the plain text first so the SGR
    // escapes never eat into the visible width.
    println!("{}", color::paint(&truncate(&line1, cols), fg, false));
    println!("{}", truncate(&format!(" {}", task.title), cols));
    let tool = task.assigned_tool.as_deref().unwrap_or("—");
    let line3 = format!(
        " tool:{}  review:{}  test:{}  diff:{} files",
        tool, task.review_status, task.test_status, task.diff_files_count
    );
    println!("{}", truncate(&line3, cols));
}

/// Inbox view: open questions with inline answering (press 1-9 to pick an
/// option). Fixes the old board, which only showed a blocking *count*.
fn draw_inbox(plugin: &DevPlugin, rows: usize, cols: usize) {
    let n = plugin.questions.len();
    let blocking = plugin
        .questions
        .iter()
        .filter(|q| q.severity == "blocking")
        .count();
    let header = format!(
        " dev inbox  open:{n}  blocking:{blocking}{}",
        match &plugin.error {
            Some(e) => format!("  [err: {e}]"),
            None => String::new(),
        }
    );
    let bar = format!("{:<width$}", truncate(&header, cols), width = cols);
    println!("{}", color::paint(&bar, Fg::Default, true));
    println!("{}", "-".repeat(cols));

    if n == 0 {
        println!("  (no open questions)");
        println!("{}", "-".repeat(cols));
        println!("{}", truncate(" Tab:board  r:reload", cols));
        return;
    }

    // Reserve the bottom third for the selected-question detail.
    let list_rows = rows.saturating_sub(2).saturating_sub(8).max(1);
    for (i, q) in plugin.questions.iter().take(list_rows).enumerate() {
        let sel = i == plugin.inbox_row;
        let sev = if q.severity == "blocking" { "!" } else { " " };
        // Truncate the plain text to width first, then color by severity and add
        // the reverse-video selection highlight — escapes stay zero-width.
        let line = truncate(&format!(" {sev} [{}] {}", q.category, q.question), cols);
        println!(
            "{}",
            color::paint(&line, color::severity_fg(&q.severity), sel)
        );
    }

    println!("{}", "-".repeat(cols));
    if let Some(q) = plugin.questions.get(plugin.inbox_row) {
        println!("{}", truncate(&format!(" Q: {}", q.question), cols));
        if !q.context.is_empty() {
            println!("{}", truncate(&format!(" ctx: {}", q.context), cols));
        }
        if let Some(rec) = &q.agent_recommendation {
            println!("{}", truncate(&format!(" rec: {rec}"), cols));
        }
        for (j, opt) in q.options.iter().take(9).enumerate() {
            let label = if opt.impact.is_empty() {
                opt.label.clone()
            } else {
                format!("{} — {}", opt.label, opt.impact)
            };
            println!("{}", truncate(&format!("  [{}] {}", j + 1, label), cols));
        }
    }
    println!("{}", "-".repeat(cols));
    println!(
        "{}",
        truncate(" Tab:board  j/k:move  1-9:answer  r:reload", cols)
    );
}

fn format_lane_header(label: &str, count: usize, width: usize, fg: Fg, selected: bool) -> String {
    if width == 0 {
        return String::new();
    }
    // Append the task count as a scroll/size hint, then char-safely fit to width.
    let text = format!("{label} ({count})");
    let trimmed = truncate(&text, width);
    let padded = format!("{trimmed:^width$}");
    // Selected lanes reverse-video (so the phase color becomes the highlight
    // background); unselected lanes render the phase color in bold-ish plain fg.
    color::paint(&padded, fg, selected)
}

fn format_task_cell(task: &DevTask, width: usize, selected: bool) -> String {
    if width < 4 {
        return " ".repeat(width);
    }
    let id = &task.id;
    let title = &task.title;
    // Compose "ID title" and truncate to lane width (char-safe: titles are often
    // Japanese, and byte slicing would panic mid-character).
    let combined = format!("{id} {title}");
    let line = if combined.chars().count() > width {
        combined.chars().take(width).collect()
    } else {
        format!("{combined:<width$}")
    };
    // Color the (already width-fitted) cell by phase and add reverse-video when
    // selected — the escape bytes are zero-width so column alignment is preserved.
    color::paint(&line, color::phase_fg(&task.phase), selected)
}
