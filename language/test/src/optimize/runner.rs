use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use destack_compiler::{
    CompositePipeline, FunctionPass, FunctionPipeline, FunctionToModuleAdaptor, ModulePass,
    ModulePipeline, OptimizationLevel, Pipeline, PipelineContext, PipelineOptions, PipelineTarget,
    RepeatedPipeline, default_pipeline,
};
use destack_core::{ImmutableStringPool, StringPool};
use destack_heap::{Heap, MemoryContext, SharedSpace, Value};
use destack_mir as mir;
use destack_source::{FileId, ModuleId, PackageId, TargetId};
use destack_vm::diagnostic::RuntimeResult;
use destack_vm::{ExecutionOutcome, ExecutionOutput, Isolate, IsolateOptions};
use mir::parse::ParseOptions;
use serde::Serialize;

use crate::core::{Case, CaseResult, RunContext, RunOptions, Suite};

use destack_test_mirbench as program;

// ansi color codes
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const RED: &str = "\x1b[31m";

// table formatting
const COLUMN_GAP: &str = "  ";

/// All optimization levels exercised by the optimizer bench suite.
const OPTIMIZATION_LEVELS: [OptimizationLevel; 5] = [
    OptimizationLevel::O0,
    OptimizationLevel::O1,
    OptimizationLevel::O2,
    OptimizationLevel::O3,
    OptimizationLevel::O4,
];

/// Build one stable synthetic target id for optimizer fixtures.
fn fixture_target_id(package_id: PackageId, name: &str) -> TargetId {
    TargetId::new(package_id, name)
}

/// Run options for optimizer bench suites.
#[derive(Debug, Clone)]
pub struct OptimizeRunOptions {
    /// Optional optimization level filter.
    pub level_filter: Option<OptimizationLevel>,
    /// Whether to enable diagnostic mismatch isolation.
    pub diagnostic: bool,
    /// Whether to enable matrix pass isolation.
    pub matrix: bool,
    /// Whether to emit trace output during runs.
    pub trace: bool,
    /// Optional instruction limit for execution.
    pub max_instruction_limit: Option<u64>,
    /// Benchmark profile for program arguments.
    pub bench_profile: program::BenchProfileKind,
    /// Number of perf warmup iterations per program.
    pub perf_warmup: u32,
    /// Number of perf samples per program.
    pub perf_samples: u32,
    /// Optional perf report output path.
    pub perf_output: Option<PathBuf>,
    /// Perf report output format.
    pub perf_output_format: PerfOutputFormat,
    /// Enable A/B pipeline comparison.
    pub perf_ab: bool,
    /// Disabled passes for the B pipeline.
    pub perf_ab_disable: Vec<String>,
    /// Allowed passes for the B pipeline.
    pub perf_ab_only: Vec<String>,
    /// Capture per-pass timing data.
    pub perf_pass_timing: bool,
}

impl OptimizeRunOptions {
    /// Return true when a level should run under this configuration.
    fn allows_level(&self, level: OptimizationLevel) -> bool {
        // honor the optional level filter
        match self.level_filter {
            Some(filter) => filter == level,
            None => true,
        }
    }
}

/// Output format for perf reports.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PerfOutputFormat {
    /// Emit a json report.
    Json,
    /// Emit a csv report.
    Csv,
}

/// Summary statistics for timing samples.
#[derive(Debug, Clone)]
struct SampleSummary {
    /// Raw sample durations in milliseconds.
    samples_ms: Vec<f64>,
    /// Mean duration in milliseconds.
    mean_ms: f64,
    /// Median duration in milliseconds.
    median_ms: f64,
    /// Minimum duration in milliseconds.
    min_ms: f64,
    /// Maximum duration in milliseconds.
    max_ms: f64,
}

/// Serializable summary for perf samples.
#[derive(Debug, Clone, Serialize)]
struct PerfSampleStats {
    /// Raw sample durations in milliseconds.
    samples_ms: Vec<f64>,
    /// Mean duration in milliseconds.
    mean_ms: f64,
    /// Median duration in milliseconds.
    median_ms: f64,
    /// Minimum duration in milliseconds.
    min_ms: f64,
    /// Maximum duration in milliseconds.
    max_ms: f64,
}

/// Serializable perf report metadata.
#[derive(Debug, Clone, Serialize)]
struct PerfReportMetadata {
    /// Unix timestamp in seconds.
    timestamp_seconds: u64,
    /// Operating system name.
    os: String,
    /// Architecture name.
    arch: String,
    /// Destack test crate version.
    crate_version: String,
    /// Bench profile name.
    bench_profile: String,
    /// Warmup iterations per program.
    perf_warmup: u32,
    /// Sample count per program.
    perf_samples: u32,
    /// Whether A/B comparison was enabled.
    perf_ab: bool,
    /// Whether per-pass timing was enabled.
    perf_pass_timing: bool,
}

/// Serializable perf report sample.
#[derive(Debug, Clone, Serialize)]
struct PerfReportSample {
    /// Program name.
    name: String,
    /// Pipeline variant label.
    variant: String,
    /// Optimization level label.
    level: String,
    /// Compile time in milliseconds.
    compile_ms: f64,
    /// Optimized runtime statistics.
    runtime: PerfSampleStats,
    /// Baseline runtime statistics.
    baseline_runtime: PerfSampleStats,
    /// Runtime delta in milliseconds.
    runtime_delta_ms: f64,
    /// Runtime speedup factor.
    runtime_speedup: f64,
    /// MIR instruction count after optimization.
    mir_instructions: u64,
    /// Baseline MIR instruction count.
    baseline_mir_instructions: u64,
    /// MIR instruction delta.
    mir_instruction_delta: i64,
    /// MIR text size after optimization.
    mir_bytes: u64,
    /// Baseline MIR text size.
    baseline_mir_bytes: u64,
    /// MIR text size delta.
    mir_bytes_delta: i64,
    /// Lowered instructions executed by the VM.
    lowered_instructions: u64,
    /// Baseline lowered instructions executed by the VM.
    baseline_lowered_instructions: u64,
    /// Lowered instruction delta.
    lowered_delta: i64,
    /// Pass timing samples when enabled.
    pass_timings: Vec<PerfPassTiming>,
}

/// Serializable perf report.
#[derive(Debug, Clone, Serialize)]
struct PerfReport {
    /// Metadata describing the run.
    metadata: PerfReportMetadata,
    /// Per program perf samples.
    samples: Vec<PerfReportSample>,
}

/// Table row for perf reporting.
struct PerfTableRow {
    /// Table cells.
    cells: Vec<String>,
}

/// Table column alignment.
#[derive(Clone, Copy)]
enum ColumnAlign {
    /// Left aligned column.
    Left,
    /// Right aligned column.
    Right,
}

/// Filter configuration for pass selection.
#[derive(Debug, Clone, Default)]
struct PassFilter {
    /// Passes that should be disabled.
    disabled: HashSet<String>,
    /// Passes that should be exclusively enabled.
    only: Option<HashSet<String>>,
    /// Force the filtered pass runner even when no filtering is configured.
    force_run: bool,
}

impl PassFilter {
    /// Create a new filter from allow/deny lists.
    fn new(disabled: &[String], only: &[String]) -> Self {
        let disabled = disabled.iter().cloned().collect::<HashSet<_>>();
        let only = if only.is_empty() {
            None
        } else {
            Some(only.iter().cloned().collect::<HashSet<_>>())
        };

        Self {
            disabled,
            only,
            force_run: false,
        }
    }

    /// Return true when no filtering is configured.
    fn is_noop(&self) -> bool {
        !self.force_run && self.disabled.is_empty() && self.only.is_none()
    }

    /// Return true when a pass should run.
    fn allows(&self, pass_name: &str) -> bool {
        if let Some(only) = &self.only {
            return only.contains(pass_name);
        }

        !self.disabled.contains(pass_name)
    }

    /// Force the filtered pass runner even when no passes are filtered.
    fn force(mut self) -> Self {
        self.force_run = true;
        self
    }
}

/// Pipeline variant configuration for perf runs.
#[derive(Debug, Clone)]
struct PipelineVariant {
    /// Variant label.
    label: String,
    /// Pass filter for this variant.
    filter: PassFilter,
}

/// Perf sample data captured for a single program and level.
#[derive(Debug, Clone)]
struct PerfSample {
    /// Program name.
    name: String,
    /// Pipeline variant label.
    variant: String,
    /// Optimization level.
    level: OptimizationLevel,
    /// Compile time in milliseconds.
    compile_ms: f64,
    /// Runtime summary for optimized output.
    runtime: SampleSummary,
    /// Runtime summary for the baseline output.
    baseline_runtime: SampleSummary,
    /// MIR instruction count after optimization.
    mir_instructions: u64,
    /// Baseline MIR instruction count.
    baseline_mir_instructions: u64,
    /// MIR text size after optimization.
    mir_bytes: u64,
    /// Baseline MIR text size.
    baseline_mir_bytes: u64,
    /// Lowered instructions executed by the VM.
    lowered_instructions: u64,
    /// Baseline lowered instructions executed by the VM.
    baseline_lowered_instructions: u64,
    /// Pass timing samples when enabled.
    pass_timings: Vec<PassTimingEntry>,
}

impl SampleSummary {
    /// Build a summary from timing samples.
    fn from_samples(mut samples_ms: Vec<f64>) -> Self {
        // sort to compute median and bounds
        samples_ms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let count = samples_ms.len() as f64;
        let mean_ms = if samples_ms.is_empty() {
            0.0
        } else {
            samples_ms.iter().sum::<f64>() / count
        };
        let median_ms = if samples_ms.is_empty() {
            0.0
        } else if samples_ms.len().is_multiple_of(2) {
            let upper = samples_ms.len() / 2;
            let lower = upper - 1;
            (samples_ms[lower] + samples_ms[upper]) * 0.5
        } else {
            samples_ms[samples_ms.len() / 2]
        };
        let min_ms = samples_ms.first().copied().unwrap_or(0.0);
        let max_ms = samples_ms.last().copied().unwrap_or(0.0);

        Self {
            samples_ms,
            mean_ms,
            median_ms,
            min_ms,
            max_ms,
        }
    }
}

