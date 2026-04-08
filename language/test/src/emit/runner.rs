use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use std::{env, fs};

use crate::core::{
    Case, CaseResult, RunContext, RunOptions, Runner, Suite, current_workspace_revision,
    fixtures_dir, provide_workspace_artifacts, render_unexpected_repository_diagnostic_collection,
    test_output_dir,
};
use destack_artifact::{ArtifactKey, MemoryCacheStore, OutputContent, OutputFile};
use destack_compiler::{Compiler, CompilerOptions};
use destack_linter::Linter;
use destack_session::Session;
use destack_source::{FileSystem, PhysicalFileSystem};
use destack_workspace::{Ref, Repository, Target};

use super::assert::compare_directory;
use super::discover::{SOURCE_EXTENSIONS, discover_emit_cases, discover_source_files};

#[derive(Debug, Clone, Copy, Default)]
pub struct EmitSuite;

impl Suite for EmitSuite {
    fn name(&self) -> &'static str {
        "emit"
    }

    fn runs_in_parallel(&self) -> bool {
        false
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        let emit_directory = fixtures_dir().join("emit");
        discover_emit_cases(&emit_directory, "destack_test::emit")
            .expect("failed to discover tests")
    }

    fn run(&self, case: &Case, context: &RunContext<'_>) -> CaseResult {
        run_emit_case(case, context)
    }
}

/// Return one runner-owned actual output root for one emit case.
fn emit_actual_root(test: &Case) -> PathBuf {
    test_output_dir().join("emit").join(&test.name).join("dist")
}

/// Rewrite one logical output path into one runner-owned actual output path.
fn rewrite_output_path_for_actual(
    package_root: &Path,
    path: &Path,
    actual_root: &Path,
) -> Result<PathBuf, String> {
    let relative = path.strip_prefix(package_root).map_err(|_| {
        format!(
            "logical output path {} is not inside package root {}",
            path.display(),
            package_root.display()
        )
    })?;

    let mut components = relative.components();
    let first = components.next();
    let rest = components.as_path();

    if first.is_some_and(|component| component.as_os_str() == "dist") {
        return Ok(actual_root.join(rest));
    }

    Ok(actual_root.join(relative))
}

/// Materialize one logical output file into one runner-owned actual output tree.
fn write_actual_output_file(
    package_root: &Path,
    actual_root: &Path,
    file: &OutputFile,
) -> Result<(), String> {
    let logical_path = file
        .uri
        .to_path_buf()
        .ok_or_else(|| format!("output file URI is not a physical path: {}", file.uri))?;
    let actual_path = rewrite_output_path_for_actual(package_root, &logical_path, actual_root)?;

    // parent directory
    if let Some(parent) = actual_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create actual output directory {}: {error}",
                parent.display()
            )
        })?;
    }

    // file content
    match &file.content {
        OutputContent::Text { code, .. } | OutputContent::Json { content: code, .. } => {
            fs::write(&actual_path, code).map_err(|error| {
                format!(
                    "failed to write actual output file {}: {error}",
                    actual_path.display()
                )
            })?;
        }
        OutputContent::Binary { bytes, .. } => {
            fs::write(&actual_path, bytes).map_err(|error| {
                format!(
                    "failed to write actual output file {}: {error}",
                    actual_path.display()
                )
            })?;
        }
    }

    Ok(())
}

/// Run all emit tests.
pub fn run_emit_tests(options: &RunOptions) -> std::process::ExitCode {
    Runner::run_suite(EmitSuite, options)
}

