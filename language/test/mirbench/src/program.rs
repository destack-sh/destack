#![cfg_attr(test, allow(dead_code, unexpected_cfgs))]

use std::any::Any;
use std::cmp::Ordering;
use std::fmt::Write;
use std::time::{Duration, Instant};

#[cfg(feature = "cli")]
use clap::ValueEnum;
use destack_heap::{Heap, MemoryContext, SharedSpace, Value};
use destack_mir as mir;
use destack_mir::parse::{ParseOptions, Parser};
use destack_source::FileId;
use destack_vm::diagnostic::RuntimeResult;
use destack_vm::{CheckPolicy, ExecutionOutcome, ExecutionOutput, Isolate, IsolateOptions};

use super::{arithmetic, calls, dispatch, function_id_by_name, intrinsics, memory, perf};

// default quick run time budget
const QUICK_TIME_BUDGET: Duration = Duration::from_secs(20);
/// Default sample interval for instruction profiling.
#[cfg(feature = "stats")]
const INSTRUCTION_PROFILE_INTERVAL: Duration = Duration::from_micros(150);
/// Target percentage for instruction profile coverage.
#[cfg(feature = "stats")]
const INSTRUCTION_PROFILE_TARGET_PERCENT: f64 = 95.0;

/// Named tags associated with a benchmark program.
pub type BenchTags = &'static [&'static str];

/// Scaling metadata for a program argument.
#[derive(Clone, Copy, Debug)]
pub struct ScaleAxis {
    /// Name of the axis for reporting.
    pub name: &'static str,
    /// Argument index to scale.
    pub arg_index: usize,
    /// Quick profile value.
    pub quick: i64,
    /// Standard profile value.
    pub standard: i64,
    /// Stress profile value.
    pub stress: i64,
    /// Whether this axis should be calibrated to a target duration.
    pub calibrate: bool,
    /// Minimum clamp for calibration.
    pub min: i64,
    /// Maximum clamp for calibration.
    pub max: i64,
}

/// Available benchmark profiles.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "cli", derive(ValueEnum))]
pub enum BenchProfileKind {
    /// Quick runs for CI and fast checks.
    Quick,
    /// Standard runs for daily benchmarking.
    Standard,
    /// Stress runs for longer profiling sessions.
    Stress,
}

/// Available output formats for benchmark results.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "cli", derive(ValueEnum))]
pub enum BenchOutputFormat {
    /// Human readable table with colors.
    Table,
    /// Machine readable json report.
    Json,
    /// Machine readable csv report.
    Csv,
}

/// Runtime profile configuration for benchmark runs.
#[derive(Clone, Copy, Debug)]
pub struct BenchProfile {
    /// Profile kind.
    pub kind: BenchProfileKind,
    /// Number of timing repeats.
    pub repeat: u32,
    /// Minimum time per program run.
    pub min_duration: Duration,
    /// Warmup time per program.
    pub warmup: Duration,
    /// Target duration for a single program invocation.
    pub target_duration: Duration,
}

/// Build a scale axis with default clamps.
pub const fn scale_axis(
    name: &'static str,
    arg_index: usize,
    quick: i64,
    standard: i64,
    stress: i64,
    calibrate: bool,
) -> ScaleAxis {
    ScaleAxis {
        name,
        arg_index,
        quick,
        standard,
        stress,
        calibrate,
        min: 1,
        max: stress,
    }
}

/// Escape a string for json output.
fn json_escape(value: &str) -> String {
    // escape reserved characters
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(escaped, "\\u{:04x}", c as u32);
            }
            _ => escaped.push(ch),
        }
    }

    // return escaped string
    escaped
}

/// Escape a string for csv output.
fn csv_escape(value: &str) -> String {
    // detect fields that need escaping
    let needs_escape = value.contains(',') || value.contains('"') || value.contains('\n');
    if !needs_escape {
        return value.to_string();
    }

    // escape quotes and wrap field
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for ch in value.chars() {
        if ch == '"' {
            escaped.push('"');
        }
        escaped.push(ch);
    }
    escaped.push('"');

    // return escaped value
    escaped
}

/// Emit benchmark results as json.
#[allow(clippy::too_many_arguments)]
fn output_json(
    rows: &[BenchRow],
    average_mops: Option<f64>,
    total_elapsed: Duration,
    options: &BenchOptions,
    profile: BenchProfile,
    min_duration: Duration,
    target_duration: Duration,
    calibrate: bool,
) {
    // prepare json output
    let mut out = String::new();
    out.push('{');

    // add profile metadata
    let _ = write!(
        out,
        "\"profile\":\"{:?}\",\"repeat\":{},\"min_duration_ms\":{},\"warmup_ms\":{},\"target_duration_ms\":{},\"calibrated\":{},\"deterministic\":{},\"fast\":{},\"time_budget_ms\":",
        profile.kind,
        profile.repeat.max(1),
        min_duration.as_millis(),
        profile.warmup.as_millis(),
        target_duration.as_millis(),
        calibrate,
        options.deterministic,
        options.fast,
    );
    if let Some(budget) = options.time_budget {
        let _ = write!(out, "{}", budget.as_millis());
    } else {
        out.push_str("null");
    }
    out.push(',');

    // add result rows
    out.push_str("\"rows\":[");
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        let _ = write!(
            out,
            "\"category\":\"{}\",\"name\":\"{}\",\"full_name\":\"{}\",\"mops\":{:.3},\"ns_per_op\":{:.3},\"min_mops\":{:.3},\"max_mops\":{:.3},\"mir\":{},\"lowered\":{},\"calls\":{},\"stack\":{},\"alloc\":{},\"gc\":{:.4},\"freed\":{:.4},\"branches\":{},\"mem\":{},\"range\":\"{}\",\"scale\":\"{}\",",
            json_escape(row.category),
            json_escape(row.name),
            json_escape(&row.full_name),
            row.mops,
            row.ns_per_op,
            row.min_mops,
            row.max_mops,
            row.mir_instructions,
            row.lowered_instructions,
            row.calls,
            row.max_stack_depth,
            row.heap_allocations,
            row.avg_gc_collections,
            row.avg_gc_freed_cells,
            row.branches,
            row.memory_ops,
            json_escape(&row.range_label),
            json_escape(&row.scale_label),
        );
        if let Some(profile) = row.instruction_profile.as_ref() {
            let _ = write!(
                out,
                "\"instruction_profile\":\"{}\",\"perf_branch_misses\":",
                json_escape(profile)
            );
        } else {
            out.push_str("\"instruction_profile\":null,\"perf_branch_misses\":");
        }
        if let Some(perf) = row.perf {
            let miss_rate = if perf.l1_accesses == 0 {
                None
            } else {
                Some(perf.l1_misses as f64 / perf.l1_accesses as f64)
            };
            let _ = write!(
                out,
                "{},\"perf_l1_misses\":{},\"perf_l1_accesses\":{},\"perf_l1_miss_rate\":",
                perf.branch_misses, perf.l1_misses, perf.l1_accesses
            );
            if let Some(rate) = miss_rate {
                let _ = write!(out, "{rate:.6}");
            } else {
                out.push_str("null");
            }
        } else {
            out.push_str(
                "null,\"perf_l1_misses\":null,\"perf_l1_accesses\":null,\"perf_l1_miss_rate\":null",
            );
        }
        out.push('}');
    }
    out.push_str("],");

    // add summary stats
    out.push_str("\"average_mops\":");
    if let Some(avg) = average_mops {
        let _ = write!(out, "{avg:.3}");
    } else {
        out.push_str("null");
    }
    let _ = write!(out, ",\"elapsed_secs\":{:.3}", total_elapsed.as_secs_f64());
    out.push('}');

    // print json payload
    println!("{out}");
}

