use std::cmp::Reverse;

use destack_repository::TraceSnapshot;

/// The drawn width of one trace timeline lane.
const TIMELINE_WIDTH: usize = 72;

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
    pub(crate) artifacts: usize,
    /// The artifacts built by providers.
    pub(crate) built: usize,
    /// The artifacts reused from the in-memory table.
    pub(crate) memory_cached: usize,
    /// The artifacts restored from the artifact store.
    pub(crate) store_cached: usize,
    /// The attempts that parked on missing dependencies.
    pub(crate) parked: usize,
    /// The attempts that failed.
    pub(crate) failed: usize,
}

impl TraceCounts {
    /// Count artifact attempt outcomes in one trace.
    pub(crate) fn from_trace(trace: &TraceSnapshot) -> Self {
        let mut counts = Self {
            artifacts: trace.artifacts.len(),
            ..Self::default()
        };

        // count the stable outcome names from the public trace snapshot
        for artifact in &trace.artifacts {
            match artifact.outcome.as_str() {
                "built" => counts.built += 1,
                "memory_cached" => counts.memory_cached += 1,
                "store_cached" => counts.store_cached += 1,
                "parked" => counts.parked += 1,
                "failed" => counts.failed += 1,
                outcome => panic!("unknown artifact trace outcome {outcome}"),
            }
        }

        counts
    }
}

/// Pretty printer for recorded session traces.
#[derive(Debug, Default)]
pub(crate) struct TraceTable<'a> {
    /// The rows printed by this table.
    rows: Vec<TraceRow<'a>>,
    /// Whether ANSI color should be printed.
    color: bool,
    /// Whether worker timelines should be printed.
    timeline: bool,
    /// Whether named span totals should be printed.
    times: bool,
    /// The number of slow attempts to print per trace.
    attempt_limit: usize,
}

impl<'a> TraceTable<'a> {
    /// Create an empty trace table.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Add one trace row.
    pub(crate) fn row(mut self, name: impl Into<String>, trace: &'a TraceSnapshot) -> Self {
        self.rows.push(TraceRow {
            name: name.into(),
            trace,
        });

        self
    }

    /// Print ANSI colors.
    pub(crate) fn color(mut self) -> Self {
        self.color = true;

        self
    }

    /// Print worker timelines.
    pub(crate) fn timeline(mut self) -> Self {
        self.timeline = true;

        self
    }

    /// Print named span totals.
    pub(crate) fn times(mut self) -> Self {
        self.times = true;

        self
    }

    /// Set how many slow attempts should be printed per row.
    pub(crate) fn slow_attempts(mut self, limit: usize) -> Self {
        self.attempt_limit = limit;

        self
    }

    /// Print this trace table to stdout.
    pub(crate) fn print(&self) {
        if self.rows.is_empty() {
            return;
        }

        // print the high level counters first
        self.print_summary();

        // print worker lanes when requested
        if self.timeline {
            self.print_timelines();
        }

        // print stage totals next
        self.print_stages();

        // print named span totals when requested
        if self.times {
            self.print_times();
        }

        // print slow artifact attempts when requested
        if self.attempt_limit > 0 {
            self.print_attempts();
        }
    }

    /// Print one table with total time and artifact outcomes.
    fn print_summary(&self) {
        let mut table = TextTable::new()
            .title("trace")
            .color_if(self.color)
            .row(vec![
                Cell::bold("trace"),
                Cell::bold("total ms"),
                Cell::bold("artifacts"),
                Cell::bold("built"),
                Cell::bold("memory"),
                Cell::bold("store"),
                Cell::bold("parked"),
                Cell::bold("failed"),
            ]);

        // add one summary row per recorded trace
        for row in &self.rows {
            let counts = TraceCounts::from_trace(row.trace);
            table = table.row(vec![
                Cell::new(row.name.clone()),
                Cell::colored(format_millis(row.trace.total_micros), "38;5;250"),
                Cell::new(counts.artifacts.to_string()),
                Cell::colored(counts.built.to_string(), outcome_color("built")),
                Cell::colored(
                    counts.memory_cached.to_string(),
                    outcome_color("memory_cached"),
                ),
                Cell::colored(
                    counts.store_cached.to_string(),
                    outcome_color("store_cached"),
                ),
                Cell::colored(counts.parked.to_string(), outcome_color("parked")),
                Cell::colored(counts.failed.to_string(), outcome_color("failed")),
            ]);
        }

        table.print();
    }