/// Return the emit timing filter from the environment.
fn emit_trace_filter() -> Option<String> {
    env::var("DESTACK_EMIT_TRACE")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Return whether emit runs should disable builtin library loading.
fn emit_trace_disables_libraries() -> bool {
    env::var("DESTACK_EMIT_NO_LIBS")
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .is_some_and(|value| value > 0)
}

/// Return whether one emit case should print compiler timings.
fn should_trace_emit_case(test: &Case) -> bool {
    let Some(filter) = emit_trace_filter() else {
        return false;
    };

    if filter == "1" || filter == "all" || filter == "*" {
        return true;
    }

    test.name.contains(&filter)
}

/// Return true when the package config explicitly requests builtin libraries.
fn emit_has_explicit_libs(
    repository: &Repository,
    revision: destack_workspace::Revision,
    package_id: destack_source::PackageId,
) -> bool {
    let Ok(package_options) = repository.package_options(revision, package_id) else {
        return false;
    };
    let Some(package_options) = package_options else {
        return false;
    };

    // compiler lib entries
    if !package_options.compiler.lib.is_empty() {
        return true;
    }

    // target lib entries
    package_options
        .targets
        .values()
        .any(|target| target.lib.is_some())
}

/// Return whether one emit case should load builtin libraries.
fn emit_case_load_libraries(
    repository: &Repository,
    revision: destack_workspace::Revision,
    package_id: destack_source::PackageId,
    module_ids: &[destack_source::ModuleId],
    targets: &[(String, Target)],
) -> bool {
    // debug override
    if emit_trace_disables_libraries() {
        return false;
    }

    // explicit config opts in immediately
    if emit_has_explicit_libs(repository, revision, package_id) {
        return true;
    }

    // derived target profiles may still require ambient libs
    for module_id in module_ids {
        for (target_name, _) in targets {
            let target_id = repository.intern_target_id(package_id, target_name);
            let Ok(profile) =
                repository.profile_for_target_or_default(revision, *module_id, &target_id)
            else {
                continue;
            };

            if !profile.key.lib.is_empty() {
                return true;
            }
        }
    }

    false
}

/// Format one duration in milliseconds for compact debug output.
fn format_duration_ms(duration: Duration) -> String {
    format!("{:.2}ms", duration.as_secs_f64() * 1000.0)
}

/// Print one compiler timing summary for one emit case.
fn print_emit_timing_summary(test: &Case, compiler: &Compiler) {
    let snapshot = compiler.stats.snapshot();

    eprintln!();
    eprintln!("emit timing: {}", test.name);
    eprintln!("  elapsed: {}", format_duration_ms(snapshot.elapsed));
    eprintln!("  modules: {}", snapshot.modules_processed());
    eprintln!(
        "  cache: hits={} misses={} writes={}",
        snapshot.cache_totals().hits_memory,
        snapshot.cache_totals().misses,
        snapshot.cache_totals().writes_memory
    );

    eprintln!("  packages:");
    for package in snapshot.packages.iter().take(8) {
        let name = package.name.as_deref().unwrap_or("<unnamed>");
        eprintln!(
            "    {}: {} modules, {} lines, {}",
            name,
            package.modules,
            package.lines,
            format_duration_ms(package.duration)
        );
    }

    eprintln!("  phases:");
    for phase in snapshot.phases.iter().take(8) {
        eprintln!(
            "    {}: {} across {} tasks",
            phase.phase.code(),
            format_duration_ms(phase.duration),
            phase.task_count
        );
    }

    eprintln!("  tasks:");
    for task in snapshot.task_names.iter().take(8) {
        eprintln!(
            "    {}: {} across {} tasks",
            task.name,
            format_duration_ms(task.duration),
            task.task_count
        );
    }

    if !snapshot.timings.is_empty() {
        eprintln!("  timings:");
        for timing in snapshot.timings.iter().take(12) {
            eprintln!(
                "    {}: {} across {} samples",
                timing.name,
                format_duration_ms(timing.duration),
                timing.sample_count
            );
        }
    }
}

/// Run a single emit test.
fn run_emit_case(test: &Case, context: &RunContext<'_>) -> CaseResult {
    let destack_config_path = test.path.join("destack.json");
    let diagnostics_snapshot = test.path.join("diagnostics.txt");
    let dist_expected = test.path.join("dist");
    let has_dist_snapshot = dist_expected.exists();

    // success-only fixture contract
    if diagnostics_snapshot.exists() {
        return CaseResult::Failed {
            message: format!(
                "emit fixtures must not use diagnostics.txt snapshots: {}",
                test.path.display()
            ),
        };
    }

    // set up the repository with the physical filesystem
    let fs: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem);
    let repository = Arc::new(
        Repository::open_root_from_fs(test.path.clone(), fs)
            .expect("failed to import repository from emit runner file system")
            .with_cache(Arc::new(MemoryCacheStore::new())),
    );
    let actual_root = emit_actual_root(test);

    // set up compiler
    let compiler = Arc::new(Compiler::new(
        repository.clone(),
        CompilerOptions {
            workers: 1,
            inject_prelude: false,
            ..Default::default()
        },
    ));

    // materialize the workspace state before reading semantic repository data
    let linter = Arc::new(Linter::new(repository.clone()));
    let session = Session::new(
        test.path.clone(),
        repository.clone(),
        Ref::for_workspace_root(repository.workspace_root()),
        None,
        compiler.clone(),
        linter,
        None,
        None,
    )
    .expect("failed to initialize emit session");
    session
        .materialize_filesystem(true)
        .expect("failed to reload emit workspace");
    let revision = current_workspace_revision(&repository);

    // load the tracked package config through the real repository path
    let declaration = match repository
        .destack_declaration_for_path(revision, &destack_config_path)
        .expect("failed to load tracked destack.json from revision")
        .map(|declaration| declaration.as_ref().clone())
    {
        Some(declaration) => declaration,
        None => {
            return CaseResult::Failed {
                message: format!(
                    "failed to load tracked destack.json: {}",
                    destack_config_path.display()
                ),
            };
        }
    };

    // extract targets
    let package_options = declaration.package_options();
    let targets: Vec<(String, Target)> = package_options
        .targets
        .iter()
        .map(|(name, opts)| (name.clone(), opts.to_target(name)))
        .collect();
    if targets.is_empty() {
        return CaseResult::Failed {
            message: "no targets defined in destack.json".to_string(),
        };
    }

    // set up compiler
    let trace_timings = should_trace_emit_case(test);
    let mut compiler = Compiler::new(
        repository.clone(),
        CompilerOptions {
            workers: 1,
            inject_prelude: false,
            load_libraries: false,
            timings: trace_timings,
            ..Default::default()
        },
    );
    // discover source files
    let source_dir = test.path.join("src");
    let source_files = match discover_source_files(&source_dir, SOURCE_EXTENSIONS) {
        Ok(files) => files,
        Err(e) => {
            return CaseResult::Failed {
                message: format!("failed to discover source files: {e}"),
            };
        }
    };
    if source_files.is_empty() {
        return CaseResult::Failed {
            message: "no source files found in source directory".to_string(),
        };
    }

    // resolve modules
    let mut module_ids = Vec::new();
    for source_path in &source_files {
        let module_id = match compiler.resolve_path_to_module(revision, &source_path.to_path_buf())
        {
            Ok(id) => id,
            Err(e) => {
                return CaseResult::Failed {
                    message: format!("failed to resolve module {}: {e:?}", source_path.display()),
                };
            }
        };
        module_ids.push(module_id);
    }
    let package_id = repository
        .module(revision, module_ids[0])
        .expect("failed to load emit module")
        .expect("missing emit module")
        .package_id;

    // load only the ambient libraries that the resolved profiles actually need
    let load_libraries =
        emit_case_load_libraries(&repository, revision, package_id, &module_ids, &targets);
    compiler.options.load_libraries = load_libraries;

    // clean
    if actual_root.exists()
        && let Err(e) = fs::remove_dir_all(&actual_root)
    {
        return CaseResult::Failed {
            message: format!("failed to clean {}: {e}", actual_root.display()),
        };
    }

    // link
    let mut artifact_keys = Vec::new();
    for (target_name, _) in &targets {
        let target_id = repository.intern_target_id(package_id, target_name);
        artifact_keys.push(ArtifactKey::package_output(package_id, target_id));
    }
    let revision = provide_workspace_artifacts(repository.clone(), compiler, &artifact_keys);

    if trace_timings {
        print_emit_timing_summary(test, &compiler);
    }

    // check for errors
    let diagnostics =
        collect_emit_diagnostics(&repository, revision, package_id, &module_ids, &targets);
    let unexpected_diagnostics = render_unexpected_repository_diagnostic_collection(
        &repository,
        revision,
        &diagnostics,
        test.min_fail_severity,
    );

    // diagnostics are always unexpected in emit fixtures
    if let Some(diagnostics) = unexpected_diagnostics {
        return CaseResult::Failed {
            message: format!("unexpected diagnostics:\n{diagnostics}"),
        };
    }

    if !has_dist_snapshot && !context.options.update_snapshots {
        return CaseResult::Failed {
            message: format!("emit fixture must define dist/: {}", test.path.display()),
        };
    }

    // materialize package outputs into the runner-owned actual output tree
    for (target_name, _) in &targets {
        let target_id = repository.intern_target_id(package_id, target_name);
        let output = repository
            .package_output(revision, package_id, target_id)
            .ok_or_else(|| CaseResult::Failed {
                message: format!("missing package output for target {target_name}"),
            });
        let output = match output {
            Ok(output) => output,
            Err(error) => return error,
        };

        for file in output.files() {
            if let Err(error) = write_actual_output_file(&test.path, &actual_root, file) {
                return CaseResult::Failed {
                    message: format!(
                        "failed to materialize package output for {target_name}: {error}"
                    ),
                };
            }
        }
    }

    // compare the checked-in snapshots against the runner-owned actual output tree
    let result = compare_directory(
        &dist_expected,
        &actual_root,
        context.options.update_snapshots,
    );

    // keep failed outputs for inspection under target/, otherwise leave the tree clean
    if let CaseResult::Failed { message } = result {
        return CaseResult::Failed {
            message: format!(
                "{message}\nactual output preserved at: {}",
                actual_root.display()
            ),
        };
    }

    CaseResult::Passed
}