/// Emit benchmark results as csv.
#[allow(clippy::too_many_arguments)]
fn output_csv(
    rows: &[BenchRow],
    average_mops: Option<f64>,
    total_elapsed: Duration,
    options: &BenchOptions,
    profile: BenchProfile,
    min_duration: Duration,
    target_duration: Duration,
    calibrate: bool,
) {
    // emit csv header
    println!(
        "profile,repeat,min_duration_ms,warmup_ms,target_duration_ms,calibrated,deterministic,fast,time_budget_ms,category,name,full_name,mops,ns_per_op,min_mops,max_mops,mir,lowered,calls,stack,alloc,gc,freed,branches,mem,range,scale,instruction_profile,perf_branch_misses,perf_l1_misses,perf_l1_accesses,perf_l1_miss_rate,elapsed_secs"
    );

    // format shared metadata
    let profile_label = format!("{:?}", profile.kind);
    let repeat = profile.repeat.max(1);
    let min_ms = min_duration.as_millis();
    let warmup_ms = profile.warmup.as_millis();
    let target_ms = target_duration.as_millis();
    let budget_ms = options.time_budget.map(|budget| budget.as_millis());
    let calibrate_label = if calibrate { "true" } else { "false" };
    let deterministic_label = if options.deterministic {
        "true"
    } else {
        "false"
    };
    let fast_label = if options.fast { "true" } else { "false" };

    // print row records
    for row in rows {
        let perf = row.perf;
        let perf_branch = perf
            .map(|p| p.branch_misses.to_string())
            .unwrap_or_default();
        let perf_l1_misses = perf.map(|p| p.l1_misses.to_string()).unwrap_or_default();
        let perf_l1_accesses = perf.map(|p| p.l1_accesses.to_string()).unwrap_or_default();
        let perf_rate = perf.and_then(|p| {
            if p.l1_accesses == 0 {
                None
            } else {
                Some(p.l1_misses as f64 / p.l1_accesses as f64)
            }
        });
        let perf_rate = perf_rate
            .map(|rate| format!("{rate:.6}"))
            .unwrap_or_default();
        let budget_ms = budget_ms.map(|value| value.to_string()).unwrap_or_default();

        println!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{:.3},{:.3},{:.3},{:.3},{},{},{},{},{},{:.4},{:.4},{},{},{},{},{},{},{},{},{}",
            csv_escape(&profile_label),
            repeat,
            min_ms,
            warmup_ms,
            target_ms,
            calibrate_label,
            deterministic_label,
            fast_label,
            budget_ms,
            csv_escape(row.category),
            csv_escape(row.name),
            csv_escape(&row.full_name),
            row.mops,
            row.ns_per_op,
            row.min_mops,
            row.max_mops,
            row.mir_instructions,
            row.lowered_instructions,
            row.calls,
            row.max_stack_depth,
            row.heap_allocations,
            row.avg_gc_collections,
            row.avg_gc_freed_cells,
            row.branches,
            row.memory_ops,
            csv_escape(&row.range_label),
            csv_escape(&row.scale_label),
            row.instruction_profile
                .as_ref()
                .map(|value| csv_escape(value))
                .unwrap_or_default(),
            perf_branch,
            perf_l1_misses,
            perf_l1_accesses,
            perf_rate,
        );
    }

    // emit summary row
    let average_label = average_mops
        .map(|value| format!("{value:.3}"))
        .unwrap_or_default();
    println!(
        "{},{},{},{},{},{},{},{},{},summary,average,,{},,,,,,,,,,,,,,,,,,{:.3}",
        csv_escape(&profile_label),
        repeat,
        min_ms,
        warmup_ms,
        target_ms,
        calibrate_label,
        deterministic_label,
        fast_label,
        budget_ms.map(|value| value.to_string()).unwrap_or_default(),
        average_label,
        total_elapsed.as_secs_f64()
    );
}

/// Build a scale axis with explicit clamps.
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub const fn scale_axis_range(
    name: &'static str,
    arg_index: usize,
    quick: i64,
    standard: i64,
    stress: i64,
    calibrate: bool,
    min: i64,
    max: i64,
) -> ScaleAxis {
    ScaleAxis {
        name,
        arg_index,
        quick,
        standard,
        stress,
        calibrate,
        min,
        max,
    }
}

impl ScaleAxis {
    /// Return the profile value for this axis.
    pub fn value_for(self, profile: BenchProfileKind) -> i64 {
        match profile {
            BenchProfileKind::Quick => self.quick,
            BenchProfileKind::Standard => self.standard,
            BenchProfileKind::Stress => self.stress,
        }
    }
}

impl BenchProfileKind {
    /// Build the default profile settings.
    pub fn defaults(self) -> BenchProfile {
        match self {
            BenchProfileKind::Quick => BenchProfile {
                kind: self,
                repeat: 3,
                min_duration: Duration::from_millis(60),
                warmup: Duration::from_millis(25),
                target_duration: Duration::from_millis(8),
            },
            BenchProfileKind::Standard => BenchProfile {
                kind: self,
                repeat: 5,
                min_duration: Duration::from_millis(200),
                warmup: Duration::from_millis(100),
                target_duration: Duration::from_millis(20),
            },
            BenchProfileKind::Stress => BenchProfile {
                kind: self,
                repeat: 7,
                min_duration: Duration::from_millis(500),
                warmup: Duration::from_millis(200),
                target_duration: Duration::from_millis(50),
            },
        }
    }
}

/// Function that produces a resume value for coroutine benchmarks.
pub type ResumeValueFn = fn(args: &[Value], yield_index: usize, yielded: Value) -> Value;

/// Runner selection for a benchmark program.
#[derive(Clone, Copy, Debug)]
pub enum ProgramRunner {
    /// Execute the entry function to completion.
    Function,
    /// Execute the entry function as a coroutine.
    Coroutine {
        /// Resume value factory for each yield.
        resume_value: ResumeValueFn,
    },
}

/// Benchmark program with metadata for validation and throughput calculation.
#[derive(Debug)]
pub struct Program {
    /// Human readable name.
    pub name: &'static str,
    /// Tags used for filtering.
    pub tags: BenchTags,
    /// MIR source code.
    pub source: &'static str,
    /// Entry point function name.
    pub entry: &'static str,
    /// Expected result for validation.
    pub expected: fn() -> Value,
    /// Default arguments for benchmarking.
    pub default_args: fn(&Isolate) -> Vec<Value>,
    /// Scale axes for this program.
    pub scales: &'static [ScaleAxis],
    /// Runner selection for this program.
    pub runner: ProgramRunner,
}

/// Benchmark options for quick runs.
#[derive(Debug)]
pub struct BenchOptions {
    /// Filter patterns for program names.
    pub filter: Option<Vec<String>>,
    /// Tag filters for program selection.
    pub tags: Option<Vec<String>>,
    /// Output format for benchmark results.
    pub output: BenchOutputFormat,
    /// Benchmark profile settings.
    pub profile: BenchProfile,
    /// Whether to calibrate program scales.
    pub calibrate: bool,
    /// Whether to collect perf counters when supported.
    pub perf_counters: bool,
    /// Whether to collect instruction profile samples.
    pub instruction_profile: bool,
    /// Optional total time budget for benchmark runs.
    pub time_budget: Option<Duration>,
    /// Whether to disable time based scaling for reproducibility.
    pub deterministic: bool,
    /// Whether to disable runtime checks for speed.
    pub fast: bool,
}

