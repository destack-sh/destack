use std::cmp::Reverse;
use std::fmt::Write;

use super::{
    ArtifactAttemptSnapshot, TraceSnapshot, TraceSpanKind, TraceTimeSnapshot, TraceTimelineOptions,
    render_trace_timeline, trace_stage_color,
};

/// The timeline width of one single-trace page.
const SINGLE_TIMELINE_WIDTH: usize = 110;

/// The full width of one proportional report bar.
const BAR_WIDTH: u64 = 24;

/// A formatted report for one or more operation traces.
#[derive(Debug, Default)]
pub struct TraceReport {
    /// The traces rendered as report rows.
    rows: Vec<TraceReportRow>,
    /// Whether ANSI color should be emitted.
    use_color: bool,
    /// Whether worker timelines should be rendered.
    is_timeline_enabled: bool,
    /// Whether named span totals should be rendered.
    is_span_total_enabled: bool,
    /// Whether provider events should be rendered.
    is_event_enabled: bool,
    /// The number of slow attempts rendered per trace.
    slow_attempt_limit: usize,
}

impl TraceReport {
    /// Create an empty trace report.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add one named trace.
    pub fn row(mut self, name: impl Into<String>, trace: TraceSnapshot) -> Self {
        self.rows.push(TraceReportRow {
            name: name.into(),
            trace,
        });

        self
    }

    /// Emit ANSI color.
    pub fn color(mut self) -> Self {
        self.use_color = true;

        self
    }

    /// Render worker timelines.
    pub fn timelines(mut self) -> Self {
        self.is_timeline_enabled = true;

        self
    }

    /// Render named span totals.
    pub fn span_totals(mut self) -> Self {
        self.is_span_total_enabled = true;

        self
    }

    /// Render provider events.
    pub fn events(mut self) -> Self {
        self.is_event_enabled = true;

        self
    }

    /// Set the number of slow attempts rendered per trace.
    pub fn slow_attempts(mut self, limit: usize) -> Self {
        self.slow_attempt_limit = limit;

        self
    }

    /// Render this report.
    pub fn render(&self) -> String {
        if self.rows.is_empty() {
            return String::new();
        }

        if let [row] = self.rows.as_slice() {
            return self.render_single(row);
        }

        let mut output = String::new();
        self.render_summary(&mut output);
        self.render_counters(&mut output);
        if self.is_timeline_enabled {
            self.render_timelines(&mut output);
        }
        self.render_stages(&mut output);
        if self.is_span_total_enabled {
            self.render_times(&mut output);
        }
        if self.slow_attempt_limit > 0 {
            self.render_slow_attempts(&mut output);
        }
        if self.is_event_enabled {
            self.render_events(&mut output);
        }

        output
    }

