use std::collections::HashSet;
use std::env::current_dir;
use std::io::IsTerminal;
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::{Parser, ValueEnum};

use destack_compiler::{BuildKey, Compiler, CompilerOptions};
use destack_source::{MemoryFileSystem, ModuleId};
use destack_workspace::{
    ArtifactKey, CacheStore, LintPreset, LinterOptions, MemoryCacheStore, Program, Session,
};

use destack_linter::{LintLevel, LintPerformanceReport, LintRunner};

/// Command line arguments.
#[derive(Parser, Debug)]
#[command(name = "bench_stats", about = "Linter rule performance benchmarks")]
struct Args {
    /// Builtin libs to load for the profile.
    #[arg(
        long,
        value_delimiter = ',',
        default_value = "std,globals,dom,es2020.full"
    )]
    libs: Vec<String>,

    /// Number of compiler workers for setup phases.
    #[arg(long, default_value_t = 1)]
    workers: u16,

    /// Linter preset used during profiling.
    #[arg(long, value_enum, default_value_t = Preset::All)]
    preset: Preset,

    /// Skip AST lint profiling.
    #[arg(long)]
    no_ast: bool,

    /// Skip DIR lint profiling.
    #[arg(long)]
    no_dir: bool,

    /// Number of top rules to print.
    #[arg(long, default_value_t = 50)]
    top: usize,

    /// Output format.
    #[arg(long, value_enum, default_value_t = Output::Table)]
    output: Output,

    /// Disable ANSI color output.
    #[arg(long)]
    no_color: bool,
}

/// Preset options for lint execution.
#[derive(Clone, Copy, Debug, ValueEnum)]
enum Preset {
    /// Only recommended rules.
    Recommended,
    /// All rules.
    All,
}

/// Output format for profile summaries.
#[derive(Clone, Copy, Debug, ValueEnum)]
enum Output {
    /// Human readable output.
    Table,
    /// Comma separated values output.
    Csv,
}

/// ANSI reset sequence.
const RESET: &str = "\x1b[0m";
/// ANSI bold sequence.
const BOLD: &str = "\x1b[1m";
/// ANSI dim sequence.
const DIM: &str = "\x1b[2m";
/// ANSI cyan sequence.
const CYAN: &str = "\x1b[36m";
/// ANSI green sequence.
const GREEN: &str = "\x1b[32m";
/// ANSI yellow sequence.
const YELLOW: &str = "\x1b[33m";
/// ANSI red sequence.
const RED: &str = "\x1b[31m";

/// Aggregate line metrics across modules.
struct LineStats {
    /// Total line count across all modules.
    total: usize,
    /// Minimum lines in a module.
    min: usize,
    /// Maximum lines in a module.
    max: usize,
    /// Mean lines per module.
    mean: f64,
}

fn main() {
    let args = Args::parse();
    if args.no_ast && args.no_dir {
        eprintln!("bench_stats: both --no-ast and --no-dir are enabled");
        std::process::exit(2);
    }

    let (session, program, compiler) = create_program(args.workers);
    let profile_id = ensure_profile_for_libs(&program, &args.libs);
    let modules = load_modules_for_libs(&session, &program, profile_id, &args.libs);
    let line_stats = compute_line_stats(&program, &modules);

    run_import_phase(&compiler, &modules);
    run_resolve_phase(&compiler, profile_id);
    run_analyze_phase(&compiler, &modules, profile_id);

    let lint_options = match args.preset {
        Preset::Recommended => LinterOptions::recommended(),
        Preset::All => LinterOptions::all(),
    };
    let runner = LintRunner::from_preset(match args.preset {
        Preset::Recommended => LintPreset::Recommended,
        Preset::All => LintPreset::All,
    })
    .with_fixes(false);
    let configured_rule_count = runner.rules().len();

    let lint_start = Instant::now();
    let (diagnostic_count, performance) = run_lint_phase(
        &runner,
        program.clone(),
        &modules,
        profile_id,
        &lint_options,
        !args.no_ast,
        !args.no_dir,
    );
    let lint_duration = lint_start.elapsed();
    let color_enabled =
        !args.no_color && std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal();

    match args.output {
        Output::Table => print_table(
            &performance,
            args.top,
            modules.len(),
            configured_rule_count,
            &line_stats,
            diagnostic_count,
            lint_duration,
            color_enabled,
        ),
        Output::Csv => print_csv(&performance, args.top, line_stats.total),
    }
}

