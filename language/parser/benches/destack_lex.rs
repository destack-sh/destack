use destack_parser::Lexer;
use destack_source::{FileId, FileType, LanguageType, glob};

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use pprof::criterion::{Output, PProfProfiler};

use std::fs;
use std::path::PathBuf;

/// Source file info for lexer benchmarks.
struct SourceFile {
    /// The file type.
    file_type: FileType,
    /// The file contents.
    content: String,
}

/// Benchmark lexing for workspace sources.
fn bench_lex(criterion: &mut Criterion) {
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
    let mut sources: Vec<SourceFile> = Vec::with_capacity(ds_files.len());
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

        // record source
        sources.push(SourceFile { file_type, content });
    }

    // benchmark
    let mut group = criterion.benchmark_group("destack_lexer");
    group.throughput(Throughput::Elements(total_lines));
    group.bench_with_input(
        BenchmarkId::new("lex", "all"),
        &sources,
        |bencher, source_files| {
            bencher.iter(|| {
                // lex each file
                for (index, source) in source_files.iter().enumerate() {
                    let file_id = FileId::new(index as u32);
                    let language_type = LanguageType::from(source.file_type);
                    let (tokens, side_tokens, _) =
                        Lexer::lex(file_id, &source.content, language_type);
                    black_box((tokens, side_tokens));
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
    targets = bench_lex
}
criterion_main!(benches);