impl From<&SampleSummary> for PerfSampleStats {
    fn from(summary: &SampleSummary) -> Self {
        Self {
            samples_ms: summary.samples_ms.clone(),
            mean_ms: summary.mean_ms,
            median_ms: summary.median_ms,
            min_ms: summary.min_ms,
            max_ms: summary.max_ms,
        }
    }
}

/// Pass timing data captured during pipeline execution.
#[derive(Debug, Clone)]
struct PassTimingEntry {
    /// Pass label.
    label: String,
    /// Total time spent in the pass.
    duration_ms: f64,
    /// Whether the pass executed on any body.
    executed: bool,
    /// Whether the pass reported changes.
    changed: bool,
    /// MIR instruction delta for the pass.
    mir_instruction_delta: Option<i64>,
    /// MIR byte delta for the pass.
    mir_bytes_delta: Option<i64>,
}

/// Serializable pass timing data.
#[derive(Debug, Clone, Serialize)]
struct PerfPassTiming {
    /// Pass label.
    label: String,
    /// Total time spent in the pass.
    duration_ms: f64,
    /// Whether the pass executed on any body.
    executed: bool,
    /// Whether the pass reported changes.
    changed: bool,
    /// MIR instruction delta for the pass.
    mir_instruction_delta: Option<i64>,
    /// MIR byte delta for the pass.
    mir_bytes_delta: Option<i64>,
}

impl PerfTableRow {
    /// Create a table row from cells.
    fn new(cells: Vec<String>) -> Self {
        Self { cells }
    }
}

/// Allowed diagnostics for a bench fixture.
#[derive(Debug, Default)]
struct BenchAllowList {
    /// Explicitly allowed error codes.
    error_codes: HashSet<String>,
    /// Explicitly allowed warning codes.
    warning_codes: HashSet<String>,
}

impl BenchAllowList {
    /// Parse allow list metadata from a source string.
    fn from_source(source: &str) -> Self {
        // start with an empty allow list
        let mut allow = Self::default();

        // scan for allow directives
        for line in source.lines() {
            let line = line.trim();
            let directive = match line.strip_prefix("//") {
                Some(rest) => rest.trim(),
                None => continue,
            };

            // parse allow list directives
            if let Some(rest) = directive.strip_prefix("expect-errors=") {
                for code in rest.split(',') {
                    let code = code.trim();
                    if !code.is_empty() {
                        allow.error_codes.insert(code.to_string());
                    }
                }
            } else if let Some(rest) = directive.strip_prefix("expect-warnings=") {
                for code in rest.split(',') {
                    let code = code.trim();
                    if !code.is_empty() {
                        allow.warning_codes.insert(code.to_string());
                    }
                }
            }
        }

        allow
    }

    /// Return true when an error is explicitly allowed.
    fn allows_error(&self, error: &destack_compiler::OptimizeError) -> bool {
        self.error_codes.contains(error.code())
    }

    /// Return true when a warning is explicitly allowed.
    fn allows_warning(&self, warning: &destack_compiler::OptimizeWarning) -> bool {
        self.warning_codes.contains(warning.code())
    }

    /// Return true when any diagnostics are expected.
    fn has_expectations(&self) -> bool {
        !self.error_codes.is_empty() || !self.warning_codes.is_empty()
    }
}

/// Optimizer bench suite for validation runs.
#[derive(Debug)]
pub struct OptimizeValidateSuite {
    /// Shared run options.
    options: OptimizeRunOptions,
    /// Root path for the fixtures.
    root: PathBuf,
    /// Discovered test cases.
    cases: Vec<Case>,
}

impl OptimizeValidateSuite {
    /// Load the validation suite with the provided options.
    pub fn load(options: OptimizeRunOptions) -> Self {
        // locate fixture root
        let root = program::fixtures_root();

        // collect bench fixtures
        let fixtures = collect_mir_files(&root);
        let cases = fixtures
            .into_iter()
            .map(|path| build_fixture_case(&root, path))
            .collect();

        Self {
            options,
            root,
            cases,
        }
    }
}

impl Suite for OptimizeValidateSuite {
    fn name(&self) -> &'static str {
        "optimize_validate"
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        self.cases.clone()
    }

    fn run(&self, case: &Case, _context: &RunContext<'_>) -> CaseResult {
        // read the source program
        let source = match fs::read_to_string(&case.path) {
            Ok(source) => source,
            Err(error) => {
                return CaseResult::Failed {
                    message: format!("failed to read {}: {error}", case.path.display()),
                };
            }
        };

        // parse allow list directives
        let allow_list = BenchAllowList::from_source(&source);
        let package_id = PackageId::from_synthetic_path(&self.root);
        let target_id = fixture_target_id(package_id, "native");
        // use vm friendly pipeline options
        let options = PipelineOptions {
            target: PipelineTarget::Vm,
            ..Default::default()
        };
        let module_id = module_id_for_fixture(&self.root, &case.path, package_id);

        // run all configured optimization levels
        let mut ran_any = false;
        for level in OPTIMIZATION_LEVELS {
            if !self.options.allows_level(level) {
                continue;
            }

            // emit trace output when requested
            if self.options.trace {
                eprintln!("optimize bench {} {level:?}", case.path.display());
            }

            ran_any = true;
            let pipeline = default_pipeline(level, options.target);
            if let Err(message) = optimize_source(
                &source,
                module_id,
                &target_id,
                &pipeline,
                options.clone(),
                &allow_list,
            ) {
                return CaseResult::Failed { message };
            }
        }

        // skip if no levels matched the filter
        if !ran_any {
            return CaseResult::Skipped {
                reason: "filtered out by optimization level".to_string(),
            };
        }

        CaseResult::Passed
    }
}

/// Optimizer bench suite for baseline runs.
#[derive(Debug)]
pub struct OptimizeBaselineSuite {
    /// Shared run options.
    options: OptimizeRunOptions,
    /// Discovered test cases.
    cases: Vec<Case>,
    /// Program lookup by name.
    programs: HashMap<String, &'static program::Program>,
}

impl OptimizeBaselineSuite {
    /// Load the baseline suite with the provided options.
    pub fn load(options: OptimizeRunOptions) -> Self {
        // locate fixture root
        let root = program::fixtures_root();

        // prepare program containers
        let mut cases = Vec::new();
        let mut programs = HashMap::new();

        // build program cases
        for entry in program::all_programs() {
            let path = root.join(format!("{}.mir", entry.name));
            let case = Case::file(entry.name, path, "destack_test::optimize::baseline");
            cases.push(case);
            programs.insert(entry.name.to_string(), entry);
        }

        Self {
            options,
            cases,
            programs,
        }
    }
}

impl Suite for OptimizeBaselineSuite {
    fn name(&self) -> &'static str {
        "optimize_baseline"
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        self.cases.clone()
    }

    fn run(&self, case: &Case, _context: &RunContext<'_>) -> CaseResult {
        // resolve the program metadata
        let program = match self.programs.get(&case.name) {
            Some(program) => *program,
            None => {
                return CaseResult::Failed {
                    message: format!("program '{}' not found", case.name),
                };
            }
        };

        // parse allow list directives
        let allow_list = BenchAllowList::from_source(program.source);
        if allow_list.has_expectations() {
            return CaseResult::Skipped {
                reason: "skipped by allow list directives".to_string(),
            };
        }

        // emit trace output when requested
        if self.options.trace {
            eprintln!("baseline bench {}", program.name);
        }

        // run the baseline program
        let output = match baseline_output_for_program_default_args(
            program,
            self.options.max_instruction_limit,
        ) {
            Ok(output) => output,
            Err(message) => return CaseResult::Failed { message },
        };

        // compare against expected output
        let expected = (program.expected)();
        if output.value != expected {
            return CaseResult::Failed {
                message: format!(
                    "'{}' (baseline): expected {:?}, got {:?}",
                    program.name, expected, output.value
                ),
            };
        }

        CaseResult::Passed
    }
}

/// Optimizer bench suite for execute runs.
#[derive(Debug)]
pub struct OptimizeExecuteSuite {
    /// Shared run options.
    options: OptimizeRunOptions,
    /// Root path for the fixtures.
    root: PathBuf,
    /// Discovered test cases.
    cases: Vec<Case>,
    /// Program lookup by name.
    programs: HashMap<String, &'static program::Program>,
}

impl OptimizeExecuteSuite {
    /// Load the execute suite with the provided options.
    pub fn load(options: OptimizeRunOptions) -> Self {
        // locate fixture root
        let root = program::fixtures_root();

        // prepare program containers
        let mut cases = Vec::new();
        let mut programs = HashMap::new();

        // build program cases
        for entry in program::all_programs() {
            let path = root.join(format!("{}.mir", entry.name));
            let case = Case::file(entry.name, path, "destack_test::optimize::execute");
            cases.push(case);
            programs.insert(entry.name.to_string(), entry);
        }

        Self {
            options,
            root,
            cases,
            programs,
        }
    }
}

