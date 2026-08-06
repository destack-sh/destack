use std::cmp::Reverse;

use super::{ArtifactAttemptSnapshot, TraceSnapshot, TraceSpanKind};

/// The default drawn width of one trace timeline lane.
const DEFAULT_TIMELINE_WIDTH: usize = 72;

/// Text rendering options for one trace timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceTimelineOptions {
    /// Whether ANSI color should be emitted.
    pub use_color: bool,
    /// The drawn width of each worker lane.
    pub width: usize,
    /// The number of slow non-parked attempts to list below the chart.
    pub slow_attempts: usize,
    /// Whether to report work, span, utilization, and the artifact critical path.
    pub parallelism: bool,
}

impl Default for TraceTimelineOptions {
    fn default() -> Self {
        Self {
            use_color: false,
            width: DEFAULT_TIMELINE_WIDTH,
            slow_attempts: 0,
            parallelism: false,
        }
    }
}

impl TraceTimelineOptions {
    /// Create default trace timeline options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether ANSI color should be emitted.
    pub fn with_color(mut self, use_color: bool) -> Self {
        self.use_color = use_color;

        self
    }

    /// Set the drawn worker lane width.
    pub fn with_width(mut self, width: usize) -> Self {
        self.width = width;

        self
    }

    /// Set how many slow non-parked attempts to list.
    pub fn with_slow_attempts(mut self, slow_attempts: usize) -> Self {
        self.slow_attempts = slow_attempts;

        self
    }

    /// Set whether to report work, span, utilization, and the artifact critical path.
    pub fn with_parallelism(mut self, parallelism: bool) -> Self {
        self.parallelism = parallelism;

        self
    }
}

/// Render the per-worker timeline of one detailed artifact trace.
pub fn render_trace_timeline(trace: &TraceSnapshot, options: TraceTimelineOptions) -> String {
    if trace.attempts.is_empty() || trace.total_micros == 0 {
        return String::new();
    }
    let Some((work_start, work_end)) = work_range(trace) else {
        return String::new();
    };

    let width = options.width.max(1);
    let work_micros = work_end - work_start;
    let kinds = TimelineKind::from_trace(trace);
    let mut output = String::new();

    // render one lane per worker
    for worker in 0..trace.workers {
        let lane = timeline_lane(
            trace,
            worker,
            &kinds,
            work_start,
            work_micros,
            width,
            options.use_color,
        );

        output.push_str(&format!("worker {worker:>2} ▕{lane}▏\n"));
    }

    // render the time scale and legend
    output.push_str(&dim(
        &format!(
            "          0{:>width$}\n",
            render_trace_duration(work_micros),
            width = width,
        ),
        options.use_color,
    ));
    output.push_str(&timeline_legend(&kinds, options.use_color));

    // render aggregate parallelism when requested
    if options.parallelism {
        output.push_str(&render_parallelism(trace, options.use_color));
    }

    // render bounded slow attempts when requested
    if options.slow_attempts > 0 {
        output.push_str(&slow_attempts(trace, options));
    }

    output
}

/// Render aggregate work, span, and worker utilization.
fn render_parallelism(trace: &TraceSnapshot, use_color: bool) -> String {
    let parallelism = &trace.parallelism;
    if parallelism.work_micros == 0 {
        return String::new();
    }

    let concurrency = ratio(parallelism.work_micros, trace.total_micros);
    let dag_parallelism = ratio(parallelism.artifact_work_micros, parallelism.span_micros);
    let capacity = trace.total_micros * trace.workers as u64;
    let utilization = 100.0 * ratio(parallelism.work_micros, capacity);
    let scheduler = 100.0 * ratio(parallelism.scheduler_micros, parallelism.parked_work_micros);
    let mut output = format!(
        concat!(
            "\n{}\n",
            "  wall {:>9}  total work {:>9}  span {:>9}  lower bound {:>9}\n",
            "  artifact work {:>9}  parked work {:>9}  park work {:>9} ({:>4.1}% of parked)\n",
            "  concurrency {:>6.2}×  DAG parallelism {:>6.2}×  utilization {:>6.1}%  bound gap {:>9}\n",
        ),
        bold("parallelism", use_color),
        render_trace_duration(trace.total_micros),
        render_trace_duration(parallelism.work_micros),
        render_trace_duration(parallelism.span_micros),
        render_trace_duration(parallelism.lower_bound_micros),
        render_trace_duration(parallelism.artifact_work_micros),
        render_trace_duration(parallelism.parked_work_micros),
        render_trace_duration(parallelism.scheduler_micros),
        scheduler,
        concurrency,
        dag_parallelism,
        utilization,
        render_trace_duration(parallelism.bound_gap_micros),
    );
    output.push_str(&render_critical_path(trace, use_color));

    output
}

