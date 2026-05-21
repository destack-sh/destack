use criterion::profiler::Profiler;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use destack_core::StringPool;
use destack_parser::{Parser, ParserOptions, ParserTriviaMode};
use destack_source::{File, FileId, FileType, LanguageType, Uri, glob};
use pprof::ProfilerGuard;
use pprof::flamegraph::Options as FlamegraphOptions;
use rayon::prelude::*;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{env, fs};

/// Return whether a file type is supported by the parser bench.
fn is_parser_source_file_type(file_type: FileType) -> bool {
    matches!(
        file_type,
        FileType::Destack
            | FileType::DestackDeclaration
            | FileType::JavaScript
            | FileType::JavaScriptXml
            | FileType::TypeScript
            | FileType::TypeScriptXml
            | FileType::TypeScriptDeclaration
    )
}

/// Return the parser bench worker count.
fn parser_bench_worker_count() -> usize {
    let physical_cores = num_cpus::get_physical();
    physical_cores.max(1)
}

/// Parse one file through the full parser pipeline.
fn parse_file(file: Arc<File>) -> Parser {
    let language_type = LanguageType::try_from(file.ty).expect("file type has no parser language");
    let trivia_mode = parser_trivia_mode();
    let mut parser = Parser::lex_file_with_options(
        file,
        language_type,
        ParserOptions {
            trivia_mode,
            ..ParserOptions::default()
        },
        Arc::new(StringPool::new()),
    );
    if trivia_mode.keeps_comments() {
        parser.parse();
    } else {
        parser.parse_without_attaching_comments();
    }

    parser
}

/// Return the parser trivia mode for benchmarks.
fn parser_trivia_mode() -> ParserTriviaMode {
    let Some(value) = env::var("DESTACK_PARSE_TRIVIA")
        .ok()
        .or_else(|| env::var("DESTACK_PARSE_RETAIN_TRIVIA").ok())
    else {
        return ParserTriviaMode::Full;
    };

    match value.as_str() {
        "0" | "false" | "False" | "FALSE" | "ignore" => ParserTriviaMode::Ignore,
        "doc" | "docs" | "documentation" => ParserTriviaMode::Documentation,
        "1" | "true" | "True" | "TRUE" | "full" | "trivia" => ParserTriviaMode::Full,
        _ => panic!("invalid parser trivia mode: {value}"),
    }
}

/// Read an optional comma separated file list from the environment.
fn parser_files_from_env() -> Option<Vec<PathBuf>> {
    let files_env = env::var("DESTACK_PARSE_FILES").ok()?;
    let files: Vec<PathBuf> = files_env
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .collect();
    if files.is_empty() { None } else { Some(files) }
}

/// Collect parser source files for the workspace benchmark.
fn collect_workspace_parser_sources(workspace_root: &str) -> Vec<PathBuf> {
    let mut source_files: Vec<PathBuf> = Vec::new();
    source_files.extend(glob(&format!("{workspace_root}/**/*.ds")));
    source_files.extend(glob(&format!("{workspace_root}/**/*.d.ds")));
    source_files.sort();
    source_files.dedup();
    source_files
}

/// Pprof profiler for Criterion benches.
struct PprofProfiler {
    /// The sampling frequency in hertz.
    frequency: i32,
    /// The active profiler guard.
    active_profiler: Option<ProfilerGuard<'static>>,
}

impl PprofProfiler {
    /// Create a new profiler with the given frequency.
    fn new(frequency: i32) -> Self {
        Self {
            frequency,
            active_profiler: None,
        }
    }
}

impl Profiler for PprofProfiler {
    /// Start a profiling session.
    fn start_profiling(&mut self, _benchmark_id: &str, _benchmark_dir: &Path) {
        self.active_profiler = Some(ProfilerGuard::new(self.frequency).unwrap());
    }

    /// Stop profiling and write the flamegraph.
    fn stop_profiling(&mut self, _benchmark_id: &str, benchmark_dir: &Path) {
        // ensure the output directory exists
        fs::create_dir_all(benchmark_dir).unwrap();

        // open the flamegraph output file
        let output_path = benchmark_dir.join("flamegraph.svg");
        let output_file = fs::File::create(&output_path).unwrap_or_else(|_| {
            panic!("file system error while creating {}", output_path.display())
        });

        // build and write the flamegraph
        if let Some(profiler) = self.active_profiler.take() {
            let mut options = FlamegraphOptions::default();
            profiler
                .report()
                .build()
                .unwrap()
                .flamegraph_with_options(output_file, &mut options)
                .expect("error while writing flamegraph");
        }
    }
}

