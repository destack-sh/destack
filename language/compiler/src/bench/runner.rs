use std::collections::{BTreeMap, HashSet};
use std::time::{Duration, Instant};

use destack_builtin::{BuiltinLib, BuiltinLibKind, LIBS, STD_LIB};
use destack_source::{DiagnosticSeverity, ModuleId};

use crate::{StatsSnapshot, TaskPhase, default_workers};

use super::output::{format_duration, output_results};
use super::program::BenchProgram;

/// Timing data for a builtin lib run.
#[derive(Debug, Clone)]
pub(crate) struct LibTiming {
    /// The builtin lib name.
    pub(crate) name: String,
    /// The number of modules in the lib.
    pub(crate) module_count: usize,
    /// The total number of lines in the lib.
    pub(crate) total_lines: usize,
    /// The minimum line count across modules.
    pub(crate) min_lines: usize,
    /// The maximum line count across modules.
    pub(crate) max_lines: usize,
    /// The mean line count across modules.
    pub(crate) mean_lines: f64,
    /// The time spent importing sources.
    pub(crate) import: Duration,
    /// The time spent resolving builtins and libs.
    pub(crate) resolve: Duration,
    /// The time spent analyzing the lib modules.
    pub(crate) analyze: Duration,
    /// The total elapsed time for the lib run.
    pub(crate) total: Duration,
}

/// Output format for bench results.
#[derive(Debug, Clone, Copy)]
pub enum BenchOutputFormat {
    /// Human-readable table output.
    Table,
    /// Machine readable json output.
    Json,
    /// Machine readable csv output.
    Csv,
}

/// Execution mode for bench runs.
#[derive(Debug, Clone, Copy)]
pub enum BenchMode {
    /// Run tasks sequentially.
    Sequential,
    /// Run tasks in parallel.
    Parallel,
}

impl BenchMode {
    /// Return the label used for reporting.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Sequential => "sequential",
            Self::Parallel => "parallel",
        }
    }
}

/// Benchmark run type.
#[derive(Debug, Clone, Copy)]
pub enum BenchRun {
    /// Resolve declared lib symbols for all builtin libs.
    ResolveSymbols,
    /// Resolve all builtin libs without analysis.
    ResolveAll,
    /// Analyze builtin libs with fast settings.
    AnalyzeFast,
    /// Analyze builtin libs with full validation.
    AnalyzeFull,
    /// Analyze a combined lib set in one program.
    AnalyzeCombined,
    /// List builtin lib modules in dependency order.
    ListModules,
}

impl BenchRun {
    /// Return the default timeout for this run.
    fn default_timeout(self) -> Duration {
        match self {
            Self::AnalyzeFull => Duration::from_secs(300),
            Self::ListModules => Duration::from_secs(30),
            _ => Duration::from_secs(60),
        }
    }

    /// Return the default validation mode for this run.
    fn default_validate(self) -> bool {
        matches!(self, Self::AnalyzeFull)
    }
}

/// Options for compiler bench runs.
#[derive(Debug, Clone)]
pub struct BenchOptions {
    /// The run type to execute.
    pub run: BenchRun,
    /// The execution mode for the run.
    pub mode: BenchMode,
    /// Whether to report per lib timing details.
    pub report_timings: bool,
    /// How many timing entries to show in summaries.
    pub report_top_n: usize,
    /// Optional CSV output path for timing summaries.
    pub csv_path: Option<String>,
    /// Timeout to use for each compile phase.
    pub timeout: Duration,
    /// Whether to validate builtin libs during analysis.
    pub validate_builtin_libs: bool,
    /// Optional lib filter for timing runs.
    pub lib_filter: Option<HashSet<String>>,
    /// Optional combined lib list for combined runs.
    pub combined_libs: Option<Vec<String>>,
    /// Output format for bench results.
    pub output: BenchOutputFormat,
    /// Whether to emit ANSI color output.
    pub color: bool,
}