/// Create an in memory compiler program for benchmarking.
fn create_program(workers: u16) -> (Arc<Session>, Arc<Program>, Arc<Compiler>) {
    let root_directory = current_dir().unwrap_or_else(|error| {
        panic!("failed to read current directory: {error}");
    });

    let fs = Arc::new(MemoryFileSystem::new());
    fs.add_file("package.json", br#"{ "name": "lint-bench" }"#)
        .unwrap_or_else(|_| panic!("failed to add package.json to memory file system"));

    let cache_store: Arc<dyn CacheStore> = Arc::new(MemoryCacheStore::new());
    let session = Arc::new(
        Session::new(root_directory.clone())
            .with_fs(fs)
            .with_cache_store(cache_store),
    );
    let program = session.add_root(root_directory);
    let compiler = Arc::new(Compiler::new(
        session.clone(),
        program.clone(),
        CompilerOptions {
            workers,
            inject_prelude: true,
            load_libs: true,
            ..CompilerOptions::default()
        },
    ));

    (session, program, compiler)
}

/// Ensure a profile with the requested lib set exists.
fn ensure_profile_for_libs(program: &Program, libs: &[String]) -> destack_workspace::ProfileId {
    let default_profile =
        program.profile(program.default_profile_id_for_module(program.root_module_id));
    let mut key = default_profile.key;
    key.lib = libs.to_vec();
    program.profiles.get_or_create(key)
}

/// Load builtin lib modules for benchmarking.
fn load_modules_for_libs(
    session: &Session,
    program: &Program,
    profile_id: destack_workspace::ProfileId,
    libs: &[String],
) -> Vec<ModuleId> {
    let profile = program.profile(profile_id);

    let mut modules = Vec::new();
    for lib in libs {
        let loaded = session
            .load_lib(lib, &profile.key)
            .unwrap_or_else(|| panic!("unknown builtin lib '{lib}'"));
        modules.extend(loaded);
    }

    // preserve insertion order while deduplicating
    let mut seen = HashSet::new();
    modules.retain(|module_id| seen.insert(*module_id));
    modules
}

/// Compute line metrics for loaded modules.
fn compute_line_stats(program: &Program, modules: &[ModuleId]) -> LineStats {
    let mut total_lines = 0usize;
    let mut min_lines = usize::MAX;
    let mut max_lines = 0usize;

    for module_id in modules {
        let file_id = program.modules.get(*module_id).read().file_id;
        let file = program.files.get(file_id);
        let module_lines = file.line_count() as usize;
        total_lines += module_lines;
        min_lines = min_lines.min(module_lines);
        max_lines = max_lines.max(module_lines);
    }

    let module_count = modules.len();
    let mean_lines = if module_count == 0 {
        0.0
    } else {
        total_lines as f64 / module_count as f64
    };

    LineStats {
        total: total_lines,
        min: if module_count == 0 { 0 } else { min_lines },
        max: max_lines,
        mean: mean_lines,
    }
}

/// Run import tasks for modules and return the duration.
fn run_import_phase(compiler: &Compiler, modules: &[ModuleId]) -> Duration {
    for module_id in modules {
        compiler.enqueue(BuildKey::Artifact(ArtifactKey::DirBase {
            module: *module_id,
        }));
    }

    let start = Instant::now();
    compiler.compile();
    start.elapsed()
}

/// Run builtin and lib resolve tasks and return the duration.
fn run_resolve_phase(compiler: &Compiler, profile_id: destack_workspace::ProfileId) -> Duration {
    compiler.enqueue(BuildKey::Artifact(ArtifactKey::LibEnvironment {
        profile: profile_id,
    }));

    let start = Instant::now();
    compiler.compile();
    start.elapsed()
}

/// Run analyze tasks and return the duration.
fn run_analyze_phase(
    compiler: &Compiler,
    modules: &[ModuleId],
    profile_id: destack_workspace::ProfileId,
) -> Duration {
    for module_id in modules {
        compiler.enqueue(BuildKey::Artifact(ArtifactKey::DirAnalyzed {
            module: *module_id,
            profile: profile_id,
        }));
    }

    let start = Instant::now();
    compiler.compile();
    start.elapsed()
}

/// Run linting for selected levels and return diagnostics plus metrics.
fn run_lint_phase(
    runner: &LintRunner,
    program: Arc<Program>,
    modules: &[ModuleId],
    profile_id: destack_workspace::ProfileId,
    options: &LinterOptions,
    include_ast: bool,
    include_dir: bool,
) -> (usize, LintPerformanceReport) {
    let mut diagnostics = 0;
    let mut performance = LintPerformanceReport::default();

    // module scope runs
    for module_id in modules {
        let module = program.modules.get(*module_id);
        if include_ast {
            let report = runner.lint_module_profiled(
                program.clone(),
                module.clone(),
                profile_id,
                options,
                LintLevel::Ast,
            );
            diagnostics += report.diagnostics.len();
            performance.merge(&report.performance);
        }

        if include_dir {
            let report = runner.lint_module_profiled(
                program.clone(),
                module.clone(),
                profile_id,
                options,
                LintLevel::Dir,
            );
            diagnostics += report.diagnostics.len();
            performance.merge(&report.performance);
        }
    }

    // program scope ast run
    if include_ast {
        let report = runner.lint_program_ast_profiled(program.clone(), options);
        diagnostics += report.diagnostics.len();
        performance.merge(&report.performance);
    }

    // program scope dir run
    if include_dir {
        let report = runner.lint_program_dir_profiled(program, profile_id, options);
        diagnostics += report.diagnostics.len();
        performance.merge(&report.performance);
    }

    (diagnostics, performance)
}

/// Format a duration for readable output.
fn format_duration(duration: Duration) -> String {
    let milliseconds = duration.as_secs_f64() * 1000.0;
    if milliseconds >= 1000.0 {
        return format!("{:.3}s", milliseconds / 1000.0);
    }

    format!("{milliseconds:.3}ms")
}

/// Format a ratio as a percentage string.
fn format_percent(ratio: f64) -> String {
    format!("{:.1}%", ratio * 100.0)
}

/// Format lines per second as a readable number.
fn format_lines_per_second(lines: usize, duration: Duration) -> String {
    let lines_per_second = lines as f64 / duration.as_secs_f64().max(0.000_001);
    if lines_per_second >= 1_000_000.0 {
        return format!("{:.2}M", lines_per_second / 1_000_000.0);
    }
    if lines_per_second >= 1_000.0 {
        return format!("{:.1}K", lines_per_second / 1_000.0);
    }

    format!("{lines_per_second:.0}")
}

/// Format milliseconds per thousand lines for normalized comparisons.
fn format_ms_per_kloc(duration: Duration, total_lines: usize) -> String {
    if total_lines == 0 {
        return "-".to_string();
    }

    let milliseconds = duration.as_secs_f64() * 1000.0;
    let kloc = total_lines as f64 / 1000.0;
    format!("{:.3}", milliseconds / kloc.max(0.000_001))
}

/// Build a fixed width share bar.
fn format_share_bar(ratio: f64, width: usize) -> String {
    let clamped = ratio.clamp(0.0, 1.0);
    let filled = (clamped * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "#".repeat(filled), ".".repeat(empty))
}

/// Style helpers for table output.
struct TableStyle {
    /// Bold text prefix.
    bold: &'static str,
    /// Dim text prefix.
    dim: &'static str,
    /// Reset code.
    reset: &'static str,
    /// Cyan text prefix.
    cyan: &'static str,
    /// Green text prefix.
    green: &'static str,
    /// Yellow text prefix.
    yellow: &'static str,
    /// Red text prefix.
    red: &'static str,
}

impl TableStyle {
    /// Build table styling with or without ANSI codes.
    fn new(color: bool) -> Self {
        if color {
            return Self {
                bold: BOLD,
                dim: DIM,
                reset: RESET,
                cyan: CYAN,
                green: GREEN,
                yellow: YELLOW,
                red: RED,
            };
        }

        Self {
            bold: "",
            dim: "",
            reset: "",
            cyan: "",
            green: "",
            yellow: "",
            red: "",
        }
    }

    /// Pick a color for ratio based highlighting.
    fn color_for_ratio(&self, ratio: f64) -> &'static str {
        if self.green.is_empty() {
            return "";
        }

        if ratio >= 0.40 {
            self.red
        } else if ratio >= 0.20 {
            self.yellow
        } else {
            self.green
        }
    }
}

