use super::{bold, dim, visible_width};

/// Spaces between adjacent table columns.
const COLUMN_GAP: &str = "  ";

/// Alignment of one table column.
#[derive(Debug)]
enum TableAlignment {
    /// Align cells to the left edge.
    Left,
    /// Align cells to the right edge.
    Right,
}

/// One table column.
#[derive(Debug)]
pub struct TableColumn<'a> {
    /// Column heading.
    heading: &'a str,
    /// Cell alignment.
    alignment: TableAlignment,
}

impl<'a> TableColumn<'a> {
    /// Create one left-aligned column.
    pub const fn left(heading: &'a str) -> Self {
        Self {
            heading,
            alignment: TableAlignment::Left,
        }
    }

    /// Create one right-aligned column.
    pub const fn right(heading: &'a str) -> Self {
        Self {
            heading,
            alignment: TableAlignment::Right,
        }
    }
}

/// One rectangular console table.
#[derive(Debug)]
pub struct Table<'a, const N: usize> {
    /// Columns in display order.
    columns: [TableColumn<'a>; N],
    /// Rows in display order.
    rows: &'a [[String; N]],
}

impl<'a, const N: usize> Table<'a, N> {
    /// Create one table from exact columns and rows.
    pub const fn new(columns: [TableColumn<'a>; N], rows: &'a [[String; N]]) -> Self {
        Self { columns, rows }
    }

    /// Render this table.
    pub fn render(&self) -> String {
        let mut widths = std::array::from_fn(|index| visible_width(self.columns[index].heading));

        // measure every cell against its column
        for row in self.rows {
            for (index, cell) in row.iter().enumerate() {
                widths[index] = widths[index].max(visible_width(cell));
            }
        }

        // render the heading and separator
        let headings = std::array::from_fn(|index| bold(self.columns[index].heading));
        let mut output = String::new();
        self.push_row(&mut output, &headings, &widths);
        let separator = widths.map(|width| dim(&"─".repeat(width)));
        self.push_row(&mut output, &separator, &widths);

        // render every exact row
        for row in self.rows {
            self.push_row(&mut output, row, &widths);
        }

        output
    }

    /// Append one row with this table's column alignment.
    fn push_row(&self, output: &mut String, cells: &[String; N], widths: &[usize; N]) {
        for (index, ((cell, width), column)) in
            cells.iter().zip(widths).zip(&self.columns).enumerate()
        {
            if index > 0 {
                output.push_str(COLUMN_GAP);
            }

            let padding = width - visible_width(cell);
            if matches!(column.alignment, TableAlignment::Right) {
                output.push_str(&" ".repeat(padding));
                output.push_str(cell);
            } else {
                output.push_str(cell);
                output.push_str(&" ".repeat(padding));
            }
        }
        output.push('\n');
    }
}