impl BenchOptions {
    /// Build bench options with environment defaults applied.
    pub fn new(run: BenchRun, mode: BenchMode) -> Self {
        // timings env
        let timings_env = std::env::var("DESTACK_TIMINGS").ok();

        // report settings
        let report_timings = timings_env
            .as_deref()
            .map(|value| value != "0")
            .unwrap_or(true);

        // top summary settings
        let report_top_n = std::env::var("DESTACK_TIMINGS_TOP")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(15);

        // csv output settings
        let csv_path = std::env::var("DESTACK_TIMINGS_CSV").ok();

        // lib filter settings
        let lib_filter = if timings_env.is_some() {
            builtin_lib_filter_from_env()
        } else {
            None
        };

        // timeout settings
        let mut timeout = run.default_timeout();
        if timings_env.is_some() {
            timeout = timeout.mul_f64(3.0);
        }

        BenchOptions {
            run,
            mode,
            report_timings,
            report_top_n,
            csv_path,
            timeout,
            validate_builtin_libs: run.default_validate(),
            lib_filter,
            combined_libs: None,
            output: BenchOutputFormat::Table,
            color: std::env::var("NO_COLOR").is_err(),
        }
    }

    /// Return the effective timeout for this run.
    pub(crate) fn effective_timeout(&self) -> Duration {
        if matches!(self.mode, BenchMode::Sequential) {
            let scale = default_workers().max(1) as f64;
            return self.timeout.mul_f64(scale);
        }

        self.timeout
    }
}

/// Execute the bench run described by the options.
pub fn run_bench(options: &BenchOptions) {
    match options.run {
        BenchRun::ResolveSymbols => run_resolve_builtin_lib_symbols(options.mode),
        BenchRun::ResolveAll => run_resolve_all_builtin_libs(options.mode),
        BenchRun::AnalyzeFast => run_analyze_all_builtin_libs_fast(options),
        BenchRun::AnalyzeFull => run_analyze_all_builtin_libs_full(options),
        BenchRun::AnalyzeCombined => run_analyze_builtin_libs_combined(options),
        BenchRun::ListModules => run_list_builtin_lib_modules(options),
    }
}

/// Resolve declared lib symbols for all builtin libs.
pub(crate) fn run_resolve_builtin_lib_symbols(mode: BenchMode) {
    // resolve declared lib symbols for all builtin libs
    for lib in LIBS.iter() {
        let test = test_program_for_mode(mode).with_profile_libs(&[lib.name]);
        test.resolve_language_environment();
        test.resolve_libs();
        test.compile();
        test.check_no_diagnostics_up_to_including_phase(TaskPhase::Resolve);

        let profile = test.default_profile_id_for_root();
        for &symbol in lib.declared_symbols {
            let name_id = test.program.strings.intern(symbol);
            let declared_symbol = test.compiler.get_declared_lib_symbol(profile, name_id);
            assert!(
                declared_symbol.is_some(),
                "missing declared lib symbol {}:{}",
                lib.name,
                symbol
            );
        }
    }
}

/// Resolve all builtin libs in one program.
pub(crate) fn run_resolve_all_builtin_libs(mode: BenchMode) {
    // collect all lib names
    let lib_names: Vec<&str> = std::iter::once(&STD_LIB)
        .chain(LIBS.iter())
        .map(|lib| lib.name)
        .collect();

    // resolve all libs in one program
    let test = test_program_for_mode(mode).with_profile_libs(&lib_names);
    test.resolve_language_environment();
    test.resolve_libs();
    test.compile();
}

/// Run the fast builtin lib analyze timing suite.
fn run_analyze_all_builtin_libs_fast(options: &BenchOptions) {
    // run per lib timing passes
    run_builtin_libs_per_lib(options);
}

/// Run the full builtin lib analyze timing suite.
fn run_analyze_all_builtin_libs_full(options: &BenchOptions) {
    // run per lib timing passes
    run_builtin_libs_per_lib(options);
}

