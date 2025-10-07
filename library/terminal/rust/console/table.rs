//! Simple, readable table renderer.

use super::{dim, visible_width};

/// Alignment of a table column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
}

/// Render a table with headers, optional secondary headers, and data rows.
/// - `headers`: column names
/// - `rows`: matrix of cell strings (not necessarily rectangular; missing cells are empty)
/// - `right_align_numeric`: detect numeric-like columns and right-align them
/// - `padding`: spaces between columns
/// - `secondary_headers`: optional dimmed second header row
pub fn render_table(
    headers: &[String],
    rows: &[Vec<String>],
    right_align_numeric: bool,
    padding: usize,
    secondary_headers: Option<&[String]>,
) -> String {
    let col_count = headers.len();
    let mut col_widths = vec![0usize; col_count];

    // calculate column widths from headers
    for (i, header) in headers.iter().enumerate() {
        col_widths[i] = col_widths[i].max(visible_width(header));
    }

    // include secondary headers in width calculation
    if let Some(sec) = secondary_headers {
        for (i, header) in sec.iter().enumerate().take(col_count) {
            col_widths[i] = col_widths[i].max(visible_width(header));
        }
    }

    // calculate column widths from data rows
    for row in rows {
        for (i, cell) in row.iter().enumerate().take(col_count) {
            col_widths[i] = col_widths[i].max(max_visible_line_len(cell));
        }
    }

    // determine which columns should be right-aligned
    let mut right_align_cols = vec![false; col_count];
    if right_align_numeric {
        (0..col_count).for_each(|i| {
            let mut all_numeric = true;
            for row in rows {
                if let Some(cell) = row.get(i)
                    && !cell.is_empty()
                    && !is_numeric_like(cell)
                {
                    all_numeric = false;
                    break;
                }
            }
            right_align_cols[i] = all_numeric;
        });
    }

    let mut out = String::new();

    // render primary headers
    push_row(&mut out, headers, &col_widths, &right_align_cols, padding);

    // render secondary headers if provided
    if let Some(sec) = secondary_headers {
        let dimmed: Vec<String> = sec.iter().map(|value| dim(value)).collect();
        push_row(&mut out, &dimmed, &col_widths, &right_align_cols, padding);
    }

    // render separator line
    for (i, w) in col_widths.iter().enumerate() {
        if i > 0 {
            out.push_str(&" ".repeat(padding));
        }
        out.push_str(&"─".repeat(*w));
    }
    out.push('\n');

    // render data rows
    for row in rows {
        // normalize row size to match column count
        let mut cells = vec![String::new(); col_count];
        for (i, c) in row.iter().enumerate().take(col_count) {
            cells[i] = c.clone();
        }
        push_row(&mut out, &cells, &col_widths, &right_align_cols, padding);
    }

    out
}

/// Push a single row to the output, handling multiline cells and alignment.
fn push_row(
    out: &mut String,
    cells: &[String],
    widths: &[usize],
    right_align: &[bool],
    padding: usize,
) {
    // split cells into lines to handle multiline content
    let split: Vec<Vec<&str>> = cells
        .iter()
        .map(|c| {
            if c.is_empty() {
                vec![""]
            } else {
                c.split('\n').collect()
            }
        })
        .collect();

    let height = split.iter().map(|v| v.len()).max().unwrap_or(1);

    // render the physical line
    for line_idx in 0..height {
        for (i, parts) in split.iter().enumerate() {
            if i > 0 {
                out.push_str(&" ".repeat(padding));
            }
            let text = parts.get(line_idx).copied().unwrap_or("");

            // apply alignment and padding (use visible length to ignore ANSI)
            if right_align.get(i).copied().unwrap_or(false) {
                let pad = widths[i].saturating_sub(visible_width(text));
                out.push_str(&" ".repeat(pad));
                out.push_str(text);
            } else {
                out.push_str(text);
                let pad = widths[i].saturating_sub(visible_width(text));
                out.push_str(&" ".repeat(pad));
            }
        }
        out.push('\n');
    }
}

/// Check if a string looks like a numeric value.
///
/// Handles common numeric formats including commas and unit suffixes.
fn is_numeric_like(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() {
        return false;
    }
    let t = t.replace(',', "");
    let t = t.trim_end_matches(['B', 's', '%', 'K', 'M', 'G', 'T']);
    t.parse::<f64>().is_ok()
}

/// Find the maximum visible line length in a potentially multiline string.
fn max_visible_line_len(s: &str) -> usize {
    s.split('\n').map(visible_width).max().unwrap_or(0)
}
