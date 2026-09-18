use criterion::profiler::Profiler;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use destack_artifact::{ArtifactKey, BuildId};
use destack_core::FxIndexSet;
use destack_repository::{
    DestackLayout, DestackLayoutOverride, Edit, Environment, Execution, Host, Repository, Revision,
    Settings,
};
use destack_session::{ArtifactPriority, Executor, Session};
use destack_source::{FileSystem, FileType, ModuleId, PhysicalFileSystem, TargetId, glob};
use futures::executor::block_on;
use pprof::ProfilerGuard;
use pprof::flamegraph::Options as FlamegraphOptions;
use std::fs;
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
    let mut ds_files =
        glob(&format!("{workspace_root}/**/*.ds")).expect("collect Destack source files");
    ds_files.extend(
        glob(&format!("{workspace_root}/**/*.d.ds")).expect("collect Destack declaration files"),
    );
    ds_files.sort();
    ds_files.dedup();

    // read content and track total lines
    let mut total_lines: u64 = 0;
    let mut sources: Vec<SourceFile> = Vec::with_capacity(ds_files.len());
    for path in ds_files.into_iter() {
        // file type
        let file_type = FileType::from_path(&path).expect("bench path should have a file type");
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

/// Create a compiler, session, and materialize modules.
fn build_workspace(
    workspace_root: &Path,
    sources: &[SourceFile],
) -> (Arc<Session>, Revision, Vec<ModuleId>) {
    // repository
    let workspace_root = workspace_root.to_path_buf();
    let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
    let environment = Environment::capture_process();
    let layout = DestackLayout::resolve(
        &workspace_root,
        &workspace_root,
        &environment,
        &Settings::default(),
        &DestackLayoutOverride::default(),
        None,
    );
    let build_id = BuildId::test();
    let host = Host::new(build_id, environment, file_system);
    let (repository, revision) =
        Repository::new(workspace_root.clone(), host, Settings::default(), layout);
    let repository = Arc::new(repository);

    // store every source Blob and build one atomic edit
    let mut edits = Vec::with_capacity(sources.len());
    for source in sources {
        let logical_path = repository.logical_path(&source.path);
        let blob = repository
            .retain_blob(source.content.as_bytes())
            .expect("benchmark source Blob should store");
        edits.push(Edit::set_file(logical_path, blob));
    }
    let revision = repository
        .edit(revision, edits)
        .unwrap_or_else(|error| panic!("failed to materialize benchmark sources: {error}"))
        .after;

    // resolve every module from the complete revision
    let modules = sources
        .iter()
        .map(|source| {
            repository
                .module_id_for_path(revision, &source.path)
                .unwrap_or_else(|error| panic!("failed to resolve benchmark module: {error}"))
                .unwrap_or_else(|| panic!("missing benchmark module for {}", source.path.display()))
        })
        .collect();

    // session
    let executor = Executor::new(Execution::Threaded, 1)
        .unwrap_or_else(|error| panic!("failed to create benchmark executor: {error}"));
    let session = Session::new(repository, executor)
        .unwrap_or_else(|error| panic!("failed to create benchmark session: {error}"));
    let session = Arc::new(session);

    (session, revision, modules)
}

/// Run a compiler pass for the selected mode.
fn run_compile(session: &Session, revision: Revision, modules: &[ModuleId], mode: CompileMode) {
    let repository = session.repository();
    let mut artifact_keys = FxIndexSet::with_capacity_and_hasher(modules.len(), Default::default());

    // collect root artifacts
    for module_id in modules.iter().copied() {
        let module = repository
            .module(revision, module_id)
            .unwrap_or_else(|error| panic!("failed to resolve benchmark module: {error}"))
            .unwrap_or_else(|| panic!("missing benchmark module {module_id:?}"));
        let target_id = TargetId::new(module.package_id, "default");
        let profile_id = repository
            .profile_for_target(revision, target_id)
            .unwrap_or_else(|error| panic!("failed to resolve benchmark profile: {error}"))
            .id();

        // choose the root for this module
        match mode {
            CompileMode::Check => {
                artifact_keys.insert(ArtifactKey::dir_checked(module_id, profile_id));
            }
            CompileMode::Lint => {
                artifact_keys.insert(ArtifactKey::module_linted(module_id, profile_id, target_id));
                artifact_keys.insert(ArtifactKey::program_linted(profile_id, target_id));
            }
        }
    }

    // stabilize unique artifact roots for the session
    let artifact_keys = artifact_keys.into_iter().collect::<Vec<_>>();

    let run = session.provide(revision, &artifact_keys, ArtifactPriority::Foreground);
    block_on(run.wait())
        .unwrap_or_else(|error| panic!("failed to provide benchmark artifacts: {error}"));
}

/// Benchmark compile workloads for the workspace.
fn bench_compile(criterion: &mut Criterion) {
    // workspace root
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root_path = manifest_dir
        .ancestors()
        .find(|p| p.join("Cargo.toml").exists() && p.join("package.json").exists())
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
                // build session
                let (session, revision, modules) =
                    build_workspace(&workspace_root_path, source_files);

                // run compile
                run_compile(session.as_ref(), revision, &modules, CompileMode::Check);
            });
        },
    );

    // benchmark lint
    group.bench_with_input(
        BenchmarkId::new("compile", "lint"),
        &sources,
        |bencher, source_files| {
            bencher.iter(|| {
                // build session
                let (session, revision, modules) =
                    build_workspace(&workspace_root_path, source_files);

                // run compile
                run_compile(session.as_ref(), revision, &modules, CompileMode::Lint);
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
