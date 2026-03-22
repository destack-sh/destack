use serde::Serialize;
use serde_json::Value;

use crate::console;

/// Spacing mode for list output.
#[derive(Clone, Copy, Debug)]
pub enum ListSpacing {
    /// Print lists without blank line separators.
    Compact,
    /// Print lists with blank line separators.
    Spaced,
}

/// A single list entry with optional detail lines.
#[derive(Debug, Clone)]
pub struct ListEntry {
    /// The main line for the entry.
    pub title: String,
    /// Additional detail lines.
    pub lines: Vec<String>,
}

impl ListEntry {
    /// Create a list entry with the provided title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            lines: Vec::new(),
        }
    }

    /// Add a detail line to the list entry.
    pub fn line(mut self, line: impl Into<String>) -> Self {
        self.lines.push(line.into());
        self
    }
}

/// A grouped list section with a heading.
#[derive(Debug, Clone)]
pub struct ListGroup {
    /// Group heading line.
    pub title: String,
    /// Entries within the group.
    pub entries: Vec<ListEntry>,
}

impl ListGroup {
    /// Create a list group with a heading and entries.
    pub fn new(title: impl Into<String>, entries: Vec<ListEntry>) -> Self {
        Self {
            title: title.into(),
            entries,
        }
    }
}

/// Printer functions for list output.
#[derive(Clone, Copy, Debug)]
pub struct ListPrinter {
    /// Print a group heading.
    pub heading: fn(&str),
    /// Print a list entry title.
    pub title: fn(&str),
    /// Print a list entry detail line.
    pub line: fn(&str),
}

impl ListPrinter {
    /// Build a printer that writes plain stdout lines.
    pub fn plain() -> Self {
        Self {
            heading: console::print,
            title: console::print,
            line: console::print,
        }
    }

    /// Build a printer that writes info-styled stdout lines.
    pub fn info() -> Self {
        Self {
            heading: console::info,
            title: console::info,
            line: console::info,
        }
    }
}

/// Build a JSON list payload with items and total count.
pub fn list_payload<T: Serialize>(items: Vec<T>) -> Value {
    let total = items.len();
    list_payload_with_count(items, total)
}

/// Build a JSON list payload with items and total count override.
pub fn list_payload_with_count<T: Serialize>(items: Vec<T>, total: usize) -> Value {
    serde_json::json!({
        "total": total,
        "items": items,
    })
}

/// Build a JSON grouped list payload with total counts.
pub fn grouped_list_payload<T: Serialize>(groups: Vec<T>, total_items: usize) -> Value {
    serde_json::json!({
        "total_groups": groups.len(),
        "total_items": total_items,
        "groups": groups,
    })
}

/// Print list entries with the requested spacing.
pub fn print_list(entries: &[ListEntry], spacing: ListSpacing) {
    let printer = ListPrinter::plain();
    print_list_with(entries, spacing, &printer);
}

/// Print list entries using a custom printer.
pub fn print_list_with(entries: &[ListEntry], spacing: ListSpacing, printer: &ListPrinter) {
    for (index, entry) in entries.iter().enumerate() {
        (printer.title)(&entry.title);
        for line in &entry.lines {
            (printer.line)(&format!("  {line}"));
        }
        if matches!(spacing, ListSpacing::Spaced) && index + 1 < entries.len() {
            println!();
        }
    }
}

/// Print grouped list entries with the requested spacing.
pub fn print_grouped_list(groups: &[ListGroup], spacing: ListSpacing) {
    let printer = ListPrinter::plain();
    print_grouped_list_with(groups, spacing, &printer);
}

/// Print grouped list entries using a custom printer.
pub fn print_grouped_list_with(groups: &[ListGroup], spacing: ListSpacing, printer: &ListPrinter) {
    let mut printed = 0;
    for group in groups {
        if group.entries.is_empty() {
            continue;
        }
        if printed > 0 {
            println!();
        }
        (printer.heading)(&group.title);
        print_list_with(&group.entries, spacing, printer);
        printed += 1;
    }
}