/// Render the artifact dependency critical path.
fn render_critical_path(trace: &TraceSnapshot, use_color: bool) -> String {
    let parallelism = &trace.parallelism;
    if parallelism.critical_path.is_empty() {
        return String::new();
    }

    let mut output = format!(
        "\n{} {} across {} artifacts\n",
        bold("critical path", use_color),
        render_trace_duration(parallelism.span_micros),
        parallelism.critical_path.len(),
    );

    // render artifacts from earliest dependency to final dependent
    for artifact in &parallelism.critical_path {
        let name = if artifact.label.is_some() {
            format!("{:<24}", artifact.name)
        } else {
            artifact.name.clone()
        };
        let name = paint(&name, trace_stage_color(&artifact.stage), use_color);
        let label = artifact
            .label
            .as_deref()
            .map(|label| format!(" {label}"))
            .unwrap_or_default();
        output.push_str(&format!(
            "  work {:>9}  span {:>9}  fan-in {:>3}  fan-out {:>3}  {name}{label}\n",
            render_trace_duration(artifact.work_micros),
            render_trace_duration(artifact.cumulative_micros),
            artifact.dependencies,
            artifact.dependents,
        ));
    }

    output
}

/// Return one finite ratio, or zero when the denominator is zero.
fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

/// Render one microsecond count as a compact duration.
pub fn render_trace_duration(micros: u64) -> String {
    if micros >= 10_000_000 {
        format!("{:.1}s", micros as f64 / 1_000_000.0)
    } else if micros >= 1_000_000 {
        format!("{:.2}s", micros as f64 / 1_000_000.0)
    } else if micros >= 1_000 {
        format!("{}ms", micros / 1_000)
    } else {
        format!("{micros}µs")
    }
}

/// One artifact kind drawn on a timeline.
#[derive(Debug)]
struct TimelineKind {
    /// The artifact kind name.
    name: String,
    /// The toolchain phase name.
    stage: String,
    /// The summed work time across the trace.
    micros: u64,
}

impl TimelineKind {
    /// Return the artifact kinds in one trace.
    fn from_trace(trace: &TraceSnapshot) -> Vec<Self> {
        let mut kinds = Vec::<Self>::new();

        // sum exclusive work by artifact name
        for artifact in &trace.attempts {
            match kinds.iter_mut().find(|kind| kind.name == artifact.name) {
                Some(kind) => kind.micros += artifact.work_micros,
                None => kinds.push(Self {
                    name: artifact.name.clone(),
                    stage: artifact.stage.clone(),
                    micros: artifact.work_micros,
                }),
            }
        }

        kinds
    }
}