/// Run the combined builtin lib analyze timing suite.
fn run_analyze_builtin_libs_combined(options: &BenchOptions) {
    // run combined timing pass
    run_builtin_libs_combined(options);
}

/// Build a test program with the requested execution mode.
fn test_program_for_mode(mode: BenchMode) -> BenchProgram {
    match mode {
        BenchMode::Sequential => BenchProgram::new(1, true, true),
        BenchMode::Parallel => BenchProgram::new(default_workers(), true, true),
    }
}

/// Run builtin lib analysis once per builtin lib.
fn run_builtin_libs_per_lib(options: &BenchOptions) {
    // collect per lib timings
    let mut timings = Vec::new();
    let lib_filter = options.lib_filter.as_ref();
    let timeout = options.effective_timeout();

    for lib in std::iter::once(&STD_LIB).chain(LIBS.iter()) {
        // apply optional lib filter
        if let Some(filter) = lib_filter
            && !filter.contains(lib.name)
        {
            continue;
        }

        let lib_start = Instant::now();
        let libs = libs_for_builtin(lib);

        let mut test = test_program_for_mode(options.mode).with_profile_libs(&libs);
        if options.validate_builtin_libs {
            test = test.with_options_mut(|options| options.validate_builtin_libs = true);
        }

        // import lib modules
        let (import_modules, import_duration) = import_lib_modules(&test, &libs, timeout);
        let line_stats = collect_line_stats(&test, &import_modules);

        // resolve builtins and libs
        let resolve_duration = resolve_builtins_and_libs(&test, timeout);
        let analyze_duration = analyze_lib_modules(&test, &import_modules, timeout);

        timings.push(LibTiming {
            name: lib.name.to_string(),
            module_count: line_stats.module_count,
            total_lines: line_stats.total_lines,
            min_lines: line_stats.min_lines,
            max_lines: line_stats.max_lines,
            mean_lines: line_stats.mean_lines,
            import: import_duration,
            resolve: resolve_duration,
            analyze: analyze_duration,
            total: lib_start.elapsed(),
        });

        // report per lib timing details
        if options.report_timings {
            eprintln!(
                "builtin lib {}: import={} resolve={} analyze={} total={}",
                lib.name,
                format_duration(import_duration),
                format_duration(resolve_duration),
                format_duration(analyze_duration),
                format_duration(lib_start.elapsed())
            );

            let snapshot = test
                .compiler
                .stats
                .snapshot_with_program(test.program.modules.len(), Some(&test.program));
            report_timing_tag_summary(&snapshot, options.report_top_n);
        }
    }

    // emit output summary
    output_results(&timings, options);
}

/// Build the timing lib filter from environment variables or defaults.
fn builtin_lib_filter_from_env() -> Option<HashSet<String>> {
    // read env list
    let env_list = std::env::var("DESTACK_TIMINGS_LIBS").ok().map(|value| {
        value
            .split(',')
            .filter_map(|entry| {
                let trimmed = entry.trim();
                if trimmed.is_empty() {
                    return None;
                }
                Some(trimmed.to_string())
            })
            .collect::<HashSet<_>>()
    });

    // return env list when present
    if let Some(list) = env_list
        && !list.is_empty()
    {
        return Some(list);
    }

    // default to a compact lib subset
    let mut defaults = HashSet::new();
    defaults.insert("std".to_string());
    defaults.insert("globals".to_string());
    defaults.insert("dom".to_string());
    defaults.insert("es2020.full".to_string());
    Some(defaults)
}

/// Resolve the combined lib list for timing runs.
fn builtin_lib_list_from_env() -> Vec<String> {
    // resolve the combined lib list
    std::env::var("DESTACK_TIMINGS_LIBS")
        .ok()
        .map(|value| {
            value
                .split(',')
                .filter_map(|entry| {
                    let trimmed = entry.trim();
                    if trimmed.is_empty() {
                        return None;
                    }
                    Some(trimmed.to_string())
                })
                .collect()
        })
        .filter(|value: &Vec<String>| !value.is_empty())
        .unwrap_or_else(|| vec!["dom".to_string(), "esnext".to_string()])
}

