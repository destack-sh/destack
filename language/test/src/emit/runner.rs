use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::{ArtifactKey, MemoryCacheStore, OutputContent, OutputFile};
use destack_compiler::{Compiler, CompilerOptions};
use destack_source::{FileSystem, PhysicalFileSystem};
use destack_workspace::{Session, Target, TargetId};

use crate::core::{
    Case, CaseResult, RunContext, RunOptions, Runner, Suite, fixtures_dir,
    render_unexpected_diagnostics, test_output_dir,
};

use super::assert::compare_directory;
use super::discover::{SOURCE_EXTENSIONS, discover_emit_cases, discover_source_files};
#[derive(Debug, Clone, Copy, Default)]
pub struct EmitSuite;

impl Suite for EmitSuite {
    fn name(&self) -> &'static str {
        "emit"
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

    // success-only fixture contract
    if diagnostics_snapshot.exists() {
        return CaseResult::Failed {
            message: format!(
                "emit fixtures must not use diagnostics.txt snapshots: {}",
                test.path.display()
            ),
        };
    }

    // set up session and program with physical filesystem
    let fs: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem);
    let session = Arc::new(
        Session::new(test.path.clone())
            .with_fs(fs)
            .with_cache_store(Arc::new(MemoryCacheStore::new())),
    );
    let program = session.add_root(test.path.clone());
    let actual_root = emit_actual_root(test);

    // load the tracked package config through the real session path
    let config = match session.load_destack_for_path(&destack_config_path) {
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
        .options
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
    let compiler = Compiler::new(
        session.clone(),
        program.clone(),
        CompilerOptions {
            workers: 1,
            inject_prelude: false,
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
        let module_id = match compiler.resolve_path_to_module(source_path) {
            Ok(id) => id,
            Err(e) => {
                return CaseResult::Failed {
                    message: format!("failed to resolve module {}: {e:?}", source_path.display()),
                };
            }
        };
        module_ids.push(module_id);
    }
    let package_id = {
        let module = program.modules.get(module_ids[0]);
        let module = module.as_ref();
        module.package_id
    };

    // set up package
    {
        let package = program.packages.get(package_id);
        let mut package = package.write();

        // set the config so root_dir is available for output path resolution
        package.config = Some(config.clone());

        // add targets directly: keep the logical output configuration intact
        for (name, target) in targets.clone() {
            let target_id = TargetId::new(package_id, &name);
            package.targets.insert(target_id, target);
        }
    }

    // clean
    if actual_root.exists()
        && let Err(e) = fs::remove_dir_all(&actual_root)
    {
        return CaseResult::Failed {
            message: format!("failed to clean {}: {e}", actual_root.display()),
        };
    }

    // link
    for (target_name, _) in &targets {
        let target_id = TargetId::new(package_id, target_name);
        compiler.enqueue(ArtifactKey::package_output(package_id, target_id));
    }
    compiler.compile();

    // check for errors
    let unexpected_diagnostics =
        render_unexpected_diagnostics(&program.files, &program.diagnostics, test.min_fail_severity);

    // diagnostics are always unexpected in emit fixtures
    if let Some(diagnostics) = unexpected_diagnostics {
        return CaseResult::Failed {
            message: format!("unexpected diagnostics:\n{diagnostics}"),
        };
    }

    // materialize package outputs into the runner-owned actual output tree
    for (target_name, _) in &targets {
        let target_id = TargetId::new(package_id, target_name);
        let output = compiler
            .artifacts
            .package_output(package_id, &target_id)
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