    /// Render one trace as a single organized page.
    fn render_single(&self, row: &TraceReportRow) -> String {
        let mut output = String::new();
        let stats = &row.trace.stats;

        // one header line names the trace, one line summarizes outcomes
        let title = paint(&format!("trace {}", row.name), "1;38;5;250", self.use_color);
        let _ = write!(output, "\n{title}\n");
        let _ = writeln!(
            output,
            "  wall {} ms  attempts {}  built {}  memory {}  parked {}  failed {}",
            paint(&millis(row.trace.total_micros), "38;5;250", self.use_color),
            row.trace.attempts.len(),
            paint(
                &stats.built.to_string(),
                outcome_color("built"),
                self.use_color
            ),
            stats.memory_cached,
            paint(
                &stats.parked.to_string(),
                outcome_color("parked"),
                self.use_color
            ),
            paint(
                &stats.failed.to_string(),
                outcome_color("failed"),
                self.use_color
            ),
        );

        // draw the timeline when the trace has spans
        if self.is_timeline_enabled && !row.trace.attempts.is_empty() && row.trace.total_micros > 0
        {
            let title = paint("timeline", "1;38;5;250", self.use_color);
            let _ = write!(output, "\n{title}\n");
            let options = TraceTimelineOptions::new()
                .with_color(self.use_color)
                .with_width(SINGLE_TIMELINE_WIDTH)
                .with_parallelism(true);
            output.push_str(&render_trace_timeline(&row.trace, options));
        }

        // render stage work totals largest first
        let mut stages = row.trace.stages.clone();
        stages.sort_unstable_by_key(|stage| Reverse(stage.micros));
        let largest = stages.first().map_or(1, |stage| stage.micros);
        let mut table = TextTable::new("stage work (sum across workers)", self.use_color);
        for stage in &stages {
            table = table.row(vec![
                Cell::colored(&stage.name, trace_stage_color(&stage.name)),
                Cell::new(format!("{} ms", millis(stage.micros))),
                Cell::left(bar(stage.micros, largest)),
            ]);
        }
        table.render(&mut output);

        // break span totals into parents and their nested children
        if self.is_span_total_enabled {
            let times = &row.trace.times;
            let mut tops = times
                .iter()
                .filter(|time| !is_child_span(&time.name, times))
                .collect::<Vec<_>>();
            tops.sort_unstable_by_key(|time| Reverse(time.micros));
            let largest = tops.first().map_or(1, |top| top.micros);

            // nest breakdown children under their owning stage
            let mut table = TextTable::new("span totals (sum across workers)", self.use_color);
            for top in tops {
                table = table.row(vec![
                    Cell::colored(&top.name, time_color(&top.name)),
                    Cell::new(format!("{} ms", millis(top.micros))),
                    Cell::left(bar(top.micros, largest)),
                ]);
                let mut children = times
                    .iter()
                    .filter(|time| {
                        time.name
                            .rsplit_once('.')
                            .is_some_and(|(parent, _)| parent == top.name)
                    })
                    .collect::<Vec<_>>();
                children.sort_unstable_by_key(|time| Reverse(time.micros));
                for child in children {
                    table = table.row(vec![
                        Cell::colored(format!("  {}", child.name), time_color(&child.name)),
                        Cell::new(format!("{} ms", millis(child.micros))),
                        Cell::left(bar(child.micros, largest)),
                    ]);
                }
            }
            table.render(&mut output);
        }

        // list the slowest attempts
        if self.slow_attempt_limit > 0 {
            let mut table = TextTable::new("slow attempts", self.use_color).row(vec![
                Cell::bold("work ms"),
                Cell::bold("latency"),
                Cell::bold("stage"),
                Cell::bold("artifact"),
                Cell::bold("subject"),
                Cell::bold("outcome"),
                Cell::bold("work spans"),
            ]);
            let mut attempts = row.trace.attempts.iter().collect::<Vec<_>>();
            attempts.sort_unstable_by_key(|attempt| Reverse(attempt.work_micros));
            for attempt in attempts.into_iter().take(self.slow_attempt_limit) {
                let subject = attempt.label.as_deref().unwrap_or_default();
                let subject = subject.strip_prefix("tspp://").unwrap_or(subject);
                table = table.row(vec![
                    Cell::colored(millis(attempt.work_micros), "38;5;250"),
                    Cell::colored(millis(attempt.latency_micros), "38;5;245"),
                    Cell::colored(&attempt.stage, trace_stage_color(&attempt.stage)),
                    Cell::colored(&attempt.name, trace_stage_color(&attempt.stage)),
                    Cell::new(subject),
                    Cell::colored(&attempt.outcome, outcome_color(&attempt.outcome)),
                    Cell::new(render_attempt_work(attempt)),
                ]);
            }
            if table.rows.len() > 1 {
                table.render(&mut output);
            }
        }

        // render counter totals largest first
        let names = self.counter_names();
        if !names.is_empty() {
            let mut counters = names
                .iter()
                .map(|name| (name.clone(), counter_total(&row.trace, name)))
                .collect::<Vec<_>>();
            counters.sort_unstable_by_key(|(_name, value)| Reverse(*value));
            let mut table = TextTable::new("counters", self.use_color);
            for (name, value) in counters {
                table = table.row(vec![Cell::new(name), Cell::new(value.to_string())]);
            }
            table.render(&mut output);
        }
        if self.is_event_enabled {
            self.render_events(&mut output);
        }

        output
    }

    /// Print this report to standard output.
    pub fn print(&self) {
        print!("{}", self.render());
    }

    /// Render total time and artifact outcomes.
    fn render_summary(&self, output: &mut String) {
        let mut table = TextTable::new("trace", self.use_color).row(vec![
            Cell::bold("trace"),
            Cell::bold("total ms"),
            Cell::bold("attempts"),
            Cell::bold("built"),
            Cell::bold("memory"),
            Cell::bold("parked"),
            Cell::bold("failed"),
        ]);

        // add one summary row per trace
        for row in &self.rows {
            let stats = &row.trace.stats;
            table = table.row(vec![
                Cell::new(&row.name),
                Cell::colored(millis(row.trace.total_micros), "38;5;250"),
                Cell::new(row.trace.attempts.len().to_string()),
                Cell::colored(stats.built.to_string(), outcome_color("built")),
                Cell::colored(
                    stats.memory_cached.to_string(),
                    outcome_color("memory_cached"),
                ),
                Cell::colored(stats.parked.to_string(), outcome_color("parked")),
                Cell::colored(stats.failed.to_string(), outcome_color("failed")),
            ]);
        }

        table.render(output);
    }