/// Report per timing tag results for quick debugging.
fn report_timing_tag_summary(snapshot: &StatsSnapshot, top_n: usize) {
    // report the slowest timing tags by total time
    let sample_count = top_n.min(snapshot.timings.len());
    if sample_count == 0 {
        return;
    }

    eprintln!("builtin lib timing tags: top {sample_count} by total");
    for entry in snapshot.timings.iter().take(sample_count) {
        eprintln!(
            "  {:<32} total={} count={}",
            entry.name,
            format_duration(entry.duration),
            entry.sample_count
        );
    }
}

/// Build the lib list for a builtin lib run.
fn libs_for_builtin(lib: &BuiltinLib) -> Vec<&'static str> {
    // include a baseline es lib for runtime libraries that require them
    let mut libs = Vec::new();
    if lib.kind == BuiltinLibKind::Lib
        && !lib.name.starts_with("es")
        && !lib.name.starts_with("decorators")
    {
        libs.push("es2020");
    }
    libs.push(lib.name);

    libs
}

/// Import lib modules and return module ids plus elapsed time.
fn import_lib_modules(
    test: &BenchProgram,
    libs: &[&str],
    timeout: Duration,
) -> (Vec<ModuleId>, Duration) {
    // import lib modules
    let import_start = Instant::now();
    let import_modules = test
        .compiler
        .load_lib_modules_for_bench(libs)
        .unwrap_or_else(|error| panic!("failed to load lib modules for bench: {error}"));
    for module_id in &import_modules {
        test.import_module(*module_id);
    }
    compile_and_check(test, timeout);

    (import_modules, import_start.elapsed())
}

/// Resolve builtins and libs for a test program.
fn resolve_builtins_and_libs(test: &BenchProgram, timeout: Duration) -> Duration {
    // resolve builtins and libs with timing
    let resolve_start = Instant::now();
    test.resolve_language_environment();
    test.resolve_libs();
    compile_and_check(test, timeout);
    resolve_start.elapsed()
}

/// Analyze imported lib modules for a test program.
fn analyze_lib_modules(test: &BenchProgram, modules: &[ModuleId], timeout: Duration) -> Duration {
    // analyze each module once
    let analyze_start = Instant::now();
    let mut seen_modules = HashSet::new();
    for module_id in modules {
        if seen_modules.insert(*module_id) {
            test.analyze_module(*module_id);
        }
    }
    compile_and_check(test, timeout);
    analyze_start.elapsed()
}

#[derive(Debug, Clone, Copy)]
/// Line count statistics for a module set.
struct LineStats {
    /// The total number of modules.
    module_count: usize,
    /// The total number of lines across modules.
    total_lines: usize,
    /// The minimum lines in a module.
    min_lines: usize,
    /// The maximum lines in a module.
    max_lines: usize,
    /// The mean line count across modules.
    mean_lines: f64,
}

/// Collect line count statistics for a module list.
fn collect_line_stats(test: &BenchProgram, modules: &[ModuleId]) -> LineStats {
    let mut seen_modules = HashSet::new();
    let mut line_counts = Vec::new();

    for module_id in modules {
        if !seen_modules.insert(*module_id) {
            continue;
        }

        let module_ref = test.program.modules.get(*module_id);
        let module = module_ref.as_ref();
        let Some(file) = test.program.files.get_maybe(module.file_id) else {
            line_counts.push(0);
            continue;
        };
        line_counts.push(file.line_count() as usize);
    }

    let module_count = line_counts.len();
    let total_lines = line_counts.iter().sum();
    let min_lines = line_counts.iter().copied().min().unwrap_or(0);
    let max_lines = line_counts.iter().copied().max().unwrap_or(0);
    let mean_lines = if module_count == 0 {
        0.0
    } else {
        total_lines as f64 / module_count as f64
    };

    LineStats {
        module_count,
        total_lines,
        min_lines,
        max_lines,
        mean_lines,
    }
}