impl Suite for OptimizeExecuteSuite {
    fn name(&self) -> &'static str {
        "optimize_execute"
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        self.cases.clone()
    }

    fn run(&self, case: &Case, _context: &RunContext<'_>) -> CaseResult {
        // resolve the program metadata
        let program = match self.programs.get(&case.name) {
            Some(program) => *program,
            None => {
                return CaseResult::Failed {
                    message: format!("program '{}' not found", case.name),
                };
            }
        };

        // parse allow list directives
        let allow_list = BenchAllowList::from_source(program.source);
        if allow_list.has_expectations() {
            return CaseResult::Skipped {
                reason: "skipped by allow list directives".to_string(),
            };
        }

        // run the baseline program for comparison
        let baseline_output = match baseline_output_for_program(
            program,
            self.options.max_instruction_limit,
            self.options.bench_profile,
        ) {
            Ok(output) => output,
            Err(message) => return CaseResult::Failed { message },
        };

        // prepare shared configuration
        let package_id = PackageId::from_synthetic_path(&self.root);
        let target_id = fixture_target_id(package_id, "native");
        // use vm friendly pipeline options
        let options = PipelineOptions {
            target: PipelineTarget::Vm,
            ..Default::default()
        };

        // optimize and validate each configured level
        let mut ran_any = false;
        for level in OPTIMIZATION_LEVELS {
            if !self.options.allows_level(level) {
                continue;
            }

            // emit trace output when requested
            if self.options.trace {
                eprintln!("execute bench {} {level:?}", program.name);
            }

            ran_any = true;
            let pipeline = default_pipeline(level, options.target);
            let module_id = module_id_for_program(package_id, program.name);
            let (tree, strings) = match optimize_source(
                program.source,
                module_id,
                &target_id,
                &pipeline,
                options.clone(),
                &allow_list,
            ) {
                Ok(output) => output,
                Err(message) => return CaseResult::Failed { message },
            };

            // execute the optimized program
            let output = run_program_with_tree_result(
                program,
                tree,
                strings,
                self.options.max_instruction_limit,
                self.options.bench_profile,
            );
            let output = match output {
                Ok(output) => output,
                Err(error) => {
                    let message =
                        format!("'{}' ({level:?}): execution failed: {error}", program.name);
                    let message = build_execute_failure(
                        program,
                        level,
                        package_id,
                        &target_id,
                        &options,
                        &baseline_output,
                        &self.options,
                        message,
                    );
                    return CaseResult::Failed { message };
                }
            };

            // compare outputs against the baseline
            if output.value != baseline_output.value {
                let message = format!(
                    "'{}' ({level:?}): expected {:?}, got {:?}",
                    program.name, baseline_output.value, output.value
                );
                let message = build_execute_failure(
                    program,
                    level,
                    package_id,
                    &target_id,
                    &options,
                    &baseline_output,
                    &self.options,
                    message,
                );
                return CaseResult::Failed { message };
            }
        }

        // skip if no levels matched the filter
        if !ran_any {
            return CaseResult::Skipped {
                reason: "filtered out by optimization level".to_string(),
            };
        }

        CaseResult::Passed
    }
}

/// Optimizer bench suite for perf runs.
#[derive(Debug)]
pub struct OptimizePerfSuite {
    /// Shared run options.
    options: OptimizeRunOptions,
    /// Root path for the fixtures.
    root: PathBuf,
    /// Discovered test cases.
    cases: Vec<Case>,
    /// Program lookup by name.
    programs: HashMap<String, &'static program::Program>,
    /// Collected perf samples.
    samples: Mutex<Vec<PerfSample>>,
}

impl OptimizePerfSuite {
    /// Load the perf suite with the provided options.
    pub fn load(options: OptimizeRunOptions) -> Self {
        // locate fixture root
        let root = program::fixtures_root();

        // prepare program containers
        let mut cases = Vec::new();
        let mut programs = HashMap::new();

        // build program cases
        for entry in program::all_programs() {
            let path = root.join(format!("{}.mir", entry.name));
            let case = Case::file(entry.name, path, "destack_test::optimize::perf");
            cases.push(case);
            programs.insert(entry.name.to_string(), entry);
        }

        Self {
            options,
            root,
            cases,
            programs,
            samples: Mutex::new(Vec::new()),
        }
    }
}

impl Suite for OptimizePerfSuite {
    fn name(&self) -> &'static str {
        "optimize_perf"
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        self.cases.clone()
    }

    fn run(&self, case: &Case, _context: &RunContext<'_>) -> CaseResult {
        // resolve the program metadata
        let program = match self.programs.get(&case.name) {
            Some(program) => *program,
            None => {
                return CaseResult::Failed {
                    message: format!("program '{}' not found", case.name),
                };
            }
        };

        // parse allow list directives
        let allow_list = BenchAllowList::from_source(program.source);
        if allow_list.has_expectations() {
            return CaseResult::Skipped {
                reason: "skipped by allow list directives".to_string(),
            };
        }

        // prepare pipeline variants
        let variants = build_perf_variants(&self.options);

        // parse baseline tree to count MIR instructions
        let (baseline_tree, baseline_strings) = match parse_mir_source(program.source) {
            Ok(output) => output,
            Err(message) => return CaseResult::Failed { message },
        };
        let baseline_mir_instructions = count_mir_instructions(&baseline_tree);
        let baseline_mir_bytes = count_mir_bytes(&baseline_tree, &baseline_strings);

        // run baseline program for correctness and baseline metrics
        let (baseline_output, baseline_runtime) = match measure_runtime_samples(
            program,
            &baseline_tree,
            &baseline_strings,
            self.options.max_instruction_limit,
            self.options.bench_profile,
            self.options.perf_warmup,
            self.options.perf_samples,
        ) {
            Ok(output) => output,
            Err(error) => {
                return CaseResult::Failed {
                    message: format!("bench '{}' baseline failed: {error}", program.name),
                };
            }
        };
        let baseline_lowered_instructions = baseline_output.stats.lowered_instructions_executed;

        // prepare shared configuration
        let package_id = PackageId::from_synthetic_path(&self.root);
        let target_id = fixture_target_id(package_id, "native");
        // use vm friendly pipeline options
        let options = PipelineOptions {
            target: PipelineTarget::Vm,
            ..Default::default()
        };

        // optimize and execute each configured level
        let mut ran_any = false;
        for level in OPTIMIZATION_LEVELS {
            if !self.options.allows_level(level) {
                continue;
            }

            // emit trace output when requested
            if self.options.trace {
                eprintln!("perf bench {} {level:?}", program.name);
            }

            ran_any = true;
            let pipeline = default_pipeline(level, options.target);

            for variant in &variants {
                // optimize with compile timing
                let compile_start = Instant::now();
                let module_id = module_id_for_program(package_id, program.name);
                let output = match optimize_source_variant(
                    program.source,
                    module_id,
                    &target_id,
                    &pipeline,
                    options.clone(),
                    &allow_list,
                    &variant.filter,
                    self.options.perf_pass_timing,
                ) {
                    Ok(output) => output,
                    Err(message) => return CaseResult::Failed { message },
                };
                let compile_ms = compile_start.elapsed().as_secs_f64() * 1000.0;

                // compute MIR instruction count after optimization
                let mir_instructions = count_mir_instructions(&output.tree);
                let mir_bytes = count_mir_bytes(&output.tree, &output.strings);

                // execute the optimized program with runtime timing
                let (run_output, runtime) = match measure_runtime_samples(
                    program,
                    &output.tree,
                    &output.strings,
                    self.options.max_instruction_limit,
                    self.options.bench_profile,
                    self.options.perf_warmup,
                    self.options.perf_samples,
                ) {
                    Ok(output) => output,
                    Err(error) => {
                        return CaseResult::Failed {
                            message: format!(
                                "bench '{}' ({level:?}) failed: {error}",
                                program.name
                            ),
                        };
                    }
                };

                // compare outputs against the baseline
                if run_output.value != baseline_output.value {
                    return CaseResult::Failed {
                        message: format!(
                            "'{}' ({level:?}): expected {:?}, got {:?}",
                            program.name, baseline_output.value, run_output.value
                        ),
                    };
                }

                let sample = PerfSample {
                    name: program.name.to_string(),
                    variant: variant.label.clone(),
                    level,
                    compile_ms,
                    runtime,
                    baseline_runtime: baseline_runtime.clone(),
                    mir_instructions,
                    baseline_mir_instructions,
                    mir_bytes,
                    baseline_mir_bytes,
                    lowered_instructions: run_output.stats.lowered_instructions_executed,
                    baseline_lowered_instructions,
                    pass_timings: output.pass_timings,
                };

                self.samples
                    .lock()
                    .expect("perf samples lock poisoned")
                    .push(sample);
            }
        }

        // skip if no levels matched the filter
        if !ran_any {
            return CaseResult::Skipped {
                reason: "filtered out by optimization level".to_string(),
            };
        }

        CaseResult::Passed
    }

    fn report(&self, _results: &[(Case, CaseResult)], _context: &RunContext<'_>) {
        let mut samples = self
            .samples
            .lock()
            .expect("perf samples lock poisoned")
            .clone();

        samples.sort_by(|a, b| match a.name.cmp(&b.name) {
            std::cmp::Ordering::Equal => match level_index(a.level).cmp(&level_index(b.level)) {
                std::cmp::Ordering::Equal => a.variant.cmp(&b.variant),
                order => order,
            },
            order => order,
        });

        if samples.is_empty() {
            return;
        }

        println!();
        println!("perf summary (median ms)");
        print_perf_table(&samples);
        if self.options.perf_ab {
            println!();
            println!("perf ab summary (b vs a, median ms)");
            print_perf_ab_table(&samples);
        }

        if let Some(path) = &self.options.perf_output {
            let report = build_perf_report(&samples, &self.options);
            if let Err(error) = write_perf_report(path, &report, self.options.perf_output_format) {
                eprintln!("failed to write perf report: {error}");
            }
        }
    }
}

/// Collect all MIR bench fixtures under the root.
fn collect_mir_files(root: &Path) -> Vec<PathBuf> {
    // collect files with a depth first walk
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();

    // walk all directories under the root
    while let Some(dir) = pending.pop() {
        let entries =
            fs::read_dir(&dir).unwrap_or_else(|error| panic!("failed to read {dir:?}: {error}"));

        // scan directory entries
        for entry in entries {
            let entry =
                entry.unwrap_or_else(|error| panic!("failed to read entry in {dir:?}: {error}"));
            let path = entry.path();

            // descend into directories
            if path.is_dir() {
                pending.push(path);
                continue;
            }

            // record mir files
            if path.extension().and_then(|ext| ext.to_str()) == Some("mir") {
                files.push(path);
            }
        }
    }

    // ensure deterministic order for test output
    files.sort();

    files
}

/// Build a test case for a fixture path.
fn build_fixture_case(root: &Path, path: PathBuf) -> Case {
    // compute a stable relative name
    let relative = path.strip_prefix(root).unwrap_or(&path);
    let name = relative.to_string_lossy().replace('\\', "/");

    // build the test case
    Case::file(name, path, "destack_test::optimize::validate")
}

/// Return the module id for a bench fixture path.
fn module_id_for_fixture(root: &Path, path: &Path, package_id: PackageId) -> ModuleId {
    // compute a stable module id for the fixture path
    ModuleId::from_path(package_id, path, Some(root))
}