    /// Print one table with named span totals.
    fn print_times(&self) {
        let names = self.time_names();
        if names.is_empty() {
            return;
        }

        let mut table = TextTable::new()
            .title("times")
            .color_if(self.color)
            .row(Self::time_header("trace", &names));

        // add one row with all time columns per trace
        for row in &self.rows {
            let mut fields = vec![Cell::new(row.name.clone())];
            for name in &names {
                fields.push(Cell::colored(
                    format_millis(time_micros(row.trace, name)),
                    time_color(name),
                ));
            }
            table = table.row(fields);
        }

        table.print();
    }

    /// Print worker timelines for all rows.
    fn print_timelines(&self) {
        for row in &self.rows {
            if row.trace.artifacts.is_empty() || row.trace.total_micros == 0 {
                continue;
            }

            println!();
            println!(
                "{}",
                paint(&format!("timeline {}", row.name), "1;38;5;250", self.color)
            );
            let kinds = timeline_kinds(row.trace);
            for worker in 0..row.trace.workers {
                let lane = timeline_lane(row.trace, worker, &kinds, self.color);

                println!("worker {worker:>2} ▕{lane}▏");
            }

            println!(
                "          0 ms{:>width$}",
                format!("{} ms", format_millis(row.trace.total_micros)),
                width = TIMELINE_WIDTH,
            );
            self.print_legend(&kinds);
        }
    }

    /// Print the artifact kind legend for one timeline.
    fn print_legend(&self, kinds: &[TimelineKind]) {
        let mut entries = kinds.iter().collect::<Vec<_>>();
        entries.sort_by_key(|kind| Reverse(kind.micros));
        let entries = entries
            .into_iter()
            .map(|kind| {
                let block = paint("█", kind_color(&kind.name), self.color);

                format!("{block} {}", kind.name)
            })
            .collect::<Vec<_>>();

        if entries.is_empty() {
            println!("          · idle");

            return;
        }

        for line in entries.chunks(4) {
            println!("          {}", line.join("  "));
        }
        println!("          · idle");
    }

    /// Print one table with artifact stage totals.
    fn print_stages(&self) {
        let names = self.stage_names();
        if names.is_empty() {
            return;
        }

        let mut table = TextTable::new()
            .title("stages")
            .color_if(self.color)
            .row(Self::header("trace", &names));

        // add one row with all stage columns per trace
        for row in &self.rows {
            let mut fields = vec![Cell::new(row.name.clone())];
            for name in &names {
                fields.push(Cell::colored(
                    format_millis(stage_micros(row.trace, name)),
                    stage_color(name),
                ));
            }
            table = table.row(fields);
        }

        table.print();
    }

    /// Print the slowest artifact attempts for each trace.
    fn print_attempts(&self) {
        let mut table = TextTable::new()
            .title("attempts")
            .color_if(self.color)
            .row(vec![
                Cell::bold("trace"),
                Cell::bold("stage"),
                Cell::bold("artifact"),
                Cell::bold("outcome"),
                Cell::bold("ms"),
            ]);

        // append bounded slow-attempt rows for every trace
        for row in &self.rows {
            for attempt in self.attempt_rows(row) {
                table = table.row(attempt);
            }
        }

        if table.rows.len() <= 1 {
            return;
        }

        table.print();
    }

    /// Return the slowest artifact attempts for one trace.
    fn attempt_rows(&self, row: &TraceRow<'_>) -> Vec<Vec<Cell>> {
        let mut attempts = row.trace.artifacts.iter().collect::<Vec<_>>();
        attempts.sort_by_key(|attempt| Reverse(attempt.micros));

        let mut rows = Vec::new();

        // keep only the largest attempts so the report stays readable
        for attempt in attempts.into_iter().take(self.attempt_limit) {
            rows.push(vec![
                Cell::new(row.name.clone()),
                Cell::colored(attempt.stage.clone(), stage_color(&attempt.stage)),
                Cell::colored(attempt.name.clone(), kind_color(&attempt.name)),
                Cell::colored(attempt.outcome.clone(), outcome_color(&attempt.outcome)),
                Cell::colored(format_millis(attempt.micros), "38;5;250"),
            ]);
        }

        rows
    }

