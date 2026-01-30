use std::collections::HashSet;
use std::time::{Duration, Instant};

use destack_builtin::{BuiltinLib, BuiltinLibKind, LIBS, STD_LIB};
use destack_source::{DiagnosticSeverity, ModuleId};

use crate::{StatsSnapshot, TaskPhase, TestProgram};

/// Configuration for builtin lib timing runs.
#[derive(Debug, Clone)]
struct BuiltinTimingConfig {
    /// Whether to report per lib timing details.
    report_timings: bool,
    /// How many timing entries to show in summaries.
    report_top_n: usize,
    /// Optional CSV output path for timing summaries.
    csv_path: Option<String>,
    /// Timeout to use for each compile phase.
    timeout: Duration,
    /// Whether to validate builtin libs during analysis.
    validate_builtin_libs: bool,
}

/// Capture timing data for a builtin lib run.
#[derive(Debug, Clone)]
struct LibTiming {
    /// The builtin lib name.
    name: String,
    /// Time spent importing sources.
    import: Duration,
    /// Time spent resolving builtins and libs.
    resolve: Duration,
    /// Time spent analyzing the lib modules.
    analyze: Duration,
    /// Total elapsed time for the lib run.
    total: Duration,
}

/// Build timing configuration for builtin lib tests.
fn builtin_timing_config(timeout: Duration, validate_builtin_libs: bool) -> BuiltinTimingConfig {
    // timings
    let report_timings = std::env::var("DESTACK_TIMINGS")
        .ok()
        .map(|value| value != "0")
        .unwrap_or(true);
    let report_top_n = std::env::var("DESTACK_TIMINGS_TOP")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(15);
    let csv_path = std::env::var("DESTACK_TIMINGS_CSV").ok();

    BuiltinTimingConfig {
        report_timings,
        report_top_n,
        csv_path,
        timeout,
        validate_builtin_libs,
    }
}

/// Run builtin lib analysis once per builtin lib.
fn run_builtin_libs_per_lib(config: &BuiltinTimingConfig) {
    // collect per lib timings
    let mut timings = Vec::new();

    for lib in std::iter::once(&STD_LIB).chain(LIBS.iter()) {
        let lib_start = Instant::now();
        let libs = libs_for_builtin(lib);

        let mut test =
            TestProgram::memory_sequential_with_prelude_and_libs().with_profile_libs(&libs);
        if config.validate_builtin_libs {
            test = test.with_options_mut(|options| options.validate_builtin_libs = true);
        }

        let (import_modules, import_duration) = import_lib_modules(&test, &libs, config.timeout);

        // resolve builtins and libs
        let resolve_duration = resolve_builtins_and_libs(&test, config.timeout);
        let analyze_duration = analyze_lib_modules(&test, &import_modules, config.timeout);

        timings.push(LibTiming {
            name: lib.name.to_string(),
            import: import_duration,
            resolve: resolve_duration,
            analyze: analyze_duration,
            total: lib_start.elapsed(),
        });

        if config.report_timings {
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
            report_timing_tag_summary(&snapshot, config.report_top_n);
        }
    }

    if config.report_timings {
        report_timing_summary(&timings, config.report_top_n);
    }

    if let Some(path) = &config.csv_path
        && let Err(error) = write_timings_csv(&timings, path)
    {
        eprintln!("timings: failed to write csv to {path}: {error}");
    }
}

/// Resolve the combined builtin lib list for timing tests.
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

/// Report timing summary information to stderr.
fn report_timing_summary(timings: &[LibTiming], top_n: usize) {
    // sort by total time to find the slowest libs
    let mut sorted = timings.to_vec();
    sorted.sort_by_key(|entry| std::cmp::Reverse(entry.total));

    // aggregate total time by phase
    let total_import = timings
        .iter()
        .fold(Duration::ZERO, |acc, entry| acc + entry.import);
    let total_resolve = timings
        .iter()
        .fold(Duration::ZERO, |acc, entry| acc + entry.resolve);
    let total_analyze = timings
        .iter()
        .fold(Duration::ZERO, |acc, entry| acc + entry.analyze);
    let total_all = timings
        .iter()
        .fold(Duration::ZERO, |acc, entry| acc + entry.total);

    eprintln!(
        "builtin lib timings: total import={} resolve={} analyze={} total={}",
        format_duration(total_import),
        format_duration(total_resolve),
        format_duration(total_analyze),
        format_duration(total_all)
    );

    // report the slowest libs by total time
    let sample_count = top_n.min(sorted.len());
    if sample_count == 0 {
        return;
    }

    eprintln!("builtin lib timings: top {sample_count} by total");
    for entry in sorted.iter().take(sample_count) {
        eprintln!(
            "  {:<24} import={} resolve={} analyze={} total={}",
            entry.name,
            format_duration(entry.import),
            format_duration(entry.resolve),
            format_duration(entry.analyze),
            format_duration(entry.total)
        );
    }
}

/// Write timing results to a CSV file.
fn write_timings_csv(timings: &[LibTiming], path: &str) -> std::io::Result<()> {
    // build CSV output in memory
    let mut output = String::new();
    output.push_str("lib,import_ms,resolve_ms,analyze_ms,total_ms\n");
    for entry in timings {
        output.push_str(&format!(
            "{},{:.3},{:.3},{:.3},{:.3}\n",
            entry.name,
            entry.import.as_secs_f64() * 1000.0,
            entry.resolve.as_secs_f64() * 1000.0,
            entry.analyze.as_secs_f64() * 1000.0,
            entry.total.as_secs_f64() * 1000.0
        ));
    }

    // write CSV output to disk
    std::fs::write(path, output)
}

