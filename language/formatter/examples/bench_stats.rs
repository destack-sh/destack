use std::collections::{BTreeSet, HashMap};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};
use std::{fs, io, thread};

use clap::{Parser, ValueEnum};
use serde_json::json;

use destack_ast::{Expression, LocalNodeId, NodeParentIndex};
use destack_formatter::{
    DestackFormatArtifacts, DestackFormatContext, DestackFormatOptions,
    FormatterCacheStatsSnapshot, FormatterCounterEntry, FormatterTimingEntry, statement_list,
};
use destack_parser::Parser as DestackParser;
use destack_source::{File, FileId, FileType, LanguageType, MultiSpan, Uri};

const DEFAULT_ROOT: &str = "test/fixtures/ecosystem/checkouts";
const QUICK_CORPUS_PACKAGES: &[&str] = &[
    "valibot",
    "zod",
    "react-hook-form",
    "preact",
    "nest",
    "rxjs",
    "typebox",
    "vite",
    "vitest",
    "graphql-js",
    "react-query",
];
const STANDARD_CORPUS_PACKAGES: &[&str] = &[
    "typescript",
    "nextjs",
    "angular",
    "eslint",
    "typescript-eslint",
    "vitest",
];
const SHARE_BAR_WIDTH: usize = 16;
const HOT_FILE_NAME_WIDTH: usize = 52;
const TIMING_TAG_WIDTH: usize = 34;

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";

/// Command line arguments.
#[derive(Parser, Debug)]
#[command(
    name = "bench_stats",
    about = "Formatter corpus performance benchmarks"
)]
struct Args {
    /// Corpus checkout root directory or source file.
    #[arg(long, default_value = DEFAULT_ROOT)]
    root: PathBuf,

    /// Preset corpus package profile.
    #[arg(long, value_enum, default_value_t = CorpusProfile::Standard)]
    corpus: CorpusProfile,

    /// Extra package directories under --root, comma separated.
    #[arg(long, value_delimiter = ',')]
    packages: Vec<String>,

    /// Number of measured benchmark runs.
    #[arg(long, default_value_t = 7)]
    runs: usize,

    /// Number of warmup runs before measurement.
    #[arg(long, default_value_t = 2)]
    warmup_runs: usize,

    /// Number of top slow files to print in table output.
    #[arg(long, default_value_t = 20)]
    top: usize,

    /// Output format.
    #[arg(long, value_enum, default_value_t = Output::Table)]
    output: Output,

    /// Disable ANSI color output.
    #[arg(long)]
    no_color: bool,

    /// Number of worker threads per run.
    #[arg(long, default_value_t = 1)]
    workers: usize,

    /// Benchmark behavior mode.
    #[arg(long, value_enum, default_value_t = BenchMode::RealWorld)]
    mode: BenchMode,

    /// Disable run progress output on stderr.
    #[arg(long)]
    no_progress: bool,

    /// Enable formatter internal timing and cache instrumentation.
    #[arg(long)]
    timings: bool,

    /// Number of top timing tags to print when timings are enabled.
    #[arg(long, default_value_t = 20)]
    timings_top: usize,
}

/// Output format for benchmark results.
#[derive(Clone, Copy, Debug, ValueEnum)]
enum Output {
    /// Human readable output.
    Table,
    /// Comma separated values output.
    Csv,
    /// Json output.
    Json,
}

/// Corpus package profile selection.
#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
enum CorpusProfile {
    /// Run the small representative set for fast local checks.
    Quick,
    /// Run the larger representative set for daily formatter perf iteration.
    Standard,
    /// Run all supported files under --root.
    Full,
}

/// Benchmark behavior mode.
#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
enum BenchMode {
    /// Respect formatter-level ignore directives and measure real tool behavior.
    RealWorld,
    /// Disable formatter-level ignore directives for cross-tool fairness.
    Engine,
}

impl BenchMode {
    /// Return the mode label.
    fn as_str(self) -> &'static str {
        match self {
            Self::RealWorld => "real_world",
            Self::Engine => "engine",
        }
    }
}

impl CorpusProfile {
    /// Return the profile label.
    fn as_str(self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Standard => "standard",
            Self::Full => "full",
        }
    }
}

#[derive(Debug, Clone)]
struct CorpusFile {
    path: PathBuf,
    file_type: FileType,
    source: String,
    source_bytes: usize,
    source_lines: usize,
}

#[derive(Debug, Clone)]
struct FileRunStat {
    path: PathBuf,
    source_lines: usize,
    formatted_lines: usize,
    skipped_lines: usize,
    source_bytes: usize,
    formatted_bytes: usize,
    skipped_bytes: usize,
    output_bytes: usize,
    skipped_by_file_ignore: bool,
    parse: Duration,
    format: Duration,
    print: Duration,
    total: Duration,
    cache: FormatterCacheStatsSnapshot,
    timings: Vec<FormatterTimingEntry>,
    counters: Vec<FormatterCounterEntry>,
}

#[derive(Debug, Clone)]
struct BenchRunStats {
    parse: Duration,
    format: Duration,
    print: Duration,
    work_total: Duration,
    total: Duration,
    source_bytes: usize,
    source_lines: usize,
    formatted_bytes: usize,
    formatted_lines: usize,
    skipped_bytes: usize,
    skipped_lines: usize,
    formatted_files: usize,
    skipped_files: usize,
    output_bytes: usize,
    sink: usize,
    cache: FormatterCacheStatsSnapshot,
    timings: Vec<FormatterTimingEntry>,
    counters: Vec<FormatterCounterEntry>,
    files: Vec<FileRunStat>,
}

#[derive(Debug, Clone)]
struct FileAggregate {
    path: PathBuf,
    runs: usize,
    source_lines: usize,
    source_bytes: usize,
    output_bytes_total: usize,
    parse_total: Duration,
    format_total: Duration,
    print_total: Duration,
    total: Duration,
}

#[derive(Debug, Clone)]
struct BenchSummary {
    root: PathBuf,
    mode: BenchMode,
    workers: usize,
    files: usize,
    formatted_files_per_run: usize,
    skipped_files_per_run: usize,
    warmup_runs: usize,
    measured_runs: usize,
    source_lines_per_run: usize,
    formatted_lines_per_run: usize,
    skipped_lines_per_run: usize,
    source_bytes_per_run: usize,
    formatted_bytes_per_run: usize,
    skipped_bytes_per_run: usize,
    output_bytes_per_run: usize,
    sink: usize,
    min: Duration,
    max: Duration,
    mean: Duration,
    median: Duration,
    p95: Duration,
    stddev_ms: f64,
    coefficient_of_variation: f64,
    mean_parse: Duration,
    mean_format: Duration,
    mean_print: Duration,
    mean_work: Duration,
    files_per_second: f64,
    lines_per_second: f64,
    formatted_lines_per_second: f64,
    parse_lines_per_second: f64,
    format_lines_per_second: f64,
    format_formatted_lines_per_second: f64,
    print_lines_per_second: f64,
    input_mib_per_second: f64,
    output_mib_per_second: f64,
    cache: FormatterCacheStatsSnapshot,
    timing_tags: Vec<TimingAggregate>,
    counters: Vec<CounterAggregate>,
    runs: Vec<BenchRunStats>,
    top_files: Vec<FileAggregate>,
}

#[derive(Debug, Clone)]
struct TimingAggregate {
    name: &'static str,
    duration: Duration,
    count: usize,
}

#[derive(Debug, Clone)]
struct CounterAggregate {
    name: &'static str,
    value: usize,
}

#[derive(Debug, Clone, Copy, Default)]
struct CorpusLoadStats {
    skipped_non_utf8: usize,
    skipped_read_errors: usize,
}

#[derive(Debug, Clone)]
struct CorpusSelection {
    roots: Vec<PathBuf>,
    package_names: Vec<String>,
}

/// Style helpers for table output.
struct TableStyle {
    bold: &'static str,
    dim: &'static str,
    reset: &'static str,
    cyan: &'static str,
    green: &'static str,
    yellow: &'static str,
    red: &'static str,
}

impl TableStyle {
    /// Build a table style with or without ANSI codes.
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

    /// Pick a heat color based on a ratio.
    fn color_for_ratio(&self, ratio: f64) -> &'static str {
        if self.green.is_empty() {
            return "";
        }

        if ratio >= 0.6 {
            self.red
        } else if ratio >= 0.3 {
            self.yellow
        } else {
            self.green
        }
    }

    /// Pick a run color by position between min and max.
    fn color_for_range(&self, value: Duration, min: Duration, max: Duration) -> &'static str {
        if self.green.is_empty() || max <= min {
            return "";
        }

        let ratio = (value.as_secs_f64() - min.as_secs_f64())
            / (max.as_secs_f64() - min.as_secs_f64()).max(0.000_001);
        self.color_for_ratio(ratio)
    }
}

