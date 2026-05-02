use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_compiler::Compiler;
use destack_parser::{Parser, source_colorizer};
use destack_source::{
    Diagnostic, DiagnosticCollection, DiagnosticSeverity, File, FileId, FileSystem, FileType,
    LanguageType, MemoryFileSystem, ModuleId, PathExt, PhysicalFileSystem, PrintOptions, TargetId,
    Uri, glob,
};
use destack_workspace::{
    FormatterOptions, LinterOptions, PackageJson, Repository, Revision, TsConfigDeclaration,
};

use crate::core::print::color;
use crate::core::{
    CaseResult, current_workspace_revision, default_profile_id_for_module, format_diagnostics,
    module_artifact_diagnostics, module_id_for_path, module_target_artifact_diagnostics,
    open_repository_with_options, profile_id_for_target_or_default, provide_workspace_artifacts,
};
use crate::ecosystem::manifest::{
    CompilerOptionsConfig, EcosystemManifest, EcosystemPhase, EcosystemTscMode, EcosystemTscTool,
    ExpectedDiagnosticConfig,
};

/// default excludes applied by typescript when `exclude` is omitted.
const TYPESCRIPT_DEFAULT_EXCLUDES: &[&str] = &["node_modules", "bower_components", "jspm_packages"];

/// Source extensions used for manifest entrypoint candidate expansion.
const ENTRYPOINT_SOURCE_EXTENSIONS: &[&str] = &[
    ".ts", ".tsx", ".mts", ".cts", ".js", ".jsx", ".mjs", ".cjs", ".d.ts", ".d.mts", ".d.cts",
];

/// One phase read stats snapshot.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct PhaseReadStats {
    /// Number of loaded modules.
    pub modules: usize,
    /// Number of source lines across loaded modules.
    pub lines: usize,
}

/// One phase run result with optional read stats.
#[derive(Debug, Clone)]
pub(super) struct PhaseTierResult {
    /// Phase test result.
    pub result: CaseResult,
    /// Optional phase read stats.
    pub stats: Option<PhaseReadStats>,
}

/// Run one phase tier for one package workload.
pub(super) fn run_phase_tier(
    package_dir: &Path,
    manifest: &EcosystemManifest,
    phase: EcosystemPhase,
    files: &[PathBuf],
    tsc_mode: EcosystemTscMode,
    tsc_tool: EcosystemTscTool,
    collect_read_stats: bool,
) -> PhaseTierResult {
    match phase {
        EcosystemPhase::Parse => run_parse_phase(package_dir, manifest, files, collect_read_stats),
        EcosystemPhase::Resolve | EcosystemPhase::Analyze | EcosystemPhase::Lower => {
            run_compiler_phase(
                package_dir,
                manifest,
                phase,
                files,
                tsc_mode,
                tsc_tool,
                collect_read_stats,
            )
        }
    }
}

/// Run parser level checks across files.
fn run_parse_phase(
    package_dir: &Path,
    manifest: &EcosystemManifest,
    files: &[PathBuf],
    _collect_read_stats: bool,
) -> PhaseTierResult {
    let mut failures = Vec::new();
    let mut failure_messages = Vec::new();

    for path in files {
        match parse_file(path, manifest) {
            Ok(()) => {}
            Err(output) => {
                let relative = path.strip_prefix(package_dir).unwrap_or(path);
                failure_messages.push(format!("{}:\n{}", relative.display(), output.trim_end()));
                failures.push(relative.to_path_buf());
            }
        }
    }

    let result = if failures.is_empty() {
        CaseResult::Passed
    } else {
        CaseResult::Failed {
            message: format!(
                "{} files failed to parse with full diagnostics:\n\n{}",
                failures.len(),
                failure_messages.join("\n\n")
            ),
        }
    };

    // parse phase does not report loaded graph stats
    PhaseTierResult {
        result,
        stats: None,
    }
}

/// Run compiler phase checks across selected entrypoints.
fn run_compiler_phase(
    package_dir: &Path,
    manifest: &EcosystemManifest,
    phase: EcosystemPhase,
    files: &[PathBuf],
    default_tsc_mode: EcosystemTscMode,
    default_tsc_tool: EcosystemTscTool,
    collect_read_stats: bool,
) -> PhaseTierResult {
    // initialize optional read stats
    let mut stats = collect_read_stats.then_some(PhaseReadStats::default());

    // discover entrypoints for this phase
    let entrypoints = match select_phase_entrypoints_with_roots(
        package_dir,
        files,
        &manifest.discovery.roots,
        &manifest.discovery.entrypoints,
    ) {
        Ok(entrypoints) => entrypoints,
        Err(message) => {
            return PhaseTierResult {
                result: CaseResult::Failed { message },
                stats,
            };
        }
    };

    // construct one compiler session for the package
    let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem);
    let repository = open_repository_with_options(
        package_dir.to_path_buf(),
        file_system,
        Default::default(),
        Default::default(),
    );
    let compiler = Arc::new(Compiler::new(repository.clone()));

    let revision = current_workspace_revision(&repository);

    // resolve entrypoint modules before task enqueue
    let mut module_ids = BTreeSet::new();
    for path in &entrypoints {
        let path = path.to_path_buf();
        let module_id = module_id_for_path(&repository, revision, &path);

        module_ids.insert(module_id);
    }

    // fail loudly when no module could be resolved
    if module_ids.is_empty() {
        return PhaseTierResult {
            result: CaseResult::Failed {
                message: "no modules resolved from selected entrypoints".to_string(),
            },
            stats,
        };
    }

    // select the requested roots for each resolved module
    let mut artifact_keys = Vec::new();
    for module_id in module_ids.iter().copied() {
        let artifact_key = phase_root_for_module(&repository, revision, module_id, phase);
        if let Some(artifact_key) = artifact_key {
            artifact_keys.push(artifact_key);
        }
    }

    // run the selected roots
    let revision = if artifact_keys.is_empty() {
        revision
    } else {
        provide_workspace_artifacts(repository.clone(), compiler.clone(), &artifact_keys)
    };
    let module_count = match repository.module_ids(revision) {
        Ok(module_ids) => module_ids.len(),
        Err(error) => {
            return PhaseTierResult {
                result: CaseResult::Failed {
                    message: format!("failed to collect workspace modules: {error}"),
                },
                stats,
            };
        }
    };
    drop(compiler);

    // collect loaded graph stats when requested
    if collect_read_stats {
        stats = match collect_compiler_phase_stats(&repository, revision) {
            Ok(stats) => Some(stats),
            Err(error) => {
                return PhaseTierResult {
                    result: CaseResult::Failed { message: error },
                    stats,
                };
            }
        };
    }

    // collect error diagnostics for this phase
    let diagnostics = collect_phase_diagnostics(&repository, revision, &module_ids, phase);
    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .cloned()
        .collect::<Vec<_>>();
    let has_destack_errors = !errors.is_empty();

    // collect expected diagnostics configured in the manifest
    let expected_phase_diagnostics = manifest
        .diagnostics_for_phase(phase)
        .cloned()
        .collect::<Vec<_>>();

    // optionally run tsc for contextual comparison
    let tsc_result = maybe_run_typescript_tsc(
        package_dir,
        manifest,
        phase,
        has_destack_errors,
        default_tsc_mode,
        default_tsc_tool,
        &entrypoints,
    );

    // pass immediately when no errors are observed or expected
    if !has_destack_errors && expected_phase_diagnostics.is_empty() {
        return PhaseTierResult {
            result: CaseResult::Passed,
            stats,
        };
    }

    // fail when expected diagnostics are missing in a clean run
    if !has_destack_errors {
        let mut message = format_expected_diagnostic_mismatch(
            phase,
            entrypoints.len(),
            files.len(),
            &expected_phase_diagnostics,
            &[],
        );

        append_tsc_context_maybe(&mut message, tsc_result.as_ref());

        return PhaseTierResult {
            result: CaseResult::Failed { message },
            stats,
        };
    }

    // match observed diagnostics against manifest expectations
    if !expected_phase_diagnostics.is_empty() {
        let observed_phase_diagnostics =
            match collect_observed_phase_diagnostics(&repository, revision, package_dir, &errors) {
                Ok(diagnostics) => diagnostics,
                Err(error) => {
                    return PhaseTierResult {
                        result: CaseResult::Failed { message: error },
                        stats,
                    };
                }
            };
        let outcome = match_expected_phase_diagnostics(
            &expected_phase_diagnostics,
            &observed_phase_diagnostics,
        );

        // pass when observed diagnostics exactly match expectations
        if outcome.missing_expected.is_empty() && outcome.unexpected_observed.is_empty() {
            return PhaseTierResult {
                result: CaseResult::Passed,
                stats,
            };
        }

        // build mismatch output with full diagnostic context
        let mut message = format_expected_diagnostic_mismatch(
            phase,
            entrypoints.len(),
            files.len(),
            &outcome.missing_expected,
            &outcome.unexpected_observed,
        );

        let diagnostic_output =
            match format_phase_error_diagnostics(&repository, revision, &errors, module_count) {
                Ok(output) => output,
                Err(error) => {
                    return PhaseTierResult {
                        result: CaseResult::Failed { message: error },
                        stats,
                    };
                }
            };
        message.push_str("\n\n");
        message.push_str(diagnostic_output.trim_end());

        append_tsc_context_maybe(&mut message, tsc_result.as_ref());

        return PhaseTierResult {
            result: CaseResult::Failed { message },
            stats,
        };
    }

    // report full diagnostics when no expectation list is configured
    let diagnostic_output =
        match format_phase_error_diagnostics(&repository, revision, &errors, module_count) {
            Ok(output) => output,
            Err(error) => {
                return PhaseTierResult {
                    result: CaseResult::Failed { message: error },
                    stats,
                };
            }
        };
    let mut message = format!(
        "phase '{}' failed with {} errors across {} entrypoints and {} discovered files:\n\n{}",
        phase.name(),
        errors.len(),
        entrypoints.len(),
        files.len(),
        diagnostic_output.trim_end()
    );

    append_tsc_context_maybe(&mut message, tsc_result.as_ref());

    PhaseTierResult {
        result: CaseResult::Failed { message },
        stats,
    }
}
/// Collect loaded module counts and source lines from one repository snapshot.
fn collect_compiler_phase_stats(
    repository: &Arc<Repository>,
    revision: Revision,
) -> Result<PhaseReadStats, String> {
    let mut modules = 0usize;
    let mut lines = 0usize;

    let module_ids = repository
        .module_ids(revision)
        .map_err(|error| format!("failed to collect module snapshots: {error}"))?;

    // sum line counts across visible module snapshots
    for module_id in module_ids {
        let module = repository
            .module(revision, module_id)
            .map_err(|error| format!("failed to read module snapshot: {error}"))?
            .ok_or_else(|| format!("missing module snapshot for {module_id:?}"))?;
        let file = repository
            .file(revision, module.file_id)
            .map_err(|error| format!("failed to read file snapshot: {error}"))?
            .ok_or_else(|| format!("missing file snapshot for {:?}", module.file_id))?;

        modules += 1;
        lines += file.line_count() as usize;
    }

    Ok(PhaseReadStats { modules, lines })
}

