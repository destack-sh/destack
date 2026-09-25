use tspp_repository::TraceSnapshot;

/// One styled text table.
#[derive(Debug, Default)]
pub(crate) struct TextTable {
    /// The table title.
    title: Option<String>,
    /// Whether ANSI color should be printed.
    color: bool,
    /// Rows printed by this table.
    rows: Vec<Vec<Cell>>,
}

impl TextTable {
    /// Create an empty text table.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Set this table title.
    pub(crate) fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());

        self
    }

    /// Print ANSI color.
    pub(crate) fn color(mut self) -> Self {
        self.color = true;

        self
    }

    /// Add one row.
    pub(crate) fn row(mut self, row: Vec<Cell>) -> Self {
        self.rows.push(row);

        self
    }

    /// Print this table.
    pub(crate) fn print(&self) {
        if self.rows.is_empty() {
            return;
        }

        // print a visible section boundary
        if let Some(title) = &self.title {
            println!();
            println!("{}", paint(title, "1;38;5;250", self.color));
        }

        let widths = self.column_widths();
        for row in &self.rows {
            let line = row
                .iter()
                .enumerate()
                .map(|(index, cell)| {
                    let alignment = if index == 0 {
                        Alignment::Left
                    } else {
                        Alignment::Right
                    };

                    cell.render(widths[index], alignment, self.color)
                })
                .collect::<Vec<_>>()
                .join("  ");

            println!("{line}");
        }
    }

    /// Return display widths for every column.
    fn column_widths(&self) -> Vec<usize> {
        let columns = self.rows.iter().map(Vec::len).max().unwrap_or(0);
        let mut widths = vec![0; columns];

        // accumulate visible widths before styles are applied
        for row in &self.rows {
            for (index, cell) in row.iter().enumerate() {
                widths[index] = widths[index].max(cell.width());
            }
        }

        widths
    }
}

/// One text table cell.
#[derive(Debug, Clone)]
pub(crate) struct Cell {
    /// The visible cell text.
    text: String,
    /// Optional ANSI color code.
    color: Option<&'static str>,
    /// Whether this cell should be bold.
    is_bold: bool,
}

impl Cell {
    /// Create one plain cell.
    pub(crate) fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: None,
            is_bold: false,
        }
    }

    /// Create one bold cell.
    pub(crate) fn bold(text: impl Into<String>) -> Self {
        Self::new(text).bolded()
    }

    /// Create one colored cell.
    pub(crate) fn colored(text: impl Into<String>, color: &'static str) -> Self {
        Self::new(text).colored_with(color)
    }

    /// Apply bold style.
    pub(crate) fn bolded(mut self) -> Self {
        self.is_bold = true;

        self
    }

    /// Apply one ANSI color code.
    pub(crate) fn colored_with(mut self, color: &'static str) -> Self {
        self.color = Some(color);

        self
    }

    /// Return this cell's visible width.
    fn width(&self) -> usize {
        self.text.len()
    }

    /// Render this cell at one width.
    fn render(&self, width: usize, alignment: Alignment, color: bool) -> String {
        let padded = match alignment {
            Alignment::Left => format!("{:<width$}", self.text),
            Alignment::Right => format!("{:>width$}", self.text),
        };

        let code = match (self.color, self.is_bold) {
            (Some(color), true) => format!("1;{color}"),
            (Some(color), false) => color.to_string(),
            (None, true) => "1".to_string(),
            (None, false) => String::new(),
        };

        paint(&padded, &code, color)
    }
}

/// Cell alignment.
#[derive(Debug, Clone, Copy)]
enum Alignment {
    /// Left aligned text.
    Left,
    /// Right aligned text.
    Right,
}

/// Attempt counts for one traced session operation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct TraceCounts {
    /// The total artifact attempts.
    pub(crate) attempts: usize,
    /// The attempts built by providers.
    pub(crate) built: usize,
    /// The attempts reused from the in-memory table.
    pub(crate) memory_cached: usize,
    /// The attempts that parked on missing dependencies.
    pub(crate) parked: usize,
    /// The attempts that failed.
    pub(crate) failed: usize,
}

impl TraceCounts {
    /// Count artifact attempt outcomes in one trace.
    pub(crate) fn from_trace(trace: &TraceSnapshot) -> Self {
        let mut counts = Self {
            attempts: trace.attempts.len(),
            ..Self::default()
        };

        // count the stable outcome names from the public trace snapshot
        for artifact in &trace.attempts {
            match artifact.outcome.as_str() {
                "built" => counts.built += 1,
                "memory_cached" => counts.memory_cached += 1,
                "parked" => counts.parked += 1,
                "failed" => counts.failed += 1,
                outcome => panic!("unknown artifact trace outcome {outcome}"),
            }
        }

        counts
    }
}

/// Apply ANSI color to one string when enabled.
fn paint(text: &str, code: &str, enabled: bool) -> String {
    if !enabled || code.is_empty() {
        return text.to_string();
    }

    format!("\x1b[{code}m{text}\x1b[0m")
}

/// Return one artifact counter from one detailed trace.
pub(crate) fn artifact_counter(
    trace: &TraceSnapshot,
    artifact: &str,
    counter: &str,
) -> Option<u64> {
    let attempt = trace
        .attempts
        .iter()
        .find(|attempt| attempt.name == artifact && attempt.outcome == "built")?;
    let counter = attempt
        .counters
        .iter()
        .find(|recorded| recorded.name == counter)?;

    Some(counter.value)
}

/// Return one stage timing from one trace.
pub(crate) fn stage_micros(trace: &TraceSnapshot, stage: &str) -> u64 {
    trace
        .stages
        .iter()
        .find(|recorded| recorded.name == stage)
        .map_or(0, |recorded| recorded.micros)
}

/// Return one named timing from one trace.
pub(crate) fn time_micros(trace: &TraceSnapshot, time: &str) -> u64 {
    trace
        .times
        .iter()
        .find(|recorded| recorded.name == time)
        .map_or(0, |recorded| recorded.micros)
}

/// Convert microseconds to milliseconds.
pub(crate) fn millis(micros: u64) -> f64 {
    micros as f64 / 1000.0
}