#[allow(dead_code)]
impl BenchOptions {
    /// Build benchmark options from explicit values.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        filter: Option<Vec<String>>,
        tags: Option<Vec<String>>,
        output: BenchOutputFormat,
        profile: BenchProfile,
        calibrate: bool,
        perf_counters: bool,
        instruction_profile: bool,
        time_budget: Option<Duration>,
        deterministic: bool,
        fast: bool,
    ) -> Self {
        // assemble options
        Self {
            filter,
            tags,
            output,
            profile,
            calibrate,
            perf_counters,
            instruction_profile,
            time_budget,
            deterministic,
            fast,
        }
    }

    /// Build default quick benchmark options.
    pub fn quick(filter: Option<&[&str]>, tags: Option<&[&str]>) -> Self {
        // normalize filter patterns
        let filter = filter.map(|patterns| patterns.iter().map(|pat| (*pat).to_string()).collect());
        let tags = tags.map(|patterns| patterns.iter().map(|pat| (*pat).to_string()).collect());

        // use quick defaults
        Self {
            filter,
            tags,
            output: BenchOutputFormat::Table,
            profile: BenchProfileKind::Quick.defaults(),
            calibrate: true,
            perf_counters: false,
            instruction_profile: false,
            time_budget: Some(QUICK_TIME_BUDGET),
            deterministic: false,
            fast: false,
        }
    }
}

/// Single benchmark timing sample.
struct BenchSample {
    /// Measured throughput in mops per second.
    mops: f64,
    /// Iterations performed (retained for debugging).
    iterations: u32,
    /// Garbage collection runs during the sample.
    gc_collections: u64,
    /// Freed heap cells during the sample.
    gc_freed_cells: u64,
}

/// Summary of benchmark samples.
struct BenchSummary {
    /// Median mops for the sample set.
    median_mops: f64,
    /// Minimum mops observed.
    min_mops: f64,
    /// Maximum mops observed.
    max_mops: f64,
    /// Average gc collections per iteration.
    avg_gc_collections: f64,
    /// Average freed heap cells per iteration.
    avg_gc_freed_cells: f64,
}

/// Perf counters collected during a benchmark run.
#[derive(Clone, Copy, Debug)]
struct BenchPerf {
    /// Branch misses recorded for the run.
    branch_misses: u64,
    /// L1 data cache misses recorded for the run.
    l1_misses: u64,
    /// L1 data cache access count recorded for the run.
    l1_accesses: u64,
}

/// Row data for a single benchmark program.
struct BenchRow {
    /// Program category name.
    category: &'static str,
    /// Program short name.
    name: &'static str,
    /// Program name including category.
    full_name: String,
    /// Median mops for the run.
    mops: f64,
    /// Median nanoseconds per MIR instruction.
    ns_per_op: f64,
    /// Min mops for the run.
    min_mops: f64,
    /// Max mops for the run.
    max_mops: f64,
    /// Total MIR instructions for a single invocation.
    mir_instructions: u64,
    /// Total lowered instructions for a single invocation.
    lowered_instructions: u64,
    /// Function calls for a single invocation.
    calls: u64,
    /// Maximum stack depth for a single invocation.
    max_stack_depth: usize,
    /// Heap allocations for a single invocation.
    heap_allocations: u64,
    /// Average gc collections per iteration.
    avg_gc_collections: f64,
    /// Average freed heap cells per iteration.
    avg_gc_freed_cells: f64,
    /// Branch count for a single invocation.
    branches: u64,
    /// Memory ops for a single invocation.
    memory_ops: u64,
    /// Range label for mops.
    range_label: String,
    /// Scale label for program inputs.
    scale_label: String,
    /// Optional instruction profile report.
    instruction_profile: Option<String>,
    /// Optional perf counters.
    perf: Option<BenchPerf>,
}

/// Row data for a stats report.
struct StatsRow {
    /// Program name including category.
    full_name: String,
    /// Total MIR instructions for a single invocation.
    mir_instructions: u64,
    /// Total lowered instructions for a single invocation.
    lowered_instructions: u64,
    /// Function calls for a single invocation.
    calls: u64,
    /// Maximum stack depth for a single invocation.
    max_stack_depth: usize,
    /// Heap allocations for a single invocation.
    heap_allocations: u64,
    /// Scale label for program inputs.
    scale_label: String,
}

/// Table column widths for stats output.
struct StatsWidths {
    /// Width for the program column.
    program: usize,
    /// Width for the mir column.
    mir: usize,
    /// Width for the lowered column.
    lowered: usize,
    /// Width for the calls column.
    calls: usize,
    /// Width for the stack column.
    stack: usize,
    /// Width for the heap column.
    heap: usize,
    /// Width for the scale column.
    scale: usize,
}

impl StatsWidths {
    /// Build base widths from header labels.
    fn new() -> Self {
        // seed widths from headers
        let program = "Program".len();
        let mir = "MIR".len();
        let lowered = "Lowered".len();
        let calls = "Calls".len();
        let stack = "Stack".len();
        let heap = "Heap".len();
        let scale = "Scale".len();

        // assemble widths
        Self {
            program,
            mir,
            lowered,
            calls,
            stack,
            heap,
            scale,
        }
    }

    /// Expand widths to fit row data.
    fn update_with_row(&mut self, row: &StatsRow) {
        // compute row widths
        let program = row.full_name.len();
        let mir = row.mir_instructions.to_string().len();
        let lowered = row.lowered_instructions.to_string().len();
        let calls = row.calls.to_string().len();
        let stack = row.max_stack_depth.to_string().len();
        let heap = row.heap_allocations.to_string().len();
        let scale = row.scale_label.len();

        // update stored widths
        self.program = self.program.max(program);
        self.mir = self.mir.max(mir);
        self.lowered = self.lowered.max(lowered);
        self.calls = self.calls.max(calls);
        self.stack = self.stack.max(stack);
        self.heap = self.heap.max(heap);
        self.scale = self.scale.max(scale);
    }

    /// Return the full table width in characters.
    fn total_width(&self) -> usize {
        // sum column widths
        let columns = self.program
            + self.mir
            + self.lowered
            + self.calls
            + self.stack
            + self.heap
            + self.scale;

        // add column gaps
        let gaps = COLUMN_GAP.len() * (STATS_COLUMN_COUNT - 1);

        // return total width
        columns + gaps
    }
}

/// Summarize benchmark samples with median and range.
fn summarize_samples(samples: &mut [BenchSample]) -> BenchSummary {
    // handle empty samples defensively
    if samples.is_empty() {
        return BenchSummary {
            median_mops: 0.0,
            min_mops: 0.0,
            max_mops: 0.0,
            avg_gc_collections: 0.0,
            avg_gc_freed_cells: 0.0,
        };
    }

    // sort by mops for median selection
    samples.sort_by(|a, b| a.mops.partial_cmp(&b.mops).unwrap_or(Ordering::Equal));

    // compute min and max
    let min_mops = samples.first().map(|s| s.mops).unwrap_or(0.0);
    let max_mops = samples.last().map(|s| s.mops).unwrap_or(0.0);

    // compute median mops
    let median_index = samples.len() / 2;
    let median_mops = if samples.len() % 2 == 1 {
        samples[median_index].mops
    } else {
        let lower = samples[median_index - 1].mops;
        let upper = samples[median_index].mops;
        (lower + upper) * 0.5
    };

    // compute average gc stats
    let total_iterations: u64 = samples.iter().map(|s| s.iterations as u64).sum();
    let total_gc_collections: u64 = samples.iter().map(|s| s.gc_collections).sum();
    let total_gc_freed: u64 = samples.iter().map(|s| s.gc_freed_cells).sum();
    let denom = total_iterations.max(1) as f64;
    let avg_gc_collections = total_gc_collections as f64 / denom;
    let avg_gc_freed_cells = total_gc_freed as f64 / denom;

    // return summary
    BenchSummary {
        median_mops,
        min_mops,
        max_mops,
        avg_gc_collections,
        avg_gc_freed_cells,
    }
}