/// Collect diagnostics for one compiled ecosystem phase.
fn collect_phase_diagnostics(
    repository: &Repository,
    revision: Revision,
    module_ids: &BTreeSet<ModuleId>,
    phase: EcosystemPhase,
) -> DiagnosticCollection {
    let mut diagnostics = DiagnosticCollection::new();

    for module_id in module_ids {
        match phase {
            EcosystemPhase::Parse => {}
            EcosystemPhase::Resolve | EcosystemPhase::Analyze => {
                let profile_id = default_profile_id_for_module(repository, revision, *module_id);
                let artifact_diagnostics =
                    module_artifact_diagnostics(repository, revision, *module_id, profile_id);
                diagnostics.merge_from(&artifact_diagnostics);
            }
            EcosystemPhase::Lower => {
                let module = repository
                    .module(revision, *module_id)
                    .unwrap_or_else(|error| panic!("failed to read module: {error}"))
                    .unwrap_or_else(|| panic!("missing module {module_id:?}"));
                let target = repository
                    .package_default_target(revision, module.package_id)
                    .unwrap_or_else(|error| panic!("failed to resolve diagnostic target: {error}"))
                    .map(|(target_id, _)| target_id)
                    .unwrap_or_else(|| TargetId::new(module.package_id, "default"));
                let profile_id =
                    profile_id_for_target_or_default(repository, revision, *module_id, &target);
                let artifact_diagnostics = module_target_artifact_diagnostics(
                    repository, revision, *module_id, profile_id, target,
                );
                diagnostics.merge_from(&artifact_diagnostics);
            }
        }
    }

    diagnostics
}

/// One observed compiler phase diagnostic used for expectation matching.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ObservedPhaseDiagnostic {
    /// The diagnostic code.
    code: String,
    /// The normalized package relative file path.
    file: String,
    /// The diagnostic message.
    message: String,
}

/// Matching outcome between expected and observed diagnostics.
#[derive(Debug, Clone)]
struct ExpectedDiagnosticMatchOutcome {
    /// Expected diagnostics that did not occur.
    missing_expected: Vec<ExpectedDiagnosticConfig>,
    /// Observed diagnostics that were not expected.
    unexpected_observed: Vec<ObservedPhaseDiagnostic>,
}

/// Collect deduplicated observed diagnostics for phase expectation matching.
fn collect_observed_phase_diagnostics(
    repository: &Repository,
    revision: Revision,
    package_dir: &Path,
    errors: &[Diagnostic],
) -> Result<Vec<ObservedPhaseDiagnostic>, String> {
    let mut observed = BTreeSet::new();
    let file_for_id = |file_id| {
        repository
            .file(revision, file_id)
            .map_err(|error| format!("failed to read diagnostic file snapshot: {error}"))
    };

    // map diagnostics to stable code, file, and message triples
    for diagnostic in errors {
        let file = normalize_diagnostic_file(&file_for_id, package_dir, diagnostic)?;
        observed.insert(ObservedPhaseDiagnostic {
            code: diagnostic.code.clone(),
            file,
            message: diagnostic.message.clone(),
        });
    }

    Ok(observed.into_iter().collect())
}

/// Normalize one diagnostic file path for stable exact matching.
fn normalize_diagnostic_file(
    file_for_id: &impl Fn(FileId) -> Result<Option<Arc<File>>, String>,
    package_dir: &Path,
    diagnostic: &Diagnostic,
) -> Result<String, String> {
    let file_id = diagnostic.primary_label().span.file;
    let file = file_for_id(file_id)?
        .ok_or_else(|| format!("missing diagnostic file snapshot for {file_id:?}"))?;

    // use package relative paths when available
    if let Some(path) = file.path.as_deref() {
        let relative = path.strip_prefix(package_dir).unwrap_or(path);
        return Ok(normalize_path_fragment(&relative.to_string_lossy()));
    }

    // fall back to uri text when no path is attached
    Ok(normalize_path_fragment(file.uri.as_ref()))
}

/// Normalize one path-like value for cross-platform exact matching.
fn normalize_path_fragment(value: &str) -> String {
    value.replace('\\', "/")
}

/// Match expected diagnostics against observed diagnostics.
fn match_expected_phase_diagnostics(
    expected: &[ExpectedDiagnosticConfig],
    observed: &[ObservedPhaseDiagnostic],
) -> ExpectedDiagnosticMatchOutcome {
    let mut missing_expected = Vec::new();
    let mut unmatched_observed = observed.to_vec();

    // match each expected diagnostic against one observed diagnostic
    for expected_diagnostic in expected {
        let expected_file = normalize_path_fragment(expected_diagnostic.file.trim());
        let expected_message = expected_diagnostic.message.trim();
        let expected_code = expected_diagnostic.code.trim();

        let matched_index = unmatched_observed.iter().position(|observed_diagnostic| {
            observed_diagnostic.code == expected_code
                && observed_diagnostic.file == expected_file
                && observed_diagnostic.message == expected_message
        });

        if let Some(index) = matched_index {
            unmatched_observed.remove(index);
            continue;
        }

        missing_expected.push(expected_diagnostic.clone());
    }

    ExpectedDiagnosticMatchOutcome {
        missing_expected,
        unexpected_observed: unmatched_observed,
    }
}

/// Format expected diagnostic mismatch details.
fn format_expected_diagnostic_mismatch(
    phase: EcosystemPhase,
    entrypoints: usize,
    discovered_files: usize,
    missing_expected: &[ExpectedDiagnosticConfig],
    unexpected_observed: &[ObservedPhaseDiagnostic],
) -> String {
    let mut lines = vec![format!(
        "phase '{}' diagnostics did not match expectations across {} entrypoints and {} discovered files",
        phase.name(),
        entrypoints,
        discovered_files,
    )];

    // report expected diagnostics that did not occur
    if !missing_expected.is_empty() {
        lines.push("".to_string());
        lines.push("missing expected diagnostics:".to_string());
        for expected_diagnostic in missing_expected {
            lines.push(format!(
                " - {} {}: {}",
                expected_diagnostic.code, expected_diagnostic.file, expected_diagnostic.message,
            ));
        }
    }

    // report observed diagnostics that were not expected
    if !unexpected_observed.is_empty() {
        lines.push("".to_string());
        lines.push("unexpected diagnostics:".to_string());
        for observed_diagnostic in unexpected_observed {
            lines.push(format!(
                " - {} {}: {}",
                observed_diagnostic.code, observed_diagnostic.file, observed_diagnostic.message,
            ));
        }
    }

    lines.join("\n")
}

/// Format phase diagnostics using existing file aware diagnostic rendering.
fn format_phase_error_diagnostics(
    repository: &Arc<Repository>,
    revision: Revision,
    errors: &[Diagnostic],
    module_count: usize,
) -> Result<String, String> {
    let mut error_diagnostics = DiagnosticCollection::new();
    let file_for_id = |file_id| {
        repository.file(revision, file_id).unwrap_or_else(|error| {
            panic!("failed to read diagnostic file snapshot {file_id:?}: {error}")
        })
    };

    // render only error level diagnostics for deterministic phase output
    for diagnostic in errors.iter().cloned() {
        error_diagnostics.insert(diagnostic);
    }

    Ok(format_diagnostics(
        &file_for_id,
        &error_diagnostics,
        PrintOptions::new()
            .with_colorizer(source_colorizer())
            .with_module_count(module_count),
    ))
}

/// Append optional tsc context to one failure message.
fn append_tsc_context_maybe(
    message: &mut String,
    tsc_result: Option<&Result<TypeScriptTscRun, String>>,
) {
    let Some(tsc_result) = tsc_result else {
        return;
    };

    // append tsc context for faster triage against TypeScript behavior
    let tsc_message = tsc_result_to_failure_context(tsc_result);
    if tsc_message.is_empty() {
        return;
    }

    message.push_str("\n\n");
    message.push_str(&tsc_message);
}

/// Captured output from one TypeScript TSC run.
#[derive(Debug, Clone)]
struct TypeScriptTscRun {
    /// TSC binary that was executed.
    binary: &'static str,
    /// TSC command arguments.
    args: Vec<String>,
    /// TSC process success status.
    success: bool,
    /// TSC output with stdout and stderr merged.
    output: String,
}

/// Return one tsc mode using manifest override or suite default.
fn tsc_mode_for_manifest(
    manifest: &EcosystemManifest,
    default_tsc_mode: EcosystemTscMode,
) -> EcosystemTscMode {
    manifest.tsc.mode.unwrap_or(default_tsc_mode)
}

/// Return one tsc tool using manifest override or suite default.
fn tsc_tool_for_manifest(
    manifest: &EcosystemManifest,
    default_tsc_tool: EcosystemTscTool,
) -> EcosystemTscTool {
    manifest.tsc.tool.unwrap_or(default_tsc_tool)
}