/// Print human readable benchmark output.
fn print_table(
    performance: &LintPerformanceReport,
    top: usize,
    module_count: usize,
    configured_rule_count: usize,
    line_stats: &LineStats,
    diagnostics: usize,
    lint_duration: Duration,
    color: bool,
) {
    let style = TableStyle::new(color);
    let rule_to_lint_ratio =
        performance.total_duration.as_secs_f64() / lint_duration.as_secs_f64().max(0.000_001);

    let rules = performance.sorted_by_total_duration();
    let count = top.min(rules.len());
    let top_one_ratio = rules
        .first()
        .map(|rule| {
            rule.total_duration.as_secs_f64()
                / performance.total_duration.as_secs_f64().max(0.000_001)
        })
        .unwrap_or(0.0);
    let top_three_duration = rules
        .iter()
        .take(3)
        .fold(Duration::ZERO, |acc, rule| acc + rule.total_duration);
    let top_three_ratio =
        top_three_duration.as_secs_f64() / performance.total_duration.as_secs_f64().max(0.000_001);

    let ast_duration = rules
        .iter()
        .filter(|rule| matches!(rule.level, LintLevel::Ast))
        .fold(Duration::ZERO, |acc, rule| acc + rule.total_duration);
    let dir_duration = rules
        .iter()
        .filter(|rule| matches!(rule.level, LintLevel::Dir))
        .fold(Duration::ZERO, |acc, rule| acc + rule.total_duration);
    let ast_ratio =
        ast_duration.as_secs_f64() / performance.total_duration.as_secs_f64().max(0.000_001);
    let dir_ratio =
        dir_duration.as_secs_f64() / performance.total_duration.as_secs_f64().max(0.000_001);

    println!(
        "{bold}slowest lint rules{reset} {dim}(top {count}){reset}",
        bold = style.bold,
        dim = style.dim,
        reset = style.reset
    );
    println!(
        "  {bold}{:<32} {:<3} {:>6} {:>8} {:>7} {:>11} {:>10} {:>10}  {:<14}{reset}",
        "rule",
        "lvl",
        "runs",
        "diag",
        "share",
        "total",
        "avg",
        "ms/KLoC",
        "hot",
        bold = style.bold,
        reset = style.reset
    );
    println!(
        "  {dim}{}{reset}",
        "-".repeat(112),
        dim = style.dim,
        reset = style.reset
    );

    for rule in rules.into_iter().take(count) {
        let level = format!("{:?}", rule.level);
        let total = format_duration(rule.total_duration);
        let average = format_duration(rule.average_duration());
        let share_ratio = rule.total_duration.as_secs_f64()
            / performance.total_duration.as_secs_f64().max(0.000_001);
        let share = format_percent(share_ratio);
        let ms_per_kloc = format_ms_per_kloc(rule.total_duration, line_stats.total);
        let bar = format_share_bar(share_ratio, 14);
        let heat_color = style.color_for_ratio(share_ratio);
        println!(
            "  {cyan}{:<32}{reset} {:<3} {:>6} {:>8} {heat}{:>7}{reset} {heat}{:>11}{reset} {:>10} {:>10}  {heat}{}{reset}",
            rule.id,
            level,
            rule.runs,
            rule.diagnostics,
            share,
            total,
            average,
            ms_per_kloc,
            bar,
            cyan = style.cyan,
            heat = heat_color,
            reset = style.reset
        );
    }

    println!(
        "  {dim}share is percentage of total rule time, ms/KLoC is normalized by loaded lines{reset}",
        dim = style.dim,
        reset = style.reset
    );
    println!();

    let lint_lines_per_second = format_lines_per_second(line_stats.total, lint_duration);
    let executed_rule_count = performance.rules.len();
    let rule_coverage_ratio = executed_rule_count as f64 / configured_rule_count.max(1) as f64;

    println!(
        "{bold}lint summary{reset}",
        bold = style.bold,
        reset = style.reset
    );
    println!("  modules:           {module_count}");
    println!(
        "  lines:             {} {dim}(min {}, max {}, mean {:.1}){reset}",
        line_stats.total,
        line_stats.min,
        line_stats.max,
        line_stats.mean,
        dim = style.dim,
        reset = style.reset
    );
    println!("  diagnostics:       {diagnostics}");
    println!("  configured rules:  {configured_rule_count}");
    println!(
        "  executed rules:    {executed_rule_count} {dim}({} of configured){reset}",
        format_percent(rule_coverage_ratio),
        dim = style.dim,
        reset = style.reset
    );
    println!("  rule executions:   {}", performance.total_rule_runs);
    println!(
        "  total rule time:   {}",
        format_duration(performance.total_duration)
    );
    println!("  lint wall time:    {}", format_duration(lint_duration));
    println!("  lint lines/s:      {lint_lines_per_second}");
    println!(
        "  rule/lint share:   {}",
        format_percent(rule_to_lint_ratio.clamp(0.0, 1.0))
    );
    println!(
        "  compiler phases:   {dim}see compiler bench_stats for import/resolve/analyze timings{reset}",
        dim = style.dim,
        reset = style.reset
    );
    println!();

    println!(
        "{bold}hotspots{reset}",
        bold = style.bold,
        reset = style.reset
    );
    println!(
        "  top rule share:    {color}{share:>7}{reset}",
        color = style.color_for_ratio(top_one_ratio),
        share = format_percent(top_one_ratio),
        reset = style.reset
    );
    println!(
        "  top 3 rule share:  {:>7}",
        format_percent(top_three_ratio)
    );
    println!("  ast rule share:    {:>7}", format_percent(ast_ratio));
    println!("  dir rule share:    {:>7}", format_percent(dir_ratio));
}

/// Print csv benchmark output.
fn print_csv(performance: &LintPerformanceReport, top: usize, total_lines: usize) {
    println!("rule,code,category,level,runs,diagnostics,share_pct,total_ms,avg_ms,ms_per_kloc");
    for rule in performance.sorted_by_total_duration().into_iter().take(top) {
        let share_ratio = rule.total_duration.as_secs_f64()
            / performance.total_duration.as_secs_f64().max(0.000_001);
        let ms_per_kloc = if total_lines == 0 {
            0.0
        } else {
            let milliseconds = rule.total_duration.as_secs_f64() * 1000.0;
            let kloc = total_lines as f64 / 1000.0;
            milliseconds / kloc.max(0.000_001)
        };
        println!(
            "{},{},{:?},{:?},{},{},{:.3},{:.3},{:.3},{:.3}",
            rule.id,
            rule.code,
            rule.category,
            rule.level,
            rule.runs,
            rule.diagnostics,
            share_ratio * 100.0,
            rule.total_duration.as_secs_f64() * 1000.0,
            rule.average_duration().as_secs_f64() * 1000.0,
            ms_per_kloc,
        );
    }
}