/// Return the module id for a bench program name.
fn module_id_for_program(package_id: PackageId, name: &str) -> ModuleId {
    // build a module id for the program name
    let relative = PathBuf::from(format!("bench/{name}.mir"));
    ModuleId::from_relative_path(package_id, &relative)
}

/// Parse a MIR source string into a tree and string pool.
fn parse_mir_source(source: &str) -> Result<(mir::NodeTree, ImmutableStringPool), String> {
    // parse the source program
    mir::parse::Parser::parse(FileId::new(0), source, ParseOptions::default())
        .validate()
        .map_err(|error| format!("failed to parse mir: {error}"))
}

/// Count MIR instructions across all function bodies.
fn count_mir_instructions(tree: &mir::NodeTree) -> u64 {
    let mut count = 0u64;

    for (_, function) in tree.iter_nodes::<mir::Function>() {
        if function.entry.is_none() {
            continue;
        }

        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            count += block.instructions.len() as u64 + 1;
        }
    }

    count
}

/// Estimate MIR text size by formatting the tree.
fn count_mir_bytes(tree: &mir::NodeTree, strings: &ImmutableStringPool) -> u64 {
    let formatted = mir::format_mir(tree, strings, mir::MirFormatOptions::default());
    formatted.len() as u64
}

/// Build a perf report from collected samples.
fn build_perf_report(samples: &[PerfSample], options: &OptimizeRunOptions) -> PerfReport {
    let timestamp_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let metadata = PerfReportMetadata {
        timestamp_seconds,
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        crate_version: env!("CARGO_PKG_VERSION").to_string(),
        bench_profile: bench_profile_label(options.bench_profile).to_string(),
        perf_warmup: options.perf_warmup,
        perf_samples: options.perf_samples,
        perf_ab: options.perf_ab,
        perf_pass_timing: options.perf_pass_timing,
    };

    let samples = samples
        .iter()
        .map(|sample| {
            let baseline_runtime = sample.baseline_runtime.median_ms;
            let runtime = sample.runtime.median_ms;
            let runtime_delta_ms = runtime - baseline_runtime;
            let runtime_speedup = if runtime > 0.0 {
                baseline_runtime / runtime
            } else {
                0.0
            };
            let mir_instruction_delta =
                sample.mir_instructions as i64 - sample.baseline_mir_instructions as i64;
            let mir_bytes_delta = sample.mir_bytes as i64 - sample.baseline_mir_bytes as i64;
            let lowered_delta =
                sample.lowered_instructions as i64 - sample.baseline_lowered_instructions as i64;

            let pass_timings = sample
                .pass_timings
                .iter()
                .map(|timing| PerfPassTiming {
                    label: timing.label.clone(),
                    duration_ms: timing.duration_ms,
                    executed: timing.executed,
                    changed: timing.changed,
                    mir_instruction_delta: timing.mir_instruction_delta,
                    mir_bytes_delta: timing.mir_bytes_delta,
                })
                .collect();

            PerfReportSample {
                name: sample.name.clone(),
                variant: sample.variant.clone(),
                level: format!("{:?}", sample.level),
                compile_ms: sample.compile_ms,
                runtime: PerfSampleStats::from(&sample.runtime),
                baseline_runtime: PerfSampleStats::from(&sample.baseline_runtime),
                runtime_delta_ms,
                runtime_speedup,
                mir_instructions: sample.mir_instructions,
                baseline_mir_instructions: sample.baseline_mir_instructions,
                mir_instruction_delta,
                mir_bytes: sample.mir_bytes,
                baseline_mir_bytes: sample.baseline_mir_bytes,
                mir_bytes_delta,
                lowered_instructions: sample.lowered_instructions,
                baseline_lowered_instructions: sample.baseline_lowered_instructions,
                lowered_delta,
                pass_timings,
            }
        })
        .collect();

    PerfReport { metadata, samples }
}

/// Write a perf report to disk.
fn write_perf_report(
    path: &Path,
    report: &PerfReport,
    format: PerfOutputFormat,
) -> Result<(), String> {
    match format {
        PerfOutputFormat::Json => {
            let payload =
                serde_json::to_string_pretty(report).map_err(|error| error.to_string())?;
            fs::write(path, payload)
                .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
        }
        PerfOutputFormat::Csv => {
            let payload = format_perf_csv(report);
            fs::write(path, payload)
                .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
        }
    }

    Ok(())
}

/// Format a perf report as csv.
fn format_perf_csv(report: &PerfReport) -> String {
    let mut output = String::new();
    output.push_str("name,variant,level,compile_ms,runtime_median_ms,runtime_mean_ms,runtime_min_ms,runtime_max_ms,baseline_median_ms,baseline_mean_ms,baseline_min_ms,baseline_max_ms,runtime_delta_ms,runtime_speedup,mir_instructions,baseline_mir_instructions,mir_instruction_delta,mir_bytes,baseline_mir_bytes,mir_bytes_delta,lowered_instructions,baseline_lowered_instructions,lowered_delta\n");

    for sample in &report.samples {
        let row = format!(
            "{name},{variant},{level},{compile_ms:.4},{runtime_median:.4},{runtime_mean:.4},{runtime_min:.4},{runtime_max:.4},{baseline_median:.4},{baseline_mean:.4},{baseline_min:.4},{baseline_max:.4},{runtime_delta:.4},{runtime_speedup:.4},{mir_instr},{baseline_mir_instr},{mir_delta},{mir_bytes},{baseline_mir_bytes},{mir_bytes_delta},{lowered},{baseline_lowered},{lowered_delta}\n",
            name = csv_escape(&sample.name),
            variant = csv_escape(&sample.variant),
            level = csv_escape(&sample.level),
            compile_ms = sample.compile_ms,
            runtime_median = sample.runtime.median_ms,
            runtime_mean = sample.runtime.mean_ms,
            runtime_min = sample.runtime.min_ms,
            runtime_max = sample.runtime.max_ms,
            baseline_median = sample.baseline_runtime.median_ms,
            baseline_mean = sample.baseline_runtime.mean_ms,
            baseline_min = sample.baseline_runtime.min_ms,
            baseline_max = sample.baseline_runtime.max_ms,
            runtime_delta = sample.runtime_delta_ms,
            runtime_speedup = sample.runtime_speedup,
            mir_instr = sample.mir_instructions,
            baseline_mir_instr = sample.baseline_mir_instructions,
            mir_delta = sample.mir_instruction_delta,
            mir_bytes = sample.mir_bytes,
            baseline_mir_bytes = sample.baseline_mir_bytes,
            mir_bytes_delta = sample.mir_bytes_delta,
            lowered = sample.lowered_instructions,
            baseline_lowered = sample.baseline_lowered_instructions,
            lowered_delta = sample.lowered_delta,
        );
        output.push_str(&row);
    }

    output
}

/// Print a formatted perf table to stdout.
fn print_perf_table(samples: &[PerfSample]) {
    let mut rows = Vec::new();
    let show_variant = samples
        .iter()
        .map(|sample| sample.variant.as_str())
        .collect::<HashSet<_>>()
        .len()
        > 1;

    let mut header_cells = vec!["Program".to_string()];
    if show_variant {
        header_cells.push("Var".to_string());
    }
    header_cells.push("Lvl".to_string());
    header_cells.extend([
        "Compile".to_string(),
        "Run".to_string(),
        "Base".to_string(),
        "d_ms".to_string(),
        "Speedup".to_string(),
        "MIR".to_string(),
        "Bytes".to_string(),
        "Lowered".to_string(),
    ]);
    let header = PerfTableRow::new(header_cells);
    rows.push(header);

    let speedup_index = if show_variant { 7 } else { 6 };

    for sample in samples {
        let baseline_runtime = sample.baseline_runtime.median_ms;
        let runtime = sample.runtime.median_ms;
        let runtime_delta = runtime - baseline_runtime;
        let runtime_speedup = if runtime > 0.0 {
            baseline_runtime / runtime
        } else {
            0.0
        };

        let mut cells = vec![sample.name.clone()];
        if show_variant {
            cells.push(sample.variant.clone());
            cells.push(format!("{:?}", sample.level));
        } else {
            cells.push(format!("{:?}", sample.level));
        }
        cells.extend([
            format!("{compile_ms:.2}", compile_ms = sample.compile_ms),
            format!("{runtime:.2}"),
            format!("{baseline_runtime:.2}"),
            format!("{runtime_delta:+.2}"),
            format!("{runtime_speedup:.2}x"),
            sample.mir_instructions.to_string(),
            sample.mir_bytes.to_string(),
            sample.lowered_instructions.to_string(),
        ]);
        let row = PerfTableRow::new(cells);
        rows.push(row);
    }

    let mut align = vec![ColumnAlign::Left];
    if show_variant {
        align.push(ColumnAlign::Left);
    }
    align.push(ColumnAlign::Left);
    align.extend([
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Right,
    ]);

    let widths = compute_column_widths(&rows);
    let header = format_perf_row(&rows[0], &widths, &align);
    let separator =
        "-".repeat(widths.iter().sum::<usize>() + COLUMN_GAP.len() * (widths.len() - 1));

    println!("{DIM}{header}{RESET}");
    println!("{DIM}{separator}{RESET}");
    for row in rows.iter().skip(1) {
        let speedup = parse_speedup_cell(row.cells.get(speedup_index));
        let speedup_color = speedup.map(speedup_color).unwrap_or(RESET);

        let mut colors = vec![None; row.cells.len()];
        colors[0] = Some(CYAN);
        if let Some(color) = color_if_not_reset(speedup_color) {
            colors[speedup_index] = Some(color);
        }

        let line = format_perf_row_colored(row, &widths, &align, &colors);
        println!("{line}");
    }
}