/// Run the TypeScript TSC when configuration requires it.
fn maybe_run_typescript_tsc(
    package_dir: &Path,
    manifest: &EcosystemManifest,
    phase: EcosystemPhase,
    has_destack_errors: bool,
    default_tsc_mode: EcosystemTscMode,
    default_tsc_tool: EcosystemTscTool,
    entrypoints: &[PathBuf],
) -> Option<Result<TypeScriptTscRun, String>> {
    // skip packages that do not enable tsc for this language
    if !manifest.tsc.enabled_for_language(manifest.package.language) {
        return None;
    }

    // skip phases outside the manifest tsc allowlist
    if !manifest.tsc.includes_phase(phase) {
        return None;
    }

    // resolve effective mode and skip when disabled
    let tsc_mode = tsc_mode_for_manifest(manifest, default_tsc_mode);
    if tsc_mode == EcosystemTscMode::Off {
        return None;
    }

    // run only on Destack failures for on-failure mode
    if tsc_mode == EcosystemTscMode::OnFailure && !has_destack_errors {
        return None;
    }

    // resolve effective tool and one runnable binary
    let tsc_tool = tsc_tool_for_manifest(manifest, default_tsc_tool);
    let Some(binary) = resolve_typescript_tsc_binary(tsc_tool) else {
        let error = match tsc_tool {
            EcosystemTscTool::Auto => {
                "TypeScript TSC binary not found: expected tsgo or tsc in PATH".to_string()
            }
            EcosystemTscTool::Tsgo => {
                "TypeScript TSC binary not found: expected tsgo in PATH".to_string()
            }
            EcosystemTscTool::Tsc => {
                "TypeScript TSC binary not found: expected tsc in PATH".to_string()
            }
        };

        return Some(Err(error));
    };

    // build args from selected entrypoints and run tsc
    let args = build_typescript_tsc_args(package_dir, entrypoints, binary);
    let output = Command::new(binary)
        .args(args.iter().map(String::as_str))
        .current_dir(package_dir)
        .env("CI", "1")
        .output()
        .map_err(|error| {
            format!(
                "failed to run {binary} in {}: {error}",
                package_dir.display()
            )
        });

    // normalize output into one stable run result
    match output {
        Ok(output) => {
            let mut merged = String::new();
            merged.push_str(&String::from_utf8_lossy(&output.stdout));
            merged.push_str(&String::from_utf8_lossy(&output.stderr));

            Some(Ok(TypeScriptTscRun {
                binary,
                args,
                success: output.status.success(),
                output: truncate_typescript_tsc_output(merged.trim_end()),
            }))
        }
        Err(error) => Some(Err(error)),
    }
}
/// Resolve one tsc binary according to the configured tool mode.
fn resolve_typescript_tsc_binary(tool: EcosystemTscTool) -> Option<&'static str> {
    match tool {
        EcosystemTscTool::Tsgo => command_is_available("tsgo").then_some("tsgo"),
        EcosystemTscTool::Tsc => command_is_available("tsc").then_some("tsc"),
        EcosystemTscTool::Auto => {
            if command_is_available("tsgo") {
                Some("tsgo")
            } else if command_is_available("tsc") {
                Some("tsc")
            } else {
                None
            }
        }
    }
}

/// Return whether one command can be executed from PATH.
fn command_is_available(command: &str) -> bool {
    Command::new(command)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

/// Build TypeScript TSC arguments from selected entrypoints.
fn build_typescript_tsc_args(
    package_dir: &Path,
    entrypoints: &[PathBuf],
    binary: &str,
) -> Vec<String> {
    let mut args = vec![
        "--noEmit".to_string(),
        "--pretty".to_string(),
        "false".to_string(),
    ];

    // tsgo requires ignoreConfig when explicit files are passed
    if binary == "tsgo" {
        args.push("--ignoreConfig".to_string());
    }

    // keep tsc workload aligned with selected compiler entrypoints
    for entrypoint in entrypoints {
        let relative = entrypoint
            .strip_prefix(package_dir)
            .unwrap_or(entrypoint.as_path());
        args.push(relative.to_string_lossy().to_string());
    }

    args
}

/// Limit tsc output noise while preserving actionable diagnostics.
fn truncate_typescript_tsc_output(output: &str) -> String {
    const MAX_LINES: usize = 80;
    const MAX_CHARS: usize = 12_000;

    let mut lines = output.lines().collect::<Vec<_>>();
    let was_line_truncated = lines.len() > MAX_LINES;
    if was_line_truncated {
        lines.truncate(MAX_LINES);
    }

    let mut truncated = lines.join("\n");
    let was_char_truncated = truncated.chars().count() > MAX_CHARS;
    if was_char_truncated {
        truncated = truncated.chars().take(MAX_CHARS).collect::<String>();
    }

    if was_line_truncated || was_char_truncated {
        let total_lines = output.lines().count();
        truncated.push_str("\n... output truncated ...");
        truncated.push_str(format!(" ({total_lines} total lines)").as_str());
    }

    truncated
}

/// Return whether one argument looks like a source entrypoint path.
fn argument_is_tsc_source_entrypoint(argument: &str) -> bool {
    const SOURCE_SUFFIXES: &[&str] = &[".ts", ".tsx", ".js", ".jsx", ".cts", ".mts"];

    SOURCE_SUFFIXES
        .iter()
        .any(|suffix| argument.ends_with(suffix))
}

/// Format one TSC command for readable failure output.
fn format_tsc_command_for_display(tsc_run: &TypeScriptTscRun) -> String {
    // split options from source entrypoint arguments
    let first_entrypoint_index = tsc_run
        .args
        .iter()
        .position(|argument| argument_is_tsc_source_entrypoint(argument));

    // keep full command when no entrypoints were detected
    let Some(first_entrypoint_index) = first_entrypoint_index else {
        return format!("{} {}", tsc_run.binary, tsc_run.args.join(" "));
    };

    let options = tsc_run.args[..first_entrypoint_index].join(" ");
    let entrypoints = &tsc_run.args[first_entrypoint_index..];

    // keep short commands verbatim
    if entrypoints.len() <= 3 {
        return format!("{} {} {}", tsc_run.binary, options, entrypoints.join(" "));
    }

    // collapse long entrypoint lists into one compact summary
    let preview = entrypoints
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{} {} <{} entrypoints: {}, ...>",
        tsc_run.binary,
        options,
        entrypoints.len(),
        preview,
    )
}

/// Parse one location based TypeScript diagnostic line.
fn parse_tsc_location_diagnostic(line: &str) -> Option<(&str, &str, &str, &str, &str)> {
    let marker = "): error TS";
    let marker_index = line.find(marker)?;
    let prefix = &line[..marker_index];

    let open_paren_index = prefix.rfind('(')?;
    let file = &prefix[..open_paren_index];
    let location = &prefix[(open_paren_index + 1)..];

    let (line_number, column_number) = location.split_once(',')?;

    let suffix = &line[(marker_index + marker.len())..];
    let (code, message) = suffix.split_once(": ")?;

    Some((file, line_number, column_number, code, message))
}

/// Parse one global TypeScript diagnostic line.
fn parse_tsc_global_diagnostic(line: &str) -> Option<(&str, &str)> {
    let suffix = line.strip_prefix("error TS")?;
    let (code, message) = suffix.split_once(": ")?;

    Some((code, message))
}

/// Format TSC diagnostics as highlighted bullet lines.
fn format_tsc_diagnostics_for_display(output: &str) -> String {
    let mut diagnostics = Vec::new();

    for line in output.lines() {
        let line = line.trim_end();

        // skip empty lines from command output
        if line.trim().is_empty() {
            continue;
        }

        // render location diagnostics with highlighted parts
        if let Some((file, line_number, column_number, code, message)) =
            parse_tsc_location_diagnostic(line)
        {
            let location = format!(
                "{}:{}:{}",
                color::cyan(file),
                color::yellow(line_number),
                color::yellow(column_number),
            );
            let code = color::red(&format!("TS{code}"));
            diagnostics.push(format!(
                " - {}: {} {}: {}",
                location,
                color::red("error"),
                code,
                color::bold(message),
            ));
            continue;
        }

        // render global diagnostics with highlighted code
        if let Some((code, message)) = parse_tsc_global_diagnostic(line) {
            diagnostics.push(format!(
                " - {} {}: {}",
                color::red("error"),
                color::red(&format!("TS{code}")),
                color::bold(message),
            ));
            continue;
        }

        // render continuation and fallback lines in dim style
        if line.starts_with(char::is_whitespace) {
            diagnostics.push(format!("   {}", color::dim(line.trim())));
        } else {
            diagnostics.push(format!(" - {}", color::dim(line)));
        }
    }

    diagnostics.join("\n")
}

/// Format one TSC status line with Destack status context.
fn format_tsc_status_line(tsc_success: bool, destack_success: bool) -> String {
    let tsc_status = if tsc_success {
        color::green("passed")
    } else {
        color::red("failed")
    };
    let destack_status = if destack_success {
        color::green("passed")
    } else {
        color::red("failed")
    };

    format!("TSC: {tsc_status} (Destack {destack_status})")
}

/// Return failure context text for one tsc result.
fn tsc_result_to_failure_context(tsc_result: &Result<TypeScriptTscRun, String>) -> String {
    match tsc_result {
        Ok(tsc_run) => {
            let mut message = format_tsc_status_line(tsc_run.success, false);
            message.push('\n');
            message.push_str(&format!(
                "tsc: {}",
                color::dim(&format_tsc_command_for_display(tsc_run))
            ));

            if !tsc_run.output.is_empty() {
                message.push('\n');
                message.push_str(&format_tsc_diagnostics_for_display(&tsc_run.output));
            }

            message
        }
        Err(error) => {
            let mut message = format!(
                "TSC: {} (Destack {})",
                color::yellow("unavailable"),
                color::red("failed"),
            );
            message.push('\n');
            message.push_str(&format!("tsc: {}", color::dim("not executed")));
            message.push('\n');
            message.push_str(&format!(" - {}", color::red(error)));
            message
        }
    }
}

/// Return the root artifact for one phase boundary.
fn phase_root_for_module(
    repository: &Arc<Repository>,
    revision: Revision,
    module_id: ModuleId,
    phase: EcosystemPhase,
) -> Option<ArtifactKey> {
    match phase {
        EcosystemPhase::Parse => None,
        EcosystemPhase::Resolve => {
            let profile_id = default_profile_id_for_module(repository, revision, module_id);
            Some(ArtifactKey::DirExported {
                module: module_id,
                profile: profile_id,
            })
        }
        EcosystemPhase::Analyze => {
            let profile_id = default_profile_id_for_module(repository, revision, module_id);
            Some(ArtifactKey::DirChecked {
                module: module_id,
                profile: profile_id,
            })
        }
        EcosystemPhase::Lower => {
            let module = repository
                .module(revision, module_id)
                .unwrap_or_else(|error| panic!("failed to read module: {error}"))
                .unwrap_or_else(|| panic!("missing module {module_id:?}"));
            let target = repository
                .package_default_target(revision, module.package_id)
                .unwrap_or_else(|error| panic!("failed to resolve diagnostic target: {error}"))
                .map(|(target_id, _)| target_id)
                .unwrap_or_else(|| TargetId::new(module.package_id, "default"));
            let profile_id =
                profile_id_for_target_or_default(repository, revision, module_id, &target);
            Some(ArtifactKey::MirOptimized {
                module: module_id,
                profile: profile_id,
                target,
            })
        }
    }
}

/// Select phase entrypoints from package manifests.
#[cfg(test)]
fn select_phase_entrypoints(package_dir: &Path, files: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    select_phase_entrypoints_with_roots(package_dir, files, &[], &[])
}

