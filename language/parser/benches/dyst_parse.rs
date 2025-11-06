use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use destack_file::glob;
use dyst_ast::{DefinitionMeta, ModuleFormat, ModuleStyle, TokenType};
use dyst_parser::Parser;
use dyst_source::{LanguageOptions, Source, SourceFormat, SourceId, Uri};
use pprof::criterion::{Output, PProfProfiler};
use std::fs;
use std::path::PathBuf;

fn bench_parse(c: &mut Criterion) {
    // find workspace root by walking up until we find a known repo marker
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root_path = manifest_dir
        .ancestors()
        .find(|p| p.join("version.txt").exists())
        .unwrap_or(&manifest_dir)
        .to_path_buf();
    let workspace_root = workspace_root_path.to_string_lossy().into_owned();

    // glob all .ds files under the workspace root
    let ds_files = glob::glob(&format!("{workspace_root}/**/*.ds"));

    // concatenate all contents into a single big string
    // pre-compute capacity to reduce reallocations
    let mut total_capacity: usize = 0;
    for path in ds_files.iter() {
        if let Ok(meta) = fs::metadata(path) {
            total_capacity = total_capacity.saturating_add(meta.len() as usize);
        }
    }
    let mut ds_str: String = String::with_capacity(total_capacity);
    for path in ds_files.iter() {
        assert!(
            path.extension().is_some() && path.extension().unwrap() == "ds",
            "path does not end with .ds: {path:?}"
        );
        let content = fs::read_to_string(path).unwrap_or_default();
        if !content.is_empty() {
            ds_str.push_str(&content);
            // add a newline separator to avoid accidental token merging across files
            if !ds_str.ends_with('\n') {
                ds_str.push('\n');
            }
        }
    }
    let source = Source::from_string(
        SourceId::new(0),
        "<string>".to_string(),
        Uri::from_string("<string>"),
        SourceFormat::Dyst,
        ds_str,
    );

    // single benchmark over the whole workspace content
    let mut session = Session::new();
    let mut group = c.benchmark_group("dyst_ast");
    let line_count = source.content.lines().count() as u64;
    group.throughput(Throughput::Elements(line_count));
    group.bench_with_input(BenchmarkId::new("parse", "all"), &source, |b, source| {
        b.iter(|| {
            let language = LanguageOptions::default();
            let mut parser = Parser::prepare(source, language, &mut session);
            let module = parser.with_recovery(
                parser.mark(),
                |parser| {
                    parser
                        .eat_module_body(
                            DefinitionMeta::default(),
                            ModuleFormat::Source,
                            ModuleStyle::Module,
                        )
                        .map(Some)
                },
                None,
                TokenType::End,
            );
            black_box(module);
        });
    });
    group.finish();
}

// configure Criterion with pprof
fn profiler() -> Criterion {
    Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)))
}

criterion_group! {
    name = benches;
    config = profiler();
    targets = bench_parse
}
criterion_main!(benches);