fn main() -> Result<(), String> {
    let args = Args::parse();
    if args.runs == 0 {
        return Err("--runs must be greater than zero".to_string());
    }

    let root = resolve_root_path(args.root.as_path())?;
    let selection = resolve_corpus_selection(root.as_path(), args.corpus, &args.packages)?;
    let (files, load_stats) = load_corpus_files(&selection.roots)?;
    if files.is_empty() {
        return Err(format!(
            "no supported source files found under {}",
            root.display()
        ));
    }

    let progress = !args.no_progress && std::io::stderr().is_terminal();
    let all_runs = args.warmup_runs + args.runs;
    let started_at = Instant::now();
    let corpus_lines = files.iter().map(|file| file.source_lines).sum::<usize>();

    if progress {
        eprintln!(
            "bench_stats: corpus {} [{}|{}] ({} files, {} lines), warmup {}, measured {}, workers {}, timings {}",
            root.display(),
            args.mode.as_str(),
            args.corpus.as_str(),
            format_count(files.len()),
            format_count(corpus_lines),
            args.warmup_runs,
            args.runs,
            format_count(args.workers.max(1)),
            if args.timings { "on" } else { "off" }
        );
        if !selection.package_names.is_empty() {
            eprintln!(
                "bench_stats: packages ({}) {}",
                format_count(selection.package_names.len()),
                format_package_list(selection.package_names.as_slice())
            );
        }
        if load_stats.skipped_non_utf8 > 0 || load_stats.skipped_read_errors > 0 {
            eprintln!(
                "bench_stats: skipped {} non-utf8 files and {} unreadable files",
                format_count(load_stats.skipped_non_utf8),
                format_count(load_stats.skipped_read_errors)
            );
        }
    }

    for warmup_index in 0..args.warmup_runs {
        let run = run_single_benchmark(&files, args.workers, args.timings, args.mode)?;
        if progress {
            eprintln!(
                "  run {}/{} warmup   total {}",
                warmup_index + 1,
                all_runs,
                format_duration(run.total)
            );
        }
    }

    let mut measured_runs = Vec::with_capacity(args.runs);
    for run_index in 0..args.runs {
        let run = run_single_benchmark(&files, args.workers, args.timings, args.mode)?;
        if progress {
            eprintln!(
                "  run {}/{} measured total {}",
                args.warmup_runs + run_index + 1,
                all_runs,
                format_duration(run.total)
            );
        }
        measured_runs.push(run);
    }

    if progress {
        eprintln!(
            "bench_stats: finished in {}",
            format_duration(started_at.elapsed())
        );
    }

    let summary = summarize(
        root.as_path(),
        args.mode,
        args.workers.max(1),
        args.warmup_runs,
        args.top,
        args.timings_top,
        measured_runs,
    );
    let color =
        !args.no_color && std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal();
    print_output(&summary, args.output, color);

    Ok(())
}

/// Resolve a corpus root from repository and language working directories.
fn resolve_root_path(root: &Path) -> Result<PathBuf, String> {
    if root.exists() {
        return Ok(root.to_path_buf());
    }

    if root.is_relative() {
        let prefixed = Path::new("language").join(root);
        if prefixed.exists() {
            return Ok(prefixed);
        }

        if let Ok(stripped) = root.strip_prefix("language")
            && stripped.exists()
        {
            return Ok(stripped.to_path_buf());
        }
    }

    Err(format!("root path does not exist: {}", root.display()))
}

/// Resolve selected roots from corpus profile and optional package list.
fn resolve_corpus_selection(
    root: &Path,
    profile: CorpusProfile,
    extra_packages: &[String],
) -> Result<CorpusSelection, String> {
    if root.is_file() {
        return Ok(CorpusSelection {
            roots: vec![root.to_path_buf()],
            package_names: Vec::new(),
        });
    }

    if !root.is_dir() {
        return Err(format!(
            "corpus root is not a directory: {}",
            root.display()
        ));
    }

    let mut package_names = BTreeSet::new();
    for package in profile_package_names(profile) {
        package_names.insert((*package).to_string());
    }
    for package in extra_packages {
        let package = package.trim();
        if package.is_empty() {
            continue;
        }
        package_names.insert(package.to_string());
    }

    if profile == CorpusProfile::Full && package_names.is_empty() {
        return Ok(CorpusSelection {
            roots: vec![root.to_path_buf()],
            package_names: Vec::new(),
        });
    }

    if package_names.is_empty() {
        return Ok(CorpusSelection {
            roots: vec![root.to_path_buf()],
            package_names: Vec::new(),
        });
    }

    let mut roots = Vec::with_capacity(package_names.len());
    let mut missing = Vec::new();
    for package in &package_names {
        let package_root = root.join(package);
        if package_root.exists() {
            roots.push(package_root);
        } else {
            missing.push(package.to_string());
        }
    }

    if !missing.is_empty() {
        return Err(format!(
            "missing package directories under {}: {}",
            root.display(),
            missing.join(", ")
        ));
    }

    Ok(CorpusSelection {
        roots,
        package_names: package_names.into_iter().collect(),
    })
}

/// Return package names for a built-in corpus profile.
fn profile_package_names(profile: CorpusProfile) -> &'static [&'static str] {
    match profile {
        CorpusProfile::Quick => QUICK_CORPUS_PACKAGES,
        CorpusProfile::Standard => STANDARD_CORPUS_PACKAGES,
        CorpusProfile::Full => &[],
    }
}

/// Format a package list for progress output.
fn format_package_list(package_names: &[String]) -> String {
    const MAX_DISPLAYED_PACKAGES: usize = 16;
    if package_names.len() <= MAX_DISPLAYED_PACKAGES {
        return package_names.join(", ");
    }

    let shown = package_names
        .iter()
        .take(MAX_DISPLAYED_PACKAGES)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{shown}, ... (+{})",
        format_count(package_names.len() - MAX_DISPLAYED_PACKAGES)
    )
}

/// Execute one full corpus benchmark run.
fn run_single_benchmark(
    files: &[CorpusFile],
    workers: usize,
    timings_enabled: bool,
    mode: BenchMode,
) -> Result<BenchRunStats, String> {
    let run_started_at = Instant::now();
    if files.is_empty() {
        return Ok(BenchRunStats {
            parse: Duration::ZERO,
            format: Duration::ZERO,
            print: Duration::ZERO,
            work_total: Duration::ZERO,
            total: Duration::ZERO,
            source_bytes: 0,
            source_lines: 0,
            formatted_bytes: 0,
            formatted_lines: 0,
            skipped_bytes: 0,
            skipped_lines: 0,
            formatted_files: 0,
            skipped_files: 0,
            output_bytes: 0,
            sink: 0,
            cache: FormatterCacheStatsSnapshot::default(),
            timings: Vec::new(),
            counters: Vec::new(),
            files: Vec::new(),
        });
    }

    let worker_count = workers.max(1).min(files.len());
    if worker_count == 1 {
        let mut file_stats = Vec::with_capacity(files.len());
        for (index, corpus_file) in files.iter().enumerate() {
            let file_stat = benchmark_file(index as u32, corpus_file, timings_enabled, mode)?;
            file_stats.push(file_stat);
        }

        return Ok(aggregate_run_stats(file_stats, run_started_at.elapsed()));
    }

    let next_file_index = AtomicUsize::new(0);
    let (sender, receiver) = mpsc::channel::<Result<Vec<(usize, FileRunStat)>, String>>();

    thread::scope(|scope| {
        for _worker_index in 0..worker_count {
            let sender = sender.clone();
            let next_file_index = &next_file_index;
            scope.spawn(move || {
                let mut chunk_stats = Vec::new();
                loop {
                    let file_index = next_file_index.fetch_add(1, Ordering::Relaxed);
                    if file_index >= files.len() {
                        break;
                    }

                    let corpus_file = &files[file_index];
                    let file_stat =
                        match benchmark_file(file_index as u32, corpus_file, timings_enabled, mode)
                        {
                            Ok(file_stat) => file_stat,
                            Err(error) => {
                                let _ = sender.send(Err(error));
                                return;
                            }
                        };
                    chunk_stats.push((file_index, file_stat));
                }

                let _ = sender.send(Ok(chunk_stats));
            });
        }

        drop(sender);
    });

    let mut indexed_stats = Vec::with_capacity(files.len());
    for worker_result in receiver {
        match worker_result {
            Ok(mut chunk_stats) => indexed_stats.append(&mut chunk_stats),
            Err(error) => return Err(error),
        }
    }

    if indexed_stats.len() != files.len() {
        return Err(format!(
            "internal bench error: expected {} file stats, got {}",
            files.len(),
            indexed_stats.len()
        ));
    }

    indexed_stats.sort_by_key(|(index, _)| *index);
    let file_stats = indexed_stats
        .into_iter()
        .map(|(_, file_stat)| file_stat)
        .collect::<Vec<_>>();

    Ok(aggregate_run_stats(file_stats, run_started_at.elapsed()))
}

