//! ASCII task board renderer for the Zellij plugin.

use crate::{DevPlugin, LANES};

pub fn draw(plugin: &DevPlugin, rows: usize, cols: usize) {
    if plugin.loading {
        println!("dev task board — loading...");
        return;
    }

    let blocking = plugin.questions.iter()
        .filter(|q| q.severity == "blocking")
        .count();

    // ── header bar ────────────────────────────────────────────────────────────
    let total = plugin.tasks.len();
    let implementing = plugin.tasks.iter().filter(|t| t.phase == "implementing").count();
    let review      = plugin.tasks.iter().filter(|t| t.phase == "review").count();
    let mergeable   = plugin.tasks.iter().filter(|t| t.phase == "mergeable").count();
    let header = format!(
        " dev tasks  total:{total}  running:{implementing}  review:{review}  mergeable:{mergeable}{}",
        if blocking > 0 { format!("  ! {blocking} blocking") } else { String::new() }
    );
    println!("{}", &header[..header.len().min(cols)]);
    println!("{}", "-".repeat(cols));

    // ── lane widths ───────────────────────────────────────────────────────────
    let n_lanes = LANES.len();
    let lane_w = (cols.saturating_sub(n_lanes + 1)) / n_lanes;

    // lane headers
    let mut header_row = String::new();
    for (i, (_, label)) in LANES.iter().enumerate() {
        let selected = i == plugin.selected_col;
        let cell = format_lane_header(label, lane_w, selected);
        if i > 0 { header_row.push('|'); }
        header_row.push_str(&cell);
    }
    println!("{}", header_row);
    println!("{}", LANES.iter().enumerate().map(|(i, _)| {
        let sep = if i > 0 { "+" } else { "" };
        format!("{}{}", sep, "-".repeat(lane_w))
    }).collect::<String>());

    // ── task rows ─────────────────────────────────────────────────────────────
    let task_rows = rows.saturating_sub(5); // header + separator + footer + padding

    // Build per-lane task lists
    let lane_tasks: Vec<Vec<&dev_core::DevTask>> = LANES.iter()
        .map(|(phase, _)| plugin.tasks.iter().filter(|t| t.phase.as_str() == *phase).collect())
        .collect();

    for row in 0..task_rows {
        let mut line = String::new();
        for (col, tasks) in lane_tasks.iter().enumerate() {
            if col > 0 { line.push('|'); }
            if let Some(task) = tasks.get(row) {
                let selected = col == plugin.selected_col && row == plugin.selected_row;
                let cell = format_task_cell(task, lane_w, selected);
                line.push_str(&cell);
            } else {
                line.push_str(&" ".repeat(lane_w));
            }
        }
        println!("{}", line);
    }

    // ── footer / key hints ────────────────────────────────────────────────────
    println!("{}", "-".repeat(cols));
    let footer = " h/l:lane  j/k:task  Enter:attach  a:approve  d:dispatch  r:reload  q:focus";
    println!("{}", &footer[..footer.len().min(cols)]);
}

fn format_lane_header(label: &str, width: usize, selected: bool) -> String {
    if width == 0 { return String::new(); }
    let trimmed = if label.len() > width { &label[..width] } else { label };
    let padded = format!("{:^width$}", trimmed, width = width);
    if selected {
        format!("\x1b[7m{padded}\x1b[0m")
    } else {
        format!("\x1b[1m{padded}\x1b[0m")
    }
}

fn format_task_cell(task: &dev_core::DevTask, width: usize, selected: bool) -> String {
    if width < 4 { return " ".repeat(width); }
    let id = &task.id;
    let title = &task.title;
    // Compose "ID title" and truncate to lane width
    let combined = format!("{} {}", id, title);
    let line = if combined.len() > width {
        combined[..width].to_string()
    } else {
        format!("{:<width$}", combined, width = width)
    };
    if selected {
        format!("\x1b[7m{line}\x1b[0m")
    } else {
        line
    }
}