/// Select phase entrypoints from package manifests with optional root overrides.
fn select_phase_entrypoints_with_roots(
    package_dir: &Path,
    files: &[PathBuf],
    roots: &[String],
    entrypoints: &[String],
) -> Result<Vec<PathBuf>, String> {
    // start from discovered files and keep runtime oriented candidates
    let mut candidates = files.to_vec();
    candidates.retain(|path| !is_declaration_file(path.as_path()));
    candidates.sort_by_key(|path| entrypoint_sort_key(package_dir, path.as_path()));

    // honor explicit discovery entrypoints when configured
    if !entrypoints.is_empty() {
        let mut configured_candidates = candidates.clone();
        include_entry_target_candidates(&mut configured_candidates, package_dir, entrypoints);
        configured_candidates.sort_by_key(|path| entrypoint_sort_key(package_dir, path.as_path()));
        configured_candidates.dedup();

        let configured_entrypoints =
            select_manifest_entry_paths(package_dir, &configured_candidates, entrypoints);
        if !configured_entrypoints.is_empty() {
            return Ok(configured_entrypoints);
        }

        return Err(format!(
            "no entrypoints matched configured discovery.entrypoints in {}",
            package_dir.display(),
        ));
    }

    // resolve manifest sources once for deterministic selection
    let manifest_sources = load_manifest_entry_sources(package_dir, &candidates, roots)?;
    if manifest_sources.is_empty() {
        return Err(format!(
            "no entrypoints discovered from package manifest entries in {} ({} candidate source files)",
            package_dir.display(),
            candidates.len(),
        ));
    }

    // first pass: select from currently discovered candidates
    let manifest_entrypoints =
        select_manifest_entrypoints_from_sources(package_dir, &candidates, &manifest_sources);
    if !manifest_entrypoints.is_empty() {
        return Ok(manifest_entrypoints);
    }

    // second pass: include explicit manifest targets filtered out by discovery
    let mut expanded_candidates = candidates.clone();
    include_manifest_target_candidates(&mut expanded_candidates, &manifest_sources);
    expanded_candidates.sort_by_key(|path| entrypoint_sort_key(package_dir, path.as_path()));
    expanded_candidates.dedup();

    let mut declaration_only_manifest_entrypoints = Vec::new();
    let manifest_entrypoints = select_manifest_entrypoints_from_sources(
        package_dir,
        &expanded_candidates,
        &manifest_sources,
    );
    if !manifest_entrypoints.is_empty() {
        // return manifest entrypoints when at least one runtime source resolved
        if manifest_entrypoints
            .iter()
            .any(|path| !is_declaration_file(path.as_path()))
        {
            return Ok(manifest_entrypoints);
        }

        // otherwise keep declaration targets as a final fallback
        declaration_only_manifest_entrypoints = manifest_entrypoints;
    }

    // third pass: use tsconfig source roots when manifests target build outputs
    let tsconfig_entrypoints = select_tsconfig_entrypoints_from_manifest_sources(
        package_dir,
        &candidates,
        &manifest_sources,
    )?;
    if !tsconfig_entrypoints.is_empty() {
        return Ok(tsconfig_entrypoints);
    }

    // use declaration targets when no runtime source entrypoint was found
    if !declaration_only_manifest_entrypoints.is_empty() {
        return Ok(declaration_only_manifest_entrypoints);
    }

    Err(format!(
        "no entrypoints discovered from package manifest entries in {} ({} candidate source files)",
        package_dir.display(),
        candidates.len(),
    ))
}
/// Include entry target files when candidate filters excluded them.
fn include_entry_target_candidates(
    candidates: &mut Vec<PathBuf>,
    package_dir: &Path,
    entry_targets: &[String],
) {
    let mut keys = candidates
        .iter()
        .map(|candidate| normalize_path_key(candidate.as_path()))
        .collect::<HashSet<_>>();

    for target in entry_targets {
        for candidate in manifest_target_candidate_paths(package_dir, target) {
            if !candidate.is_file() {
                continue;
            }

            if !path_is_supported_entrypoint_file(candidate.as_path()) {
                continue;
            }

            let key = normalize_path_key(candidate.as_path());
            if keys.insert(key) {
                candidates.push(candidate);
            }
        }
    }
}

/// Include explicit manifest entry target files when candidate filters excluded them.
fn include_manifest_target_candidates(
    candidates: &mut Vec<PathBuf>,
    manifest_sources: &[ManifestEntrySource],
) {
    for source in manifest_sources {
        include_entry_target_candidates(
            candidates,
            source.package_dir.as_path(),
            &source.entry_targets,
        );
    }
}

/// Return possible source file paths for one manifest entry target.
fn manifest_target_candidate_paths(package_dir: &Path, target: &str) -> Vec<PathBuf> {
    let normalized_target = target.replace('\\', "/");
    let mut candidates = Vec::new();
    let target_path = package_dir.normalize_with(&normalized_target);

    candidates.push(target_path.clone());

    // add source extension variants for extensionless targets
    let target_has_extension = target_path.extension().is_some();
    if !target_has_extension {
        for extension in ENTRYPOINT_SOURCE_EXTENSIONS {
            candidates.push(PathBuf::from(format!(
                "{}{extension}",
                target_path.display()
            )));
        }
    }

    // add index file variants for directory-like targets
    let target_is_directory_like = normalized_target.ends_with('/') || !target_has_extension;
    if target_is_directory_like {
        for extension in ENTRYPOINT_SOURCE_EXTENSIONS {
            candidates.push(target_path.join(format!("index{extension}")));
        }
    }

    candidates
}

/// Build deterministic sort keys for phase entrypoints.
fn entrypoint_sort_key(package_dir: &Path, path: &Path) -> (u8, usize, String) {
    let relative = path.strip_prefix(package_dir).unwrap_or(path);
    let file_name = relative
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");

    let root_priority = match file_name {
        "index.ts" | "index.tsx" | "mod.ts" | "mod.tsx" => 0,
        "main.ts" | "main.tsx" | "app.ts" | "app.tsx" => 1,
        _ => 2,
    };
    let depth = relative.components().count();

    (root_priority, depth, relative.to_string_lossy().to_string())
}

/// Return whether this path points to a declaration file.
fn is_declaration_file(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    file_name.ends_with(".d.ts") || file_name.ends_with(".d.mts") || file_name.ends_with(".d.cts")
}

/// Return whether one path is a supported compiler entrypoint source file.
fn path_is_supported_entrypoint_file(path: &Path) -> bool {
    matches!(
        FileType::from_path(path),
        Some(
            FileType::TypeScript
                | FileType::TypeScriptXml
                | FileType::TypeScriptDeclaration
                | FileType::JavaScript
                | FileType::JavaScriptXml
        )
    )
}

/// One package manifest source used for entrypoint resolution.
#[derive(Debug, Clone)]
struct ManifestEntrySource {
    /// The package directory containing this manifest.
    package_dir: PathBuf,
    /// Ordered manifest entry targets.
    entry_targets: Vec<String>,
}

/// Select candidate files that correspond to package manifest entry fields.
fn select_manifest_entrypoints_from_sources(
    package_dir: &Path,
    candidates: &[PathBuf],
    manifest_sources: &[ManifestEntrySource],
) -> Vec<PathBuf> {
    let mut selected = Vec::new();
    let mut selected_keys = HashSet::new();

    // resolve entry targets for each manifest source
    for source in manifest_sources {
        let source_candidates = candidates
            .iter()
            .filter(|path| path.starts_with(source.package_dir.as_path()))
            .cloned()
            .collect::<Vec<_>>();
        if source_candidates.is_empty() {
            continue;
        }

        let source_selected = select_manifest_entry_paths(
            source.package_dir.as_path(),
            &source_candidates,
            &source.entry_targets,
        );

        for selected_path in source_selected {
            let selected_key = normalize_path_key(selected_path.as_path());
            if selected_keys.insert(selected_key) {
                selected.push(selected_path);
            }
        }
    }

    selected.sort_by_key(|path| entrypoint_sort_key(package_dir, path.as_path()));

    selected
}

/// Select entrypoints from tsconfig source roots when manifest entry targets are build outputs.
fn select_tsconfig_entrypoints_from_manifest_sources(
    package_dir: &Path,
    candidates: &[PathBuf],
    manifest_sources: &[ManifestEntrySource],
) -> Result<Vec<PathBuf>, String> {
    let mut selected = Vec::new();
    let mut selected_keys = HashSet::new();

    // resolve one tsconfig source set for each manifest source package
    for source in manifest_sources {
        let source_candidates = candidates
            .iter()
            .filter(|path| path.starts_with(source.package_dir.as_path()))
            .cloned()
            .collect::<Vec<_>>();
        if source_candidates.is_empty() {
            continue;
        }

        let Some(tsconfig_source) = load_tsconfig_entry_source(source.package_dir.as_path())?
        else {
            continue;
        };

        let source_selected = select_tsconfig_entry_paths(&source_candidates, &tsconfig_source);
        for selected_path in source_selected {
            let selected_key = normalize_path_key(selected_path.as_path());
            if selected_keys.insert(selected_key) {
                selected.push(selected_path);
            }
        }
    }

    selected.sort_by_key(|path| entrypoint_sort_key(package_dir, path.as_path()));

    Ok(selected)
}

/// Tsconfig data used for source entrypoint fallback selection.
#[derive(Debug, Clone)]
struct TsConfigEntrySource {
    /// The directory containing the tsconfig file.
    directory: PathBuf,
    /// Explicit source files from tsconfig.
    files: Vec<String>,
    /// Include globs from tsconfig.
    include: Vec<String>,
    /// Exclude globs from tsconfig.
    exclude: Vec<String>,
    /// Output directory from tsconfig compiler options.
    out_dir: Option<PathBuf>,
}

/// Load one package tsconfig source selection when available.
fn load_tsconfig_entry_source(package_dir: &Path) -> Result<Option<TsConfigEntrySource>, String> {
    // find one package level tsconfig or jsconfig
    let Some(tsconfig_path) = resolve_package_tsconfig_path(package_dir) else {
        return Ok(None);
    };

    // load tsconfig source text
    let content = fs::read_to_string(tsconfig_path.as_path())
        .map_err(|error| format!("failed to read {}: {error}", tsconfig_path.display()))?;

    // parse jsonc config into a workspace tsconfig model
    let (name, uri) = Uri::from_path_with_name(tsconfig_path.as_path());
    let file_type = FileType::from_path(tsconfig_path.as_path()).unwrap_or(FileType::Json);
    let file = File::from_text(
        FileId::new(0),
        name,
        uri,
        Some(tsconfig_path.clone()),
        file_type,
        content,
    );
    let file = Arc::new(file);
    let tsconfig = TsConfigDeclaration::parse(true, &file)
        .map_err(|error| format!("failed to parse {}: {error}", tsconfig_path.display()))?;
    let tsconfig_options = tsconfig.options();

    // project tsconfig source selection fields used by fallback discovery
    let out_dir = tsconfig_options.compiler.out_dir.clone();
    Ok(Some(TsConfigEntrySource {
        directory: tsconfig.directory,
        files: tsconfig_options.files,
        include: tsconfig_options.include,
        exclude: tsconfig_options.exclude,
        out_dir,
    }))
}