/// Aggregate per-file run statistics into one run summary.
fn aggregate_run_stats(file_stats: Vec<FileRunStat>, wall_total: Duration) -> BenchRunStats {
    let mut parse_total = Duration::ZERO;
    let mut format_total = Duration::ZERO;
    let mut print_total = Duration::ZERO;
    let mut run_total = Duration::ZERO;
    let mut source_bytes = 0usize;
    let mut source_lines = 0usize;
    let mut formatted_bytes = 0usize;
    let mut formatted_lines = 0usize;
    let mut skipped_bytes = 0usize;
    let mut skipped_lines = 0usize;
    let mut formatted_files = 0usize;
    let mut skipped_files = 0usize;
    let mut output_bytes = 0usize;
    let mut sink = 0usize;
    let mut cache = FormatterCacheStatsSnapshot::default();
    let mut timing_totals = HashMap::<&'static str, (Duration, usize)>::new();
    let mut counter_totals = HashMap::<&'static str, usize>::new();
    for file_stat in &file_stats {
        parse_total += file_stat.parse;
        format_total += file_stat.format;
        print_total += file_stat.print;
        run_total += file_stat.total;

        source_bytes = source_bytes.saturating_add(file_stat.source_bytes);
        source_lines = source_lines.saturating_add(file_stat.source_lines);
        formatted_bytes = formatted_bytes.saturating_add(file_stat.formatted_bytes);
        formatted_lines = formatted_lines.saturating_add(file_stat.formatted_lines);
        skipped_bytes = skipped_bytes.saturating_add(file_stat.skipped_bytes);
        skipped_lines = skipped_lines.saturating_add(file_stat.skipped_lines);
        output_bytes = output_bytes.saturating_add(file_stat.output_bytes);
        sink ^= file_stat.output_bytes;
        if file_stat.skipped_by_file_ignore {
            skipped_files = skipped_files.saturating_add(1);
        } else {
            formatted_files = formatted_files.saturating_add(1);
        }

        cache.span_text_hits = cache
            .span_text_hits
            .saturating_add(file_stat.cache.span_text_hits);
        cache.span_text_misses = cache
            .span_text_misses
            .saturating_add(file_stat.cache.span_text_misses);
        cache.span_has_newline_hits = cache
            .span_has_newline_hits
            .saturating_add(file_stat.cache.span_has_newline_hits);
        cache.span_has_newline_misses = cache
            .span_has_newline_misses
            .saturating_add(file_stat.cache.span_has_newline_misses);
        cache.span_has_comment_hits = cache
            .span_has_comment_hits
            .saturating_add(file_stat.cache.span_has_comment_hits);
        cache.span_has_comment_misses = cache
            .span_has_comment_misses
            .saturating_add(file_stat.cache.span_has_comment_misses);
        cache.annotation_cache_hits = cache
            .annotation_cache_hits
            .saturating_add(file_stat.cache.annotation_cache_hits);
        cache.annotation_cache_misses = cache
            .annotation_cache_misses
            .saturating_add(file_stat.cache.annotation_cache_misses);

        for entry in &file_stat.timings {
            let aggregate = timing_totals
                .entry(entry.name)
                .or_insert((Duration::ZERO, 0usize));
            aggregate.0 += entry.duration;
            aggregate.1 = aggregate.1.saturating_add(entry.count);
        }

        for entry in &file_stat.counters {
            let aggregate = counter_totals.entry(entry.name).or_insert(0);
            *aggregate = aggregate.saturating_add(entry.value);
        }
    }

    let mut timings = timing_totals
        .into_iter()
        .map(|(name, (duration, count))| FormatterTimingEntry {
            name,
            duration,
            count,
        })
        .collect::<Vec<_>>();
    timings.sort_by_key(|entry| std::cmp::Reverse(entry.duration));

    let mut counters = counter_totals
        .into_iter()
        .map(|(name, value)| FormatterCounterEntry { name, value })
        .collect::<Vec<_>>();
    counters.sort_by_key(|entry| std::cmp::Reverse(entry.value));

    BenchRunStats {
        parse: parse_total,
        format: format_total,
        print: print_total,
        work_total: run_total,
        total: wall_total,
        source_bytes,
        source_lines,
        formatted_bytes,
        formatted_lines,
        skipped_bytes,
        skipped_lines,
        formatted_files,
        skipped_files,
        output_bytes,
        sink,
        cache,
        timings,
        counters,
        files: file_stats,
    }
}

/// Benchmark one source file through parse, format, and print stages.
fn benchmark_file(
    file_id: u32,
    corpus_file: &CorpusFile,
    timings_enabled: bool,
    mode: BenchMode,
) -> Result<FileRunStat, String> {
    let total_started_at = Instant::now();

    let parse_started_at = Instant::now();
    let name = corpus_file.path.to_string_lossy().to_string();
    let file = Arc::new(File::from_text(
        FileId::new(file_id),
        name,
        Uri::from_path(corpus_file.path.as_path()),
        None,
        corpus_file.file_type,
        corpus_file.source.clone(),
    ));
    let language = LanguageType::from(corpus_file.file_type);
    let mut parser = DestackParser::lex_file(file.clone(), language);
    let expressions: Vec<LocalNodeId<Expression>> = parser.parse();
    parser.finish();
    let side_span: MultiSpan = parser.compute_side_span();
    let (tokens, side_tokens) = parser.take_tokens();
    let tree = parser.tree;
    let strings = parser.strings.into_immutable();
    let parents = NodeParentIndex::from_tree(&tree);
    let parse = parse_started_at.elapsed();

    let options =
        DestackFormatOptions::default().with_respect_file_ignore(mode == BenchMode::RealWorld);
    let context = DestackFormatContext::new_with_timings(
        options,
        DestackFormatArtifacts {
            file: file.as_ref(),
            tree: &tree,
            tokens: &tokens,
            side_tokens: &side_tokens,
            side_span: &side_span,
            strings: &strings,
            parents,
        },
        timings_enabled,
    );
    let timing_collector = context.timings.clone();
    let cache_collector = context.cache_stats.clone();
    let counter_collector = context.counters.clone();
    let file_ignore_applied = context.file_ignore_applied.clone();

    let format_started_at = Instant::now();
    let formatted = destack_fir::format!(context, [statement_list(&expressions)])
        .map_err(|error| format!("format error for {}: {error:?}", corpus_file.path.display()))?;
    let format = format_started_at.elapsed();

    let print_started_at = Instant::now();
    let printed = formatted
        .print()
        .map_err(|error| format!("print error for {}: {error:?}", corpus_file.path.display()))?;
    let print = print_started_at.elapsed();

    let output_bytes = printed.as_str().len();
    let total = total_started_at.elapsed();
    let cache = FormatterCacheStatsSnapshot {
        span_text_hits: cache_collector.span_text_hits.get(),
        span_text_misses: cache_collector.span_text_misses.get(),
        span_has_newline_hits: cache_collector.span_has_newline_hits.get(),
        span_has_newline_misses: cache_collector.span_has_newline_misses.get(),
        span_has_comment_hits: cache_collector.span_has_comment_hits.get(),
        span_has_comment_misses: cache_collector.span_has_comment_misses.get(),
        annotation_cache_hits: cache_collector.annotation_cache_hits.get(),
        annotation_cache_misses: cache_collector.annotation_cache_misses.get(),
    };
    let timings = timing_collector
        .as_ref()
        .map_or_else(Vec::new, |timings| timings.snapshot());
    let counters = counter_collector.snapshot();
    let skipped_by_file_ignore = file_ignore_applied.get();
    let (formatted_lines, skipped_lines) = if skipped_by_file_ignore {
        (0usize, corpus_file.source_lines)
    } else {
        (corpus_file.source_lines, 0usize)
    };
    let (formatted_bytes, skipped_bytes) = if skipped_by_file_ignore {
        (0usize, corpus_file.source_bytes)
    } else {
        (corpus_file.source_bytes, 0usize)
    };

    Ok(FileRunStat {
        path: corpus_file.path.clone(),
        source_lines: corpus_file.source_lines,
        formatted_lines,
        skipped_lines,
        source_bytes: corpus_file.source_bytes,
        formatted_bytes,
        skipped_bytes,
        output_bytes,
        skipped_by_file_ignore,
        parse,
        format,
        print,
        total,
        cache,
        timings,
        counters,
    })
}