/// Format a duration for readable timing output.
fn format_duration(duration: Duration) -> String {
    // pick a human readable unit
    let ms = duration.as_secs_f64() * 1000.0;
    if ms >= 1000.0 {
        return format!("{:.3}s", ms / 1000.0);
    }

    format!("{ms:.3}ms")
}

/// Report per-task timing results for quick debugging.
fn report_task_name_summary(snapshot: &StatsSnapshot, top_n: usize) {
    // report the slowest tasks by total time
    let sample_count = top_n.min(snapshot.task_names.len());
    if sample_count == 0 {
        return;
    }

    eprintln!("builtin lib task timings: top {sample_count} by total");
    for entry in snapshot.task_names.iter().take(sample_count) {
        eprintln!(
            "  {:<32} total={} count={}",
            entry.name,
            format_duration(entry.duration),
            entry.task_count
        );
    }
}

/// Report per-timing tag results for quick debugging.
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
    test: &TestProgram,
    libs: &[&str],
    timeout: Duration,
) -> (Vec<ModuleId>, Duration) {
    // collect and import lib modules
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
fn resolve_builtins_and_libs(test: &TestProgram, timeout: Duration) -> Duration {
    // resolve builtins and libs with timing
    let resolve_start = Instant::now();
    test.resolve_builtins();
    test.resolve_libs();
    compile_and_check(test, timeout);
    resolve_start.elapsed()
}

/// Analyze imported lib modules for a test program.
fn analyze_lib_modules(test: &TestProgram, modules: &[ModuleId], timeout: Duration) -> Duration {
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

/// Compile queued tasks and assert no diagnostics.
fn compile_and_check(test: &TestProgram, timeout: Duration) {
    // compile with timeout then verify diagnostics
    test.compile_with_timeout(timeout);
    test.check_no_diagnostic(DiagnosticSeverity::Note);
}

/// Resolve declared lib symbols for all builtin libs.
#[test]
#[ignore = "slow"]
fn test_resolve_builtin_lib_symbols() {
    for lib in LIBS {
        let test =
            TestProgram::memory_sequential_with_prelude_and_libs().with_profile_libs(&[lib.name]);
        test.resolve_builtins();
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

/// Resolve all builtin libs (without errors).
#[test]
#[ignore = "slow"]
fn test_resolve_all_builtin_libs() {
    // collect all lib names
    let lib_names: Vec<&str> = std::iter::once(&STD_LIB)
        .chain(LIBS.iter())
        .map(|lib| lib.name)
        .collect();

    // create single TestProgram with all libs and resolve once
    let test = TestProgram::memory_sequential_with_prelude_and_libs().with_profile_libs(&lib_names);
    test.resolve_builtins();
    test.resolve_libs();
    test.compile();
}

/// Analyze all builtin libs with fast settings.
#[test]
#[ignore = "slow"]
fn test_analyze_all_builtin_libs_fast() {
    // fast smoke test
    let config = builtin_timing_config(Duration::from_secs(60), false);
    run_builtin_libs_per_lib(&config);
}

/// Analyze all builtin libs with full validation.
#[test]
#[ignore = "slow"]
fn test_analyze_all_builtin_libs_full() {
    // full validation for builtin libs
    let config = builtin_timing_config(Duration::from_secs(300), true);
    run_builtin_libs_per_lib(&config);
}

/// Analyze a combined lib set in a single program with fast settings.
#[test]
#[ignore = "slow"]
fn test_analyze_builtin_libs_combined() {
    // combined lib run for performance debugging
    let config = builtin_timing_config(Duration::from_secs(60), false);

    // resolve the combined lib list
    let mut libs = builtin_lib_list_from_env();

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
    let test = TestProgram::memory_sequential_with_prelude_and_libs().with_profile_libs(&lib_refs);

    let (import_modules, import_duration) = import_lib_modules(&test, &lib_refs, config.timeout);

    // resolve builtins and libs
    let resolve_duration = resolve_builtins_and_libs(&test, config.timeout);
    let analyze_duration = analyze_lib_modules(&test, &import_modules, config.timeout);

    let label = format!("combined({})", libs.join(","));
    let timing = LibTiming {
        name: label,
        import: import_duration,
        resolve: resolve_duration,
        analyze: analyze_duration,
        total: import_duration + resolve_duration + analyze_duration,
    };

    if config.report_timings {
        eprintln!(
            "builtin libs combined {}: import={} resolve={} analyze={} total={}",
            timing.name,
            format_duration(timing.import),
            format_duration(timing.resolve),
            format_duration(timing.analyze),
            format_duration(timing.total)
        );
    }

    if config.report_timings {
        report_timing_summary(std::slice::from_ref(&timing), 1);
    }

    if config.report_timings {
        let snapshot = test
            .compiler
            .stats
            .snapshot_with_program(test.program.modules.len(), Some(&test.program));
        report_task_name_summary(&snapshot, config.report_top_n);
        report_timing_tag_summary(&snapshot, config.report_top_n);
    }

    if let Some(path) = &config.csv_path
        && let Err(error) = write_timings_csv(&[timing], path)
    {
        eprintln!("timings: failed to write csv to {path}: {error}");
    }
}