/// Return the worker timeline lane for one trace.
fn timeline_lane(
    trace: &TraceSnapshot,
    worker: usize,
    kinds: &[TimelineKind],
    work_start: u64,
    work_micros: u64,
    width: usize,
    use_color: bool,
) -> String {
    let cell_micros = work_micros.div_ceil(width as u64).max(1);
    let mut busy = vec![vec![0u64; kinds.len()]; width];

    // accumulate work overlap per artifact kind and fixed-width cell
    for artifact in trace
        .attempts
        .iter()
        .filter(|artifact| artifact.worker == worker)
    {
        let kind = kinds
            .iter()
            .position(|kind| kind.name == artifact.name)
            .expect("every timeline artifact kind should be indexed");
        for span in artifact
            .spans
            .iter()
            .filter(|span| span.kind == TraceSpanKind::Work)
        {
            let span_start = span.start_micros - work_start;
            let end = span_start + span.micros.max(1);
            let first = (span_start / cell_micros) as usize;
            let last = ((end - 1) / cell_micros) as usize;

            for (cell, lanes) in busy
                .iter_mut()
                .enumerate()
                .take(last.min(width - 1) + 1)
                .skip(first)
            {
                let cell_start = cell as u64 * cell_micros;
                let cell_end = cell_start + cell_micros;
                let overlap = end.min(cell_end).saturating_sub(span_start.max(cell_start));

                lanes[kind] += overlap;
            }
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

    timeline_runs(&cells, kinds, use_color)
}

/// Return the first and final artifact work offsets in one trace.
fn work_range(trace: &TraceSnapshot) -> Option<(u64, u64)> {
    let spans = trace
        .attempts
        .iter()
        .flat_map(|attempt| &attempt.spans)
        .filter(|span| span.kind == TraceSpanKind::Work);
    let mut start = None::<u64>;
    let mut end = 0;
    for span in spans {
        start = Some(start.map_or(span.start_micros, |start| start.min(span.start_micros)));
        end = end.max(span.start_micros + span.micros.max(1));
    }

    start.map(|start| (start, end))
}

/// Render coalesced timeline cell runs.
fn timeline_runs(cells: &[Option<usize>], kinds: &[TimelineKind], use_color: bool) -> String {
    let mut lane = String::new();
    let mut run = String::new();
    let mut run_kind = None;

    // group neighboring cells with the same artifact kind
    for cell in cells {
        if *cell != run_kind && !run.is_empty() {
            lane.push_str(&paint_timeline_run(&run, run_kind, kinds, use_color));
            run.clear();
        }

        run_kind = *cell;
        run.push(if cell.is_some() { '█' } else { '·' });
    }
    lane.push_str(&paint_timeline_run(&run, run_kind, kinds, use_color));

    lane
}

/// Paint one timeline run.
fn paint_timeline_run(
    run: &str,
    kind: Option<usize>,
    kinds: &[TimelineKind],
    use_color: bool,
) -> String {
    match kind {
        Some(kind) => paint(run, trace_stage_color(&kinds[kind].stage), use_color),
        None => dim(run, use_color),
    }
}

/// Format one work total as a compact duration.
fn compact_duration(micros: u64) -> String {
    if micros >= 1_000_000 {
        format!("{:.3}s", micros as f64 / 1_000_000.0)
    } else {
        format!("{}ms", micros / 1_000)
    }
}

/// Render the timeline legend.
fn timeline_legend(kinds: &[TimelineKind], use_color: bool) -> String {
    if !use_color {
        return dim("          █ busy  · idle\n", use_color);
    }

    let mut entries = kinds.iter().collect::<Vec<_>>();
    entries.sort_by_key(|kind| Reverse(kind.micros));
    let entries = entries
        .into_iter()
        .map(|kind| {
            let block = paint("█", trace_stage_color(&kind.stage), use_color);
            let work = dim(&compact_duration(kind.micros), use_color);

            format!("{block} {} {work}", kind.name)
        })
        .collect::<Vec<_>>();
    let mut output = String::new();

    // print compact legend rows
    for line in entries.chunks(3) {
        output.push_str(&format!("          {}\n", line.join("  ")));
    }

    output
}

/// Render the slowest non-parked artifact attempts by exclusive work.
fn slow_attempts(trace: &TraceSnapshot, options: TraceTimelineOptions) -> String {
    let mut attempts = trace
        .attempts
        .iter()
        .filter(|artifact| artifact.outcome != "parked")
        .collect::<Vec<_>>();
    attempts.sort_by_key(|artifact| Reverse(artifact.work_micros));
    if attempts.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    output.push_str(&format!(
        "\n{}\n",
        bold("slowest attempts", options.use_color)
    ));

    // render bounded slow attempt rows
    for artifact in attempts.into_iter().take(options.slow_attempts) {
        output.push_str(&slow_attempt(artifact, options.use_color));
    }

    output
}

/// Render one slow artifact attempt row.
fn slow_attempt(artifact: &ArtifactAttemptSnapshot, use_color: bool) -> String {
    let label = artifact
        .label
        .as_deref()
        .map(|label| format!(" {label}"))
        .unwrap_or_default();
    let name = paint(
        &format!("{:<24}", artifact.name),
        trace_stage_color(&artifact.stage),
        use_color,
    );
    let target = artifact
        .target
        .as_deref()
        .map(|target| dim(&format!(" ({target})"), use_color))
        .unwrap_or_default();

    format!(
        "  work {:>9}  latency {:>9}  {name}{label}{target}\n",
        render_trace_duration(artifact.work_micros),
        render_trace_duration(artifact.latency_micros),
    )
}

/// Apply bold style to one string when enabled.
fn bold(text: &str, use_color: bool) -> String {
    paint(text, "1", use_color)
}

/// Apply dim style to one string when enabled.
fn dim(text: &str, use_color: bool) -> String {
    paint(text, "2", use_color)
}

/// Apply ANSI color to one string when enabled.
fn paint(text: &str, code: &str, enabled: bool) -> String {
    if !enabled || code.is_empty() {
        return text.to_string();
    }

    format!("\x1b[{code}m{text}\x1b[0m")
}

/// Return the shared 256-color code of one toolchain phase.
pub fn trace_stage_color(stage: &str) -> &'static str {
    match stage {
        "init" => "38;5;245",
        "parse" => "38;5;75",
        "bind" => "38;5;80",
        "macro" => "38;5;115",
        "resolve" => "38;5;79",
        "graph" | "index" => "38;5;147",
        "check" => "38;5;170",
        "lower" => "38;5;208",
        "emit" => "38;5;114",
        "link" => "38;5;84",
        "lint" => "38;5;228",
        _ => "38;5;250",
    }
}