/// Scale a duration by the given ratio, clamping to at least one millisecond.
fn scale_duration(duration: Duration, scale: f64) -> Duration {
    // scale duration in milliseconds
    let duration_ms = duration.as_secs_f64() * 1000.0;
    let scaled_ms = (duration_ms * scale).round().max(1.0);

    // return scaled duration
    Duration::from_millis(scaled_ms as u64)
}

/// Apply a total time budget by scaling per program durations.
fn apply_time_budget(
    profile: BenchProfile,
    program_count: usize,
    budget: Duration,
    calibrate: bool,
) -> BenchProfile {
    // skip scaling for empty budgets
    if program_count == 0 || budget.is_zero() {
        return profile;
    }

    // estimate per program time cost in milliseconds
    let repeat = profile.repeat.max(1) as f64;
    let warmup_ms = profile.warmup.as_secs_f64() * 1000.0;
    let min_ms = profile.min_duration.as_secs_f64() * 1000.0;
    let target_ms = profile.target_duration.as_secs_f64() * 1000.0;
    let calibration_ms = if calibrate { target_ms * 2.0 } else { 0.0 };
    let per_program_ms = warmup_ms + min_ms * repeat + calibration_ms;
    let total_ms = per_program_ms * program_count as f64;
    let budget_ms = budget.as_secs_f64() * 1000.0;

    // skip scaling if within budget
    if total_ms <= budget_ms || per_program_ms == 0.0 {
        return profile;
    }

    // scale durations to fit within budget
    let scale = (budget_ms / total_ms).min(1.0);
    let mut scaled = profile;
    scaled.warmup = scale_duration(profile.warmup, scale);
    scaled.min_duration = scale_duration(profile.min_duration, scale);
    scaled.target_duration = scale_duration(profile.target_duration, scale);

    // return scaled profile
    scaled
}

/// Program entry with category metadata.
struct ProgramEntry {
    /// Category name for display.
    category: &'static str,
    /// Program metadata.
    program: &'static Program,
}

/// Gather all benchmark programs with categories.
fn program_entries() -> Vec<ProgramEntry> {
    let mut entries = Vec::new();

    for program in dispatch::ALL {
        entries.push(ProgramEntry {
            category: "dispatch",
            program,
        });
    }

    for program in arithmetic::ALL {
        entries.push(ProgramEntry {
            category: "arithmetic",
            program,
        });
    }

    for program in calls::ALL {
        entries.push(ProgramEntry {
            category: "calls",
            program,
        });
    }

    for program in memory::ALL {
        entries.push(ProgramEntry {
            category: "memory",
            program,
        });
    }

    for program in intrinsics::ALL {
        entries.push(ProgramEntry {
            category: "intrinsics",
            program,
        });
    }

    entries
}

/// Update arguments with profile scale values.
fn apply_profile_scales(program: &Program, args: &mut [Value], profile: BenchProfileKind) {
    for axis in program.scales {
        if axis.arg_index < args.len() {
            args[axis.arg_index] = Value::int64(axis.value_for(profile));
        }
    }
}

/// Return the first calibratable axis.
fn calibrate_axis(program: &Program) -> Option<ScaleAxis> {
    program.scales.iter().copied().find(|axis| axis.calibrate)
}

/// Read an int argument for reporting.
fn read_int_arg(args: &[Value], index: usize) -> Option<i64> {
    let value = args.get(index)?;
    value.as_int()
}

/// Build a label for scale values.
fn scale_label(program: &Program, args: &[Value]) -> String {
    if program.scales.is_empty() {
        return "-".to_string();
    }

    let mut parts = Vec::new();
    for axis in program.scales {
        if let Some(value) = read_int_arg(args, axis.arg_index) {
            parts.push(format!("{}={value}", axis.name));
        }
    }

    if parts.is_empty() {
        return "-".to_string();
    }

    parts.join(" ")
}

/// Calibrate the primary scale axis to hit a target duration.
fn calibrate_scale(
    program: &Program,
    isolate: &mut Isolate,
    heap: &mut Heap,
    shared: &mut SharedSpace,
    entry_id: mir::LocalNodeId<mir::Function>,
    args: &mut [Value],
    axis: ScaleAxis,
    target_duration: Duration,
    needs_gc: bool,
) {
    if target_duration.is_zero() {
        return;
    }

    let Some(current) = read_int_arg(args, axis.arg_index) else {
        return;
    };

    // sample duration for the current scale
    let mut sample = Duration::ZERO;
    for _ in 0..2 {
        if needs_gc {
            let _ = isolate
                .collect_garbage(heap, shared)
                .expect("failed to collect garbage");
        }
        let start = Instant::now();
        let _ = program.run_or_panic(isolate, heap, shared, entry_id, args);
        sample = sample.max(start.elapsed());
    }

    if sample.is_zero() {
        return;
    }

    // scale value proportionally and clamp
    let scaled = (current as f64 * target_duration.as_secs_f64() / sample.as_secs_f64()).round();
    let mut next = scaled as i64;
    if next < axis.min {
        next = axis.min;
    }
    if next > axis.max {
        next = axis.max;
    }

    if next != current {
        args[axis.arg_index] = Value::int64(next);
    }
}

/// Check if a program passes filter options.
fn matches_filters(entry: &ProgramEntry, options: &BenchOptions) -> bool {
    let full_name = format!("{}/{}", entry.category, entry.program.name);

    if let Some(patterns) = options.filter.as_ref()
        && !patterns.iter().any(|pat| full_name.contains(pat))
    {
        return false;
    }

    if let Some(tags) = options.tags.as_ref() {
        let mut matched = false;
        for tag in tags {
            if tag.eq_ignore_ascii_case(entry.category)
                || entry
                    .program
                    .tags
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case(tag))
            {
                matched = true;
                break;
            }
        }
        if !matched {
            return false;
        }
    }

    true
}

/// Run a coroutine program to completion.
fn run_coroutine(
    isolate: &mut Isolate,
    heap: &mut Heap,
    shared: &mut SharedSpace,
    entry_id: mir::LocalNodeId<mir::Function>,
    args: &[Value],
    resume_value: ResumeValueFn,
) -> RuntimeResult<ExecutionOutput> {
    // start execution
    let mut memory = MemoryContext::new(heap, shared);
    let mut outcome = isolate.run_function_yielding(&mut memory, entry_id, args)?;
    let mut yield_index = 0usize;

    // continue until completion
    loop {
        // handle completion or yield
        match outcome {
            // return on completion
            ExecutionOutcome::Completed { output } => return Ok(output),
            // resume after suspension
            ExecutionOutcome::Yielded { yielded } => {
                // compute resume value
                let resume = resume_value(args, yield_index, yielded.value);
                yield_index += 1;
                // resume execution
                let mut memory = MemoryContext::new(heap, shared);
                outcome = isolate.resume(&mut memory, yielded.continuation, resume)?;
            }
        }
    }
}