    /// Render named trace and artifact attempt counters.
    fn render_counters(&self, output: &mut String) {
        let names = self.counter_names();
        if names.is_empty() {
            return;
        }

        let mut table = TextTable::new("counters", self.use_color);
        let mut header = vec![Cell::bold("trace")];
        header.extend(names.iter().map(Cell::bold));
        table = table.row(header);

        // add one counter row per trace
        for row in &self.rows {
            let mut cells = vec![Cell::new(&row.name)];
            for name in &names {
                cells.push(Cell::new(counter_total(&row.trace, name).to_string()));
            }
            table = table.row(cells);
        }

        table.render(output);
    }

    /// Render one worker timeline per non-empty trace.
    fn render_timelines(&self, output: &mut String) {
        for row in &self.rows {
            if row.trace.attempts.is_empty() || row.trace.total_micros == 0 {
                continue;
            }

            let title = paint(
                &format!("timeline {}", row.name),
                "1;38;5;250",
                self.use_color,
            );
            let _ = write!(output, "\n{title}\n");
            let options = TraceTimelineOptions::new()
                .with_color(self.use_color)
                .with_parallelism(true);
            output.push_str(&render_trace_timeline(&row.trace, options));
        }
    }

    /// Render artifact work by toolchain stage.
    fn render_stages(&self, output: &mut String) {
        let names = self.stage_names();
        if names.is_empty() {
            return;
        }

        let mut table =
            TextTable::new("stage work", self.use_color).row(stage_header("trace", &names));

        // add one stage row per trace
        for row in &self.rows {
            let mut cells = vec![Cell::new(&row.name)];
            for name in &names {
                let micros = row
                    .trace
                    .stages
                    .iter()
                    .find(|stage| stage.name == *name)
                    .map_or(0, |stage| stage.micros);
                cells.push(Cell::colored(millis(micros), trace_stage_color(name)));
            }
            table = table.row(cells);
        }

        table.render(output);
    }

    /// Render named operation and provider spans.
    fn render_times(&self, output: &mut String) {
        let names = self.time_names();
        if names.is_empty() {
            return;
        }

        let mut table =
            TextTable::new("span totals", self.use_color).row(time_header("trace", &names));

        // add one span row per trace
        for row in &self.rows {
            let mut cells = vec![Cell::new(&row.name)];
            for name in &names {
                let micros = row
                    .trace
                    .times
                    .iter()
                    .find(|time| time.name == *name)
                    .map_or(0, |time| time.micros);
                cells.push(Cell::colored(millis(micros), time_color(name)));
            }
            table = table.row(cells);
        }

        table.render(output);
    }

    /// Render the slowest artifact attempts.
    fn render_slow_attempts(&self, output: &mut String) {
        let mut table = TextTable::new("slow attempts", self.use_color).row(vec![
            Cell::bold("trace"),
            Cell::bold("stage"),
            Cell::bold("artifact"),
            Cell::bold("subject"),
            Cell::bold("outcome"),
            Cell::bold("work ms"),
            Cell::bold("latency ms"),
            Cell::bold("work spans"),
        ]);

        // retain only the largest attempts from each trace
        for row in &self.rows {
            let mut attempts = row.trace.attempts.iter().collect::<Vec<_>>();
            attempts.sort_unstable_by_key(|attempt| Reverse(attempt.work_micros));
            for attempt in attempts.into_iter().take(self.slow_attempt_limit) {
                table = table.row(vec![
                    Cell::new(&row.name),
                    Cell::colored(&attempt.stage, trace_stage_color(&attempt.stage)),
                    Cell::colored(&attempt.name, trace_stage_color(&attempt.stage)),
                    Cell::new(attempt.label.as_deref().unwrap_or_default()),
                    Cell::colored(&attempt.outcome, outcome_color(&attempt.outcome)),
                    Cell::colored(millis(attempt.work_micros), "38;5;250"),
                    Cell::colored(millis(attempt.latency_micros), "38;5;245"),
                    Cell::new(render_attempt_work(attempt)),
                ]);
            }
        }
        if table.rows.len() > 1 {
            table.render(output);
        }
    }