    /// Return the stage names present in this table.
    fn stage_names(&self) -> Vec<String> {
        let mut names = Vec::new();

        // keep first-seen order from the trace snapshots
        for row in &self.rows {
            for stage in &row.trace.stages {
                if !names.contains(&stage.name) {
                    names.push(stage.name.clone());
                }
            }
        }

        names
    }

    /// Return the named times present in this table.
    fn time_names(&self) -> Vec<String> {
        let mut names = Vec::new();

        // keep first-seen order from the trace snapshots
        for row in &self.rows {
            for time in &row.trace.times {
                if !names.contains(&time.name) {
                    names.push(time.name.clone());
                }
            }
        }

        names
    }

    /// Build one string header from dynamic column names.
    fn header(first: &str, names: &[String]) -> Vec<Cell> {
        let mut header = vec![Cell::bold(first)];
        header.extend(
            names
                .iter()
                .map(|name| Cell::colored(format!("{name} ms"), stage_color(name)).bolded()),
        );

        header
    }

    /// Build one time header from dynamic column names.
    fn time_header(first: &str, names: &[String]) -> Vec<Cell> {
        let mut header = vec![Cell::bold(first)];
        header.extend(
            names
                .iter()
                .map(|name| Cell::colored(format!("{name} ms"), time_color(name)).bolded()),
        );

        header
    }
}

impl TextTable {
    /// Enable color when requested.
    fn color_if(mut self, color: bool) -> Self {
        self.color = color;

        self
    }
}

/// One named trace row.
#[derive(Debug)]
struct TraceRow<'a> {
    /// The row display name.
    name: String,
    /// The trace snapshot to print.
    trace: &'a TraceSnapshot,
}

/// One artifact kind drawn on a timeline.
#[derive(Debug)]
struct TimelineKind {
    /// The artifact kind name.
    name: String,
    /// The summed busy time across the trace.
    micros: u64,
}

/// Return the artifact kinds in one trace.
fn timeline_kinds(trace: &TraceSnapshot) -> Vec<TimelineKind> {
    let mut kinds = Vec::<TimelineKind>::new();

    // sum busy time by artifact name
    for artifact in &trace.artifacts {
        match kinds.iter_mut().find(|kind| kind.name == artifact.name) {
            Some(kind) => kind.micros += artifact.micros,
            None => kinds.push(TimelineKind {
                name: artifact.name.clone(),
                micros: artifact.micros,
            }),
        }
    }

    kinds
}

/// Return the worker timeline lane for one trace.
fn timeline_lane(
    trace: &TraceSnapshot,
    worker: usize,
    kinds: &[TimelineKind],
    color: bool,
) -> String {
    let cell_micros = trace.total_micros.div_ceil(TIMELINE_WIDTH as u64).max(1);
    let mut busy = vec![vec![0u64; kinds.len()]; TIMELINE_WIDTH];

    // accumulate busy overlap per artifact kind and fixed-width cell
    for artifact in trace
        .artifacts
        .iter()
        .filter(|artifact| artifact.worker == worker)
    {
        let kind = kinds
            .iter()
            .position(|kind| kind.name == artifact.name)
            .expect("every timeline artifact kind should be indexed");
        let end = artifact.start_micros + artifact.micros.max(1);
        let first = (artifact.start_micros / cell_micros) as usize;
        let last = ((end - 1) / cell_micros) as usize;

        for (cell, lanes) in busy
            .iter_mut()
            .enumerate()
            .take(last.min(TIMELINE_WIDTH - 1) + 1)
            .skip(first)
        {
            let cell_start = cell as u64 * cell_micros;
            let cell_end = cell_start + cell_micros;
            let overlap = end
                .min(cell_end)
                .saturating_sub(artifact.start_micros.max(cell_start));

            lanes[kind] += overlap;
        }
    }

    // choose each cell's busiest artifact kind
    let cells = busy
        .into_iter()
        .map(|cell| {
            cell.iter()
                .enumerate()
                .filter(|(_kind, micros)| **micros > 0)
                .max_by_key(|(_kind, micros)| **micros)
                .map(|(kind, _micros)| kind)
        })
        .collect::<Vec<_>>();

    timeline_runs(&cells, kinds, color)
}