/// Resolve one package tsconfig path from common config file names.
fn resolve_package_tsconfig_path(package_dir: &Path) -> Option<PathBuf> {
    for config_name in [
        "tsconfig.json",
        "tsconfig.jsonc",
        "jsconfig.json",
        "jsconfig.jsonc",
    ] {
        let config_path = package_dir.join(config_name);
        if config_path.is_file() {
            return Some(config_path);
        }
    }

    None
}

/// Select source entrypoints from one tsconfig source set.
fn select_tsconfig_entry_paths(
    candidates: &[PathBuf],
    tsconfig_source: &TsConfigEntrySource,
) -> Vec<PathBuf> {
    // resolve explicit files through existing entry target matching
    if !tsconfig_source.files.is_empty() {
        return select_manifest_entry_paths(
            tsconfig_source.directory.as_path(),
            candidates,
            &tsconfig_source.files,
        );
    }

    // apply include semantics: omitted include means all discovered source candidates
    let include_patterns = if tsconfig_source.include.is_empty() {
        None
    } else {
        Some(tsconfig_source.include.clone())
    };

    // apply exclude defaults when tsconfig omits exclude
    let mut exclude_patterns = if tsconfig_source.exclude.is_empty() {
        TYPESCRIPT_DEFAULT_EXCLUDES
            .iter()
            .map(|pattern| pattern.to_string())
            .collect::<Vec<_>>()
    } else {
        tsconfig_source.exclude.clone()
    };

    // exclude configured output directories to avoid selecting build artifacts
    if let Some(out_dir) = tsconfig_source.out_dir.as_ref() {
        let out_dir_pattern =
            normalize_tsconfig_out_dir_pattern(tsconfig_source.directory.as_path(), out_dir);
        if !out_dir_pattern.is_empty() {
            exclude_patterns.push(out_dir_pattern);
        }
    }

    // filter source candidates with tsconfig include and exclude semantics
    candidates
        .iter()
        .filter_map(|candidate| {
            let relative = candidate
                .strip_prefix(tsconfig_source.directory.as_path())
                .unwrap_or(candidate.as_path());
            let path_fragment = normalize_path_key(relative);
            if path_fragment.is_empty() {
                return None;
            }

            let is_included = match include_patterns.as_ref() {
                Some(patterns) => patterns
                    .iter()
                    .any(|pattern| tsconfig_path_matches_pattern(pattern, &path_fragment)),
                None => true,
            };
            if !is_included {
                return None;
            }

            let is_excluded = exclude_patterns
                .iter()
                .any(|pattern| tsconfig_path_matches_pattern(pattern, &path_fragment));
            if is_excluded {
                return None;
            }

            Some(candidate.clone())
        })
        .collect()
}

/// Normalize one outDir pattern to a package relative glob fragment.
fn normalize_tsconfig_out_dir_pattern(directory: &Path, out_dir: &Path) -> String {
    let relative = if out_dir.is_absolute() {
        out_dir.strip_prefix(directory).unwrap_or(out_dir)
    } else {
        out_dir
    };

    normalize_tsconfig_pattern(relative.to_string_lossy().as_ref())
}

/// Return true when a path fragment matches one tsconfig style pattern.
fn tsconfig_path_matches_pattern(pattern: &str, path_fragment: &str) -> bool {
    let pattern = normalize_tsconfig_pattern(pattern);
    if pattern.is_empty() {
        return false;
    }

    // match tsconfig style glob patterns with star behavior that allows extensions
    if tsconfig_glob_matches_pattern(pattern.as_str(), path_fragment) {
        return true;
    }

    // stop after glob matching for patterns with wildcards
    if pattern.contains('*') || pattern.contains('?') {
        return false;
    }

    // treat non-glob directory names as recursive includes
    let trimmed = pattern.trim_end_matches('/');
    if path_fragment == trimmed || path_fragment.starts_with(&format!("{trimmed}/")) {
        return true;
    }

    false
}

/// Return whether one path fragment matches one tsconfig glob pattern.
fn tsconfig_glob_matches_pattern(pattern: &str, path_fragment: &str) -> bool {
    let pattern_segments = tsconfig_path_segments(pattern);
    let path_segments = tsconfig_path_segments(path_fragment);

    tsconfig_glob_matches_segments(&pattern_segments, &path_segments)
}

/// Match normalized tsconfig path segments with support for `**`, `*`, and `?`.
fn tsconfig_glob_matches_segments(pattern_segments: &[&str], path_segments: &[&str]) -> bool {
    if pattern_segments.is_empty() {
        return path_segments.is_empty();
    }

    // let `**` consume zero or more path segments
    if pattern_segments[0] == "**" {
        let mut rest_pattern_segments = &pattern_segments[1..];
        while rest_pattern_segments
            .first()
            .is_some_and(|segment| *segment == "**")
        {
            rest_pattern_segments = &rest_pattern_segments[1..];
        }

        if rest_pattern_segments.is_empty() {
            return true;
        }

        if tsconfig_glob_matches_segments(rest_pattern_segments, path_segments) {
            return true;
        }

        if path_segments.is_empty() {
            return false;
        }

        return tsconfig_glob_matches_segments(pattern_segments, &path_segments[1..]);
    }

    if path_segments.is_empty() {
        return false;
    }

    if !tsconfig_segment_matches_pattern(pattern_segments[0], path_segments[0]) {
        return false;
    }

    tsconfig_glob_matches_segments(&pattern_segments[1..], &path_segments[1..])
}