/// Compile queued tasks and assert no diagnostics.
fn compile_and_check(test: &BenchProgram, timeout: Duration) {
    // compile with timeout
    test.compile_with_timeout(timeout);

    // verify diagnostics
    test.check_no_diagnostic(DiagnosticSeverity::Note);
}

/// Run a combined builtin lib timing pass.
fn run_builtin_libs_combined(options: &BenchOptions) {
    // resolve the combined lib list
    let mut libs = if let Some(list) = options.combined_libs.as_ref() {
        list.clone()
    } else {
        builtin_lib_list_from_env()
    };

    // ensure std and globals are always present
    if !libs.iter().any(|name| name == "std") {
        libs.push("std".to_string());
    }
    if !libs.iter().any(|name| name == "globals") {
        libs.push("globals".to_string());
    }

    // include a baseline es lib for runtime libraries that require them
    let has_es = libs.iter().any(|name| name.starts_with("es"));
    let has_decorators = libs.iter().any(|name| name.starts_with("decorators"));
    if !has_es && !has_decorators {
        libs.push("es2020".to_string());
    }

    // run the combined timing pass
    let lib_refs: Vec<&str> = libs.iter().map(|name| name.as_str()).collect();
    let test = test_program_for_mode(options.mode).with_profile_libs(&lib_refs);
    let timeout = options.effective_timeout();

    let (import_modules, import_duration) = import_lib_modules(&test, &lib_refs, timeout);
    let line_stats = collect_line_stats(&test, &import_modules);

    // resolve builtins and libs
    let resolve_duration = resolve_builtins_and_libs(&test, timeout);
    let analyze_duration = analyze_lib_modules(&test, &import_modules, timeout);

    let label = format!("combined({})", libs.join(","));
    let timing = LibTiming {
        name: label,
        module_count: line_stats.module_count,
        total_lines: line_stats.total_lines,
        min_lines: line_stats.min_lines,
        max_lines: line_stats.max_lines,
        mean_lines: line_stats.mean_lines,
        import: import_duration,
        resolve: resolve_duration,
        analyze: analyze_duration,
        total: import_duration + resolve_duration + analyze_duration,
    };

    // report timing output
    if options.report_timings {
        eprintln!(
            "builtin libs combined {}: import={} resolve={} analyze={} total={}",
            timing.name,
            format_duration(timing.import),
            format_duration(timing.resolve),
            format_duration(timing.analyze),
            format_duration(timing.total)
        );
    }

    output_results(std::slice::from_ref(&timing), options);
}