    /// Render provider events in attempt completion order.
    fn render_events(&self, output: &mut String) {
        let is_multi_trace = self.rows.len() > 1;
        let mut header = Vec::new();
        if is_multi_trace {
            header.push(Cell::bold("trace"));
        }
        header.extend([
            Cell::bold("stage"),
            Cell::bold("artifact"),
            Cell::bold("subject"),
            Cell::bold("event"),
        ]);
        let mut table = TextTable::new("provider events", self.use_color).row(header);

        // append each event under its owning attempt
        for row in &self.rows {
            for attempt in &row.trace.attempts {
                let subject = attempt.label.as_deref().unwrap_or_default();
                let subject = subject.strip_prefix("tspp://").unwrap_or(subject);
                for event in &attempt.events {
                    let mut cells = Vec::new();
                    if is_multi_trace {
                        cells.push(Cell::new(&row.name));
                    }
                    cells.extend([
                        Cell::colored(&attempt.stage, trace_stage_color(&attempt.stage)),
                        Cell::colored(&attempt.name, trace_stage_color(&attempt.stage)),
                        Cell::new(subject),
                        Cell::new(event.to_string()),
                    ]);
                    table = table.row(cells);
                }
            }
        }

        // omit the section when no provider recorded events
        if table.rows.len() > 1 {
            table.render(output);
        }
    }

    /// Return stage names in first-seen order.
    fn stage_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for row in &self.rows {
            for stage in &row.trace.stages {
                if !names.contains(&stage.name) {
                    names.push(stage.name.clone());
                }
            }
        }

        names
    }

    /// Return span names in first-seen order.
    fn time_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for row in &self.rows {
            for time in &row.trace.times {
                if !names.contains(&time.name) {
                    names.push(time.name.clone());
                }
            }
        }

        names
    }

    /// Return counter names in first-seen order.
    fn counter_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for row in &self.rows {
            for counter in &row.trace.counters {
                if !names.contains(&counter.name) {
                    names.push(counter.name.clone());
                }
            }
            for attempt in &row.trace.attempts {
                for counter in &attempt.counters {
                    if !names.contains(&counter.name) {
                        names.push(counter.name.clone());
                    }
                }
            }
        }

        names
    }
}

/// Render one proportional bar against the largest value.
fn bar(value: u64, largest: u64) -> String {
    let width = (value * BAR_WIDTH / largest.max(1)) as usize;
    let width = if value > 0 { width.max(1) } else { width };

    "\u{2588}".repeat(width)
}

/// Return whether one span name nests under another recorded span.
fn is_child_span(name: &str, times: &[TraceTimeSnapshot]) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(parent, _)| times.iter().any(|time| time.name == parent))
}

/// Sum one named counter across a trace and its attempts.
fn counter_total(trace: &TraceSnapshot, name: &str) -> u64 {
    let trace_value = trace
        .counters
        .iter()
        .filter(|counter| counter.name == name)
        .map(|counter| counter.value)
        .sum::<u64>();
    let attempt_value = trace
        .attempts
        .iter()
        .flat_map(|attempt| &attempt.counters)
        .filter(|counter| counter.name == name)
        .map(|counter| counter.value)
        .sum::<u64>();

    trace_value + attempt_value
}

