use tspp_parser::Lexer;
use tspp_source::{File, FileId, FileType, Uri, glob};

use criterion::profiler::Profiler;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use pprof::ProfilerGuard;
use pprof::flamegraph::Options as FlamegraphOptions;

use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{env, fs};

/// Source file info for lexer benchmarks.
struct SourceFile {
    /// The source file used by the lexer.
    file: Arc<File>,
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

/// Benchmark lexing for workspace sources.
fn bench_lex(criterion: &mut Criterion) {
    // workspace root
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root_path = manifest_dir
        .ancestors()
        .find(|p| p.join("destack.json").exists())
        .unwrap_or(&manifest_dir)
        .to_path_buf();
    let workspace_root = workspace_root_path.to_string_lossy().into_owned();

    // collect source files
    let mut ds_files =
        glob(&format!("{workspace_root}/**/*.tspp")).expect("collect TS++ source files");
    ds_files.extend(
        glob(&format!("{workspace_root}/**/*.d.tspp")).expect("collect TS++ declaration files"),
    );
    ds_files.sort();
    ds_files.dedup();

    // load sources and count lines
    let mut total_lines: u64 = 0;
    let mut sources: Vec<SourceFile> = Vec::with_capacity(ds_files.len());
    for path in ds_files.iter() {
        // file type
        let file_type = FileType::from_path(path).expect("bench path should have a file type");
        let is_destack_source = matches!(file_type, FileType::Tspp | FileType::TsppDeclaration);
        assert!(is_destack_source, "path is not a destack source: {path:?}");

        // file content
        let content = fs::read_to_string(path).unwrap_or_default();
        let line_count = content.lines().count() as u64;
        total_lines = total_lines.saturating_add(line_count);

        // record source
        let file_id = FileId::new(sources.len() as u64);
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("<unknown>")
            .to_string();
        let file_uri = Uri::from_string(path.to_string_lossy().to_string());
        let file = File::from_text(
            file_id,
            file_name,
            file_uri,
            Some(path.clone()),
            file_type,
            content,
        )
        .expect("benchmark source should load");
        sources.push(SourceFile {
            file: Arc::new(file),
        });
    }

    // benchmark
    let mut group = criterion.benchmark_group("tspp_lexer");
    group.throughput(Throughput::Elements(total_lines));
    group.bench_with_input(
        BenchmarkId::new("lex", "all"),
        &sources,
        |bencher, source_files| {
            bencher.iter(|| {
                // lex each file
                for source in source_files {
                    let (tokens, eof_token) = Lexer::lex(source.file.clone());
                    black_box((tokens, eof_token));
                }
            });
        },
    );
    group.finish();
}

/// Configure Criterion with pprof.
fn profiler() -> Criterion {
    // allow a higher sample rate for deeper flamegraphs
    let sample_rate = env::var("TSPP_PPROF_HZ")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(100);

    Criterion::default().with_profiler(PprofProfiler::new(sample_rate))
}

criterion_group! {
    name = benches;
    config = profiler();
    targets = bench_lex
}
criterion_main!(benches);