/// Render coalesced timeline cell runs.
fn timeline_runs(cells: &[Option<usize>], kinds: &[TimelineKind], color: bool) -> String {
    let mut lane = String::new();
    let mut run = String::new();
    let mut run_kind = None;

    // group neighboring cells with the same artifact kind
    for cell in cells {
        if *cell != run_kind && !run.is_empty() {
            lane.push_str(&paint_timeline_run(&run, run_kind, kinds, color));
            run.clear();
        }

        run_kind = *cell;
        run.push(if cell.is_some() { '█' } else { '·' });
    }
    lane.push_str(&paint_timeline_run(&run, run_kind, kinds, color));

    lane
}

/// Paint one timeline run.
fn paint_timeline_run(
    run: &str,
    kind: Option<usize>,
    kinds: &[TimelineKind],
    color: bool,
) -> String {
    match kind {
        Some(kind) => paint(run, kind_color(&kinds[kind].name), color),
        None => paint(run, "2", color),
    }
}

/// Apply ANSI color to one string when enabled.
fn paint(text: &str, code: &str, enabled: bool) -> String {
    if !enabled || code.is_empty() {
        return text.to_string();
    }

    format!("\x1b[{code}m{text}\x1b[0m")
}

/// Return the 256-color code of one artifact kind.
fn kind_color(name: &str) -> &'static str {
    match name {
        "dir.parse" => "38;5;75",
        "data" => "38;5;67",
        "dir.bind" => "38;5;80",
        "dir.import" => "38;5;73",
        "dir.expand" => "38;5;115",
        "dir.export" => "38;5;72",
        "dir.resolve" => "38;5;79",
        "component.graph" => "38;5;147",
        "dir.check.component" => "38;5;170",
        "dir.check" => "38;5;176",
        "dir.materialize" => "38;5;178",
        "dir.elaborate" => "38;5;179",
        "mir.lower" => "38;5;208",
        "mir.verify" => "38;5;209",
        "mir.optimize" => "38;5;214",
        "module.emit" => "38;5;114",
        "package.link" => "38;5;84",
        "module.lint" | "package.lint" | "workspace.lint" => "38;5;228",
        "workspace.index" => "38;5;147",
        "environment" | "dependency.index" => "38;5;245",
        _ => "38;5;250",
    }
}

/// Return the 256-color code of one stage.
fn stage_color(stage: &str) -> &'static str {
    if stage.starts_with("emit") {
        return "38;5;114";
    }

    match stage {
        "parse" => "38;5;75",
        "bind" => "38;5;80",
        "macro" => "38;5;115",
        "resolve" => "38;5;79",
        "graph" => "38;5;147",
        "check" => "38;5;170",
        "lower" => "38;5;208",
        "link" => "38;5;84",
        "lint" => "38;5;228",
        "query" => "38;5;147",
        "init" => "38;5;245",
        _ => "38;5;250",
    }
}

/// Return the 256-color code of one named timing.
fn time_color(time: &str) -> &'static str {
    match time {
        "enqueue" | "scheduler" | "cleanup" => "38;5;245",
        "execute" => "38;5;250",
        "collect" | "dependencies" | "version" | "base" => "38;5;229",
        "memory_cache" | "store_cache" | "bind" => "38;5;80",
        "provider" => "38;5;114",
        "publish" | "store" => "38;5;147",
        "modules" | "edges" | "scc" | "condensation" => "38;5;147",
        _ => "38;5;250",
    }
}

/// Return the 256-color code of one artifact attempt outcome.
fn outcome_color(outcome: &str) -> &'static str {
    match outcome {
        "built" => "38;5;114",
        "memory_cached" => "38;5;80",
        "store_cached" => "38;5;147",
        "parked" => "38;5;220",
        "failed" => "38;5;196",
        _ => "38;5;250",
    }
}

/// Return one artifact counter from one detailed trace.
pub(crate) fn artifact_counter(
    trace: &TraceSnapshot,
    artifact: &str,
    counter: &str,
) -> Option<u64> {
    let attempt = trace
        .artifacts
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

/// Convert microseconds to formatted milliseconds.
pub(crate) fn format_millis(micros: u64) -> String {
    format!("{:.3}", millis(micros))
}

/// Convert microseconds to milliseconds.
pub(crate) fn millis(micros: u64) -> f64 {
    micros as f64 / 1000.0
}