/// Benchmark parsing for workspace sources.
fn bench_parse(criterion: &mut Criterion) {
    // workspace root
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root_path = manifest_dir
        .ancestors()
        .find(|p| p.join("VERSION.txt").exists())
        .unwrap_or(&manifest_dir)
        .to_path_buf();
    let workspace_root = workspace_root_path.to_string_lossy().into_owned();

    // collect source files
    let parser_files = parser_files_from_env()
        .unwrap_or_else(|| collect_workspace_parser_sources(&workspace_root));

    // load sources and count lines
    let mut total_lines = 0u64;
    let mut source_files: Vec<Arc<File>> = Vec::with_capacity(parser_files.len());

    for path in parser_files.iter() {
        // file type
        let file_type = FileType::from_path_or_unknown(path);
        let is_parser_source = is_parser_source_file_type(file_type);
        assert!(is_parser_source, "path is not a parser source: {path:?}");

        // file content
        let content = fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("file system error while reading {}", path.display()));
        let line_count = content.lines().count() as u64;
        total_lines = total_lines.saturating_add(line_count);

        // register file
        let file_id = FileId::new(source_files.len() as u128);
        let (file_name, uri) = Uri::from_path_with_name(path);
        let file = File::from_text(file_id, file_name, uri, None, file_type, content);
        source_files.push(Arc::new(file));
    }

    // benchmark
    let mut group = criterion.benchmark_group("destack_parser");
    group.throughput(Throughput::Elements(total_lines));
    group.bench_with_input(
        BenchmarkId::new("parse", "all"),
        &source_files,
        |bencher, source_files| {
            bencher.iter(|| {
                // parse each file
                for file in source_files.iter() {
                    let parser = parse_file(file.clone());
                    black_box(parser);
                }
            });
        },
    );
    group.finish();
}

/// Resolve a single-file path for targeted benchmarks.
fn resolve_single_file_path(workspace_root: &Path) -> PathBuf {
    // use env override if provided
    if let Ok(path) = env::var("DESTACK_PARSE_FILE") {
        return PathBuf::from(path);
    }

    // fall back to a representative library file
    workspace_root.join("language/library/fs/host/file.ds")
}

/// Load a single source file for benchmarking.
fn load_single_file(path: &Path) -> (Arc<File>, u64) {
    // file type
    let file_type = FileType::from_path_or_unknown(path);
    let is_parser_source = is_parser_source_file_type(file_type);
    assert!(is_parser_source, "path is not a parser source: {path:?}");

    // file content
    let content = fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("file system error while reading {}", path.display()));
    let line_count = content.lines().count() as u64;

    // register file
    let file_id = FileId::new(0);
    let (file_name, uri) = Uri::from_path_with_name(path);
    let file = File::from_text(file_id, file_name, uri, None, file_type, content);

    (Arc::new(file), line_count)
}

/// Benchmark parsing for a single source file.
fn bench_parse_single(criterion: &mut Criterion) {
    // workspace root
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root_path = manifest_dir
        .ancestors()
        .find(|p| p.join("VERSION.txt").exists())
        .unwrap_or(&manifest_dir)
        .to_path_buf();

    // load file
    let source_path = resolve_single_file_path(&workspace_root_path);
    let (file, total_lines) = load_single_file(&source_path);

    // benchmark
    let mut group = criterion.benchmark_group("destack_parser_single");
    group.throughput(Throughput::Elements(total_lines));
    let worker_count = parser_bench_worker_count();

    // oxc style: single thread parse
    group.bench_with_input(
        BenchmarkId::new("parse", "single-thread"),
        &file,
        |bencher, file| {
            bencher.iter(|| {
                // parse full pipeline
                let parser = parse_file(file.clone());
                black_box(parser);
            });
        },
    );

    // oxc style: no drop parse
    group.bench_with_input(
        BenchmarkId::new("parse", "no-drop"),
        &file,
        |bencher, file| {
            bencher.iter_with_large_drop(|| parse_file(file.clone()));
        },
    );

    // oxc style: parallel parse throughput
    group.bench_with_input(
        BenchmarkId::new("parse", "parallel"),
        &file,
        |bencher, file| {
            bencher.iter(|| {
                (0..worker_count).into_par_iter().for_each(|_| {
                    let parser = parse_file(file.clone());
                    black_box(parser);
                });
            });
        },
    );

    // keep compatibility alias for existing profile commands
    group.bench_with_input(
        BenchmarkId::new("parse", "single"),
        &file,
        |bencher, file| {
            bencher.iter(|| {
                let parser = parse_file(file.clone());
                black_box(parser);
            });
        },
    );

    // parse main alias: kept for historical benchmark compatibility
    group.bench_with_input(
        BenchmarkId::new("main", "single-thread"),
        &file,
        |bencher, file| {
            bencher.iter(|| {
                let parser = parse_file(file.clone());
                black_box(parser);
            });
        },
    );

    // parse main alias: no drop
    group.bench_with_input(
        BenchmarkId::new("main", "no-drop"),
        &file,
        |bencher, file| {
            bencher.iter_with_large_drop(|| parse_file(file.clone()));
        },
    );

    // parse main alias: parallel throughput
    group.bench_with_input(
        BenchmarkId::new("main", "parallel"),
        &file,
        |bencher, file| {
            bencher.iter(|| {
                (0..worker_count).into_par_iter().for_each(|_| {
                    let parser = parse_file(file.clone());
                    black_box(parser);
                });
            });
        },
    );

    group.finish();
}

/// Configure Criterion with pprof.
fn profiler() -> Criterion {
    // allow a higher sample rate for deeper flamegraphs
    let sample_rate = env::var("DESTACK_PPROF_HZ")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(100);

    Criterion::default().with_profiler(PprofProfiler::new(sample_rate))
}

criterion_group! {
    name = benches;
    config = profiler();
    targets = bench_parse, bench_parse_single
}
criterion_main!(benches);