/// Print a formatted A/B comparison table to stdout.
fn print_perf_ab_table(samples: &[PerfSample]) {
    let mut pairs: HashMap<(String, OptimizationLevel), (Option<usize>, Option<usize>)> =
        HashMap::new();

    for (index, sample) in samples.iter().enumerate() {
        let key = (sample.name.clone(), sample.level);
        let entry = pairs.entry(key).or_insert((None, None));
        if sample.variant == "A" {
            entry.0 = Some(index);
        } else if sample.variant == "B" {
            entry.1 = Some(index);
        }
    }

    let mut keys: Vec<_> = pairs.keys().cloned().collect();
    keys.sort_by(
        |(name_a, level_a), (name_b, level_b)| match name_a.cmp(name_b) {
            std::cmp::Ordering::Equal => level_index(*level_a).cmp(&level_index(*level_b)),
            order => order,
        },
    );

    let mut rows = Vec::new();
    let header = PerfTableRow::new(vec![
        "Program".to_string(),
        "Lvl".to_string(),
        "RunA".to_string(),
        "RunB".to_string(),
        "d_ms".to_string(),
        "Speedup".to_string(),
        "CompA".to_string(),
        "CompB".to_string(),
        "d_comp".to_string(),
        "MIR_d".to_string(),
        "Bytes_d".to_string(),
        "Thread_d".to_string(),
    ]);
    rows.push(header);

    for (name, level) in keys {
        let Some((a_index, b_index)) = pairs.get(&(name.clone(), level)) else {
            continue;
        };
        let (Some(a_index), Some(b_index)) = (a_index.as_ref(), b_index.as_ref()) else {
            continue;
        };
        let sample_a = &samples[*a_index];
        let sample_b = &samples[*b_index];

        let runtime_a = sample_a.runtime.median_ms;
        let runtime_b = sample_b.runtime.median_ms;
        let runtime_delta = runtime_b - runtime_a;
        let runtime_speedup = if runtime_b > 0.0 {
            runtime_a / runtime_b
        } else {
            0.0
        };

        let compile_a = sample_a.compile_ms;
        let compile_b = sample_b.compile_ms;
        let compile_delta = compile_b - compile_a;

        let mir_delta = sample_b.mir_instructions as i64 - sample_a.mir_instructions as i64;
        let bytes_delta = sample_b.mir_bytes as i64 - sample_a.mir_bytes as i64;
        let lowered_delta =
            sample_b.lowered_instructions as i64 - sample_a.lowered_instructions as i64;

        let row = PerfTableRow::new(vec![
            name,
            format!("{:?}", level),
            format!("{:.2}", runtime_a),
            format!("{:.2}", runtime_b),
            format!("{:+.2}", runtime_delta),
            format!("{:.2}x", runtime_speedup),
            format!("{:.2}", compile_a),
            format!("{:.2}", compile_b),
            format!("{:+.2}", compile_delta),
            format!("{:+}", mir_delta),
            format!("{:+}", bytes_delta),
            format!("{:+}", lowered_delta),
        ]);
        rows.push(row);
    }

    if rows.len() <= 1 {
        return;
    }

    let mut align = vec![ColumnAlign::Left, ColumnAlign::Left];
    align.extend([
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Right,
    ]);

    let widths = compute_column_widths(&rows);
    let header = format_perf_row(&rows[0], &widths, &align);
    let separator =
        "-".repeat(widths.iter().sum::<usize>() + COLUMN_GAP.len() * (widths.len() - 1));

    println!("{DIM}{header}{RESET}");
    println!("{DIM}{separator}{RESET}");
    for row in rows.iter().skip(1) {
        let speedup = parse_speedup_cell(row.cells.get(5));
        let speedup_color = speedup.map(speedup_color).unwrap_or(RESET);

        let mut colors = vec![None; row.cells.len()];
        colors[0] = Some(CYAN);
        if let Some(color) = color_if_not_reset(speedup_color) {
            colors[5] = Some(color);
        }

        let line = format_perf_row_colored(row, &widths, &align, &colors);
        println!("{line}");
    }
}

/// Compute column widths for perf rows.
fn compute_column_widths(rows: &[PerfTableRow]) -> Vec<usize> {
    let mut widths = Vec::new();
    for row in rows {
        if widths.is_empty() {
            widths.resize(row.cells.len(), 0);
        }

        for (idx, cell) in row.cells.iter().enumerate() {
            widths[idx] = widths[idx].max(cell.len());
        }
    }

    widths
}

/// Format a perf row with widths and alignment.
fn format_perf_row(row: &PerfTableRow, widths: &[usize], align: &[ColumnAlign]) -> String {
    let mut output = String::new();
    for (idx, cell) in row.cells.iter().enumerate() {
        let width = widths[idx];
        let aligned = match align.get(idx).copied().unwrap_or(ColumnAlign::Right) {
            ColumnAlign::Left => format!("{cell:<width$}"),
            ColumnAlign::Right => format!("{cell:>width$}"),
        };

        output.push_str(&aligned);
        if idx + 1 < row.cells.len() {
            output.push_str(COLUMN_GAP);
        }
    }

    output
}

/// Format a perf row with widths, alignment, and optional colors.
fn format_perf_row_colored(
    row: &PerfTableRow,
    widths: &[usize],
    align: &[ColumnAlign],
    colors: &[Option<&'static str>],
) -> String {
    let mut output = String::new();
    for (idx, cell) in row.cells.iter().enumerate() {
        let width = widths[idx];
        let aligned = match align.get(idx).copied().unwrap_or(ColumnAlign::Right) {
            ColumnAlign::Left => format!("{cell:<width$}"),
            ColumnAlign::Right => format!("{cell:>width$}"),
        };

        if let Some(color) = colors.get(idx).copied().flatten() {
            output.push_str(color);
            output.push_str(&aligned);
            output.push_str(RESET);
        } else {
            output.push_str(&aligned);
        }

        if idx + 1 < row.cells.len() {
            output.push_str(COLUMN_GAP);
        }
    }

    output
}

/// Return the color for a speedup value.
fn speedup_color(speedup: f64) -> &'static str {
    if speedup >= 1.05 {
        GREEN
    } else if speedup >= 0.98 {
        YELLOW
    } else {
        RED
    }
}

/// Parse a speedup cell value formatted as `Nx`.
fn parse_speedup_cell(cell: Option<&String>) -> Option<f64> {
    let cell = cell?;
    let trimmed = cell.trim_end_matches('x');
    trimmed.parse::<f64>().ok()
}

/// Return the color unless it is the reset code.
fn color_if_not_reset(color: &'static str) -> Option<&'static str> {
    if color == RESET { None } else { Some(color) }
}

/// Escape a csv field.
fn csv_escape(value: &str) -> String {
    let needs_escape = value.contains(',') || value.contains('"') || value.contains('\n');
    if !needs_escape {
        return value.to_string();
    }

    let escaped = value.replace('"', "\"\"");
    format!("\"{escaped}\"")
}

/// Return the bench profile label.
fn bench_profile_label(profile: program::BenchProfileKind) -> &'static str {
    match profile {
        program::BenchProfileKind::Quick => "quick",
        program::BenchProfileKind::Standard => "standard",
        program::BenchProfileKind::Stress => "stress",
    }
}

/// Build pipeline variants for perf runs.
fn build_perf_variants(options: &OptimizeRunOptions) -> Vec<PipelineVariant> {
    if options.perf_ab {
        let filter = PassFilter::new(&options.perf_ab_disable, &options.perf_ab_only).force();
        vec![
            PipelineVariant {
                label: "A".to_string(),
                filter: PassFilter::default().force(),
            },
            PipelineVariant {
                label: "B".to_string(),
                filter,
            },
        ]
    } else {
        vec![PipelineVariant {
            label: "default".to_string(),
            filter: PassFilter::default(),
        }]
    }
}

/// Map optimization levels to a stable sort index.
fn level_index(level: OptimizationLevel) -> u8 {
    match level {
        OptimizationLevel::O0 => 0,
        OptimizationLevel::O1 => 1,
        OptimizationLevel::O2 => 2,
        OptimizationLevel::O3 => 3,
        OptimizationLevel::O4 => 4,
    }
}

/// Optimize a source program and return the optimized tree and strings.
fn optimize_source(
    source: &str,
    module_id: ModuleId,
    target_id: &TargetId,
    pipeline: &dyn Pipeline,
    options: PipelineOptions,
    allow_list: &BenchAllowList,
) -> Result<(mir::NodeTree, ImmutableStringPool), String> {
    // parse the source program
    let (mut tree, strings) = parse_mir_source(source)?;

    // build the pipeline context
    let strings_pool = StringPool::new();
    strings_pool.copy_from_immutable(&strings);
    let mut ctx = PipelineContext::new(&strings_pool, options, module_id, *target_id, None);

    // run optimization and check diagnostics
    pipeline.run(&mut tree, &mut ctx);
    let diagnostics = ctx.diagnostics();
    let errors = diagnostics.take_errors();
    let warnings = diagnostics.take_warnings();

    let unexpected_errors: Vec<_> = errors
        .iter()
        .filter(|error| !allow_list.allows_error(error))
        .collect();
    let unexpected_warnings: Vec<_> = warnings
        .iter()
        .filter(|warning| !allow_list.allows_warning(warning))
        .collect();

    if !unexpected_errors.is_empty() || !unexpected_warnings.is_empty() {
        return Err(format!(
            "unexpected diagnostics:\nerrors: {unexpected_errors:#?}\nwarnings: {unexpected_warnings:#?}"
        ));
    }

    // return optimized output
    let optimized_strings = strings_pool.clone().into_immutable();
    Ok((tree, optimized_strings))
}

/// Output of an optimized pipeline run.
struct OptimizeOutput {
    /// Optimized tree.
    tree: mir::NodeTree,
    /// String pool for the optimized tree.
    strings: ImmutableStringPool,
    /// Per-pass timing data.
    pass_timings: Vec<PassTimingEntry>,
}