/// Collect diagnostics across one emit compilation run.
fn collect_emit_diagnostics(
    repository: &Repository,
    revision: destack_workspace::Revision,
    package_id: destack_source::PackageId,
    module_ids: &[destack_source::ModuleId],
    targets: &[(String, Target)],
) -> destack_source::DiagnosticCollection {
    let mut diagnostics = destack_source::DiagnosticCollection::new();

    // module level diagnostics
    for module_id in module_ids {
        let profile_id = repository
            .default_profile_id_for_module(revision, *module_id)
            .unwrap_or_else(|error| panic!("failed to resolve default profile: {error}"));
        diagnostics
            .merge_from(&repository.module_artifact_diagnostics(revision, *module_id, profile_id));
    }

    // target level diagnostics
    for (target_name, _target) in targets {
        let target_id = repository.intern_target_id(package_id, target_name);

        for module_id in module_ids {
            let profile_id = repository
                .profile_id_for_target_or_default(revision, *module_id, &target_id)
                .unwrap_or_else(|error| panic!("failed to resolve target profile: {error}"));
            diagnostics.merge_from(
                &repository.module_target_artifact_diagnostics(
                    revision, *module_id, profile_id, target_id,
                ),
            );
        }

        diagnostics.merge_from(&repository.artifact_diagnostics(
            revision,
            &ArtifactKey::package_output(package_id, target_id),
        ));
    }

    diagnostics
}