/// Render the largest work spans in one artifact attempt.
fn render_attempt_work(attempt: &ArtifactAttemptSnapshot) -> String {
    let mut spans = attempt
        .spans
        .iter()
        .filter(|span| span.kind == TraceSpanKind::Work)
        .collect::<Vec<_>>();
    spans.sort_unstable_by_key(|span| Reverse(span.micros));

    spans
        .into_iter()
        .take(4)
        .map(|span| format!("{}={}", span.name, millis(span.micros)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// One named trace in a report.
#[derive(Debug)]
struct TraceReportRow {
    /// The row display name.
    name: String,
    /// The trace snapshot.
    trace: TraceSnapshot,
}

/// One aligned terminal table.
#[derive(Debug)]
struct TextTable {
    /// The table title.
    title: &'static str,
    /// Whether ANSI color should be emitted.
    use_color: bool,
    /// The table rows.
    rows: Vec<Vec<Cell>>,
}

impl TextTable {
    /// Create an empty table.
    fn new(title: &'static str, use_color: bool) -> Self {
        Self {
            title,
            use_color,
            rows: Vec::new(),
        }
    }

    /// Add one row.
    fn row(mut self, row: Vec<Cell>) -> Self {
        self.rows.push(row);

        self
    }

    /// Render this table.
    fn render(&self, output: &mut String) {
        if self.rows.is_empty() {
            return;
        }

        let title = paint(self.title, "1;38;5;250", self.use_color);
        let _ = write!(output, "\n{title}\n");
        let widths = self.column_widths();
        for row in &self.rows {
            let line = row
                .iter()
                .enumerate()
                .map(|(index, cell)| {
                    let is_left_aligned = index == 0 || cell.is_left_aligned;
                    cell.render(widths[index], is_left_aligned, self.use_color)
                })
                .collect::<Vec<_>>()
                .join("  ");
            let _ = writeln!(output, "{}", line.trim_end());
        }
    }

    /// Return the visible width of each column.
    fn column_widths(&self) -> Vec<usize> {
        let columns = self.rows.iter().map(Vec::len).max().unwrap_or(0);
        let mut widths = vec![0; columns];
        for row in &self.rows {
            for (index, cell) in row.iter().enumerate() {
                widths[index] = widths[index].max(cell.text.len());
            }
        }

        widths
    }
}

/// One styled terminal table cell.
#[derive(Debug)]
struct Cell {
    /// The visible cell text.
    text: String,
    /// The optional ANSI color code.
    color: Option<&'static str>,
    /// Whether the cell is bold.
    is_bold: bool,
    /// Whether this cell aligns left within its column.
    is_left_aligned: bool,
}

impl Cell {
    /// Create one plain cell.
    fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: None,
            is_bold: false,
            is_left_aligned: false,
        }
    }

    /// Create one left-aligned cell.
    fn left(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: None,
            is_bold: false,
            is_left_aligned: true,
        }
    }

    /// Create one bold cell.
    fn bold(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: None,
            is_bold: true,
            is_left_aligned: false,
        }
    }

    /// Create one colored cell.
    fn colored(text: impl Into<String>, color: &'static str) -> Self {
        Self {
            text: text.into(),
            color: Some(color),
            is_bold: false,
            is_left_aligned: false,
        }
    }

    /// Render this cell at one width.
    fn render(&self, width: usize, is_left_aligned: bool, use_color: bool) -> String {
        let padded = if is_left_aligned {
            format!("{:<width$}", self.text)
        } else {
            format!("{:>width$}", self.text)
        };
        let code = match (self.color, self.is_bold) {
            (Some(color), true) => format!("1;{color}"),
            (Some(color), false) => color.to_string(),
            (None, true) => "1".to_string(),
            (None, false) => String::new(),
        };

        paint(&padded, &code, use_color)
    }
}

/// Build one stage header.
fn stage_header(first: &str, names: &[String]) -> Vec<Cell> {
    let mut header = vec![Cell::bold(first)];
    header.extend(
        names
            .iter()
            .map(|name| Cell::colored(format!("{name} ms"), trace_stage_color(name))),
    );

    header
}

/// Build one span header.
fn time_header(first: &str, names: &[String]) -> Vec<Cell> {
    let mut header = vec![Cell::bold(first)];
    header.extend(
        names
            .iter()
            .map(|name| Cell::colored(format!("{name} ms"), time_color(name))),
    );

    header
}

/// Format microseconds as decimal milliseconds.
fn millis(micros: u64) -> String {
    format!("{:.3}", micros as f64 / 1_000.0)
}

/// Apply ANSI color when enabled.
fn paint(text: &str, code: &str, is_enabled: bool) -> String {
    if !is_enabled || code.is_empty() {
        return text.to_string();
    }

    format!("\x1b[{code}m{text}\x1b[0m")
}

/// Return the ANSI color of one named span.
fn time_color(time: &str) -> &'static str {
    // color by the root stage so children inherit their parent
    let root = time.split_once('.').map_or(time, |(root, _)| root);

    match root {
        "park" | "run" => "38;5;245",
        "resolve" | "select" | "collect" | "assemble" => "38;5;229",
        "reuse" => "38;5;80",
        "provide" => "38;5;114",
        "commit" => "38;5;147",
        _ => "38;5;250",
    }
}

/// Return the ANSI color of one attempt outcome.
fn outcome_color(outcome: &str) -> &'static str {
    match outcome {
        "built" => "38;5;114",
        "memory_cached" => "38;5;80",
        "parked" => "38;5;229",
        "failed" => "38;5;203",
        _ => "38;5;250",
    }
}