/// Split one normalized path fragment into path segments.
fn tsconfig_path_segments(path_fragment: &str) -> Vec<&str> {
    path_fragment
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

/// Match one single path segment against tsconfig wildcards.
fn tsconfig_segment_matches_pattern(pattern_segment: &str, text_segment: &str) -> bool {
    let pattern_bytes = pattern_segment.as_bytes();
    let text_bytes = text_segment.as_bytes();
    let pattern_length = pattern_bytes.len();
    let text_length = text_bytes.len();

    let mut pattern_index = 0;
    let mut text_index = 0;
    let mut star_index = None;
    let mut match_index = 0;

    while text_index < text_length {
        if pattern_index < pattern_length
            && (pattern_bytes[pattern_index] == b'?'
                || pattern_bytes[pattern_index] == text_bytes[text_index])
        {
            pattern_index += 1;
            text_index += 1;
            continue;
        }

        if pattern_index < pattern_length && pattern_bytes[pattern_index] == b'*' {
            star_index = Some(pattern_index);
            pattern_index += 1;
            match_index = text_index;
            continue;
        }

        let Some(star_index) = star_index else {
            return false;
        };

        pattern_index = star_index + 1;
        match_index += 1;
        text_index = match_index;
    }

    while pattern_index < pattern_length && pattern_bytes[pattern_index] == b'*' {
        pattern_index += 1;
    }

    pattern_index == pattern_length
}

/// Normalize one tsconfig glob pattern for stable matching.
fn normalize_tsconfig_pattern(pattern: &str) -> String {
    pattern
        .trim()
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string()
}

/// Load package manifest entry sources from root and workspace package manifests.
fn load_manifest_entry_sources(
    package_dir: &Path,
    candidates: &[PathBuf],
    roots: &[String],
) -> Result<Vec<ManifestEntrySource>, String> {
    // load root package manifest first
    let root_manifest_path = package_dir.join("package.json");
    let Some(root_package_json) = load_package_json(root_manifest_path.as_path())? else {
        return Ok(Vec::new());
    };

    // initialize source collection with duplicate manifest guards
    let mut sources = Vec::new();
    let mut seen_manifest_paths = HashSet::new();

    // track whether the root manifest manages workspace package discovery
    let has_workspace_configuration = root_package_json.workspaces.is_some();

    // include root and workspace manifests when roots are not configured
    if roots.is_empty() {
        let root_entry_targets = root_package_json.entry_targets();
        seen_manifest_paths.insert(normalize_path_key(root_manifest_path.as_path()));
        sources.push(ManifestEntrySource {
            package_dir: package_dir.to_path_buf(),
            entry_targets: root_entry_targets,
        });

        if let Some(workspaces) = root_package_json.workspaces.as_ref() {
            let workspace_manifest_paths =
                collect_workspace_manifest_paths(package_dir, workspaces.patterns());

            for workspace_manifest_path in workspace_manifest_paths {
                let workspace_manifest_key = normalize_path_key(workspace_manifest_path.as_path());
                if !seen_manifest_paths.insert(workspace_manifest_key) {
                    continue;
                }

                let Some(workspace_package_json) =
                    load_package_json(workspace_manifest_path.as_path())?
                else {
                    continue;
                };

                let workspace_entry_targets = workspace_package_json.entry_targets();
                let Some(workspace_package_dir) = workspace_manifest_path.parent() else {
                    continue;
                };
                if !workspace_package_dir.starts_with(package_dir) {
                    continue;
                }

                sources.push(ManifestEntrySource {
                    package_dir: workspace_package_dir.to_path_buf(),
                    entry_targets: workspace_entry_targets,
                });
            }
        }
    }

    // use candidate ownership fallback only when the root manifest does not declare workspaces
    let candidate_manifest_paths = if roots.is_empty() {
        if has_workspace_configuration {
            Vec::new()
        } else {
            collect_candidate_manifest_paths(package_dir, candidates)
        }
    } else {
        collect_root_manifest_paths(package_dir, roots)?
    };

    for candidate_manifest_path in candidate_manifest_paths {
        let candidate_manifest_key = normalize_path_key(candidate_manifest_path.as_path());
        if !seen_manifest_paths.insert(candidate_manifest_key) {
            continue;
        }

        let Some(candidate_package_json) = load_package_json(candidate_manifest_path.as_path())?
        else {
            continue;
        };

        let candidate_entry_targets = candidate_package_json.entry_targets();
        let Some(candidate_package_dir) = candidate_manifest_path.parent() else {
            continue;
        };
        if !candidate_package_dir.starts_with(package_dir) {
            continue;
        }

        sources.push(ManifestEntrySource {
            package_dir: candidate_package_dir.to_path_buf(),
            entry_targets: candidate_entry_targets,
        });
    }

    // return deterministic ordering by package directory
    sources.sort_by_key(|source| normalize_path_key(source.package_dir.as_path()));

    Ok(sources)
}
/// Load one package json file when present.
fn load_package_json(path: &Path) -> Result<Option<PackageJson>, String> {
    if !path.is_file() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let package_json = serde_json::from_str::<PackageJson>(&content)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;

    Ok(Some(package_json))
}

/// Normalize one path key for stable matching.
fn normalize_path_key(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Select candidate paths matching one list of manifest entry targets.
fn select_manifest_entry_paths(
    package_dir: &Path,
    candidates: &[PathBuf],
    entry_targets: &[String],
) -> Vec<PathBuf> {
    let mut selected = Vec::new();
    let mut selected_keys = HashSet::new();

    for entry_target in entry_targets {
        let entry_target = entry_target
            .trim()
            .replace('\\', "/")
            .trim_start_matches("./")
            .trim_start_matches('/')
            .to_string();
        if entry_target.is_empty() {
            continue;
        }
        let entry_target_without_extension = ENTRYPOINT_SOURCE_EXTENSIONS
            .iter()
            .find_map(|extension| entry_target.strip_suffix(extension))
            .unwrap_or(entry_target.as_str());

        for candidate in candidates {
            let relative = candidate
                .strip_prefix(package_dir)
                .unwrap_or(candidate.as_path());
            let relative = normalize_path_key(relative);
            let relative_without_extension = ENTRYPOINT_SOURCE_EXTENSIONS
                .iter()
                .find_map(|extension| relative.strip_suffix(extension))
                .unwrap_or(relative.as_str());

            let matches_entry = relative == entry_target
                || relative_without_extension == entry_target
                || relative == entry_target_without_extension
                || relative_without_extension == entry_target_without_extension;
            if !matches_entry {
                continue;
            }

            let candidate_key = normalize_path_key(candidate.as_path());
            if selected_keys.insert(candidate_key) {
                selected.push(candidate.clone());
            }
        }
    }

    selected
}

/// Collect package manifest paths from explicit discovery roots.
fn collect_root_manifest_paths(
    package_dir: &Path,
    roots: &[String],
) -> Result<Vec<PathBuf>, String> {
    let mut manifest_paths = Vec::new();
    let mut manifest_keys = HashSet::new();

    // resolve each configured root to one package manifest
    for root in roots {
        let trimmed_root = root.trim();
        if trimmed_root.is_empty() {
            return Err("discovery root cannot be empty".to_string());
        }

        let root_path = Path::new(trimmed_root);
        if root_path.is_absolute() {
            return Err(format!("discovery root '{trimmed_root}' must be relative"));
        }

        if root_path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return Err(format!(
                "discovery root '{trimmed_root}' cannot contain '..'"
            ));
        }

        let root_directory = package_dir.join(root_path);
        let manifest_path = root_directory.join("package.json");
        if !manifest_path.is_file() {
            return Err(format!(
                "discovery root '{}' missing package.json at {}",
                trimmed_root,
                manifest_path.display(),
            ));
        }

        let manifest_key = normalize_path_key(manifest_path.as_path());
        if manifest_keys.insert(manifest_key) {
            manifest_paths.push(manifest_path);
        }
    }

    manifest_paths.sort_by_key(|path| normalize_path_key(path.as_path()));

    Ok(manifest_paths)
}

/// Collect one nearest package manifest for each discovered candidate source file.
fn collect_candidate_manifest_paths(package_dir: &Path, candidates: &[PathBuf]) -> Vec<PathBuf> {
    let mut manifest_paths = Vec::new();
    let mut manifest_keys = HashSet::new();

    // discover one nearest package manifest while walking each candidate ancestor chain
    for candidate in candidates {
        let mut current_directory = candidate.parent();
        while let Some(directory) = current_directory {
            if !directory.starts_with(package_dir) {
                break;
            }

            let manifest_path = directory.join("package.json");
            if manifest_path.is_file() {
                let manifest_key = normalize_path_key(manifest_path.as_path());
                if manifest_keys.insert(manifest_key) {
                    manifest_paths.push(manifest_path);
                }

                break;
            }

            if directory == package_dir {
                break;
            }

            current_directory = directory.parent();
        }
    }

    manifest_paths.sort_by_key(|path| normalize_path_key(path.as_path()));

    manifest_paths
}
/// Collect workspace package manifest paths from npm style workspace patterns.
fn collect_workspace_manifest_paths(package_dir: &Path, patterns: &[String]) -> Vec<PathBuf> {
    let mut selected_paths: Vec<PathBuf> = Vec::new();
    let mut selected_path_keys: HashSet<String> = HashSet::new();

    // apply include and exclude workspace patterns in order
    for raw_pattern in patterns {
        let trimmed_pattern = raw_pattern.trim();
        if trimmed_pattern.is_empty() {
            continue;
        }

        // parse workspace exclusion syntax
        let (is_exclude_pattern, workspace_pattern) =
            if let Some(workspace_pattern) = trimmed_pattern.strip_prefix('!') {
                (true, workspace_pattern.trim())
            } else {
                (false, trimmed_pattern)
            };
        if workspace_pattern.is_empty() {
            continue;
        }

        // resolve one pattern to matching package manifests
        let matched_manifest_paths =
            glob_workspace_manifest_paths_for_pattern(package_dir, workspace_pattern);

        // remove excluded matches from the current set
        if is_exclude_pattern {
            for manifest_path in matched_manifest_paths {
                let manifest_key = normalize_path_key(manifest_path.as_path());
                if !selected_path_keys.remove(&manifest_key) {
                    continue;
                }

                selected_paths.retain(|path| normalize_path_key(path.as_path()) != manifest_key);
            }

            continue;
        }

        // add included matches while preserving order
        for manifest_path in matched_manifest_paths {
            let manifest_key = normalize_path_key(manifest_path.as_path());
            if selected_path_keys.insert(manifest_key) {
                selected_paths.push(manifest_path);
            }
        }
    }

    selected_paths
}

/// Resolve one workspace pattern to matching package manifest paths.
fn glob_workspace_manifest_paths_for_pattern(package_dir: &Path, pattern: &str) -> Vec<PathBuf> {
    let mut manifest_paths = Vec::new();
    let pattern = pattern.trim_end_matches('/');
    if pattern.is_empty() {
        return manifest_paths;
    }

    // normalize one workspace pattern to package json targets
    let manifest_pattern = if pattern.ends_with("package.json") {
        pattern.to_string()
    } else {
        format!("{pattern}/package.json")
    };
    let absolute_manifest_pattern = package_dir.join(manifest_pattern);

    // collect matching package manifests inside the package root
    for matched_path in glob(&absolute_manifest_pattern.to_string_lossy()) {
        if !matched_path.is_file() {
            continue;
        }
        if !matched_path.starts_with(package_dir) {
            continue;
        }

        manifest_paths.push(matched_path);
    }

    manifest_paths.sort_by_key(|path| normalize_path_key(path.as_path()));
    manifest_paths
}
/// Parse one source file and fail when parser diagnostics contain errors.
fn parse_file(path: &Path, manifest: &EcosystemManifest) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|error| format!("read error: {error}"))?;

    let file_type = FileType::from_path(path).unwrap_or(FileType::TypeScript);
    let language_type = language_type_for_parse(file_type, &manifest.compiler_options);

    let uri = Uri::from_path(path);
    let file_system: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::new());
    let cwd = path.parent().unwrap_or(Path::new(".")).to_path_buf();

    let _program = open_repository_with_options(
        cwd,
        file_system,
        FormatterOptions::default(),
        LinterOptions::default(),
    );
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("input")
        .to_string();
    let file_id = FileId::from_logical_path(path);
    let file = Arc::new(File::from_text(
        file_id,
        name,
        uri,
        Some(path.to_path_buf()),
        file_type,
        content,
    ));
    let file_for_id = |current_file_id| {
        if current_file_id == file_id {
            Some(file.clone())
        } else {
            None
        }
    };

    let mut parser = Parser::lex_file(file.clone(), language_type);
    let _ = parser.parse();

    let errors = parser
        .diagnostics
        .to_vec()
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .collect::<Vec<_>>();

    if errors.is_empty() {
        return Ok(());
    }

    let mut error_diagnostics = DiagnosticCollection::new();
    for diagnostic in errors.iter().cloned() {
        error_diagnostics.insert(diagnostic);
    }

    let diagnostic_output = format_diagnostics(
        &file_for_id,
        &error_diagnostics,
        PrintOptions::new()
            .with_colorizer(source_colorizer())
            .with_module_count(1),
    );

    Err(diagnostic_output.trim_end().to_string())
}