/// Summarize all measured runs.
fn summarize(
    root: &Path,
    mode: BenchMode,
    workers: usize,
    warmup_runs: usize,
    top: usize,
    timings_top: usize,
    runs: Vec<BenchRunStats>,
) -> BenchSummary {
    let measured_runs = runs.len();
    let mut totals: Vec<Duration> = runs.iter().map(|run| run.total).collect();
    totals.sort_unstable();

    let min = totals.first().copied().unwrap_or_default();
    let max = totals.last().copied().unwrap_or_default();
    let total_duration: Duration = totals.iter().copied().sum();
    let mean = total_duration / measured_runs as u32;
    let median = percentile_duration(&totals, 0.5);
    let p95 = percentile_duration(&totals, 0.95);

    let mean_ms = mean.as_secs_f64() * 1000.0;
    let variance = runs
        .iter()
        .map(|run| {
            let delta = run.total.as_secs_f64() * 1000.0 - mean_ms;
            delta * delta
        })
        .sum::<f64>()
        / measured_runs as f64;
    let stddev_ms = variance.sqrt();
    let coefficient_of_variation = if mean_ms > 0.0 {
        stddev_ms / mean_ms
    } else {
        0.0
    };

    let parse_total: Duration = runs.iter().map(|run| run.parse).sum();
    let format_total: Duration = runs.iter().map(|run| run.format).sum();
    let print_total: Duration = runs.iter().map(|run| run.print).sum();
    let work_total: Duration = runs.iter().map(|run| run.work_total).sum();
    let mean_parse = parse_total / measured_runs as u32;
    let mean_format = format_total / measured_runs as u32;
    let mean_print = print_total / measured_runs as u32;
    let mean_work = work_total / measured_runs as u32;

    let source_bytes_per_run = runs.first().map_or(0, |run| run.source_bytes);
    let source_lines_per_run = runs.first().map_or(0, |run| run.source_lines);
    let formatted_bytes_per_run = runs.first().map_or(0, |run| run.formatted_bytes);
    let formatted_lines_per_run = runs.first().map_or(0, |run| run.formatted_lines);
    let skipped_bytes_per_run = runs.first().map_or(0, |run| run.skipped_bytes);
    let skipped_lines_per_run = runs.first().map_or(0, |run| run.skipped_lines);
    let formatted_files_per_run = runs.first().map_or(0, |run| run.formatted_files);
    let skipped_files_per_run = runs.first().map_or(0, |run| run.skipped_files);
    let output_bytes_per_run = runs.first().map_or(0, |run| run.output_bytes);
    let sink = runs.iter().fold(0usize, |acc, run| acc ^ run.sink);

    let files = runs.first().map_or(0, |run| run.files.len());
    let files_per_second = if mean.is_zero() {
        f64::INFINITY
    } else {
        files as f64 / mean.as_secs_f64()
    };
    let lines_per_second = if mean.is_zero() {
        f64::INFINITY
    } else {
        source_lines_per_run as f64 / mean.as_secs_f64()
    };
    let formatted_lines_per_second = if mean.is_zero() {
        f64::INFINITY
    } else {
        formatted_lines_per_run as f64 / mean.as_secs_f64()
    };
    let parse_lines_per_second = if mean_parse.is_zero() {
        f64::INFINITY
    } else {
        source_lines_per_run as f64 / mean_parse.as_secs_f64()
    };
    let format_lines_per_second = if mean_format.is_zero() {
        f64::INFINITY
    } else {
        source_lines_per_run as f64 / mean_format.as_secs_f64()
    };
    let format_formatted_lines_per_second = if mean_format.is_zero() {
        f64::INFINITY
    } else {
        formatted_lines_per_run as f64 / mean_format.as_secs_f64()
    };
    let print_lines_per_second = if mean_print.is_zero() {
        f64::INFINITY
    } else {
        source_lines_per_run as f64 / mean_print.as_secs_f64()
    };
    let input_mib_per_second = if mean.is_zero() {
        f64::INFINITY
    } else {
        (source_bytes_per_run as f64 / (1024.0 * 1024.0)) / mean.as_secs_f64()
    };
    let output_mib_per_second = if mean.is_zero() {
        f64::INFINITY
    } else {
        (output_bytes_per_run as f64 / (1024.0 * 1024.0)) / mean.as_secs_f64()
    };

    let mut cache_total = FormatterCacheStatsSnapshot::default();
    for run in &runs {
        cache_total.span_text_hits = cache_total
            .span_text_hits
            .saturating_add(run.cache.span_text_hits);
        cache_total.span_text_misses = cache_total
            .span_text_misses
            .saturating_add(run.cache.span_text_misses);
        cache_total.span_has_newline_hits = cache_total
            .span_has_newline_hits
            .saturating_add(run.cache.span_has_newline_hits);
        cache_total.span_has_newline_misses = cache_total
            .span_has_newline_misses
            .saturating_add(run.cache.span_has_newline_misses);
        cache_total.span_has_comment_hits = cache_total
            .span_has_comment_hits
            .saturating_add(run.cache.span_has_comment_hits);
        cache_total.span_has_comment_misses = cache_total
            .span_has_comment_misses
            .saturating_add(run.cache.span_has_comment_misses);
        cache_total.annotation_cache_hits = cache_total
            .annotation_cache_hits
            .saturating_add(run.cache.annotation_cache_hits);
        cache_total.annotation_cache_misses = cache_total
            .annotation_cache_misses
            .saturating_add(run.cache.annotation_cache_misses);
    }
    let cache = FormatterCacheStatsSnapshot {
        span_text_hits: cache_total.span_text_hits / measured_runs.max(1),
        span_text_misses: cache_total.span_text_misses / measured_runs.max(1),
        span_has_newline_hits: cache_total.span_has_newline_hits / measured_runs.max(1),
        span_has_newline_misses: cache_total.span_has_newline_misses / measured_runs.max(1),
        span_has_comment_hits: cache_total.span_has_comment_hits / measured_runs.max(1),
        span_has_comment_misses: cache_total.span_has_comment_misses / measured_runs.max(1),
        annotation_cache_hits: cache_total.annotation_cache_hits / measured_runs.max(1),
        annotation_cache_misses: cache_total.annotation_cache_misses / measured_runs.max(1),
    };

    let timing_tags = summarize_timing_tags(&runs, timings_top);
    let counters = summarize_counters(&runs, timings_top);
    let top_files = summarize_files(&runs, top);

    BenchSummary {
        root: root.to_path_buf(),
        mode,
        workers,
        files,
        formatted_files_per_run,
        skipped_files_per_run,
        warmup_runs,
        measured_runs,
        source_lines_per_run,
        formatted_lines_per_run,
        skipped_lines_per_run,
        source_bytes_per_run,
        formatted_bytes_per_run,
        skipped_bytes_per_run,
        output_bytes_per_run,
        sink,
        min,
        max,
        mean,
        median,
        p95,
        stddev_ms,
        coefficient_of_variation,
        mean_parse,
        mean_format,
        mean_print,
        mean_work,
        files_per_second,
        lines_per_second,
        formatted_lines_per_second,
        parse_lines_per_second,
        format_lines_per_second,
        format_formatted_lines_per_second,
        print_lines_per_second,
        input_mib_per_second,
        output_mib_per_second,
        cache,
        timing_tags,
        counters,
        runs,
        top_files,
    }
}