/// Optimize a source program with optional pass filtering and timing.
#[allow(clippy::too_many_arguments)]
fn optimize_source_variant(
    source: &str,
    module_id: ModuleId,
    target_id: &TargetId,
    pipeline: &dyn Pipeline,
    options: PipelineOptions,
    allow_list: &BenchAllowList,
    filter: &PassFilter,
    capture_pass_timing: bool,
) -> Result<OptimizeOutput, String> {
    // parse the source program
    let (mut tree, strings) = parse_mir_source(source)?;

    // build the pipeline context
    let strings_pool = StringPool::new();
    strings_pool.copy_from_immutable(&strings);
    let mut ctx = PipelineContext::new(&strings_pool, options, module_id, *target_id, None);

    let mut pass_timings = Vec::new();
    if filter.is_noop() && !capture_pass_timing {
        pipeline.run(&mut tree, &mut ctx);
    } else {
        let mut path = Vec::new();
        run_pipeline_with_timings(
            &mut tree,
            &mut ctx,
            pipeline,
            filter,
            capture_pass_timing,
            &mut pass_timings,
            &mut path,
            &strings_pool,
        );
    }

    // check diagnostics
    let diagnostics = ctx.diagnostics();
    let errors = diagnostics.take_errors();
    let warnings = diagnostics.take_warnings();

    let unexpected_errors: Vec<_> = errors
        .iter()
        .filter(|error| !allow_list.allows_error(error))
        .collect();
    let unexpected_warnings: Vec<_> = warnings
        .iter()
        .filter(|warning| !allow_list.allows_warning(warning))
        .collect();

    if !unexpected_errors.is_empty() || !unexpected_warnings.is_empty() {
        return Err(format!(
            "unexpected diagnostics:\nerrors: {unexpected_errors:#?}\nwarnings: {unexpected_warnings:#?}"
        ));
    }

    // return optimized output
    let optimized_strings = strings_pool.clone().into_immutable();
    Ok(OptimizeOutput {
        tree,
        strings: optimized_strings,
        pass_timings,
    })
}

/// Return the baseline output for a bench program.
fn baseline_output_for_program(
    program: &program::Program,
    max_instruction_limit: Option<u64>,
    profile: program::BenchProfileKind,
) -> Result<ExecutionOutput, String> {
    // parse the source program
    let (tree, strings) = parse_mir_source(program.source)?;

    // run the baseline program with quick profile args
    run_program_with_tree_result(program, tree, strings, max_instruction_limit, profile)
        .map_err(|error| format!("bench '{}' baseline failed: {error}", program.name))
}

/// Return the baseline output for a bench program using default args.
fn baseline_output_for_program_default_args(
    program: &program::Program,
    max_instruction_limit: Option<u64>,
) -> Result<ExecutionOutput, String> {
    // parse the source program
    let (tree, strings) = parse_mir_source(program.source)?;

    // run the baseline program with default args
    run_program_with_tree_result_default_args(program, tree, strings, max_instruction_limit)
        .map_err(|error| format!("bench '{}' baseline failed: {error}", program.name))
}

/// Execute a program repeatedly and collect timing samples.
fn measure_runtime_samples(
    program: &program::Program,
    tree: &mir::NodeTree,
    strings: &ImmutableStringPool,
    max_instruction_limit: Option<u64>,
    profile: program::BenchProfileKind,
    warmup: u32,
    samples: u32,
) -> Result<(ExecutionOutput, SampleSummary), String> {
    // run warmup iterations
    for _ in 0..warmup {
        run_program_with_tree_profile(program, tree, strings, max_instruction_limit, profile)
            .map_err(|error| format!("bench '{}' warmup failed: {error}", program.name))?;
    }

    // run timing samples
    let sample_count = samples.max(1) as usize;
    let mut timings = Vec::with_capacity(sample_count);
    let mut baseline_output: Option<ExecutionOutput> = None;

    for _ in 0..sample_count {
        let start = Instant::now();
        let output =
            run_program_with_tree_profile(program, tree, strings, max_instruction_limit, profile)
                .map_err(|error| format!("bench '{}' run failed: {error}", program.name))?;
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;

        if let Some(expected) = baseline_output.as_ref() {
            if output.value != expected.value {
                return Err(format!(
                    "bench '{}' produced mismatched output during perf run",
                    program.name
                ));
            }
        } else {
            baseline_output = Some(output);
        }

        timings.push(elapsed);
    }

    let baseline_output = baseline_output
        .ok_or_else(|| format!("bench '{}' failed to produce a perf sample", program.name))?;

    Ok((baseline_output, SampleSummary::from_samples(timings)))
}

/// Run a program and compare against the baseline output.
fn run_and_compare_output(
    program: &program::Program,
    tree: mir::NodeTree,
    strings: ImmutableStringPool,
    baseline_output: &ExecutionOutput,
    max_instruction_limit: Option<u64>,
    profile: program::BenchProfileKind,
    context: &str,
) -> Result<(), String> {
    // execute the program
    let output =
        run_program_with_tree_result(program, tree, strings, max_instruction_limit, profile);
    let output = match output {
        Ok(output) => output,
        Err(error) => {
            return Err(format!(
                "bench '{}' {context}: execution failed: {error}",
                program.name
            ));
        }
    };

    // compare outputs
    if output.value != baseline_output.value {
        return Err(format!(
            "bench '{}' {context}: expected {:?}, got {:?}",
            program.name, baseline_output.value, output.value
        ));
    }

    Ok(())
}

/// Run a single function pass over the module.
fn run_function_pass(
    pass: &dyn FunctionPass,
    tree: &mut mir::NodeTree,
    ctx: &PipelineContext<'_>,
) -> bool {
    // track whether any pass invalidated analyses
    let mut any_changed = false;

    // collect function ids
    let function_ids: Vec<_> = tree
        .iter_nodes::<mir::Function>()
        .map(|(id, _)| id)
        .collect();

    for function_id in function_ids {
        let mut function = tree.get(function_id).clone();

        // skip imported functions
        if function.entry.is_none() {
            continue;
        }

        // recompute next_value_id so passes can allocate fresh values
        function.recompute_next_value_id(tree);
        let preserved = pass.run(&mut function, tree, ctx);
        if !preserved.preserves_all() {
            any_changed = true;
        }

        // write function back
        *tree.get_mut(function_id) = function;
    }

    any_changed
}

/// Run a single module pass over the module.
fn run_module_pass(
    pass: &dyn ModulePass,
    tree: &mut mir::NodeTree,
    ctx: &PipelineContext<'_>,
) -> bool {
    // run the pass and track invalidation
    let preserved = pass.run(tree, ctx);

    !preserved.preserves_all()
}

/// Run a function pass with filtering and requirement checks.
fn run_function_pass_filtered(
    pass: &dyn FunctionPass,
    tree: &mut mir::NodeTree,
    ctx: &PipelineContext<'_>,
    filter: &PassFilter,
) -> (bool, bool) {
    if !filter.allows(pass.name()) {
        return (false, false);
    }

    let mut any_changed = false;
    let mut executed = false;

    // collect function ids
    let function_ids: Vec<_> = tree
        .iter_nodes::<mir::Function>()
        .map(|(id, _)| id)
        .collect();

    for function_id in function_ids {
        let mut function = tree.get(function_id).clone();

        // skip imported functions
        if function.entry.is_none() {
            continue;
        }

        // enforce pass requirements
        if !ctx.enforce_function_requirements(pass.metadata(), function_id, &function, tree) {
            continue;
        }

        executed = true;
        function.recompute_next_value_id(tree);
        let preserved = pass.run(&mut function, tree, ctx);
        if !preserved.preserves_all() {
            any_changed = true;
        }

        // write function back
        *tree.get_mut(function_id) = function;
    }

    (any_changed, executed)
}

/// Run a module pass with filtering and requirement checks.
fn run_module_pass_filtered(
    pass: &dyn ModulePass,
    tree: &mut mir::NodeTree,
    ctx: &PipelineContext<'_>,
    filter: &PassFilter,
) -> (bool, bool) {
    if !filter.allows(pass.name()) {
        return (false, false);
    }

    // enforce pass requirements
    if !ctx.enforce_module_requirements(pass.metadata(), tree) {
        return (false, false);
    }

    let preserved = pass.run(tree, ctx);
    (!preserved.preserves_all(), true)
}

/// Run a pipeline while capturing pass timings.
#[allow(clippy::too_many_arguments)]
fn run_pipeline_with_timings(
    tree: &mut mir::NodeTree,
    ctx: &mut PipelineContext<'_>,
    pipeline: &dyn Pipeline,
    filter: &PassFilter,
    capture_metrics: bool,
    timings: &mut Vec<PassTimingEntry>,
    path: &mut Vec<String>,
    strings_pool: &StringPool,
) -> bool {
    // handle composite pipelines
    if let Some(composite) = pipeline.as_any().downcast_ref::<CompositePipeline>() {
        let mut any_changed = false;

        path.push(composite.name().to_string());
        for child in composite.pipelines() {
            let changed = run_pipeline_with_timings(
                tree,
                ctx,
                child.as_ref(),
                filter,
                capture_metrics,
                timings,
                path,
                strings_pool,
            );
            any_changed |= changed;
        }
        path.pop();

        return any_changed;
    }

    // handle repeated pipelines
    if let Some(repeat) = pipeline.as_any().downcast_ref::<RepeatedPipeline>() {
        let mut any_changed = false;
        let max_iterations = repeat.max_iterations();
        let repeat_label = format!("repeat({max_iterations})");
        path.push(repeat_label);

        for iteration_index in 0..max_iterations {
            let iteration_label = format!("iter {}", iteration_index + 1);
            path.push(iteration_label);
            let changed = run_pipeline_with_timings(
                tree,
                ctx,
                repeat.inner(),
                filter,
                capture_metrics,
                timings,
                path,
                strings_pool,
            );
            path.pop();

            any_changed |= changed;
            if !changed {
                break;
            }
        }

        path.pop();
        return any_changed;
    }

    // handle function pipelines
    if let Some(function_pipeline) = pipeline.as_any().downcast_ref::<FunctionPipeline>() {
        let mut any_changed = false;

        for pass in function_pipeline.passes() {
            let label = build_pass_label(path, pass.name());
            let (before_instructions, before_bytes) = if capture_metrics {
                (
                    Some(count_mir_instructions(tree)),
                    Some(count_mir_bytes(
                        tree,
                        &strings_pool.clone().into_immutable(),
                    )),
                )
            } else {
                (None, None)
            };

            let start = Instant::now();
            let (changed, executed) = run_function_pass_filtered(pass.as_ref(), tree, ctx, filter);
            let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
            any_changed |= changed;

            let (mir_instruction_delta, mir_bytes_delta) = if capture_metrics && executed {
                let after_instructions = count_mir_instructions(tree);
                let after_bytes = count_mir_bytes(tree, &strings_pool.clone().into_immutable());
                let before_instructions = before_instructions.unwrap_or(after_instructions);
                let before_bytes = before_bytes.unwrap_or(after_bytes);
                (
                    Some(after_instructions as i64 - before_instructions as i64),
                    Some(after_bytes as i64 - before_bytes as i64),
                )
            } else {
                (None, None)
            };

            timings.push(PassTimingEntry {
                label,
                duration_ms,
                executed,
                changed,
                mir_instruction_delta,
                mir_bytes_delta,
            });
        }

        return any_changed;
    }

    // handle function adaptor pipelines
    if let Some(adaptor) = pipeline.as_any().downcast_ref::<FunctionToModuleAdaptor>() {
        return run_pipeline_with_timings(
            tree,
            ctx,
            adaptor.inner(),
            filter,
            capture_metrics,
            timings,
            path,
            strings_pool,
        );
    }

    // handle module pipelines
    if let Some(module_pipeline) = pipeline.as_any().downcast_ref::<ModulePipeline>() {
        let mut any_changed = false;

        for pass in module_pipeline.passes() {
            let label = build_pass_label(path, pass.name());
            let (before_instructions, before_bytes) = if capture_metrics {
                (
                    Some(count_mir_instructions(tree)),
                    Some(count_mir_bytes(
                        tree,
                        &strings_pool.clone().into_immutable(),
                    )),
                )
            } else {
                (None, None)
            };

            let start = Instant::now();
            let (changed, executed) = run_module_pass_filtered(pass.as_ref(), tree, ctx, filter);
            let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
            any_changed |= changed;

            let (mir_instruction_delta, mir_bytes_delta) = if capture_metrics && executed {
                let after_instructions = count_mir_instructions(tree);
                let after_bytes = count_mir_bytes(tree, &strings_pool.clone().into_immutable());
                let before_instructions = before_instructions.unwrap_or(after_instructions);
                let before_bytes = before_bytes.unwrap_or(after_bytes);
                (
                    Some(after_instructions as i64 - before_instructions as i64),
                    Some(after_bytes as i64 - before_bytes as i64),
                )
            } else {
                (None, None)
            };

            timings.push(PassTimingEntry {
                label,
                duration_ms,
                executed,
                changed,
                mir_instruction_delta,
                mir_bytes_delta,
            });
        }

        return any_changed;
    }

    false
}