impl Program {
    /// Create an isolate for this program.
    pub fn isolate(&self) -> Isolate {
        let (tree, strings) = Parser::parse(FileId::new(0), self.source, ParseOptions::default())
            .validate()
            .unwrap_or_else(|e| panic!("failed to parse '{}': {}", self.name, e.message));

        // relax runtime limits for benchmarks
        let mut options = IsolateOptions::unbounded();
        options.limits.max_stack_depth = 4096;
        options.limits.max_managed_allocations = 5_000_000;
        options.limits.max_raw_allocations = 5_000_000;

        Isolate::build_with_options(tree, strings, options)
            .unwrap_or_else(|error| panic!("failed to initialize isolate: {error}"))
    }

    /// Create an isolate with benchmark options applied.
    pub fn isolate_with_options(&self, bench_options: &BenchOptions) -> Isolate {
        let (tree, strings) = Parser::parse(FileId::new(0), self.source, ParseOptions::default())
            .validate()
            .unwrap_or_else(|e| panic!("failed to parse '{}': {}", self.name, e.message));

        // relax runtime limits for benchmarks
        let mut options = IsolateOptions::unbounded();
        options.limits.max_stack_depth = 4096;
        options.limits.max_managed_allocations = 5_000_000;
        options.limits.max_raw_allocations = 5_000_000;

        // disable runtime checks for fast benchmarking
        if bench_options.fast {
            options.checks.bounds = CheckPolicy::Never;
            options.checks.null = CheckPolicy::Never;
            options.checks.enforce_reference_kinds = false;
            options.checks.enforce_reference_mutability = false;
        }

        Isolate::build_with_options(tree, strings, options)
            .unwrap_or_else(|error| panic!("failed to initialize isolate: {error}"))
    }

    /// Resolve the entry function id for this program.
    pub fn entry_id(&self, isolate: &Isolate) -> mir::LocalNodeId<mir::Function> {
        function_id_by_name(isolate, self.entry)
    }

    /// Run the program once using the configured runner.
    pub fn run_once(
        &self,
        isolate: &mut Isolate,
        heap: &mut Heap,
        shared: &mut SharedSpace,
        entry_id: mir::LocalNodeId<mir::Function>,
        args: &[Value],
    ) -> RuntimeResult<ExecutionOutput> {
        // dispatch to the selected runner
        match self.runner {
            ProgramRunner::Function => {
                let mut memory = MemoryContext::new(heap, shared);
                isolate.run_function(&mut memory, entry_id, args)
            }
            ProgramRunner::Coroutine { resume_value } => {
                run_coroutine(isolate, heap, shared, entry_id, args, resume_value)
            }
        }
    }

    /// Run the program once and panic on failure.
    pub fn run_or_panic(
        &self,
        isolate: &mut Isolate,
        heap: &mut Heap,
        shared: &mut SharedSpace,
        entry_id: mir::LocalNodeId<mir::Function>,
        args: &[Value],
    ) -> ExecutionOutput {
        // execute program
        self.run_once(isolate, heap, shared, entry_id, args)
            .unwrap_or_else(|e| panic!("'{}' failed: {:?}", self.name, e))
    }

    /// Build arguments for a given profile.
    pub fn args_for_profile(&self, isolate: &Isolate, profile: BenchProfileKind) -> Vec<Value> {
        // build base args
        let mut args = (self.default_args)(isolate);

        // apply scale axes
        apply_profile_scales(self, &mut args, profile);

        // return args
        args
    }

    /// Get actual instruction count by running once.
    #[allow(dead_code)]
    pub fn actual_instruction_count(&self, args: &[Value]) -> u64 {
        let mut isolate = self.isolate();
        let mut heap = Heap::default();
        let mut shared = SharedSpace::default();
        let entry_id = self.entry_id(&isolate);
        let result = self.run_or_panic(&mut isolate, &mut heap, &mut shared, entry_id, args);
        result.stats.lowered_instructions_executed
    }

    /// Validate that the program produces expected output.
    #[allow(dead_code)]
    pub fn validate(&self) {
        let expected = (self.expected)();

        // build isolate and arguments
        let mut isolate = self.isolate();
        let mut heap = Heap::default();
        let mut shared = SharedSpace::default();
        let args = (self.default_args)(&isolate);
        let entry_id = self.entry_id(&isolate);
        let result = self.run_or_panic(&mut isolate, &mut heap, &mut shared, entry_id, &args);

        assert_eq!(
            result.value, expected,
            "'{}': expected {:?}, got {:?}",
            self.name, expected, result.value
        );
    }
}

/// Validate all benchmark programs produce expected outputs.
#[allow(dead_code)]
pub fn validate_all() {
    for p in dispatch::ALL {
        p.validate();
    }

    for p in arithmetic::ALL {
        p.validate();
    }

    for p in calls::ALL {
        p.validate();
    }

    for p in memory::ALL {
        p.validate();
    }

    for p in intrinsics::ALL {
        p.validate();
    }
}

// ansi color codes
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const RED: &str = "\x1b[31m";
const COLUMN_GAP: &str = "  ";
const COLUMN_COUNT: usize = 14;
const STATS_COLUMN_COUNT: usize = 7;
const MIN_PROGRAM_WIDTH: usize = 28;
const MIN_MOPS_WIDTH: usize = 7;
const MIN_NSOP_WIDTH: usize = 6;
const MIN_MIR_WIDTH: usize = 8;
const MIN_THREADED_WIDTH: usize = 8;
const MIN_CALLS_WIDTH: usize = 6;
const MIN_STACK_WIDTH: usize = 4;
const MIN_ALLOC_WIDTH: usize = 5;
const MIN_GC_WIDTH: usize = 4;
const MIN_FREED_WIDTH: usize = 6;
const MIN_BRANCHES_WIDTH: usize = 5;
const MIN_MEM_WIDTH: usize = 5;
const MIN_RANGE_WIDTH: usize = 11;
const MIN_SCALE_WIDTH: usize = 18;

/// Table column widths for benchmark output.
struct TableWidths {
    /// Width for the program column.
    program: usize,
    /// Width for the mops column.
    mops: usize,
    /// Width for the ns/op column.
    ns_per_op: usize,
    /// Width for the mir column.
    mir: usize,
    /// Width for the lowered column.
    lowered: usize,
    /// Width for the calls column.
    calls: usize,
    /// Width for the stack column.
    stack: usize,
    /// Width for the alloc column.
    alloc: usize,
    /// Width for the gc column.
    gc: usize,
    /// Width for the freed column.
    freed: usize,
    /// Width for the branch column.
    branches: usize,
    /// Width for the mem column.
    mem: usize,
    /// Width for the range column.
    range: usize,
    /// Width for the scale column.
    scale: usize,
}

impl TableWidths {
    /// Build base widths from header labels.
    fn new() -> Self {
        // seed widths from headers
        let program = MIN_PROGRAM_WIDTH.max("Program".len());
        let mops = MIN_MOPS_WIDTH.max("Mops/s".len());
        let ns_per_op = MIN_NSOP_WIDTH.max("ns/op".len());
        let mir = MIN_MIR_WIDTH.max("MIR".len());
        let lowered = MIN_THREADED_WIDTH.max("Lowered".len());
        let calls = MIN_CALLS_WIDTH.max("Calls".len());
        let stack = MIN_STACK_WIDTH.max("Stk".len());
        let alloc = MIN_ALLOC_WIDTH.max("Alloc".len());
        let gc = MIN_GC_WIDTH.max("GC".len());
        let freed = MIN_FREED_WIDTH.max("Freed".len());
        let branches = MIN_BRANCHES_WIDTH.max("Br".len());
        let mem = MIN_MEM_WIDTH.max("Mem".len());
        let range = MIN_RANGE_WIDTH.max("Range".len());
        let scale = MIN_SCALE_WIDTH.max("Scale".len());

        // assemble widths
        Self {
            program,
            mops,
            ns_per_op,
            mir,
            lowered,
            calls,
            stack,
            alloc,
            gc,
            freed,
            branches,
            mem,
            range,
            scale,
        }
    }