/// Aggregate per-file statistics and keep the top slow files.
fn summarize_files(runs: &[BenchRunStats], top: usize) -> Vec<FileAggregate> {
    let Some(first_run) = runs.first() else {
        return Vec::new();
    };

    let mut aggregates = first_run
        .files
        .iter()
        .map(|file| FileAggregate {
            path: file.path.clone(),
            runs: 0,
            source_lines: file.source_lines,
            source_bytes: file.source_bytes,
            output_bytes_total: 0,
            parse_total: Duration::ZERO,
            format_total: Duration::ZERO,
            print_total: Duration::ZERO,
            total: Duration::ZERO,
        })
        .collect::<Vec<_>>();

    for run in runs {
        for (index, file) in run.files.iter().enumerate() {
            let entry = &mut aggregates[index];
            entry.runs += 1;
            entry.output_bytes_total = entry.output_bytes_total.saturating_add(file.output_bytes);
            entry.parse_total += file.parse;
            entry.format_total += file.format;
            entry.print_total += file.print;
            entry.total += file.total;
        }
    }

    aggregates.sort_by(|left, right| {
        let left_avg = left.total.as_secs_f64() / left.runs.max(1) as f64;
        let right_avg = right.total.as_secs_f64() / right.runs.max(1) as f64;
        right_avg
            .partial_cmp(&left_avg)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    aggregates.truncate(top.min(aggregates.len()));
    aggregates
}

/// Aggregate timing tags across runs and keep the top slow tags.
fn summarize_timing_tags(runs: &[BenchRunStats], top: usize) -> Vec<TimingAggregate> {
    let mut timing_totals = HashMap::<&'static str, (Duration, usize)>::new();
    for run in runs {
        for entry in &run.timings {
            let aggregate = timing_totals
                .entry(entry.name)
                .or_insert((Duration::ZERO, 0usize));
            aggregate.0 += entry.duration;
            aggregate.1 = aggregate.1.saturating_add(entry.count);
        }
    }

    let mut aggregates = timing_totals
        .into_iter()
        .map(|(name, (duration, count))| TimingAggregate {
            name,
            duration: duration / runs.len().max(1) as u32,
            count: count / runs.len().max(1),
        })
        .collect::<Vec<_>>();
    aggregates.sort_by_key(|entry| std::cmp::Reverse(entry.duration));
    aggregates.truncate(top.min(aggregates.len()));
    aggregates
}

/// Aggregate instrumentation counters across runs and keep the top counters.
fn summarize_counters(runs: &[BenchRunStats], top: usize) -> Vec<CounterAggregate> {
    let mut counter_totals = HashMap::<&'static str, usize>::new();
    for run in runs {
        for entry in &run.counters {
            let aggregate = counter_totals.entry(entry.name).or_insert(0);
            *aggregate = aggregate.saturating_add(entry.value);
        }
    }

    let mut aggregates = counter_totals
        .into_iter()
        .map(|(name, value)| CounterAggregate {
            name,
            value: value / runs.len().max(1),
        })
        .collect::<Vec<_>>();
    aggregates.sort_by_key(|entry| std::cmp::Reverse(entry.value));
    aggregates.truncate(top.min(aggregates.len()));
    aggregates
}

/// Print benchmark output in the requested format.
fn print_output(summary: &BenchSummary, output: Output, color: bool) {
    match output {
        Output::Table => print_table(summary, color),
        Output::Csv => print_csv(summary),
        Output::Json => print_json(summary),
    }
}

/// Print human-readable benchmark output.
fn print_table(summary: &BenchSummary, color: bool) {
    let style = TableStyle::new(color);
    let output_ratio =
        summary.output_bytes_per_run as f64 / summary.source_bytes_per_run.max(1) as f64;
    let output_ratio_delta = (output_ratio - 1.0).abs();
    let output_ratio_heat = style.color_for_ratio((output_ratio_delta / 0.20).clamp(0.0, 1.0));

    println!(
        "{bold}formatter bench stats{reset}",
        bold = style.bold,
        reset = style.reset
    );
    println!();

    println!(
        "{bold}run timings{reset}",
        bold = style.bold,
        reset = style.reset
    );
    println!(
        "  {bold}{:<5} {:>10} {:>10} {:>10} {:>10} {:>9} {:>10} {:>10}{reset}",
        "run",
        "parse",
        "format",
        "print",
        "total",
        "share",
        "lines/s",
        "fmt cpu/s",
        bold = style.bold,
        reset = style.reset
    );
    println!(
        "  {dim}{}{reset}",
        "-".repeat(87),
        dim = style.dim,
        reset = style.reset
    );

    for (index, run) in summary.runs.iter().enumerate() {
        let share = run.total.as_secs_f64() / summary.mean.as_secs_f64().max(0.000_001);
        let heat = style.color_for_range(run.total, summary.min, summary.max);
        let run_lines_per_second = run.source_lines as f64 / run.total.as_secs_f64().max(0.000_001);
        let run_format_lines_per_second =
            run.formatted_lines as f64 / run.format.as_secs_f64().max(0.000_001);
        println!(
            "  {:<5} {:>10} {:>10} {:>10} {heat}{:>10}{reset} {:>8} {:>10} {:>10}",
            index + 1,
            format_duration(run.parse),
            format_duration(run.format),
            format_duration(run.print),
            format_duration(run.total),
            format_percent(share),
            format_count_rate_per_second(run_lines_per_second),
            format_count_rate_per_second(run_format_lines_per_second),
            heat = heat,
            reset = style.reset
        );
    }

    println!(
        "  {dim}share is run total divided by run mean total{reset}",
        dim = style.dim,
        reset = style.reset
    );
    println!(
        "  {dim}fmt cpu/s is formatted lines divided by cumulative format stage time{reset}",
        dim = style.dim,
        reset = style.reset
    );

    if !summary.timing_tags.is_empty() {
        println!();
        println!(
            "{bold}timing tags (avg per run){reset}",
            bold = style.bold,
            reset = style.reset
        );
        println!(
            "  {bold}{:<tag_width$} {:>10} {:>9} {:>8} {:>10}{reset}",
            "tag",
            "total",
            "share",
            "count",
            "avg",
            tag_width = TIMING_TAG_WIDTH,
            bold = style.bold,
            reset = style.reset
        );
        println!(
            "  {dim}{}{reset}",
            "-".repeat(TIMING_TAG_WIDTH + 47),
            dim = style.dim,
            reset = style.reset
        );

        for entry in &summary.timing_tags {
            let share =
                entry.duration.as_secs_f64() / summary.mean_work.as_secs_f64().max(0.000_001);
            let heat = style.color_for_ratio(share);
            let avg_duration = entry.duration / entry.count.max(1) as u32;
            let name = truncate_middle(entry.name, TIMING_TAG_WIDTH);

            println!(
                "  {cyan}{:<tag_width$}{reset} {heat}{:>10}{reset} {:>8} {:>8} {:>10}",
                name,
                format_duration(entry.duration),
                format_percent(share),
                format_count(entry.count),
                format_duration(avg_duration),
                tag_width = TIMING_TAG_WIDTH,
                cyan = style.cyan,
                heat = heat,
                reset = style.reset
            );
        }

        println!(
            "  {dim}share is timing tag mean total divided by run mean work total, nested tags may exceed 100%{reset}",
            dim = style.dim,
            reset = style.reset
        );
    }

    if !summary.counters.is_empty() {
        println!();
        println!(
            "{bold}instrumentation counters (avg per run){reset}",
            bold = style.bold,
            reset = style.reset
        );
        println!(
            "  {bold}{:<tag_width$} {:>12}{reset}",
            "counter",
            "value",
            tag_width = TIMING_TAG_WIDTH,
            bold = style.bold,
            reset = style.reset
        );
        println!(
            "  {dim}{}{reset}",
            "-".repeat(TIMING_TAG_WIDTH + 15),
            dim = style.dim,
            reset = style.reset
        );

        for entry in &summary.counters {
            let name = truncate_middle(entry.name, TIMING_TAG_WIDTH);
            println!(
                "  {cyan}{:<tag_width$}{reset} {:>12}",
                name,
                format_count(entry.value),
                tag_width = TIMING_TAG_WIDTH,
                cyan = style.cyan,
                reset = style.reset
            );
        }
    }

    if !summary.top_files.is_empty() {
        println!();
        println!(
            "{bold}hot files (avg per run){reset}",
            bold = style.bold,
            reset = style.reset
        );
        println!(
            "  {bold}{:<name_width$} {:>10} {:>10} {:>10} {:>10} {:>9} {:>8} {:>8} {:<16}{reset}",
            "file",
            "parse",
            "format",
            "print",
            "total",
            "share",
            "lines",
            "bytes",
            "hot",
            name_width = HOT_FILE_NAME_WIDTH,
            bold = style.bold,
            reset = style.reset
        );
        println!(
            "  {dim}{}{reset}",
            "-".repeat(HOT_FILE_NAME_WIDTH + 102),
            dim = style.dim,
            reset = style.reset
        );

        for file in &summary.top_files {
            let runs = file.runs.max(1) as u32;
            let parse = file.parse_total / runs;
            let format = file.format_total / runs;
            let print = file.print_total / runs;
            let total = file.total / runs;
            let share = total.as_secs_f64() / summary.mean_work.as_secs_f64().max(0.000_001);
            let heat = style.color_for_ratio(share);

            let relative = file
                .path
                .strip_prefix(summary.root.as_path())
                .unwrap_or(file.path.as_path())
                .display()
                .to_string();
            let name = truncate_middle(relative.as_str(), HOT_FILE_NAME_WIDTH);

            println!(
                "  {cyan}{:<name_width$}{reset} {:>10} {:>10} {:>10} {heat}{:>10}{reset} {:>8} {:>8} {:>8} {heat}{:<16}{reset}",
                name,
                format_duration(parse),
                format_duration(format),
                format_duration(print),
                format_duration(total),
                format_percent(share),
                format_count(file.source_lines),
                format_bytes_compact(file.source_bytes),
                format_share_bar(share.min(1.0), SHARE_BAR_WIDTH),
                name_width = HOT_FILE_NAME_WIDTH,
                cyan = style.cyan,
                heat = heat,
                reset = style.reset,
            );
        }

        println!(
            "  {dim}share is file mean total divided by run mean work total{reset}",
            dim = style.dim,
            reset = style.reset
        );
    }

    println!();
    println!(
        "{bold}summary{reset}",
        bold = style.bold,
        reset = style.reset
    );
    println!(
        "  root:              {cyan}{}{reset}",
        summary.root.display(),
        cyan = style.cyan,
        reset = style.reset
    );
    println!("  mode:              {}", summary.mode.as_str());
    println!(
        "  corpus:            {} files, {} lines, {} source, {} output",
        format_count(summary.files),
        format_count(summary.source_lines_per_run),
        format_bytes(summary.source_bytes_per_run),
        format_bytes(summary.output_bytes_per_run)
    );
    println!(
        "  formatted:         {} files, {} lines, {} source",
        format_count(summary.formatted_files_per_run),
        format_count(summary.formatted_lines_per_run),
        format_bytes(summary.formatted_bytes_per_run)
    );
    println!(
        "  skipped:           {} files, {} lines, {} source",
        format_count(summary.skipped_files_per_run),
        format_count(summary.skipped_lines_per_run),
        format_bytes(summary.skipped_bytes_per_run)
    );
    println!(
        "  runs:              warmup {}, measured {}",
        format_count(summary.warmup_runs),
        format_count(summary.measured_runs)
    );
    println!();

    println!(
        "  min total:         {min}{:>10}{reset}",
        format_duration(summary.min),
        min = style.color_for_range(summary.min, summary.min, summary.max),
        reset = style.reset
    );
    println!(
        "  mean total:        {mean}{:>10}{reset}",
        format_duration(summary.mean),
        mean = style.cyan,
        reset = style.reset
    );
    println!(
        "  mean work total:   {:>10}",
        format_duration(summary.mean_work)
    );
    println!(
        "  median total:      {median}{:>10}{reset}",
        format_duration(summary.median),
        median = style.color_for_range(summary.median, summary.min, summary.max),
        reset = style.reset
    );
    println!(
        "  p95 total:         {p95}{:>10}{reset}",
        format_duration(summary.p95),
        p95 = style.color_for_range(summary.p95, summary.min, summary.max),
        reset = style.reset
    );
    println!(
        "  max total:         {max}{:>10}{reset}",
        format_duration(summary.max),
        max = style.color_for_range(summary.max, summary.min, summary.max),
        reset = style.reset
    );
    println!(
        "  stddev:            {cv_heat}{:>10}{reset}",
        format_duration(Duration::from_secs_f64(summary.stddev_ms / 1000.0)),
        cv_heat = style.color_for_ratio((summary.coefficient_of_variation / 0.10).clamp(0.0, 1.0)),
        reset = style.reset
    );
    println!(
        "  jitter (cv):       {cv_heat}{:>9}{reset}",
        format_percent(summary.coefficient_of_variation),
        cv_heat = style.color_for_ratio((summary.coefficient_of_variation / 0.10).clamp(0.0, 1.0)),
        reset = style.reset
    );

    let phase_total = summary.mean_parse + summary.mean_format + summary.mean_print;
    let parse_ratio = summary.mean_parse.as_secs_f64() / phase_total.as_secs_f64().max(0.000_001);
    let format_ratio = summary.mean_format.as_secs_f64() / phase_total.as_secs_f64().max(0.000_001);
    let print_ratio = summary.mean_print.as_secs_f64() / phase_total.as_secs_f64().max(0.000_001);
    let workers = summary.workers.max(1);
    let effective_parallelism =
        phase_total.as_secs_f64() / summary.mean.as_secs_f64().max(0.000_001);
    let parallel_efficiency = effective_parallelism / workers as f64;
    let wall_stage_denominator = effective_parallelism.max(0.000_001);
    let parse_wall_est =
        Duration::from_secs_f64(summary.mean_parse.as_secs_f64() / wall_stage_denominator);
    let format_wall_est =
        Duration::from_secs_f64(summary.mean_format.as_secs_f64() / wall_stage_denominator);
    let print_wall_est =
        Duration::from_secs_f64(summary.mean_print.as_secs_f64() / wall_stage_denominator);

    println!("  workers:           {:>10}", format_count(workers));
    println!(
        "  effective workers: {:>10}",
        format!("{effective_parallelism:>.2}x")
    );
    println!(
        "  parallel eff:      {:>10}",
        format_percent(parallel_efficiency)
    );
    println!("  phase cpu total:   {:>10}", format_duration(phase_total));

    println!(
        "  parse stage cpu:   {parse_heat}{:>10}{reset} ({:>6})  {}",
        format_duration(summary.mean_parse),
        format_percent(parse_ratio),
        format_share_bar(parse_ratio, SHARE_BAR_WIDTH),
        parse_heat = style.color_for_ratio(parse_ratio),
        reset = style.reset
    );
    println!(
        "  format stage cpu:  {format_heat}{:>10}{reset} ({:>6})  {}",
        format_duration(summary.mean_format),
        format_percent(format_ratio),
        format_share_bar(format_ratio, SHARE_BAR_WIDTH),
        format_heat = style.color_for_ratio(format_ratio),
        reset = style.reset
    );
    println!(
        "  print stage cpu:   {print_heat}{:>10}{reset} ({:>6})  {}",
        format_duration(summary.mean_print),
        format_percent(print_ratio),
        format_share_bar(print_ratio, SHARE_BAR_WIDTH),
        print_heat = style.color_for_ratio(print_ratio),
        reset = style.reset
    );
    println!(
        "  parse stage wall:  {:>10} ({:>6})",
        format_duration(parse_wall_est),
        format_percent(parse_ratio)
    );
    println!(
        "  format stage wall: {:>10} ({:>6})",
        format_duration(format_wall_est),
        format_percent(format_ratio)
    );
    println!(
        "  print stage wall:  {:>10} ({:>6})",
        format_duration(print_wall_est),
        format_percent(print_ratio)
    );
    println!(
        "  parse cpu lines/s: {:>10}",
        format_count_rate_per_second(summary.parse_lines_per_second)
    );
    println!(
        "  format cpu lines/s: {:>9}",
        format_count_rate_per_second(summary.format_lines_per_second)
    );
    println!(
        "  fmt work cpu/s:    {:>10}",
        format_count_rate_per_second(summary.format_formatted_lines_per_second)
    );
    println!(
        "  print cpu lines/s: {:>10}",
        format_count_rate_per_second(summary.print_lines_per_second)
    );
    println!(
        "  {dim}stage cpu rows sum cumulative worker cpu time{reset}",
        dim = style.dim,
        reset = style.reset
    );
    println!(
        "  {dim}stage wall rows estimate wall share using effective workers{reset}",
        dim = style.dim,
        reset = style.reset
    );

    println!(
        "  files/s:           {:>10}",
        format_count_rate_per_second(summary.files_per_second)
    );
    println!(
        "  lines/s:           {:>10}",
        format_count_rate_per_second(summary.lines_per_second)
    );
    println!(
        "  lines/s work:      {:>10}",
        format_count_rate_per_second(summary.formatted_lines_per_second)
    );
    println!(
        "  input rate:        {:>10}",
        format_data_rate_mib_per_second(summary.input_mib_per_second)
    );
    println!(
        "  output rate:       {:>10}",
        format_data_rate_mib_per_second(summary.output_mib_per_second)
    );
    println!(
        "  output/input:      {ratio_heat}{:>9}{reset}",
        format_percent(output_ratio),
        ratio_heat = output_ratio_heat,
        reset = style.reset
    );

    let span_text_lookups = summary
        .cache
        .span_text_hits
        .saturating_add(summary.cache.span_text_misses);
    let span_newline_lookups = summary
        .cache
        .span_has_newline_hits
        .saturating_add(summary.cache.span_has_newline_misses);
    let span_comment_lookups = summary
        .cache
        .span_has_comment_hits
        .saturating_add(summary.cache.span_has_comment_misses);
    let annotation_lookups = summary
        .cache
        .annotation_cache_hits
        .saturating_add(summary.cache.annotation_cache_misses);
    if span_text_lookups > 0
        || span_newline_lookups > 0
        || span_comment_lookups > 0
        || annotation_lookups > 0
    {
        println!(
            "  cache span text:   {} / {} ({})",
            format_count(summary.cache.span_text_hits),
            format_count(span_text_lookups),
            format_hit_rate(summary.cache.span_text_hits, summary.cache.span_text_misses)
        );
        println!(
            "  cache newline:     {} / {} ({})",
            format_count(summary.cache.span_has_newline_hits),
            format_count(span_newline_lookups),
            format_hit_rate(
                summary.cache.span_has_newline_hits,
                summary.cache.span_has_newline_misses
            )
        );
        println!(
            "  cache comment:     {} / {} ({})",
            format_count(summary.cache.span_has_comment_hits),
            format_count(span_comment_lookups),
            format_hit_rate(
                summary.cache.span_has_comment_hits,
                summary.cache.span_has_comment_misses
            )
        );
        println!(
            "  cache annotation:  {} / {} ({})",
            format_count(summary.cache.annotation_cache_hits),
            format_count(annotation_lookups),
            format_hit_rate(
                summary.cache.annotation_cache_hits,
                summary.cache.annotation_cache_misses
            )
        );
    }
    println!("  sink:              {:>10}", format_count(summary.sink));
}

/// Print csv benchmark output.
fn print_csv(summary: &BenchSummary) {
    println!(
        "root,mode,files,formatted_files_per_run,skipped_files_per_run,warmup_runs,measured_runs,source_lines_per_run,formatted_lines_per_run,skipped_lines_per_run,source_bytes_per_run,formatted_bytes_per_run,skipped_bytes_per_run,output_bytes_per_run,min_ms,mean_ms,median_ms,p95_ms,max_ms,stddev_ms,cv_pct,parse_mean_ms,format_mean_ms,print_mean_ms,files_per_sec,lines_per_sec,formatted_lines_per_sec,parse_lines_per_sec,format_cpu_lines_per_sec,format_work_cpu_lines_per_sec,print_lines_per_sec,input_mib_per_sec,output_mib_per_sec,span_text_hits,span_text_misses,span_newline_hits,span_newline_misses,span_comment_hits,span_comment_misses,annotation_hits,annotation_misses,sink"
    );
    let summary_row = vec![
        format!("\"{}\"", summary.root.display()),
        format!("\"{}\"", summary.mode.as_str()),
        summary.files.to_string(),
        summary.formatted_files_per_run.to_string(),
        summary.skipped_files_per_run.to_string(),
        summary.warmup_runs.to_string(),
        summary.measured_runs.to_string(),
        summary.source_lines_per_run.to_string(),
        summary.formatted_lines_per_run.to_string(),
        summary.skipped_lines_per_run.to_string(),
        summary.source_bytes_per_run.to_string(),
        summary.formatted_bytes_per_run.to_string(),
        summary.skipped_bytes_per_run.to_string(),
        summary.output_bytes_per_run.to_string(),
        format!("{:.3}", duration_ms(summary.min)),
        format!("{:.3}", duration_ms(summary.mean)),
        format!("{:.3}", duration_ms(summary.median)),
        format!("{:.3}", duration_ms(summary.p95)),
        format!("{:.3}", duration_ms(summary.max)),
        format!("{:.3}", summary.stddev_ms),
        format!("{:.3}", summary.coefficient_of_variation * 100.0),
        format!("{:.3}", duration_ms(summary.mean_parse)),
        format!("{:.3}", duration_ms(summary.mean_format)),
        format!("{:.3}", duration_ms(summary.mean_print)),
        format!("{:.3}", summary.files_per_second),
        format!("{:.3}", summary.lines_per_second),
        format!("{:.3}", summary.formatted_lines_per_second),
        format!("{:.3}", summary.parse_lines_per_second),
        format!("{:.3}", summary.format_lines_per_second),
        format!("{:.3}", summary.format_formatted_lines_per_second),
        format!("{:.3}", summary.print_lines_per_second),
        format!("{:.3}", summary.input_mib_per_second),
        format!("{:.3}", summary.output_mib_per_second),
        summary.cache.span_text_hits.to_string(),
        summary.cache.span_text_misses.to_string(),
        summary.cache.span_has_newline_hits.to_string(),
        summary.cache.span_has_newline_misses.to_string(),
        summary.cache.span_has_comment_hits.to_string(),
        summary.cache.span_has_comment_misses.to_string(),
        summary.cache.annotation_cache_hits.to_string(),
        summary.cache.annotation_cache_misses.to_string(),
        summary.sink.to_string(),
    ];
    println!("{}", summary_row.join(","));

    println!();
    println!(
        "run,total_ms,work_ms,parse_ms,format_ms,print_ms,source_lines,formatted_lines,skipped_lines,lines_per_sec,format_cpu_lines_per_sec,format_work_cpu_lines_per_sec"
    );
    for (index, run) in summary.runs.iter().enumerate() {
        let run_lines_per_second = run.source_lines as f64 / run.total.as_secs_f64().max(0.000_001);
        let run_format_lines_per_second =
            run.source_lines as f64 / run.format.as_secs_f64().max(0.000_001);
        let run_format_work_lines_per_second =
            run.formatted_lines as f64 / run.format.as_secs_f64().max(0.000_001);
        println!(
            "{}",
            [
                (index + 1).to_string(),
                format!("{:.3}", duration_ms(run.total)),
                format!("{:.3}", duration_ms(run.work_total)),
                format!("{:.3}", duration_ms(run.parse)),
                format!("{:.3}", duration_ms(run.format)),
                format!("{:.3}", duration_ms(run.print)),
                run.source_lines.to_string(),
                run.formatted_lines.to_string(),
                run.skipped_lines.to_string(),
                format!("{run_lines_per_second:.3}"),
                format!("{run_format_lines_per_second:.3}"),
                format!("{run_format_work_lines_per_second:.3}"),
            ]
            .join(","),
        );
    }

    println!();
    println!("file,parse_ms,format_ms,print_ms,total_ms,share_pct,source_lines,source_bytes");
    for file in &summary.top_files {
        let runs = file.runs.max(1) as u32;
        let parse = file.parse_total / runs;
        let format = file.format_total / runs;
        let print = file.print_total / runs;
        let total = file.total / runs;
        let share = total.as_secs_f64() / summary.mean_work.as_secs_f64().max(0.000_001);
        println!(
            "\"{}\",{:.3},{:.3},{:.3},{:.3},{:.3},{},{}",
            file.path.display(),
            duration_ms(parse),
            duration_ms(format),
            duration_ms(print),
            duration_ms(total),
            share * 100.0,
            file.source_lines,
            file.source_bytes,
        );
    }

    if !summary.timing_tags.is_empty() {
        println!();
        println!("timing_tag,total_ms,share_pct,samples,avg_ms");
        for entry in &summary.timing_tags {
            let share =
                entry.duration.as_secs_f64() / summary.mean_work.as_secs_f64().max(0.000_001);
            let avg_duration = entry.duration / entry.count.max(1) as u32;
            println!(
                "\"{}\",{:.3},{:.3},{},{:.3}",
                entry.name,
                duration_ms(entry.duration),
                share * 100.0,
                entry.count,
                duration_ms(avg_duration),
            );
        }
    }

    if !summary.counters.is_empty() {
        println!();
        println!("counter,value");
        for entry in &summary.counters {
            println!("\"{}\",{}", entry.name, entry.value);
        }
    }
}

/// Print json benchmark output.
fn print_json(summary: &BenchSummary) {
    let runs = summary
        .runs
        .iter()
        .map(|run| {
            json!({
                "total_ms": duration_ms(run.total),
                "work_ms": duration_ms(run.work_total),
                "parse_ms": duration_ms(run.parse),
                "format_ms": duration_ms(run.format),
                "print_ms": duration_ms(run.print),
                "lines_per_sec": run.source_lines as f64 / run.total.as_secs_f64().max(0.000_001),
                "formatted_lines_per_sec": run.formatted_lines as f64 / run.total.as_secs_f64().max(0.000_001),
                "format_lines_per_sec": run.source_lines as f64 / run.format.as_secs_f64().max(0.000_001),
                "format_work_lines_per_sec": run.formatted_lines as f64 / run.format.as_secs_f64().max(0.000_001),
                "source_lines": run.source_lines,
                "formatted_lines": run.formatted_lines,
                "skipped_lines": run.skipped_lines,
                "source_bytes": run.source_bytes,
                "formatted_bytes": run.formatted_bytes,
                "skipped_bytes": run.skipped_bytes,
                "output_bytes": run.output_bytes,
            })
        })
        .collect::<Vec<_>>();

    let top_files = summary
        .top_files
        .iter()
        .map(|file| {
            let runs = file.runs.max(1) as u32;
            let parse = file.parse_total / runs;
            let format = file.format_total / runs;
            let print = file.print_total / runs;
            let total = file.total / runs;
            let share = total.as_secs_f64() / summary.mean_work.as_secs_f64().max(0.000_001);
            json!({
                "path": file.path.display().to_string(),
                "runs": file.runs,
                "source_lines": file.source_lines,
                "source_bytes": file.source_bytes,
                "output_bytes_per_run": file.output_bytes_total / file.runs.max(1),
                "parse_ms": duration_ms(parse),
                "format_ms": duration_ms(format),
                "print_ms": duration_ms(print),
                "total_ms": duration_ms(total),
                "share_pct": share * 100.0,
            })
        })
        .collect::<Vec<_>>();

    let timing_tags = summary
        .timing_tags
        .iter()
        .map(|entry| {
            let share =
                entry.duration.as_secs_f64() / summary.mean_work.as_secs_f64().max(0.000_001);
            let avg_duration = entry.duration / entry.count.max(1) as u32;
            json!({
                "name": entry.name,
                "total_ms": duration_ms(entry.duration),
                "share_pct": share * 100.0,
                "samples": entry.count,
                "avg_ms": duration_ms(avg_duration),
            })
        })
        .collect::<Vec<_>>();

    let counters = summary
        .counters
        .iter()
        .map(|entry| {
            json!({
                "name": entry.name,
                "value": entry.value,
            })
        })
        .collect::<Vec<_>>();

    let payload = json!({
        "root": summary.root.display().to_string(),
        "mode": summary.mode.as_str(),
        "files": summary.files,
        "formatted_files_per_run": summary.formatted_files_per_run,
        "skipped_files_per_run": summary.skipped_files_per_run,
        "warmup_runs": summary.warmup_runs,
        "measured_runs": summary.measured_runs,
        "source_lines_per_run": summary.source_lines_per_run,
        "formatted_lines_per_run": summary.formatted_lines_per_run,
        "skipped_lines_per_run": summary.skipped_lines_per_run,
        "source_bytes_per_run": summary.source_bytes_per_run,
        "formatted_bytes_per_run": summary.formatted_bytes_per_run,
        "skipped_bytes_per_run": summary.skipped_bytes_per_run,
        "output_bytes_per_run": summary.output_bytes_per_run,
        "sink": summary.sink,
        "timing_ms": {
            "min": duration_ms(summary.min),
            "mean": duration_ms(summary.mean),
            "work_mean": duration_ms(summary.mean_work),
            "median": duration_ms(summary.median),
            "p95": duration_ms(summary.p95),
            "max": duration_ms(summary.max),
            "stddev": summary.stddev_ms,
            "coefficient_of_variation_pct": summary.coefficient_of_variation * 100.0,
        },
        "phase_mean_ms": {
            "parse": duration_ms(summary.mean_parse),
            "format": duration_ms(summary.mean_format),
            "print": duration_ms(summary.mean_print),
        },
        "throughput": {
            "files_per_sec": summary.files_per_second,
            "lines_per_sec": summary.lines_per_second,
            "formatted_lines_per_sec": summary.formatted_lines_per_second,
            "parse_lines_per_sec": summary.parse_lines_per_second,
            "format_lines_per_sec": summary.format_lines_per_second,
            "format_work_lines_per_sec": summary.format_formatted_lines_per_second,
            "print_lines_per_sec": summary.print_lines_per_second,
            "input_mib_per_sec": summary.input_mib_per_second,
            "output_mib_per_sec": summary.output_mib_per_second,
        },
        "cache": {
            "span_text_hits": summary.cache.span_text_hits,
            "span_text_misses": summary.cache.span_text_misses,
            "span_newline_hits": summary.cache.span_has_newline_hits,
            "span_newline_misses": summary.cache.span_has_newline_misses,
            "span_comment_hits": summary.cache.span_has_comment_hits,
            "span_comment_misses": summary.cache.span_has_comment_misses,
            "annotation_hits": summary.cache.annotation_cache_hits,
            "annotation_misses": summary.cache.annotation_cache_misses,
        },
        "runs": runs,
        "top_files": top_files,
        "timing_tags": timing_tags,
        "counters": counters,
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&payload).expect("bench_stats json payload should serialize")
    );
}

/// Load all supported source files under one or more roots.
fn load_corpus_files(roots: &[PathBuf]) -> Result<(Vec<CorpusFile>, CorpusLoadStats), String> {
    if roots.is_empty() {
        return Err("no corpus roots selected".to_string());
    }

    let mut file_paths = Vec::new();
    for root in roots {
        if !root.exists() {
            return Err(format!("root path does not exist: {}", root.display()));
        }

        collect_supported_files(root.as_path(), &mut file_paths).map_err(|error| {
            format!(
                "failed collecting source files under {}: {error}",
                root.display()
            )
        })?;
    }
    file_paths.sort();
    file_paths.dedup();

    let mut files = Vec::with_capacity(file_paths.len());
    let mut stats = CorpusLoadStats::default();
    for path in file_paths {
        let file_type = FileType::from_path(path.as_path())
            .ok_or_else(|| format!("unsupported source file extension: {}", path.display()))?;
        let source_bytes = match fs::read(path.as_path()) {
            Ok(source_bytes) => source_bytes,
            Err(_) => {
                stats.skipped_read_errors = stats.skipped_read_errors.saturating_add(1);
                continue;
            }
        };
        let source = match String::from_utf8(source_bytes) {
            Ok(source) => source,
            Err(_) => {
                stats.skipped_non_utf8 = stats.skipped_non_utf8.saturating_add(1);
                continue;
            }
        };
        let source_bytes = source.len();
        let source_lines = source.lines().count();
        files.push(CorpusFile {
            path,
            file_type,
            source,
            source_bytes,
            source_lines,
        });
    }
    Ok((files, stats))
}

/// Recursively collect supported source files.
fn collect_supported_files(root: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    if root.is_file() {
        if path_is_supported_source(root) {
            files.push(root.to_path_buf());
        }
        return Ok(());
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_supported_files(path.as_path(), files)?;
            continue;
        }

        if path_is_supported_source(path.as_path()) {
            files.push(path);
        }
    }

    Ok(())
}

/// Return whether a path is a formatter-supported source file.
fn path_is_supported_source(path: &Path) -> bool {
    matches!(
        FileType::from_path(path),
        Some(
            FileType::Destack
                | FileType::TypeScript
                | FileType::TypeScriptXml
                | FileType::JavaScript
                | FileType::JavaScriptXml
        )
    )
}

/// Compute an interpolated percentile duration from a sorted sample list.
fn percentile_duration(sorted: &[Duration], percentile: f64) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }

    let last_index = sorted.len() - 1;
    let position = (last_index as f64) * percentile.clamp(0.0, 1.0);
    let lower_index = position.floor() as usize;
    let upper_index = position.ceil() as usize;

    if lower_index == upper_index {
        return sorted[lower_index];
    }

    let lower = sorted[lower_index].as_secs_f64();
    let upper = sorted[upper_index].as_secs_f64();
    let weight = position - lower_index as f64;
    Duration::from_secs_f64(lower + (upper - lower) * weight)
}

