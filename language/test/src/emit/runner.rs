use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::{
    Case, CaseResult, RunContext, RunOptions, Runner, Suite, current_workspace_revision,
    default_profile_id_for_module, fixtures_dir, module_artifact_diagnostics, module_id_for_path,
    module_target_artifact_diagnostics, profile_id_for_target_or_default,
    provide_workspace_artifacts, render_unexpected_repository_diagnostic_collection,
    test_output_dir,
};
use destack_artifact::{ArtifactKey, MemoryCacheStore, OutputContent, OutputFile};
use destack_compiler::Compiler;
use destack_linter::Linter;
use destack_query::Query;
use destack_session::Session;
use destack_source::{FileSystem, PhysicalFileSystem, TargetId};
use destack_workspace::{HostEnvironment, Ref, Repository, Target};

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
    let repository = Arc::new(Repository::new(
        test.path.clone(),
        Arc::new(MemoryCacheStore::new()),
        fs,
        HostEnvironment::capture_process(),
    ));
    let actual_root = emit_actual_root(test);

    // set up compiler
    let compiler = Arc::new(Compiler::new(repository.clone()));

    // materialize the workspace state before reading semantic repository data
    let linter = Arc::new(Linter::new(repository.clone()));
    let query = Arc::new(Query::new(repository.clone()));
    let session = Session::new(
        test.path.clone(),
        test.path.clone(),
        repository.clone(),
        Ref::for_workspace_root(repository.workspace_root()),
        compiler.clone(),
        linter,
        query,
        1,
        None,
    )
    .expect("failed to create emit session");
    session
        .reload_from_fs(session.head())
        .expect("failed to reload emit workspace");
    let revision = current_workspace_revision(&repository);

    // load the tracked package config through the real repository path
    let destack_config_file_id = repository.file_id(&destack_config_path);
    let config = match repository
        .destack_for_file(revision, destack_config_file_id)
        .expect("failed to load tracked destack.json from revision")
    {
        Some(config) => config,
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
    let targets: Vec<(String, Target)> = config
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
    let compiler = Arc::new(Compiler::new(repository.clone()));
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
        let module_id = module_id_for_path(&repository, revision, source_path);
        module_ids.push(module_id);
    }
    let package_id = repository
        .module(revision, module_ids[0])
        .expect("failed to load emit module")
        .expect("missing emit module")
        .package_id;

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
        let target_id = TargetId::new(package_id, target_name);
        artifact_keys.push(ArtifactKey::package_output(package_id, target_id));
    }
    let revision =
        provide_workspace_artifacts(repository.clone(), compiler.clone(), &artifact_keys);

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
        let target_id = TargetId::new(package_id, target_name);
        let key = ArtifactKey::package_output(package_id, target_id);
        let version = repository
            .artifact_version(revision, &key)
            .expect("failed to read package output version")
            .ok_or_else(|| CaseResult::Failed {
                message: format!("missing package output version for target {target_name}"),
            });
        let output = match version {
            Ok(version) => repository
                .artifact_store()
                .package_output(&version)
                .ok_or_else(|| CaseResult::Failed {
                    message: format!("missing package output payload for target {target_name}"),
                }),
            Err(error) => return error,
        };
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
        let profile_id = default_profile_id_for_module(repository, revision, *module_id);
        let artifact_diagnostics =
            module_artifact_diagnostics(repository, revision, *module_id, profile_id);
        diagnostics.merge_from(&artifact_diagnostics);
    }

    // target level diagnostics
    for (target_name, _target) in targets {
        let target_id = TargetId::new(package_id, target_name);

        for module_id in module_ids {
            let profile_id =
                profile_id_for_target_or_default(repository, revision, *module_id, &target_id);
            let artifact_diagnostics = module_target_artifact_diagnostics(
                repository, revision, *module_id, profile_id, target_id,
            );
            diagnostics.merge_from(&artifact_diagnostics);
        }

        let key = ArtifactKey::package_output(package_id, target_id);
        let artifact_diagnostics = repository
            .diagnostics(revision, Some(key))
            .unwrap_or_else(|error| panic!("failed to read package diagnostics: {error}"));
        diagnostics.merge_from(&artifact_diagnostics);
    }

    diagnostics
}
