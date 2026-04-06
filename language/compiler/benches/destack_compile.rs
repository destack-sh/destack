use criterion::profiler::Profiler;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use destack_compiler::{Compiler, CompilerOptions};
use destack_linter::Linter;
use destack_source::{FileType, ModuleId, glob};
use destack_workspace::{Change, Edit, Ref, Repository};
use pprof::ProfilerGuard;
use pprof::flamegraph::Options as FlamegraphOptions;
use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Source file info for compiler benchmarks.
struct SourceFile {
    /// The file path.
    path: PathBuf,
    /// The file contents.
    content: String,
}

/// Compile benchmark mode.
#[derive(Clone, Copy)]
enum CompileMode {
    /// Analyze modules without linting.
    Check,
    /// Analyze and lint modules.
    Lint,
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

/// Load destack sources for compilation.
fn load_sources(workspace_root: &Path) -> (Vec<SourceFile>, u64) {
    // collect file paths
    let workspace_root = workspace_root.to_string_lossy();
    let mut ds_files = glob(&format!("{workspace_root}/**/*.ds"));
    ds_files.extend(glob(&format!("{workspace_root}/**/*.d.ds")));
    ds_files.sort();
    ds_files.dedup();

    // read content and track total lines
    let mut total_lines: u64 = 0;
    let mut sources: Vec<SourceFile> = Vec::with_capacity(ds_files.len());
    for path in ds_files.into_iter() {
        // file type
        let file_type = FileType::from_path_or_unknown(&path);
        let is_destack_source =
            matches!(file_type, FileType::Destack | FileType::DestackDeclaration);
        assert!(is_destack_source, "path is not a destack source: {path:?}");

        // file content
        let content = fs::read_to_string(&path).unwrap_or_default();
        let line_count = content.lines().count() as u64;
        total_lines = total_lines.saturating_add(line_count);

        // record source
        sources.push(SourceFile {
            path: path.clone(),
            content,
        });
    }

    (sources, total_lines)
}

/// Create a compiler and materialize modules.
fn build_compiler(workspace_root: &Path, sources: &[SourceFile]) -> (Compiler, Vec<ModuleId>) {
    // repository
    let workspace_root = workspace_root.to_path_buf();
    let repository = Arc::new(Repository::open_root(workspace_root.clone()));

    // materialize modules into the workspace revision
    let mut modules = Vec::with_capacity(sources.len());
    let reference = Ref::for_workspace_root(&workspace_root);
    for source in sources.iter() {
        let logical_path = repository.normalize_workspace_path(&source.path);
        repository
            .apply(
                &reference,
                Change::single(Edit::set_text(&logical_path, source.content.clone())),
            )
            .unwrap_or_else(|error| panic!("failed to materialize benchmark source: {error}"));
        let module_id = repository
            .module_id_for_path(
                repository
                    .current(&reference)
                    .unwrap_or_else(|error| panic!("missing workspace revision: {error}")),
                &source.path,
            )
            .unwrap_or_else(|error| panic!("failed to resolve benchmark module: {error}"))
            .unwrap_or_else(|| panic!("missing benchmark module for {}", source.path.display()));
        modules.push(module_id);
    }

    // compiler
    let compiler = Compiler::new(
        repository,
        CompilerOptions {
            workers: 1,
            ..Default::default()
        },
    );

    (compiler, modules)
}

/// Return the current workspace revision for one compiler repository.
fn current_workspace_revision(compiler: &Compiler) -> destack_workspace::Revision {
    let reference = Ref::for_workspace_root(compiler.repository.workspace_root());

    compiler
        .repository
        .current(&reference)
        .unwrap_or_else(|error| panic!("missing current workspace revision: {error}"))
}

/// Run a compiler pass for the selected mode.
fn run_compile(compiler: &Compiler, modules: &[ModuleId], mode: CompileMode) {
    let revision = current_workspace_revision(compiler);

    // enqueue work
    for module_id in modules.iter().copied() {
        // profile for module
        let profile_id = compiler
            .context(revision)
            .unwrap_or_else(|message| panic!("{message}"))
            .default_profile_id_for_module(module_id);

        // build analyzed dir requirements to completion
        match mode {
            CompileMode::Check | CompileMode::Lint => compiler
                .run_to_completion(revision, |compiler, _context| {
                    compiler.require_dir_analyzed(module_id, profile_id)
                })
                .unwrap_or_else(|error| {
                    panic!("failed to analyze benchmark module {module_id:?}: {error:?}")
                }),
        }
    }

    // lint after compiler products are ready
    if matches!(mode, CompileMode::Lint) {
        let linter = Linter::new(compiler.repository.clone());
        for module_id in modules.iter().copied() {
            let profile_id = compiler
                .context(revision)
                .unwrap_or_else(|message| panic!("{message}"))
                .default_profile_id_for_module(module_id);
            let profile = compiler
                .repository
                .default_profile_for_module(revision, module_id)
                .unwrap_or_else(|error| {
                    panic!("failed to resolve default profile for {module_id:?}: {error}")
                });
            assert_eq!(profile.id(), profile_id);
            linter
                .lint_module(revision, module_id, profile)
                .unwrap_or_else(|error| {
                    panic!("failed to lint benchmark module {module_id:?}: {error}")
                });
        }
    }
}

/// Benchmark compile workloads for the workspace.
fn bench_compile(criterion: &mut Criterion) {
    // workspace root
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root_path = manifest_dir
        .ancestors()
        .find(|p| p.join("VERSION.txt").exists())
        .unwrap_or(&manifest_dir)
        .to_path_buf();

    // load sources once
    let (sources, total_lines) = load_sources(&workspace_root_path);

    // benchmark check
    let mut group = criterion.benchmark_group("destack_compiler");
    group.throughput(Throughput::Elements(total_lines));
    group.bench_with_input(
        BenchmarkId::new("compile", "check"),
        &sources,
        |bencher, source_files| {
            bencher.iter(|| {
                // build compiler
                let (compiler, modules) = build_compiler(&workspace_root_path, source_files);

                // run compile
                run_compile(&compiler, &modules, CompileMode::Check);

                // capture stats
                black_box(compiler.stats.snapshot());
            });
        },
    );

    // benchmark lint
    group.bench_with_input(
        BenchmarkId::new("compile", "lint"),
        &sources,
        |bencher, source_files| {
            bencher.iter(|| {
                // build compiler
                let (compiler, modules) = build_compiler(&workspace_root_path, source_files);

                // run compile
                run_compile(&compiler, &modules, CompileMode::Lint);

                // capture stats
                black_box(compiler.stats.snapshot());
            });
        },
    );
    group.finish();
}

/// Configure Criterion with pprof.
fn profiler() -> Criterion {
    // allow a higher sample rate for deeper flamegraphs
    let sample_rate = std::env::var("DESTACK_PPROF_HZ")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(100);

    Criterion::default().with_profiler(PprofProfiler::new(sample_rate))
}

criterion_group! {
    name = benches;
    config = profiler();
    targets = bench_compile
}
criterion_main!(benches);