/// Format a duration for readable output.
fn format_duration(duration: Duration) -> String {
    let milliseconds = duration_ms(duration);
    if milliseconds >= 1000.0 {
        return format!("{:.3}s", milliseconds / 1000.0);
    }
    if milliseconds >= 1.0 {
        return format!("{milliseconds:.3}ms");
    }

    let microseconds = duration.as_secs_f64() * 1_000_000.0;
    if microseconds >= 1.0 {
        return format!("{microseconds:.1}us");
    }

    format!("{}ns", duration.as_nanos())
}

/// Return milliseconds for a duration.
fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

/// Format a ratio as a percentage string.
fn format_percent(ratio: f64) -> String {
    format!("{:.1}%", ratio * 100.0)
}

/// Format cache hit rate from hit and miss counts.
fn format_hit_rate(hits: usize, misses: usize) -> String {
    let total = hits.saturating_add(misses);
    if total == 0 {
        return "n/a".to_string();
    }

    format_percent(hits as f64 / total as f64)
}

/// Build a fixed-width share bar.
fn format_share_bar(ratio: f64, width: usize) -> String {
    let clamped = ratio.clamp(0.0, 1.0);
    let filled = (clamped * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "#".repeat(filled), ".".repeat(empty))
}

/// Format an integer with grouping separators.
fn format_count(value: usize) -> String {
    let raw = value.to_string();
    let mut out = String::with_capacity(raw.len() + raw.len() / 3);
    for (index, ch) in raw.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

/// Format count-based throughput values with compact units.
fn format_count_rate_per_second(value: f64) -> String {
    if !value.is_finite() {
        return "inf/s".to_string();
    }
    if value >= 1_000_000.0 {
        return format!("{:.2}M/s", value / 1_000_000.0);
    }
    if value >= 1_000.0 {
        return format!("{:.1}K/s", value / 1_000.0);
    }

    format!("{value:.1}/s")
}

/// Format byte counts as human-readable units.
fn format_bytes(bytes: usize) -> String {
    if bytes >= 1024 * 1024 {
        return format!("{:.2} MiB", bytes as f64 / (1024.0 * 1024.0));
    }
    if bytes >= 1024 {
        return format!("{:.2} KiB", bytes as f64 / 1024.0);
    }

    format!("{bytes} B")
}

/// Format MiB/s values with human scale units.
fn format_data_rate_mib_per_second(value: f64) -> String {
    if !value.is_finite() {
        return "inf".to_string();
    }
    if value >= 1024.0 {
        return format!("{:.2} GiB/s", value / 1024.0);
    }
    if value >= 1.0 {
        return format!("{value:.2} MiB/s");
    }

    format!("{:.2} KiB/s", value * 1024.0)
}

/// Format bytes as compact single-token units.
fn format_bytes_compact(bytes: usize) -> String {
    if bytes >= 1024 * 1024 {
        return format!("{:.1}MiB", bytes as f64 / (1024.0 * 1024.0));
    }
    if bytes >= 1024 {
        return format!("{:.1}KiB", bytes as f64 / 1024.0);
    }

    format!("{bytes}B")
}

/// Truncate a string in the middle to preserve suffix visibility.
fn truncate_middle(input: &str, max_len: usize) -> String {
    let chars = input.chars().collect::<Vec<_>>();
    if chars.len() <= max_len {
        return input.to_string();
    }
    if max_len <= 3 {
        return ".".repeat(max_len);
    }

    let head = (max_len - 3) / 2;
    let tail = max_len - head - 3;
    let prefix = chars[..head].iter().collect::<String>();
    let suffix = chars[chars.len() - tail..].iter().collect::<String>();
    format!("{prefix}...{suffix}")
}
