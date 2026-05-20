use std::collections::BTreeMap;
use std::ops::Range;

use super::{SnapshotAnchor, SnapshotField, SnapshotRow};

/// Assert rendered snapshot text.
pub(crate) fn assert_snapshot(actual: String, expected: &str) {
    let expected = expected.trim_matches('\n');

    assert_eq!(actual, expected);
}

/// Renderer for source text with snapshot rows.
pub(super) struct SnapshotRenderer<'a> {
    /// The source text to render.
    source: &'a str,
    /// The ordered rows to overlay.
    rows: &'a [SnapshotRow],
}

impl<'a> SnapshotRenderer<'a> {
    /// Sort rows into source, table, entry, and field order.
    pub(super) fn sort_rows(rows: &mut [SnapshotRow]) {
        rows.sort_by(|left, right| {
            let left_fields = Self::render_fields(&left.fields);
            let right_fields = Self::render_fields(&right.fields);

            (
                left.anchor.sort_key(),
                Self::table_rank(left.tag.table),
                Self::entry_rank(left.tag.entry),
                left_fields,
            )
                .cmp(&(
                    right.anchor.sort_key(),
                    Self::table_rank(right.tag.table),
                    Self::entry_rank(right.tag.entry),
                    right_fields,
                ))
        });
    }

    /// Create a snapshot renderer.
    pub(super) fn new(source: &'a str, rows: &'a [SnapshotRow]) -> Self {
        Self { source, rows }
    }

    /// Render source text with inserted rows.
    pub(super) fn render(&self) -> String {
        let line_ranges = self.source_line_ranges();
        let rendered_ranges = self.rendered_line_ranges(&line_ranges);
        let rows_by_line = self.rows_by_line(&rendered_ranges);
        let end_rows = self.end_rows();
        let summary_rows = self.summary_rows();
        let mut lines = Vec::new();

        // overlay rows after their source line
        for (line_index, range) in rendered_ranges.iter().enumerate() {
            let line = self.source[range.start as usize..range.end as usize].trim_end_matches('\r');
            lines.push(line.to_string());

            if let Some(rows) = rows_by_line.get(&line_index) {
                let indent = Self::source_line_indent(line);
                for row in rows {
                    lines.push(format!("{indent}{}", Self::render_row(row)));
                }

                if self.should_separate_row_group(&rendered_ranges, line_index) {
                    lines.push(String::new());
                }
            }
        }

        // append rows without a source anchor
        for row in end_rows {
            lines.push(Self::render_row(row));
        }

        // render table summaries as a footer
        if !summary_rows.is_empty() {
            if lines.last().is_some_and(|line| !line.is_empty()) {
                lines.push(String::new());
            }

            for row in summary_rows {
                lines.push(Self::render_row(row));
            }
        }

        lines.join("\n")
    }

    /// Render row fields.
    pub(super) fn render_fields(fields: &[SnapshotField]) -> String {
        fields
            .iter()
            .map(|field| format!("{}={}", field.key, Self::quote_value(&field.value)))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Collect rows rendered at the end before summaries.
    fn end_rows(&self) -> Vec<&SnapshotRow> {
        self.rows
            .iter()
            .filter(|row| matches!(row.anchor, SnapshotAnchor::End) && !Self::is_summary_row(row))
            .collect()
    }

    /// Collect summary rows rendered in the footer.
    fn summary_rows(&self) -> Vec<&SnapshotRow> {
        self.rows
            .iter()
            .filter(|row| Self::is_summary_row(row))
            .collect()
    }

    /// Check whether one row belongs to the summary footer.
    fn is_summary_row(row: &SnapshotRow) -> bool {
        row.tag.entry == "summary"
    }

    /// Check whether to insert a blank line after an annotation group.
    fn should_separate_row_group(&self, ranges: &[Range<u32>], line_index: usize) -> bool {
        let Some(next_range) = ranges.get(line_index + 1) else {
            return false;
        };

        !self.source_range_is_blank(next_range)
    }

    /// Build line ranges over the original source offsets.
    fn source_line_ranges(&self) -> Vec<Range<u32>> {
        let mut ranges = Vec::new();
        let mut start = 0;

        for (index, byte) in self.source.bytes().enumerate() {
            if byte == b'\n' {
                ranges.push(start..index as u32);
                start = index as u32 + 1;
            }
        }

        if start <= self.source.len() as u32 {
            ranges.push(start..self.source.len() as u32);
        }

        ranges
    }

    /// Return line ranges after trimming outer blank lines.
    fn rendered_line_ranges(&self, ranges: &[Range<u32>]) -> Vec<Range<u32>> {
        let mut first = 0;
        let mut last = ranges.len();

        while first < last && self.source_range_is_blank(&ranges[first]) {
            first += 1;
        }

        while last > first && self.source_range_is_blank(&ranges[last - 1]) {
            last -= 1;
        }

        ranges[first..last].to_vec()
    }

    /// Group rows by rendered source line.
    fn rows_by_line(&self, ranges: &[Range<u32>]) -> BTreeMap<usize, Vec<&'a SnapshotRow>> {
        let mut grouped = BTreeMap::new();

        for row in self.rows {
            let SnapshotAnchor::After(span) = row.anchor else {
                continue;
            };

            let Some(line_index) = self.source_line_index(ranges, span.start) else {
                continue;
            };

            grouped.entry(line_index).or_insert_with(Vec::new).push(row);
        }

        grouped
    }

