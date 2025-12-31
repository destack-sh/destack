use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use destack_parser::Parser;
use destack_source::{File, FileId, FileType, LanguageType, Uri, glob};
use pprof::criterion::{Output, PProfProfiler};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

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

/// Configure Criterion with pprof.
fn profiler() -> Criterion {
    Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)))
}

criterion_group! {
    name = benches;
    config = profiler();
    targets = bench_parse
}
criterion_main!(benches);
