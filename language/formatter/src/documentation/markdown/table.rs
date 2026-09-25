use markdown::mdast::{AlignKind, Node, Table};
use tspp_fir::format::{FormatError, FormatResult};

use super::super::line::LineBuffer;
use super::MarkdownFormatter;

impl MarkdownFormatter<'_> {
    /// Format one Markdown table.
    pub(super) fn format_table(&self, table: &Table, lines: &mut LineBuffer) -> FormatResult<()> {
        let mut rows = Vec::with_capacity(table.children.len());
        for child in &table.children {
            let Node::TableRow(row) = child else {
                return Err(FormatError::SyntaxError {
                    message: "Markdown table contains a non-row child",
                });
            };

            let mut cells = Vec::with_capacity(row.children.len());
            for child in &row.children {
                let Node::TableCell(cell) = child else {
                    return Err(FormatError::SyntaxError {
                        message: "Markdown table row contains a non-cell child",
                    });
                };
                let content = self
                    .format_inline_children(&cell.children)?
                    .replace('|', "\\|");
                cells.push(content);
            }
            rows.push(cells);
        }

        // require the header row supplied by the Markdown parser
        let Some(header) = rows.first() else {
            return Err(FormatError::SyntaxError {
                message: "Markdown table requires a header row",
            });
        };
        let column_count = header.len();
        if column_count == 0 || rows.iter().any(|row| row.len() != column_count) {
            return Err(FormatError::SyntaxError {
                message: "Markdown table rows require equal column counts",
            });
        }
        if table.align.len() != column_count {
            return Err(FormatError::SyntaxError {
                message: "Markdown table alignment does not match its columns",
            });
        }

        // measure each column once
        let mut widths = vec![3; column_count];
        for row in &rows {
            for (column, cell) in row.iter().enumerate() {
                widths[column] = widths[column].max(Self::text_width(cell));
            }
        }

        // write the header, alignment, and remaining rows
        Self::push_table_row(header, &widths, lines);
        Self::push_table_separator(&table.align, &widths, lines);
        for row in &rows[1..] {
            Self::push_table_row(row, &widths, lines);
        }

        Ok(())
    }

    /// Append one Markdown table row.
    fn push_table_row(cells: &[String], widths: &[usize], lines: &mut LineBuffer) {
        let line = lines.begin_line();
        for (cell, width) in cells.iter().zip(widths.iter().copied()) {
            line.push_str("| ");
            line.push_str(cell);
            for _ in Self::text_width(cell)..width {
                line.push(' ');
            }
            line.push(' ');
        }
        line.push('|');
    }

    /// Append one Markdown table alignment row.
    fn push_table_separator(alignments: &[AlignKind], widths: &[usize], lines: &mut LineBuffer) {
        let line = lines.begin_line();
        for (alignment, width) in alignments.iter().zip(widths.iter().copied()) {
            line.push_str("| ");
            let is_left = matches!(alignment, AlignKind::Left | AlignKind::Center);
            let is_right = matches!(alignment, AlignKind::Right | AlignKind::Center);
            if is_left {
                line.push(':');
            }
            for _ in usize::from(is_left)..width - usize::from(is_right) {
                line.push('-');
            }
            if is_right {
                line.push(':');
            }
            line.push(' ');
        }
        line.push('|');
    }
}
