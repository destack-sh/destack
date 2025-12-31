use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use destack_compiler::{AnalyzeTask, Compiler, CompilerOptions, LintTask};
use destack_source::{FileType, ModuleId, Uri, glob};
use destack_workspace::{ProfileId, Session};
use pprof::criterion::{Output, PProfProfiler};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

/// Source file info for compiler benchmarks.
struct SourceFile {
    /// The file uri.
    uri: Uri,
    /// The file type.
    file_type: FileType,
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

/// Load destack sources for compilation.
fn load_sources(workspace_root: &PathBuf) -> (Vec<SourceFile>, u64) {
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
            uri: Uri::from_path(&path),
            file_type,
            content,
        });
    }

    (sources, total_lines)
}

/// Create a compiler and register modules.
fn build_compiler(workspace_root: &PathBuf, sources: &[SourceFile]) -> (Compiler, Vec<ModuleId>) {
    // session and program
    let session = Arc::new(Session::new(workspace_root.clone()));
    let program = session.add_root(workspace_root.clone());

    // register modules
    let mut modules = Vec::with_capacity(sources.len());
    for source in sources.iter() {
        let module_id = program.register_inline_module(
            source.uri.clone(),
            source.content.clone(),
            source.file_type,
        );
        modules.push(module_id);
    }

    // compiler
    let compiler = Compiler::new(
        session,
        program,
        CompilerOptions {
            workers: 1,
            ..Default::default()
        },
    );

    (compiler, modules)
}

/// Run a compiler pass for the selected mode.
fn run_compile(compiler: &Compiler, modules: &[ModuleId], mode: CompileMode) {
    // enqueue work
    for module_id in modules.iter().copied() {
        // profile for module
        let profile: ProfileId = compiler.program.default_profile_id_for_module(module_id);

        // enqueue task
        match mode {
            CompileMode::Check => {
                compiler.enqueue(AnalyzeTask::AnalyzeModuleValidate {
                    module: module_id,
                    profile,
                });
            }
            CompileMode::Lint => {
                compiler.enqueue(LintTask::LintModule {
                    module: module_id,
                    profile,
                });
            }
        }
    }

    // compile
    compiler.compile();
}

/// Benchmark compile workloads for the workspace.
fn bench_compile(criterion: &mut Criterion) {
    // workspace root
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root_path = manifest_dir
        .ancestors()
        .find(|p| p.join("version.txt").exists())
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
    Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)))
}

criterion_group! {
    name = benches;
    config = profiler();
    targets = bench_compile
}
criterion_main!(benches);