/// List builtin lib modules in dependency order.
fn run_list_builtin_lib_modules(options: &BenchOptions) {
    // resolve the combined lib list
    let mut libs = if let Some(list) = options.combined_libs.as_ref() {
        list.clone()
    } else {
        builtin_lib_list_from_env()
    };

    // ensure std and globals are always present
    if !libs.iter().any(|name| name == "std") {
        libs.push("std".to_string());
    }
    if !libs.iter().any(|name| name == "globals") {
        libs.push("globals".to_string());
    }

    // include a baseline es lib for runtime libraries that require them
    let has_es = libs.iter().any(|name| name.starts_with("es"));
    let has_decorators = libs.iter().any(|name| name.starts_with("decorators"));
    if !has_es && !has_decorators {
        libs.push("es2020".to_string());
    }

    let lib_refs: Vec<&str> = libs.iter().map(|name| name.as_str()).collect();
    let test = test_program_for_mode(options.mode).with_profile_libs(&lib_refs);
    let timeout = options.effective_timeout();

    let (import_modules, _import_duration) = import_lib_modules(&test, &lib_refs, timeout);

    let mut entries = Vec::new();
    let mut lib_stats = BTreeMap::new();
    let mut histogram = LineHistogram::new();
    let mut seen_modules = HashSet::new();
    for module_id in import_modules {
        if !seen_modules.insert(module_id) {
            continue;
        }

        let module_ref = test.program.modules.get(module_id);
        let module = module_ref.as_ref();
        let display = module
            .path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| module.uri.to_string());
        let line_count = test
            .program
            .files
            .get_maybe(module.file_id)
            .map(|file| file.line_count() as usize)
            .unwrap_or(0);
        let label = builtin_lib_label_for_module(&display);
        lib_stats
            .entry(label)
            .or_insert_with(LibLineStats::default)
            .push(line_count);
        histogram.observe(line_count);
        entries.push((display, line_count));
    }

    entries.sort_by(|left, right| left.0.cmp(&right.0));

    println!(
        "builtin lib modules: {} (libs: {})",
        entries.len(),
        libs.join(", ")
    );

    println!("line stats by lib:");
    for (label, stats) in lib_stats {
        let mean_lines = stats.mean_lines();
        println!(
            "  {label}: modules={} lines={} min={} max={} mean={mean_lines:.1}",
            stats.modules, stats.total_lines, stats.min_lines, stats.max_lines
        );
    }

    println!("per-module line histogram:");
    for bucket in histogram.buckets() {
        println!("  {:>8}: {}", bucket.label, bucket.count);
    }

    for (entry, line_count) in entries {
        println!("- {entry} ({line_count} lines)");
    }
}

#[derive(Default)]
struct LibLineStats {
    modules: usize,
    total_lines: usize,
    min_lines: usize,
    max_lines: usize,
}

impl LibLineStats {
    fn push(&mut self, lines: usize) {
        if self.modules == 0 {
            self.min_lines = lines;
            self.max_lines = lines;
        } else {
            self.min_lines = self.min_lines.min(lines);
            self.max_lines = self.max_lines.max(lines);
        }
        self.modules += 1;
        self.total_lines += lines;
    }

    fn mean_lines(&self) -> f64 {
        if self.modules == 0 {
            0.0
        } else {
            self.total_lines as f64 / self.modules as f64
        }
    }
}

struct LineHistogram {
    buckets: Vec<LineBucket>,
}

impl LineHistogram {
    fn new() -> Self {
        Self {
            buckets: vec![
                LineBucket::new("0-9", 9),
                LineBucket::new("10-49", 49),
                LineBucket::new("50-99", 99),
                LineBucket::new("100-499", 499),
                LineBucket::new("500-999", 999),
                LineBucket::new("1000-4999", 4_999),
                LineBucket::new("5000+", usize::MAX),
            ],
        }
    }

    fn observe(&mut self, lines: usize) {
        for bucket in &mut self.buckets {
            if lines <= bucket.max {
                bucket.count += 1;
                break;
            }
        }
    }

    fn buckets(&self) -> &[LineBucket] {
        &self.buckets
    }
}

struct LineBucket {
    label: &'static str,
    max: usize,
    count: usize,
}

impl LineBucket {
    fn new(label: &'static str, max: usize) -> Self {
        Self {
            label,
            max,
            count: 0,
        }
    }
}

fn builtin_lib_label_for_module(display: &str) -> String {
    let Some(rest) = display.strip_prefix("builtin://") else {
        return "other".to_string();
    };

    let mut segments = rest.split('/');
    let Some(head) = segments.next() else {
        return "other".to_string();
    };
    if head == "std" {
        return "std".to_string();
    }
    if head == "globals" {
        return "globals".to_string();
    }
    if head != "lib" {
        return head.to_string();
    }

    let Some(lib_name) = segments.next() else {
        return "lib".to_string();
    };
    format!("lib/{lib_name}")
}