/// Resolve parse language mode for a discovered source file.
fn language_type_for_parse(
    file_type: FileType,
    compiler_options: &CompilerOptionsConfig,
) -> LanguageType {
    // allow explicit js to jsx parse mode override
    if file_type == FileType::JavaScript && compiler_options.js_as_jsx() {
        return LanguageType::JavaScriptXml;
    }

    LanguageType::try_from(file_type).expect("file type has no parser language")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        ExpectedDiagnosticConfig, ObservedPhaseDiagnostic, match_expected_phase_diagnostics,
        normalize_path_fragment, select_phase_entrypoints, select_phase_entrypoints_with_roots,
        tsconfig_path_matches_pattern,
    };
    use crate::ecosystem::manifest::EcosystemPhase;

    /// Allocate a unique temporary directory for one test.
    fn unique_temp_dir(label: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        let process_id = std::process::id();
        std::env::temp_dir().join(format!(
            "destack-ecosystem-tier-{label}-{process_id}-{timestamp}"
        ))
    }

    /// Write one file and create parent directories.
    fn write_text_file(path: &Path, text: &str) {
        let parent = path.parent().expect("expected parent directory");
        fs::create_dir_all(parent).expect("failed to create parent directory");
        fs::write(path, text).expect("failed to write file");
    }

    #[test]
    fn test_match_expected_phase_diagnostics_matches_exact_normalized_values() {
        let expected = vec![ExpectedDiagnosticConfig {
            phase: EcosystemPhase::Resolve,
            code: "ER200".to_string(),
            file: "src/utils/format.ts".to_string(),
            message: "unresolved module 'printj'".to_string(),
        }];
        let observed = vec![ObservedPhaseDiagnostic {
            code: "ER200".to_string(),
            file: r"src\utils\format.ts".to_string(),
            message: "unresolved module 'printj'".to_string(),
        }];
        let observed = observed
            .into_iter()
            .map(|diagnostic| ObservedPhaseDiagnostic {
                code: diagnostic.code,
                file: normalize_path_fragment(&diagnostic.file),
                message: diagnostic.message,
            })
            .collect::<Vec<_>>();

        // compare expected and observed diagnostics
        let outcome = match_expected_phase_diagnostics(&expected, &observed);

        assert!(outcome.missing_expected.is_empty());
        assert!(outcome.unexpected_observed.is_empty());
    }

    #[test]
    fn test_match_expected_phase_diagnostics_rejects_path_fragments() {
        let expected = vec![ExpectedDiagnosticConfig {
            phase: EcosystemPhase::Resolve,
            code: "ER200".to_string(),
            file: "format.ts".to_string(),
            message: "unresolved module 'printj'".to_string(),
        }];
        let observed = vec![ObservedPhaseDiagnostic {
            code: "ER200".to_string(),
            file: "src/utils/format.ts".to_string(),
            message: "unresolved module 'printj'".to_string(),
        }];

        let outcome = match_expected_phase_diagnostics(&expected, &observed);

        assert_eq!(outcome.missing_expected.len(), 1);
        assert_eq!(outcome.unexpected_observed.len(), 1);
    }

    #[test]
    fn test_match_expected_phase_diagnostics_rejects_message_fragments() {
        let expected = vec![ExpectedDiagnosticConfig {
            phase: EcosystemPhase::Resolve,
            code: "ER200".to_string(),
            file: "src/utils/format.ts".to_string(),
            message: "printj".to_string(),
        }];
        let observed = vec![ObservedPhaseDiagnostic {
            code: "ER200".to_string(),
            file: "src/utils/format.ts".to_string(),
            message: "unresolved module 'printj'".to_string(),
        }];

        let outcome = match_expected_phase_diagnostics(&expected, &observed);

        assert_eq!(outcome.missing_expected.len(), 1);
        assert_eq!(outcome.unexpected_observed.len(), 1);
    }

    #[test]
    fn test_match_expected_phase_diagnostics_reports_missing_and_unexpected() {
        let expected = vec![ExpectedDiagnosticConfig {
            phase: EcosystemPhase::Resolve,
            code: "ER200".to_string(),
            file: "src/index.ts".to_string(),
            message: "unresolved module 'a'".to_string(),
        }];
        let observed = vec![ObservedPhaseDiagnostic {
            code: "ER201".to_string(),
            file: "src/index.ts".to_string(),
            message: "unresolved module 'b'".to_string(),
        }];

        // compare expected and observed diagnostics
        let outcome = match_expected_phase_diagnostics(&expected, &observed);

        assert_eq!(outcome.missing_expected.len(), 1);
        assert_eq!(outcome.missing_expected[0].code, "ER200");
        assert_eq!(outcome.unexpected_observed.len(), 1);
        assert_eq!(outcome.unexpected_observed[0].code, "ER201");
    }

    #[test]
    fn test_select_phase_entrypoints_from_workspace_manifests() {
        let temp_dir = unique_temp_dir("workspace-entrypoints");
        fs::create_dir_all(&temp_dir).expect("failed to create temp directory");

        // create root workspace package json
        write_text_file(
            &temp_dir.join("package.json"),
            r#"{
  "name": "root",
  "private": true,
  "workspaces": ["packages/*"]
}"#,
        );

        // create workspace package manifests with explicit entry targets
        write_text_file(
            &temp_dir.join("packages/a/package.json"),
            r#"{
  "name": "a",
  "main": "./src/index.js"
}"#,
        );
        write_text_file(
            &temp_dir.join("packages/b/package.json"),
            r#"{
  "name": "b",
  "exports": {
    ".": "./src/main.ts"
  }
}"#,
        );

        // create source candidates for both packages
        let candidates = vec![
            temp_dir.join("packages/a/src/index.ts"),
            temp_dir.join("packages/a/src/extra.ts"),
            temp_dir.join("packages/b/src/main.ts"),
            temp_dir.join("packages/b/src/extra.ts"),
        ];
        for path in &candidates {
            write_text_file(path, "export {};");
        }

        // select entrypoints from manifest targets only
        let selected =
            select_phase_entrypoints(&temp_dir, &candidates).expect("expected entrypoints");
        let selected_relative = selected
            .iter()
            .map(|path| {
                path.strip_prefix(&temp_dir)
                    .expect("expected package relative path")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect::<Vec<_>>();

        assert_eq!(
            selected_relative,
            vec!["packages/a/src/index.ts", "packages/b/src/main.ts"]
        );

        fs::remove_dir_all(&temp_dir).expect("failed to remove temp directory");
    }

    #[test]
    fn test_select_phase_entrypoints_respects_workspace_exclusions() {
        let temp_dir = unique_temp_dir("workspace-exclude");
        fs::create_dir_all(&temp_dir).expect("failed to create temp directory");

        // create root workspace package json with one exclusion
        write_text_file(
            &temp_dir.join("package.json"),
            r#"{
  "name": "root",
  "private": true,
  "workspaces": ["packages/*", "!packages/b"]
}"#,
        );

        // create workspace package manifests with explicit entry targets
        write_text_file(
            &temp_dir.join("packages/a/package.json"),
            r#"{
  "name": "a",
  "main": "./src/index.ts"
}"#,
        );
        write_text_file(
            &temp_dir.join("packages/b/package.json"),
            r#"{
  "name": "b",
  "main": "./src/main.ts"
}"#,
        );

        // create source candidates for both packages
        let candidates = vec![
            temp_dir.join("packages/a/src/index.ts"),
            temp_dir.join("packages/b/src/main.ts"),
        ];
        for path in &candidates {
            write_text_file(path, "export {};");
        }

        // select only non excluded workspace entrypoints
        let selected =
            select_phase_entrypoints(&temp_dir, &candidates).expect("expected entrypoints");
        let selected_relative = selected
            .iter()
            .map(|path| {
                path.strip_prefix(&temp_dir)
                    .expect("expected package relative path")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect::<Vec<_>>();

        assert_eq!(selected_relative, vec!["packages/a/src/index.ts"]);

        fs::remove_dir_all(&temp_dir).expect("failed to remove temp directory");
    }

    #[test]
    fn test_select_phase_entrypoints_from_candidate_manifest_sources() {
        let temp_dir = unique_temp_dir("candidate-manifest-sources");
        fs::create_dir_all(&temp_dir).expect("failed to create temp directory");

        // create root package manifest without workspace configuration
        write_text_file(
            &temp_dir.join("package.json"),
            r#"{
  "name": "root",
  "private": true
}"#,
        );

        // create nested publish package manifest with build output entries
        write_text_file(
            &temp_dir.join("library/package.json"),
            r#"{
  "name": "library",
  "main": "./dist/index.js"
}"#,
        );

        // create nested tsconfig with explicit source files
        write_text_file(
            &temp_dir.join("library/tsconfig.json"),
            r#"{
  "files": ["src/index.ts"]
}"#,
        );

        // create source candidates inside the nested package
        let candidates = vec![
            temp_dir.join("library/src/index.ts"),
            temp_dir.join("library/src/extra.ts"),
        ];
        for path in &candidates {
            write_text_file(path, "export {};");
        }

        // select entrypoints through candidate discovered package manifests
        let selected =
            select_phase_entrypoints(&temp_dir, &candidates).expect("expected entrypoints");
        let selected_relative = selected
            .iter()
            .map(|path| {
                path.strip_prefix(&temp_dir)
                    .expect("expected package relative path")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect::<Vec<_>>();

        assert_eq!(selected_relative, vec!["library/src/index.ts"]);

        fs::remove_dir_all(&temp_dir).expect("failed to remove temp directory");
    }

    #[test]
    fn test_select_phase_entrypoints_uses_discovery_roots() {
        let temp_dir = unique_temp_dir("entrypoint-roots");
        fs::create_dir_all(&temp_dir).expect("failed to create temp directory");

        // create root and nested package manifests with source entry targets
        write_text_file(
            &temp_dir.join("package.json"),
            r#"{
  "name": "root",
  "main": "./src/root.ts"
}"#,
        );
        write_text_file(
            &temp_dir.join("library/package.json"),
            r#"{
  "name": "library",
  "main": "./src/index.ts"
}"#,
        );

        // create source candidates from both roots
        let candidates = vec![
            temp_dir.join("src/root.ts"),
            temp_dir.join("library/src/index.ts"),
        ];
        for path in &candidates {
            write_text_file(path, "export {};");
        }

        // select only entrypoints from configured discovery roots
        let roots = vec!["library".to_string()];
        let selected = select_phase_entrypoints_with_roots(&temp_dir, &candidates, &roots, &[])
            .expect("expected entrypoints");
        let selected_relative = selected
            .iter()
            .map(|path| {
                path.strip_prefix(&temp_dir)
                    .expect("expected package relative path")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect::<Vec<_>>();

        assert_eq!(selected_relative, vec!["library/src/index.ts"]);

        fs::remove_dir_all(&temp_dir).expect("failed to remove temp directory");
    }

    #[test]
    fn test_select_phase_entrypoints_errors_on_invalid_discovery_root() {
        let temp_dir = unique_temp_dir("entrypoint-roots-invalid");
        fs::create_dir_all(&temp_dir).expect("failed to create temp directory");

        // create one root package manifest and source candidate
        write_text_file(
            &temp_dir.join("package.json"),
            r#"{
  "name": "root",
  "main": "./src/index.ts"
}"#,
        );
        let candidates = vec![temp_dir.join("src/index.ts")];
        write_text_file(&candidates[0], "export {};");

        // fail loudly when configured roots do not contain a package manifest
        let roots = vec!["library".to_string()];
        let error = select_phase_entrypoints_with_roots(&temp_dir, &candidates, &roots, &[])
            .expect_err("expected invalid discovery root error");

        assert!(error.contains("discovery root 'library' missing package.json"));

        fs::remove_dir_all(&temp_dir).expect("failed to remove temp directory");
    }

    #[test]
    fn test_select_phase_entrypoints_falls_back_to_tsconfig_files() {
        let temp_dir = unique_temp_dir("tsconfig-files-fallback");
        fs::create_dir_all(&temp_dir).expect("failed to create temp directory");

        // create package manifest with build output entries only
        write_text_file(
            &temp_dir.join("package.json"),
            r#"{
  "name": "root",
  "main": "./dist/index.js"
}"#,
        );

        // create tsconfig with explicit source file entries
        write_text_file(
            &temp_dir.join("tsconfig.json"),
            r#"{
  "files": ["src/index.ts"],
  "exclude": ["dist"]
}"#,
        );

        // create source candidates in the package
        let candidates = vec![temp_dir.join("src/index.ts"), temp_dir.join("src/extra.ts")];
        for path in &candidates {
            write_text_file(path, "export {};");
        }

        // select entrypoints from tsconfig fallback
        let selected =
            select_phase_entrypoints(&temp_dir, &candidates).expect("expected entrypoints");
        let selected_relative = selected
            .iter()
            .map(|path| {
                path.strip_prefix(&temp_dir)
                    .expect("expected package relative path")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect::<Vec<_>>();

        assert_eq!(selected_relative, vec!["src/index.ts"]);

        fs::remove_dir_all(&temp_dir).expect("failed to remove temp directory");
    }

    #[test]
    fn test_select_phase_entrypoints_falls_back_to_tsconfig_include_patterns() {
        let temp_dir = unique_temp_dir("tsconfig-include-fallback");
        fs::create_dir_all(&temp_dir).expect("failed to create temp directory");

        // create package manifest with build output entries only
        write_text_file(
            &temp_dir.join("package.json"),
            r#"{
  "name": "root",
  "main": "./dist/index.js"
}"#,
        );

        // create tsconfig with include and exclude source patterns
        write_text_file(
            &temp_dir.join("tsconfig.json"),
            r#"{
  "include": ["src/**/*.ts"],
  "exclude": ["src/extra.ts", "dist"]
}"#,
        );

        // create source candidates in the package
        let candidates = vec![temp_dir.join("src/index.ts"), temp_dir.join("src/extra.ts")];
        for path in &candidates {
            write_text_file(path, "export {};");
        }

        // select entrypoints from tsconfig include fallback
        let selected =
            select_phase_entrypoints(&temp_dir, &candidates).expect("expected entrypoints");
        let selected_relative = selected
            .iter()
            .map(|path| {
                path.strip_prefix(&temp_dir)
                    .expect("expected package relative path")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect::<Vec<_>>();

        assert_eq!(selected_relative, vec!["src/index.ts"]);

        fs::remove_dir_all(&temp_dir).expect("failed to remove temp directory");
    }

    #[test]
    fn test_select_phase_entrypoints_falls_back_to_tsconfig_include_wildcard_paths() {
        let temp_dir = unique_temp_dir("tsconfig-include-wildcard-fallback");
        fs::create_dir_all(&temp_dir).expect("failed to create temp directory");

        // create package manifest with build output entries only
        write_text_file(
            &temp_dir.join("package.json"),
            r#"{
  "name": "root",
  "main": "./dist/index.js"
}"#,
        );

        // create tsconfig with include wildcard paths without an extension suffix
        write_text_file(
            &temp_dir.join("tsconfig.json"),
            r#"{
  "include": ["src/**/*"],
  "exclude": ["dist"]
}"#,
        );

        // create source candidates in the package
        let candidates = vec![
            temp_dir.join("src/index.ts"),
            temp_dir.join("src/utils/math.ts"),
        ];
        for path in &candidates {
            write_text_file(path, "export {};");
        }

        // select entrypoints from tsconfig include fallback
        let selected =
            select_phase_entrypoints(&temp_dir, &candidates).expect("expected entrypoints");
        let selected_relative = selected
            .iter()
            .map(|path| {
                path.strip_prefix(&temp_dir)
                    .expect("expected package relative path")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect::<Vec<_>>();

        assert_eq!(selected_relative, vec!["src/index.ts", "src/utils/math.ts"]);

        fs::remove_dir_all(&temp_dir).expect("failed to remove temp directory");
    }

    #[test]
    fn test_select_phase_entrypoints_falls_back_to_tsconfig_without_manifest_entries() {
        let temp_dir = unique_temp_dir("tsconfig-no-manifest-entry-targets");
        fs::create_dir_all(&temp_dir).expect("failed to create temp directory");

        // create package manifest without explicit entry targets
        write_text_file(
            &temp_dir.join("package.json"),
            r#"{
  "name": "root"
}"#,
        );

        // create tsconfig with explicit source file entries
        write_text_file(
            &temp_dir.join("tsconfig.json"),
            r#"{
  "files": ["src/index.ts"]
}"#,
        );

        // create source candidates in the package
        let candidates = vec![temp_dir.join("src/index.ts"), temp_dir.join("src/extra.ts")];
        for path in &candidates {
            write_text_file(path, "export {};");
        }

        // select entrypoints from tsconfig fallback
        let selected =
            select_phase_entrypoints(&temp_dir, &candidates).expect("expected entrypoints");
        let selected_relative = selected
            .iter()
            .map(|path| {
                path.strip_prefix(&temp_dir)
                    .expect("expected package relative path")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect::<Vec<_>>();

        assert_eq!(selected_relative, vec!["src/index.ts"]);

        fs::remove_dir_all(&temp_dir).expect("failed to remove temp directory");
    }

    #[test]
    fn test_select_phase_entrypoints_includes_manifest_entry_targets_outside_candidates() {
        let temp_dir = unique_temp_dir("manifest-entry-target-candidate-expansion");
        fs::create_dir_all(&temp_dir).expect("failed to create temp directory");

        // create package manifest with a root main entrypoint
        write_text_file(
            &temp_dir.join("package.json"),
            r#"{
  "name": "root",
  "main": "./fastify.js"
}"#,
        );

        // create one source entrypoint and a filtered candidate set that excludes it
        write_text_file(&temp_dir.join("fastify.js"), "module.exports = {};");
        write_text_file(&temp_dir.join("src/index.ts"), "export const value = 1;");
        let candidates = vec![temp_dir.join("src/index.ts")];

        // select entrypoints and include package main when candidates excluded it
        let selected =
            select_phase_entrypoints(&temp_dir, &candidates).expect("expected entrypoints");
        let selected_relative = selected
            .iter()
            .map(|path| {
                path.strip_prefix(&temp_dir)
                    .expect("expected package relative path")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect::<Vec<_>>();

        assert_eq!(selected_relative, vec!["fastify.js"]);

        fs::remove_dir_all(&temp_dir).expect("failed to remove temp directory");
    }

    #[test]
    fn test_select_phase_entrypoints_errors_without_manifest_or_tsconfig_entries() {
        let temp_dir = unique_temp_dir("missing-entrypoints");
        fs::create_dir_all(&temp_dir).expect("failed to create temp directory");

        // create a package manifest without explicit entry targets
        write_text_file(
            &temp_dir.join("package.json"),
            r#"{
  "name": "root"
}"#,
        );

        // create one candidate source file
        let candidates = vec![temp_dir.join("src/index.ts")];
        write_text_file(&candidates[0], "export const value = 1;");

        // fail loudly when neither manifest nor tsconfig defines phase entrypoints
        let error = select_phase_entrypoints(&temp_dir, &candidates)
            .expect_err("expected entrypoint discovery failure");
        assert!(
            error.contains("no entrypoints discovered from package manifest entries"),
            "{error}",
        );

        fs::remove_dir_all(&temp_dir).expect("failed to remove temp directory");
    }

    #[test]
    fn test_select_phase_entrypoints_uses_configured_discovery_entrypoints() {
        let temp_dir = unique_temp_dir("configured-discovery-entrypoints");
        fs::create_dir_all(&temp_dir).expect("failed to create temp directory");

        // create one package manifest without runtime entry fields
        write_text_file(
            &temp_dir.join("package.json"),
            r#"{
  "name": "root"
}"#,
        );

        // create source candidates with one configured entrypoint
        let candidates = vec![
            temp_dir.join("src/index.ts"),
            temp_dir.join("src/server.ts"),
        ];
        for path in &candidates {
            write_text_file(path, "export const value = 1;");
        }

        // select configured discovery entrypoints in strict mode
        let entrypoints = vec!["src/server.ts".to_string()];
        let selected =
            select_phase_entrypoints_with_roots(&temp_dir, &candidates, &[], &entrypoints)
                .expect("expected configured entrypoint selection");
        let selected_relative = selected
            .iter()
            .map(|path| {
                path.strip_prefix(&temp_dir)
                    .expect("expected package relative path")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect::<Vec<_>>();

        assert_eq!(selected_relative, vec!["src/server.ts"]);

        fs::remove_dir_all(&temp_dir).expect("failed to remove temp directory");
    }

    #[test]
    fn test_select_phase_entrypoints_prefers_tsconfig_sources_over_declaration_targets() {
        let temp_dir = unique_temp_dir("manifest-declaration-entry-fallback");
        fs::create_dir_all(&temp_dir).expect("failed to create temp directory");

        // create package manifest with build output targets and one declaration entry
        write_text_file(
            &temp_dir.join("package.json"),
            r#"{
  "name": "root",
  "main": "./lib/index.js",
  "types": "./index.d.ts"
}"#,
        );

        // create declaration entry that points to missing build output
        write_text_file(
            &temp_dir.join("index.d.ts"),
            "export * from './lib';
",
        );

        // create tsconfig source entry fallback
        write_text_file(
            &temp_dir.join("tsconfig.json"),
            r#"{
  "files": ["src/index.ts"]
}"#,
        );

        // create source candidates in the package
        let candidates = vec![temp_dir.join("src/index.ts"), temp_dir.join("src/extra.ts")];
        for path in &candidates {
            write_text_file(path, "export const value = 1;");
        }

        // prefer tsconfig source entrypoints over declaration-only manifest targets
        let selected =
            select_phase_entrypoints(&temp_dir, &candidates).expect("expected tsconfig fallback");
        let selected_relative = selected
            .iter()
            .map(|path| {
                path.strip_prefix(&temp_dir)
                    .expect("expected package relative path")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect::<Vec<_>>();

        assert_eq!(selected_relative, vec!["src/index.ts"]);

        fs::remove_dir_all(&temp_dir).expect("failed to remove temp directory");
    }
    #[test]
    fn test_tsconfig_path_matches_pattern_with_extension_wildcards() {
        // match root and nested paths for extension scoped double star patterns
        assert!(tsconfig_path_matches_pattern("src/**/*.ts", "src/index.ts"));
        assert!(tsconfig_path_matches_pattern(
            "src/**/*.ts",
            "src/utils/math.ts"
        ));
        assert!(!tsconfig_path_matches_pattern(
            "src/**/*.ts",
            "src/index.js"
        ));
    }

    #[test]
    fn test_tsconfig_path_matches_pattern_with_wildcard_paths() {
        // match root and nested paths for wildcard path patterns without extension suffixes
        assert!(tsconfig_path_matches_pattern("src/**/*", "src/index.ts"));
        assert!(tsconfig_path_matches_pattern(
            "src/**/*",
            "src/utils/math.ts"
        ));
    }
}