    /// Expand widths to fit row data.
    fn update_with_row(&mut self, row: &BenchRow) {
        // format row values
        let program = row.full_name.len();
        let mops = format!("{:.1}", row.mops).len();
        let ns_per_op = format!("{:.2}", row.ns_per_op).len();
        let mir = row.mir_instructions.to_string().len();
        let lowered = row.lowered_instructions.to_string().len();
        let calls = row.calls.to_string().len();
        let stack = row.max_stack_depth.to_string().len();
        let alloc = row.heap_allocations.to_string().len();
        let gc = format!("{:.1}", row.avg_gc_collections).len();
        let freed = format!("{:.0}", row.avg_gc_freed_cells).len();
        let branches = row.branches.to_string().len();
        let mem = row.memory_ops.to_string().len();
        let range = row.range_label.len();
        let scale = row.scale_label.len();

        // update stored widths
        self.program = self.program.max(program);
        self.mops = self.mops.max(mops);
        self.ns_per_op = self.ns_per_op.max(ns_per_op);
        self.mir = self.mir.max(mir);
        self.lowered = self.lowered.max(lowered);
        self.calls = self.calls.max(calls);
        self.stack = self.stack.max(stack);
        self.alloc = self.alloc.max(alloc);
        self.gc = self.gc.max(gc);
        self.freed = self.freed.max(freed);
        self.branches = self.branches.max(branches);
        self.mem = self.mem.max(mem);
        self.range = self.range.max(range);
        self.scale = self.scale.max(scale);
    }

    /// Return the full table width in characters.
    fn total_width(&self) -> usize {
        // sum column widths
        let columns = self.program
            + self.mops
            + self.ns_per_op
            + self.mir
            + self.lowered
            + self.calls
            + self.stack
            + self.alloc
            + self.gc
            + self.freed
            + self.branches
            + self.mem
            + self.range
            + self.scale;

        // add column gaps
        let gaps = COLUMN_GAP.len() * (COLUMN_COUNT - 1);
        columns + gaps
    }
}

/// Compute table widths for all rows.
fn table_widths(rows: &[BenchRow]) -> TableWidths {
    // seed widths with header defaults
    let mut widths = TableWidths::new();

    // expand based on row values
    for row in rows {
        widths.update_with_row(row);
    }

    // return widths
    widths
}

/// Emit benchmark results as a formatted table.
#[allow(clippy::too_many_arguments)]
fn output_table(
    rows: &[BenchRow],
    average_mops: Option<f64>,
    total_elapsed: Duration,
    options: &BenchOptions,
    profile: BenchProfile,
    repeat: u32,
    min_duration: Duration,
    target_duration: Duration,
    calibrate: bool,
) {
    // compute column widths
    let widths = table_widths(rows);
    let table_width = widths.total_width();

    // print header row
    println!();
    println!(
        "{BOLD}{:<program_width$}{RESET}{gap}{:>mops_width$}{gap}{:>ns_per_op_width$}{gap}{:>mir_width$}{gap}{:>lowered_width$}{gap}{:>calls_width$}{gap}{:>stack_width$}{gap}{:>alloc_width$}{gap}{:>gc_width$}{gap}{:>freed_width$}{gap}{:>branches_width$}{gap}{:>mem_width$}{gap}{:>range_width$}{gap}{:<scale_width$}{RESET}",
        "Program",
        "Mops/s",
        "ns/op",
        "MIR",
        "Lowered",
        "Calls",
        "Stk",
        "Alloc",
        "GC",
        "Freed",
        "Br",
        "Mem",
        "Range",
        "Scale",
        program_width = widths.program,
        mops_width = widths.mops,
        ns_per_op_width = widths.ns_per_op,
        mir_width = widths.mir,
        lowered_width = widths.lowered,
        calls_width = widths.calls,
        stack_width = widths.stack,
        alloc_width = widths.alloc,
        gc_width = widths.gc,
        freed_width = widths.freed,
        branches_width = widths.branches,
        mem_width = widths.mem,
        range_width = widths.range,
        scale_width = widths.scale,
        gap = COLUMN_GAP,
    );

    // print profile metadata
    if repeat > 1 {
        println!(
            "{DIM}profile {:?} · median of {repeat} samples · min {:.0}ms · warmup {:.0}ms · target {:.0}ms{RESET}",
            profile.kind,
            min_duration.as_secs_f64() * 1000.0,
            profile.warmup.as_secs_f64() * 1000.0,
            target_duration.as_secs_f64() * 1000.0
        );
    }

    // print budget metadata
    if let Some(budget) = options.time_budget {
        let mode = if calibrate { "calibrated" } else { "fixed" };
        let determinism = if options.deterministic {
            "deterministic"
        } else {
            "adaptive"
        };
        println!(
            "{DIM}budget {:.0}ms · {mode} · {determinism}{RESET}",
            budget.as_secs_f64() * 1000.0
        );
    }

    // print header rule
    println!("{DIM}{}{RESET}", "─".repeat(table_width));

    // print rows
    for row in rows {
        // select the throughput color
        let mops_color = if row.mops >= 100.0 {
            GREEN
        } else if row.mops >= 50.0 {
            YELLOW
        } else {
            RED
        };
        println!(
            "{CYAN}{:<program_width$}{RESET}{gap}{mops_color}{:>mops_width$.1}{RESET}{gap}{DIM}{:>ns_per_op_width$.2}{RESET}{gap}{BOLD}{:>mir_width$}{RESET}{gap}{DIM}{:>lowered_width$}{RESET}{gap}{:>calls_width$}{gap}{:>stack_width$}{gap}{:>alloc_width$}{gap}{:>gc_width$.1}{gap}{:>freed_width$.0}{gap}{:>branches_width$}{gap}{:>mem_width$}{gap}{:>range_width$}{gap}{:<scale_width$}{RESET}",
            row.full_name,
            row.mops,
            row.ns_per_op,
            row.mir_instructions,
            row.lowered_instructions,
            row.calls,
            row.max_stack_depth,
            row.heap_allocations,
            row.avg_gc_collections,
            row.avg_gc_freed_cells,
            row.branches,
            row.memory_ops,
            row.range_label,
            row.scale_label,
            program_width = widths.program,
            mops_width = widths.mops,
            ns_per_op_width = widths.ns_per_op,
            mir_width = widths.mir,
            lowered_width = widths.lowered,
            calls_width = widths.calls,
            stack_width = widths.stack,
            alloc_width = widths.alloc,
            gc_width = widths.gc,
            freed_width = widths.freed,
            branches_width = widths.branches,
            mem_width = widths.mem,
            range_width = widths.range,
            scale_width = widths.scale,
            gap = COLUMN_GAP,
            mops_color = mops_color,
        );

        // print perf counters when available
        if let Some(perf) = row.perf {
            let miss_rate = if perf.l1_accesses == 0 {
                0.0
            } else {
                perf.l1_misses as f64 / perf.l1_accesses as f64
            };
            println!(
                "{DIM}  perf: branch_miss={} l1_miss={} ({:.2}%) {RESET}",
                perf.branch_misses,
                perf.l1_misses,
                miss_rate * 100.0
            );
        }
        if let Some(profile) = row.instruction_profile.as_deref() {
            println!("{DIM}  inst: {profile}{RESET}");
        }
    }

    // print summary line
    println!("{DIM}{}{RESET}", "─".repeat(table_width));
    if let Some(avg) = average_mops {
        // compute summary alignment
        let elapsed_label = format!("in {:.2}s", total_elapsed.as_secs_f64());
        let gaps = COLUMN_GAP.len() * 3;
        let prefix = widths.program + widths.mops + widths.ns_per_op + gaps;
        let padding = table_width
            .saturating_sub(prefix)
            .saturating_sub(elapsed_label.len())
            .max(1);
        let average_ns_per_op = if avg > 0.0 { 1000.0 / avg } else { 0.0 };

        // print summary row
        println!(
            "{BOLD}{:<program_width$}{RESET}{gap}{BOLD}{:>mops_width$.1}{RESET}{gap}{DIM}{:>ns_per_op_width$.2}{RESET}{gap}{DIM}{:>padding$}{elapsed_label}{RESET}",
            "Average",
            avg,
            average_ns_per_op,
            "",
            program_width = widths.program,
            mops_width = widths.mops,
            ns_per_op_width = widths.ns_per_op,
            padding = padding,
            gap = COLUMN_GAP,
            elapsed_label = elapsed_label,
        );
    } else {
        // print empty result state
        println!("{DIM}no programs matched filter{RESET}");
    }
    println!();
}