/// Run a matrix case and return a mismatch reason if any.
#[allow(clippy::too_many_arguments)]
fn run_matrix_case(
    program: &program::Program,
    package_id: PackageId,
    target_id: &TargetId,
    options: &PipelineOptions,
    baseline_output: &ExecutionOutput,
    max_instruction_limit: Option<u64>,
    profile: program::BenchProfileKind,
    label: &str,
    apply: impl FnOnce(&mut mir::NodeTree, &StringPool, &PipelineContext<'_>),
) -> Option<String> {
    // parse the source program
    let (mut tree, strings) =
        mir::parse::Parser::parse(FileId::new(0), program.source, ParseOptions::default())
            .validate()
            .ok()?;

    // build the pipeline context
    let strings_pool = StringPool::new();
    strings_pool.copy_from_immutable(&strings);
    let module_id = ModuleId::from_relative_path(
        package_id,
        &PathBuf::from(format!("bench/{}.mir", program.name)),
    );
    let ctx = PipelineContext::new(&strings_pool, options.clone(), module_id, *target_id, None);

    // apply the case and execute
    apply(&mut tree, &strings_pool, &ctx);

    // validate the resulting MIR tree
    if let Err(error) = mir::Validator::new(&tree).validate() {
        let message = format_validate_error(&tree, error);
        return Some(format!("case '{label}': validation failed: {message}"));
    }

    let context = format!("case '{label}'");
    run_and_compare_output(
        program,
        tree,
        strings_pool.into_immutable(),
        baseline_output,
        max_instruction_limit,
        profile,
        &context,
    )
    .err()
}

/// Format a validator error with context when available.
fn format_validate_error(tree: &mir::NodeTree, error: mir::ValidateError) -> String {
    // include instruction context for undefined value errors
    match error {
        mir::ValidateError::UseOfUndefinedValue { value, anchor } => {
            if anchor.node.ty == mir::NodeType::Instruction {
                let instruction_id = mir::LocalNodeId::<mir::Instruction>::new(anchor.node.id);
                let instruction = tree.get(instruction_id);
                if let Some((block_id, block)) = find_instruction_block(tree, instruction_id) {
                    let definition = find_value_definition(tree, value);
                    return format!(
                        "{error:?} at instruction {instruction_id:?} in {block_id:?}: {instruction:?} (params: {:?}, def: {definition})",
                        block.parameters
                    );
                }

                return format!("{error:?} at instruction {instruction_id:?}: {instruction:?}");
            }

            format!("{error:?}")
        }
        _ => format!("{error:?}"),
    }
}

/// Locate the block containing an instruction.
fn find_instruction_block(
    tree: &mir::NodeTree,
    instruction_id: mir::LocalNodeId<mir::Instruction>,
) -> Option<(mir::LocalNodeId<mir::Block>, mir::Block)> {
    // scan blocks to find the instruction
    for (block_id, block) in tree.iter_nodes::<mir::Block>() {
        if block.instructions.contains(&instruction_id) {
            return Some((block_id, block.clone()));
        }
    }

    None
}

/// Describe where a value is defined, if known.
fn find_value_definition(tree: &mir::NodeTree, value: mir::Value) -> String {
    let value_reference = mir::ValueReference::Value(value);

    // scan blocks for parameter definitions
    for (block_id, block) in tree.iter_nodes::<mir::Block>() {
        if let Some(param) = block
            .parameters
            .iter()
            .find(|param| param.value == value_reference)
        {
            return format!("{block_id:?} param {param:?}");
        }

        // scan instructions for destination definitions
        for instruction_id in &block.instructions {
            let instruction = tree.get(*instruction_id);
            if instruction.destination() == Some(value_reference) {
                return format!("{block_id:?} {instruction_id:?} {instruction:?}");
            }
        }
    }

    "unknown".to_string()
}

/// Run matrix diagnostics for the given pipeline.
#[allow(clippy::too_many_arguments)]
fn run_matrix_pipeline(
    program: &program::Program,
    package_id: PackageId,
    target_id: &TargetId,
    options: &PipelineOptions,
    baseline_output: &ExecutionOutput,
    max_instruction_limit: Option<u64>,
    profile: program::BenchProfileKind,
    pipeline: &dyn Pipeline,
    path: &mut Vec<String>,
) -> Option<String> {
    // handle composite pipelines
    if let Some(composite) = pipeline.as_any().downcast_ref::<CompositePipeline>() {
        for child in composite.pipelines() {
            let reason = run_matrix_pipeline(
                program,
                package_id,
                target_id,
                options,
                baseline_output,
                max_instruction_limit,
                profile,
                child.as_ref(),
                path,
            );
            if reason.is_some() {
                return reason;
            }
        }

        return None;
    }

    // handle repeated pipelines
    if let Some(repeat) = pipeline.as_any().downcast_ref::<RepeatedPipeline>() {
        let repeat_label = format!("repeat({})", repeat.max_iterations());
        path.push(repeat_label);
        let reason = run_matrix_pipeline(
            program,
            package_id,
            target_id,
            options,
            baseline_output,
            max_instruction_limit,
            profile,
            repeat.inner(),
            path,
        );
        path.pop();
        return reason;
    }

    // handle function pipelines
    if let Some(function_pipeline) = pipeline.as_any().downcast_ref::<FunctionPipeline>() {
        for pass in function_pipeline.passes() {
            let mut label_parts = path.clone();
            label_parts.push(pass.name().to_string());
            let label = label_parts.join(" / ");

            let reason = run_matrix_case(
                program,
                package_id,
                target_id,
                options,
                baseline_output,
                max_instruction_limit,
                profile,
                &label,
                |tree, _strings_pool, ctx| {
                    run_function_pass(pass.as_ref(), tree, ctx);
                },
            );
            if reason.is_some() {
                return reason;
            }
        }

        return None;
    }

    // handle function adaptor pipelines
    if let Some(adaptor) = pipeline.as_any().downcast_ref::<FunctionToModuleAdaptor>() {
        return run_matrix_pipeline(
            program,
            package_id,
            target_id,
            options,
            baseline_output,
            max_instruction_limit,
            profile,
            adaptor.inner(),
            path,
        );
    }

    // handle module pipelines
    if let Some(module_pipeline) = pipeline.as_any().downcast_ref::<ModulePipeline>() {
        for pass in module_pipeline.passes() {
            let mut label_parts = path.clone();
            label_parts.push(pass.name().to_string());
            let label = label_parts.join(" / ");

            let reason = run_matrix_case(
                program,
                package_id,
                target_id,
                options,
                baseline_output,
                max_instruction_limit,
                profile,
                &label,
                |tree, _strings_pool, ctx| {
                    run_module_pass(pass.as_ref(), tree, ctx);
                },
            );
            if reason.is_some() {
                return reason;
            }
        }

        return None;
    }

    Some(format!(
        "bench '{}' encountered unsupported pipeline '{}'",
        program.name,
        pipeline.name()
    ))
}

/// Diagnose the first pass that changes execution output.
fn diagnose_mismatch(
    program: &program::Program,
    level: OptimizationLevel,
    package_id: PackageId,
    target_id: &TargetId,
    options: &PipelineOptions,
    baseline_output: &ExecutionOutput,
    max_instruction_limit: Option<u64>,
    profile: program::BenchProfileKind,
) -> Option<String> {
    // parse the source program
    let (mut tree, strings) =
        mir::parse::Parser::parse(FileId::new(0), program.source, ParseOptions::default())
            .validate()
            .ok()?;

    // build the pipeline context
    let strings_pool = StringPool::new();
    strings_pool.copy_from_immutable(&strings);
    let module_id = ModuleId::from_relative_path(
        package_id,
        &PathBuf::from(format!("bench/{}.mir", program.name)),
    );
    let mut ctx = PipelineContext::new(&strings_pool, options.clone(), module_id, *target_id, None);

    // run each pass in order using a pass major traversal
    let pipeline = default_pipeline(level, options.target);
    diagnose_pipeline(
        program,
        &mut tree,
        &strings_pool,
        &mut ctx,
        &pipeline,
        baseline_output,
        max_instruction_limit,
        profile,
        0,
        None,
    )
    .err()
}

/// Format a diagnostic mismatch message.
fn format_pass_context(depth: usize, iteration: Option<(usize, usize)>, pass_name: &str) -> String {
    // format context with optional iteration
    if let Some((iteration, max_iterations)) = iteration {
        format!("{depth} iter {iteration}/{max_iterations} pass '{pass_name}'")
    } else {
        format!("{depth} pass '{pass_name}'")
    }
}

/// Build a label for pass timing entries.
fn build_pass_label(path: &[String], pass_name: &str) -> String {
    if path.is_empty() {
        pass_name.to_string()
    } else {
        format!("{}/{}", path.join(" / "), pass_name)
    }
}

/// Walk a pipeline and return whether any changes occurred.
#[allow(clippy::too_many_arguments)]
fn diagnose_pipeline(
    program: &program::Program,
    tree: &mut mir::NodeTree,
    strings_pool: &StringPool,
    ctx: &mut PipelineContext<'_>,
    pipeline: &dyn Pipeline,
    baseline_output: &ExecutionOutput,
    max_instruction_limit: Option<u64>,
    profile: program::BenchProfileKind,
    depth: usize,
    iteration: Option<(usize, usize)>,
) -> Result<bool, String> {
    // handle composite pipelines
    if let Some(composite) = pipeline.as_any().downcast_ref::<CompositePipeline>() {
        let mut any_changed = false;

        for child in composite.pipelines() {
            let changed = diagnose_pipeline(
                program,
                tree,
                strings_pool,
                ctx,
                child.as_ref(),
                baseline_output,
                max_instruction_limit,
                profile,
                depth,
                iteration,
            )?;
            any_changed |= changed;
        }

        return Ok(any_changed);
    }

    // handle repeated pipelines
    if let Some(repeat) = pipeline.as_any().downcast_ref::<RepeatedPipeline>() {
        let mut any_changed = false;
        let max_iterations = repeat.max_iterations();

        for iteration_index in 0..max_iterations {
            let iteration = (iteration_index + 1, max_iterations);
            let changed = diagnose_pipeline(
                program,
                tree,
                strings_pool,
                ctx,
                repeat.inner(),
                baseline_output,
                max_instruction_limit,
                profile,
                depth + 1,
                Some(iteration),
            )?;
            any_changed |= changed;

            if !changed {
                break;
            }
        }

        return Ok(any_changed);
    }

    // handle function pipelines
    if let Some(function_pipeline) = pipeline.as_any().downcast_ref::<FunctionPipeline>() {
        let mut any_changed = false;

        for pass in function_pipeline.passes() {
            let changed = run_function_pass(pass.as_ref(), tree, ctx);
            any_changed |= changed;

            let context = format_pass_context(depth, iteration, pass.name());
            run_and_compare_output(
                program,
                tree.clone(),
                strings_pool.clone().into_immutable(),
                baseline_output,
                max_instruction_limit,
                profile,
                &context,
            )?;
        }

        return Ok(any_changed);
    }

    // handle function adaptor pipelines
    if let Some(adaptor) = pipeline.as_any().downcast_ref::<FunctionToModuleAdaptor>() {
        return diagnose_pipeline(
            program,
            tree,
            strings_pool,
            ctx,
            adaptor.inner(),
            baseline_output,
            max_instruction_limit,
            profile,
            depth,
            iteration,
        );
    }

    // handle module pipelines
    if let Some(module_pipeline) = pipeline.as_any().downcast_ref::<ModulePipeline>() {
        let mut any_changed = false;

        for pass in module_pipeline.passes() {
            let changed = run_module_pass(pass.as_ref(), tree, ctx);
            any_changed |= changed;

            let context = format_pass_context(depth, iteration, pass.name());
            run_and_compare_output(
                program,
                tree.clone(),
                strings_pool.clone().into_immutable(),
                baseline_output,
                max_instruction_limit,
                profile,
                &context,
            )?;
        }

        return Ok(any_changed);
    }

    Err(format!(
        "bench '{}' encountered unsupported pipeline '{}'",
        program.name,
        pipeline.name()
    ))
}

/// Run a coroutine program with a resume handler.
fn run_coroutine(
    isolate: &mut Isolate,
    heap: &mut Heap,
    shared: &mut SharedSpace,
    entry_id: mir::LocalNodeId<mir::Function>,
    args: &[Value],
    resume_value: fn(args: &[Value], yield_index: usize, yielded: Value) -> Value,
) -> RuntimeResult<ExecutionOutput> {
    // start execution
    let mut memory = MemoryContext::new(heap, shared);
    let mut outcome = isolate.run_function_yielding(&mut memory, entry_id, args)?;
    let mut yield_index = 0usize;

    // continue until completion
    loop {
        // return on completion or resume after one yield
        match outcome {
            ExecutionOutcome::Completed { output } => return Ok(output),
            ExecutionOutcome::Yielded { yielded } => {
                let resume = resume_value(args, yield_index, yielded.value);
                yield_index += 1;

                let mut memory = MemoryContext::new(heap, shared);
                outcome = isolate.resume(&mut memory, yielded.continuation, resume)?;
            }
        }
    }
}

/// Execute a program using an already optimized tree.
fn run_program_with_tree_result(
    program: &program::Program,
    tree: mir::NodeTree,
    strings: ImmutableStringPool,
    max_instruction_limit: Option<u64>,
    profile: program::BenchProfileKind,
) -> RuntimeResult<ExecutionOutput> {
    // configure an isolate like the bench harness
    let mut options = IsolateOptions::unbounded();
    options.limits.max_stack_depth = 4096;
    options.limits.max_managed_allocations = 5_000_000;
    options.limits.max_raw_allocations = 5_000_000;

    // apply instruction limits when requested
    if let Some(limit) = max_instruction_limit {
        options.limits.max_instructions = Some(limit);
    }

    // build isolate and arguments
    let mut isolate = Isolate::build_with_options(tree, strings, options)?;
    let mut heap = Heap::new();
    let mut shared = SharedSpace::new();
    let mut memory = MemoryContext::new(&mut heap, &mut shared);
    isolate.initialize(&mut memory)?;
    let args = program.args_for_profile(&isolate, profile);

    // execute using the requested runner
    run_program_with_isolate(program, &mut isolate, &mut heap, &mut shared, &args)
}

/// Execute a program by cloning the provided tree and strings.
fn run_program_with_tree_profile(
    program: &program::Program,
    tree: &mir::NodeTree,
    strings: &ImmutableStringPool,
    max_instruction_limit: Option<u64>,
    profile: program::BenchProfileKind,
) -> RuntimeResult<ExecutionOutput> {
    run_program_with_tree_result(
        program,
        tree.clone(),
        strings.clone(),
        max_instruction_limit,
        profile,
    )
}

/// Execute a program using default arguments.
fn run_program_with_tree_result_default_args(
    program: &program::Program,
    tree: mir::NodeTree,
    strings: ImmutableStringPool,
    max_instruction_limit: Option<u64>,
) -> RuntimeResult<ExecutionOutput> {
    // configure an isolate like the bench harness
    let mut options = IsolateOptions::unbounded();
    options.limits.max_stack_depth = 4096;
    options.limits.max_managed_allocations = 5_000_000;
    options.limits.max_raw_allocations = 5_000_000;

    // apply instruction limits when requested
    if let Some(limit) = max_instruction_limit {
        options.limits.max_instructions = Some(limit);
    }

    // build isolate and arguments
    let mut isolate = Isolate::build_with_options(tree, strings, options)?;
    let mut heap = Heap::new();
    let mut shared = SharedSpace::new();
    let mut memory = MemoryContext::new(&mut heap, &mut shared);
    isolate.initialize(&mut memory)?;
    let args = (program.default_args)(&isolate);

    // execute using the requested runner
    run_program_with_isolate(program, &mut isolate, &mut heap, &mut shared, &args)
}

/// Execute a program using an isolate and explicit arguments.
fn run_program_with_isolate(
    program: &program::Program,
    isolate: &mut Isolate,
    heap: &mut Heap,
    shared: &mut SharedSpace,
    args: &[Value],
) -> RuntimeResult<ExecutionOutput> {
    // resolve entry id
    let entry_id = isolate
        .function_id_by_name(program.entry)
        .unwrap_or_else(|_| panic!("function '{}' not found", program.entry));

    // execute based on runner configuration
    match program.runner {
        program::ProgramRunner::Function => {
            let mut memory = MemoryContext::new(heap, shared);
            isolate.run_function(&mut memory, entry_id, args)
        }
        program::ProgramRunner::Coroutine { resume_value } => {
            run_coroutine(isolate, heap, shared, entry_id, args, resume_value)
        }
    }
}

/// Build a detailed failure message for execute mismatches.
fn build_execute_failure(
    program: &program::Program,
    level: OptimizationLevel,
    package_id: PackageId,
    target_id: &TargetId,
    options: &PipelineOptions,
    baseline_output: &ExecutionOutput,
    run_options: &OptimizeRunOptions,
    message: String,
) -> String {
    // run individual pass cases in matrix mode
    if run_options.matrix {
        let pipeline = default_pipeline(level, options.target);
        let mut path = Vec::new();
        if let Some(reason) = run_matrix_pipeline(
            program,
            package_id,
            target_id,
            options,
            baseline_output,
            run_options.max_instruction_limit,
            run_options.bench_profile,
            &pipeline,
            &mut path,
        ) {
            return reason;
        }
    }

    // run prefix diagnostics when enabled or requested by the matrix
    if (run_options.diagnostic || run_options.matrix)
        && let Some(reason) = diagnose_mismatch(
            program,
            level,
            package_id,
            target_id,
            options,
            baseline_output,
            run_options.max_instruction_limit,
            run_options.bench_profile,
        )
    {
        return reason;
    }

    message
}