    /// Return the rendered line index for an absolute source byte.
    fn source_line_index(&self, ranges: &[Range<u32>], position: u32) -> Option<usize> {
        for (index, range) in ranges.iter().enumerate() {
            if position >= range.start && position <= range.end {
                return Some(index);
            }

            if range.is_empty() && position == range.start {
                return Some(index);
            }
        }

        if position == self.source.len() as u32 {
            return ranges.len().checked_sub(1);
        }

        None
    }

    /// Check whether one source line range is blank.
    fn source_range_is_blank(&self, range: &Range<u32>) -> bool {
        self.source[range.start as usize..range.end as usize]
            .trim()
            .is_empty()
    }

    /// Return leading whitespace for one rendered source line.
    fn source_line_indent(line: &str) -> String {
        line.chars()
            .take_while(|character| character.is_whitespace())
            .collect()
    }

    /// Render one snapshot row.
    fn render_row(row: &SnapshotRow) -> String {
        let fields = Self::render_fields(&row.fields);
        if fields.is_empty() {
            format!("/// @{}.{}", row.tag.table, row.tag.entry)
        } else {
            format!("/// @{}.{} {fields}", row.tag.table, row.tag.entry)
        }
    }

    /// Return table ordering rank inside one anchor.
    fn table_rank(table: &str) -> u8 {
        match table {
            "binding" => 0,
            "type" => 1,
            "generic" => 2,
            "static" => 3,
            "relation" => 4,
            "extension" => 5,
            "resolution" => 6,
            "instance" => 7,
            "dependency" => 8,
            "export" => 9,
            "global" => 10,
            "capture" => 11,
            "macro" => 12,
            "layout" => 13,
            _ => u8::MAX,
        }
    }

    /// Return row ordering rank inside one table.
    fn entry_rank(entry: &str) -> u8 {
        match entry {
            "symbol" => 0,
            "scope" => 1,
            "parameters" => 2,
            "parameter" => 3,
            "slots" => 4,
            "slot" => 5,
            "static" => 6,
            "node" => 7,
            "application" => 8,
            "entry" => 9,
            "edge" => 10,
            "local" => 11,
            "indirect" => 12,
            "star" => 13,
            "module" => 14,
            "name" => 15,
            "label" => 16,
            "member" => 17,
            "call" => 18,
            "function" => 19,
            "binding" => 20,
            "directive" => 21,
            "rule" => 22,
            "invocation" => 23,
            "type" => 24,
            "field" => 25,
            "element" => 26,
            "tag" => 27,
            "variant" => 28,
            "newtype" => 29,
            "replaced_symbol" => 30,
            "summary" => u8::MAX,
            _ => 128,
        }
    }

    /// Quote a row field value when needed.
    fn quote_value(value: &str) -> String {
        let is_list = value.starts_with('[') && value.ends_with(']');
        let needs_quotes = value.is_empty()
            || (!is_list
                && value.chars().any(|character| {
                    character.is_whitespace() || character == '"' || character == '\\'
                }));

        if needs_quotes {
            format!("{value:?}")
        } else {
            value.to_string()
        }
    }
}