/// Run quick benchmarks with default settings.
#[allow(dead_code)]
pub fn quick_bench(filter: Option<&[&str]>, tags: Option<&[&str]>) {
    // build quick defaults
    let options = BenchOptions::quick(filter, tags);

    // run with defaults
    quick_bench_with_options(&options);
}

/// Run quick benchmarks with configurable timing.
#[allow(dead_code)]
pub fn quick_bench_with_options(options: &BenchOptions) {
    // collect matching programs
    let mut programs = Vec::new();
    for entry in program_entries() {
        if matches_filters(&entry, options) {
            programs.push(entry);
        }
    }

    // derive effective profile settings
    let mut profile = options.profile;
    let calibrate = options.calibrate && !options.deterministic;
    if let Some(budget) = options.time_budget {
        profile = apply_time_budget(profile, programs.len(), budget, calibrate);
    }

    // clamp repeat and duration for safety
    let repeat = profile.repeat.max(1);
    let min_duration = if profile.min_duration.is_zero() {
        Duration::from_millis(1)
    } else {
        profile.min_duration
    };
    let target_duration = if profile.target_duration.is_zero() {
        Duration::from_millis(10)
    } else {
        profile.target_duration
    };

    // prepare output settings
    let output = options.output;
    let mut perf_counters = perf::PerfCounters::new(options.perf_counters);
    if output == BenchOutputFormat::Table && options.perf_counters && !perf_counters.is_supported()
    {
        println!("{DIM}perf counters not supported on this platform{RESET}");
    }

    // track summary stats
    let bench_start = Instant::now();
    let mut total_mops = 0.0;
    let mut rows = Vec::new();

    for entry in programs {
        // build display name
        let full_name = format!("{}/{}", entry.category, entry.program.name);

        // build isolate and arguments
        let mut isolate = entry.program.isolate_with_options(options);
        let mut heap = Heap::default();
        let mut shared = SharedSpace::default();
        let entry_id = entry.program.entry_id(&isolate);
        let mut args = entry.program.args_for_profile(&isolate, profile.kind);

        // run once for stats
        let result =
            entry
                .program
                .run_or_panic(&mut isolate, &mut heap, &mut shared, entry_id, &args);
        let mut stats = result.stats;
        let mut needs_gc = stats.heap_allocations > 0;

        // calibrate main axis if requested
        if calibrate {
            if let Some(axis) = calibrate_axis(entry.program) {
                calibrate_scale(
                    entry.program,
                    &mut isolate,
                    &mut heap,
                    &mut shared,
                    entry_id,
                    &mut args,
                    axis,
                    target_duration,
                    needs_gc,
                );
            }

            let result =
                entry
                    .program
                    .run_or_panic(&mut isolate, &mut heap, &mut shared, entry_id, &args);
            stats = result.stats;
            needs_gc = stats.heap_allocations > 0;
        }

        // disable stats for fast timing runs
        if options.fast {
            isolate.set_collect_stats(false);
        }

        // resolve scale label
        let scale_label = scale_label(entry.program, &args);

        // run warmup loop
        if profile.warmup > Duration::ZERO {
            let warmup_start = Instant::now();
            while warmup_start.elapsed() < profile.warmup {
                if needs_gc {
                    let _ = isolate
                        .collect_garbage(&mut heap, &mut shared)
                        .expect("failed to collect garbage");
                }
                let _ = entry.program.run_or_panic(
                    &mut isolate,
                    &mut heap,
                    &mut shared,
                    entry_id,
                    &args,
                );
            }
        }

        // collect timing samples
        let mut samples = Vec::with_capacity(repeat as usize);
        let mut perf_report = None;
        for sample_index in 0..repeat {
            // run for time budget
            let mut iterations = 0u32;
            let mut gc_collections = 0u64;
            let mut gc_freed_cells = 0u64;

            if perf_report.is_none() && sample_index == 0 {
                perf_counters.start();
            }

            let run_start = Instant::now();
            while run_start.elapsed() < min_duration {
                if needs_gc {
                    let gc = isolate
                        .collect_garbage(&mut heap, &mut shared)
                        .expect("failed to collect garbage");
                    gc_collections += 1;
                    gc_freed_cells += gc.freed_allocations as u64;
                }
                let _ = entry.program.run_or_panic(
                    &mut isolate,
                    &mut heap,
                    &mut shared,
                    entry_id,
                    &args,
                );
                iterations += 1;
            }
            let elapsed = run_start.elapsed();

            if perf_report.is_none() && sample_index == 0 {
                perf_report = perf_counters.stop();
            }

            // compute throughput (based on MIR instructions for fair comparison)
            let total_instructions = stats.mir_instructions_executed * iterations as u64;
            let mops = total_instructions as f64 / elapsed.as_secs_f64() / 1_000_000.0;
            samples.push(BenchSample {
                mops,
                iterations,
                gc_collections,
                gc_freed_cells,
            });
        }

        // summarize samples
        let summary = summarize_samples(&mut samples);
        let range_label = if repeat > 1 {
            format!("{:.1}-{:.1}", summary.min_mops, summary.max_mops)
        } else {
            "-".to_string()
        };

        // capture perf counter data
        let perf = perf_report.map(|report| BenchPerf {
            branch_misses: report.branch_misses,
            l1_misses: report.l1_misses,
            l1_accesses: report.l1_accesses,
        });

        // collect instruction profile samples if requested
        let instruction_profile = if options.instruction_profile {
            #[cfg(feature = "stats")]
            {
                isolate.enable_instruction_profile(INSTRUCTION_PROFILE_INTERVAL);
                isolate.reset_instruction_profile();

                let profile_start = Instant::now();
                while profile_start.elapsed() < min_duration {
                    if needs_gc {
                        let _ = isolate
                            .collect_garbage(&mut heap, &mut shared)
                            .expect("failed to collect garbage");
                    }
                    let _ = entry.program.run_or_panic(
                        &mut isolate,
                        &mut heap,
                        &mut shared,
                        entry_id,
                        &args,
                    );
                }

                let report = isolate.instruction_profile_report(INSTRUCTION_PROFILE_TARGET_PERCENT);
                isolate.clear_instruction_profile();
                report
            }
            #[cfg(not(feature = "stats"))]
            {
                None
            }
        } else {
            None
        };

        // assemble row data
        let mem = stats.loads + stats.stores;
        let ns_per_op = if summary.median_mops > 0.0 {
            1000.0 / summary.median_mops
        } else {
            0.0
        };
        let row = BenchRow {
            category: entry.category,
            name: entry.program.name,
            full_name,
            mops: summary.median_mops,
            ns_per_op,
            min_mops: summary.min_mops,
            max_mops: summary.max_mops,
            mir_instructions: stats.mir_instructions_executed,
            lowered_instructions: stats.lowered_instructions_executed,
            calls: stats.calls_made,
            max_stack_depth: stats.max_stack_depth,
            heap_allocations: stats.heap_allocations,
            avg_gc_collections: summary.avg_gc_collections,
            avg_gc_freed_cells: summary.avg_gc_freed_cells,
            branches: stats.branches,
            memory_ops: mem,
            range_label,
            scale_label,
            instruction_profile,
            perf,
        };

        // update summary totals
        total_mops += row.mops;
        rows.push(row);
    }

    // compute summary stats
    let total_elapsed = bench_start.elapsed();
    let average_mops = if rows.is_empty() {
        None
    } else {
        Some(total_mops / rows.len() as f64)
    };

    // emit output by format
    match output {
        BenchOutputFormat::Table => {
            output_table(
                &rows,
                average_mops,
                total_elapsed,
                options,
                profile,
                repeat,
                min_duration,
                target_duration,
                calibrate,
            );
        }
        BenchOutputFormat::Json => {
            output_json(
                &rows,
                average_mops,
                total_elapsed,
                options,
                profile,
                min_duration,
                target_duration,
                calibrate,
            );
        }
        BenchOutputFormat::Csv => {
            output_csv(
                &rows,
                average_mops,
                total_elapsed,
                options,
                profile,
                min_duration,
                target_duration,
                calibrate,
            );
        }
    }
}

