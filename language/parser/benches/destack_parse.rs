use criterion::{
    BatchSize, BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main,
};
use destack_parser::Parser;
use destack_source::{File, FileId, FileType, LanguageType, Uri, glob};
use pprof::criterion::{Output, PProfProfiler};
use std::path::PathBuf;
use std::sync::Arc;
use std::{env, fs};

/// Benchmark parsing for workspace sources.
fn bench_parse(criterion: &mut Criterion) {
    // workspace root
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root_path = manifest_dir
        .ancestors()
        .find(|p| p.join("version.txt").exists())
        .unwrap_or(&manifest_dir)
        .to_path_buf();
    let workspace_root = workspace_root_path.to_string_lossy().into_owned();

    // collect source files
    let mut ds_files = glob(&format!("{workspace_root}/**/*.ds"));
    ds_files.extend(glob(&format!("{workspace_root}/**/*.d.ds")));
    ds_files.sort();
    ds_files.dedup();

    // load sources and count lines
    let mut total_lines: u64 = 0;
    let mut source_files: Vec<Arc<File>> = Vec::with_capacity(ds_files.len());

    for path in ds_files.iter() {
        // file type
        let file_type = FileType::from_path_or_unknown(path);
        let is_destack_source =
            matches!(file_type, FileType::Destack | FileType::DestackDeclaration);
        assert!(is_destack_source, "path is not a destack source: {path:?}");

        // file content
        let content = fs::read_to_string(path).unwrap_or_default();
        let line_count = content.lines().count() as u64;
        total_lines = total_lines.saturating_add(line_count);

        // register file
        let file_id = FileId::new(source_files.len() as u32);
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
                    let language_type = LanguageType::from(file.ty);
                    let mut parser = Parser::lex_file(file.clone(), language_type);
                    parser.parse();
                    black_box(parser);
                }
            });
        },
    );
    group.finish();
}

/// Resolve a single-file path for targeted benchmarks.
fn resolve_single_file_path(workspace_root: &PathBuf) -> PathBuf {
    // use env override if provided
    if let Ok(path) = env::var("DESTACK_PARSE_FILE") {
        return PathBuf::from(path);
    }

    // fall back to a representative builtin file
    workspace_root.join("language/builtin/lib/dom/index.d.ds")
}

/// Load a single source file for benchmarking.
fn load_single_file(path: &PathBuf) -> (Arc<File>, u64) {
    // file type
    let file_type = FileType::from_path_or_unknown(path);
    let is_destack_source = matches!(file_type, FileType::Destack | FileType::DestackDeclaration);
    assert!(is_destack_source, "path is not a destack source: {path:?}");

    // file content
    let content = fs::read_to_string(path).unwrap_or_default();
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
        .find(|p| p.join("version.txt").exists())
        .unwrap_or(&manifest_dir)
        .to_path_buf();

    // load file
    let source_path = resolve_single_file_path(&workspace_root_path);
    let (file, total_lines) = load_single_file(&source_path);

    // benchmark
    let mut group = criterion.benchmark_group("destack_parser_single");
    group.throughput(Throughput::Elements(total_lines));

    group.bench_with_input(
        BenchmarkId::new("parse", "single"),
        &file,
        |bencher, file| {
            bencher.iter(|| {
                // parse full pipeline
                let language_type = LanguageType::from(file.ty);
                let mut parser = Parser::lex_file(file.clone(), language_type);
                parser.parse();
                black_box(parser);
            });
        },
    );

    // benchmark path with finish isolated from setup
    group.bench_with_input(
        BenchmarkId::new("finish", "single"),
        &file,
        |bencher, file| {
            bencher.iter_batched(
                || {
                    // parse up to finish
                    let language_type = LanguageType::from(file.ty);
                    let mut parser = Parser::lex_file(file.clone(), language_type);
                    parser.parse_without_finish();
                    parser
                },
                |mut parser| {
                    // attach annotations and build indexes
                    parser.finish();
                    black_box(parser);
                },
                BatchSize::SmallInput,
            );
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

    Criterion::default().with_profiler(PProfProfiler::new(sample_rate, Output::Flamegraph(None)))
}

criterion_group! {
    name = benches;
    config = profiler();
    targets = bench_parse, bench_parse_single
}
criterion_main!(benches);