/// Run validation for all programs, printing results like cargo test.
#[allow(dead_code)]
pub fn quick_check(filter: Option<Vec<String>>, tags: Option<Vec<String>>) -> bool {
    let options = BenchOptions::new(
        filter,
        tags,
        BenchOutputFormat::Table,
        BenchProfileKind::Quick.defaults(),
        false,
        false,
        false,
        None,
        false,
        false,
    );

    let programs = program_entries();
    let total = programs
        .iter()
        .filter(|entry| matches_filters(entry, &options))
        .count();
    println!();
    println!("running {total} tests");

    let start = Instant::now();
    let mut passed = 0;
    let mut failed: Vec<(String, String)> = Vec::new();

    for entry in programs {
        if !matches_filters(&entry, &options) {
            continue;
        }

        print!("test {}::{} ... ", entry.category, entry.program.name);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            entry.program.validate();
        }));

        if result.is_ok() {
            println!("{BOLD}{GREEN}ok{RESET}");
            passed += 1;
            continue;
        }

        let full_name = format!("{}::{}", entry.category, entry.program.name);
        let panic_message = panic_message(result.err().unwrap());
        println!("{BOLD}{RED}FAILED{RESET}");
        failed.push((full_name, panic_message));
    }

    let elapsed = start.elapsed();
    let failed_count = failed.len();
    println!();
    if failed_count == 0 {
        print!("test result: {BOLD}{GREEN}ok{RESET}. ");
        println!(
            "{BOLD}{GREEN}{passed} passed{RESET}; {BOLD}0 failed{RESET}; finished in {:.2}s",
            elapsed.as_secs_f64()
        );
        println!();
        return true;
    }

    print!("test result: {BOLD}{RED}FAILED{RESET}. ");
    println!(
        "{BOLD}{GREEN}{passed} passed{RESET}; {BOLD}{RED}{failed_count} failed{RESET}; finished in {:.2}s",
        elapsed.as_secs_f64()
    );
    println!();
    println!("{BOLD}{RED}failures:{RESET}");
    for (full_name, panic_message) in failed {
        println!("    {BOLD}{full_name}{RESET}");
        if !panic_message.is_empty() {
            println!("    {DIM}{panic_message}{RESET}");
        }
    }
    println!();
    false
}

/// Extract the panic payload message when available.
fn panic_message(payload: Box<dyn Any + Send>) -> String {
    // string payload
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }

    // str payload
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }

    // unknown payload
    String::new()
}

/// Print detailed execution stats for all programs (single run).
#[allow(dead_code)]
pub fn print_stats(options: &BenchOptions) {
    // print leading spacing
    println!();

    // collect stats rows
    let mut rows = Vec::new();
    for entry in program_entries() {
        if !matches_filters(&entry, options) {
            continue;
        }

        // build isolate and arguments
        let mut isolate = entry.program.isolate_with_options(options);
        let mut heap = Heap::default();
        let mut shared = SharedSpace::default();
        let args = entry
            .program
            .args_for_profile(&isolate, options.profile.kind);
        let entry_id = entry.program.entry_id(&isolate);

        // run program and capture stats
        let result =
            entry
                .program
                .run_or_panic(&mut isolate, &mut heap, &mut shared, entry_id, &args);
        let stats = result.stats;
        let label = scale_label(entry.program, &args);
        rows.push(StatsRow {
            full_name: format!("{}/{}", entry.category, entry.program.name),
            mir_instructions: stats.mir_instructions_executed,
            lowered_instructions: stats.lowered_instructions_executed,
            calls: stats.calls_made,
            max_stack_depth: stats.max_stack_depth,
            heap_allocations: stats.heap_allocations,
            scale_label: label,
        });
    }

    // compute column widths
    let mut widths = StatsWidths::new();
    for row in &rows {
        widths.update_with_row(row);
    }

    // print header row
    println!(
        "{BOLD}{:<program_width$}{RESET}{gap}{:>mir_width$}{gap}{:>lowered_width$}{gap}{:>calls_width$}{gap}{:>stack_width$}{gap}{:>heap_width$}{gap}{:<scale_width$}{RESET}",
        "Program",
        "MIR",
        "Lowered",
        "Calls",
        "Stack",
        "Heap",
        "Scale",
        program_width = widths.program,
        mir_width = widths.mir,
        lowered_width = widths.lowered,
        calls_width = widths.calls,
        stack_width = widths.stack,
        heap_width = widths.heap,
        scale_width = widths.scale,
        gap = COLUMN_GAP,
    );

    // print header rule
    println!("{DIM}{}{RESET}", "─".repeat(widths.total_width()));

    // print rows
    for row in rows {
        println!(
            "{CYAN}{:<program_width$}{RESET}{gap}{:>mir_width$}{gap}{DIM}{:>lowered_width$}{RESET}{gap}{:>calls_width$}{gap}{:>stack_width$}{gap}{:>heap_width$}{gap}{:<scale_width$}{RESET}",
            row.full_name,
            row.mir_instructions,
            row.lowered_instructions,
            row.calls,
            row.max_stack_depth,
            row.heap_allocations,
            row.scale_label,
            program_width = widths.program,
            mir_width = widths.mir,
            lowered_width = widths.lowered,
            calls_width = widths.calls,
            stack_width = widths.stack,
            heap_width = widths.heap,
            scale_width = widths.scale,
            gap = COLUMN_GAP,
        );
    }
    // print trailing spacing
    println!();
}
