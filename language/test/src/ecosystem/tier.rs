use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use destack_compiler::{AnalyzeTask, Compiler, CompilerOptions, OptimizeTask, ResolveTask};
use destack_parser::{Parser, source_colorizer};
use destack_source::{
    Diagnostic, DiagnosticCollection, DiagnosticSeverity, File, FileRegistry, FileSystem, FileType,
    LanguageType, MemoryFileSystem, ModuleId, PhysicalFileSystem, PrintOptions, Uri,
};
use destack_workspace::{
    FormatterOptions, LinterOptions, PackageJson, Program, Session, select_manifest_entry_paths,
};

use crate::ecosystem::manifest::{
    CompilerOptionsConfig, EcosystemManifest, EcosystemPhase, EcosystemTscMode, EcosystemTscTool,
    ExpectedDiagnosticConfig,
};
use crate::harness::print::color;
use crate::harness::{TestResult, format_diagnostics};

/// Run one phase tier for one package workload.
pub(super) fn run_phase_tier(
    package_dir: &Path,
    manifest: &EcosystemManifest,
    phase: EcosystemPhase,
    files: &[PathBuf],
    tsc_mode: EcosystemTscMode,
    tsc_tool: EcosystemTscTool,
) -> TestResult {
    match phase {
        EcosystemPhase::Parse => run_parse_phase(package_dir, manifest, files),
        EcosystemPhase::Resolve | EcosystemPhase::Analyze | EcosystemPhase::Lower => {
            run_compiler_phase(package_dir, manifest, phase, files, tsc_mode, tsc_tool)
        }
    }
}

/// Run parser level checks across files.
fn run_parse_phase(
    package_dir: &Path,
    manifest: &EcosystemManifest,
    files: &[PathBuf],
) -> TestResult {
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

    if failures.is_empty() {
        return TestResult::Passed;
    }

    TestResult::Failed {
        message: format!(
            "{} files failed to parse with full diagnostics:\n\n{}",
            failures.len(),
            failure_messages.join("\n\n")
        ),
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
) -> TestResult {
    let entrypoints = select_phase_entrypoints(package_dir, files);
    if entrypoints.is_empty() {
        return TestResult::Failed {
            message: format!("no entrypoints selected for phase '{}'", phase.name(),),
        };
    }

    let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem);
    let session = Arc::new(Session::new(package_dir.to_path_buf()).with_fs(file_system));
    let program = session.add_root(package_dir.to_path_buf());

    let compiler = Compiler::new(
        session,
        program.clone(),
        CompilerOptions {
            workers: 1,
            ..Default::default()
        },
    );

    let mut module_ids = BTreeSet::new();
    for path in &entrypoints {
        let module_id = match compiler.resolve_path_to_module(path) {
            Ok(id) => id,
            Err(error) => {
                return TestResult::Failed {
                    message: format!("failed to resolve module {}: {error:?}", path.display()),
                };
            }
        };

        module_ids.insert(module_id);
    }

    if module_ids.is_empty() {
        return TestResult::Failed {
            message: "no modules resolved from selected entrypoints".to_string(),
        };
    }

    for module_id in module_ids.iter().copied() {
        enqueue_phase_task(&compiler, &program, module_id, phase);
    }

    compiler.compile();
    drop(compiler);

    let diagnostics = program.diagnostics.collect();
    let errors = diagnostics
        .iter()
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .collect::<Vec<_>>();

    let has_destack_errors = !errors.is_empty();

    // collect configured expectations for this phase
    let expected_phase_diagnostics = manifest
        .diagnostics_for_phase(phase)
        .cloned()
        .collect::<Vec<_>>();

    // run tsc only for configured languages, phases, and modes
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
        return TestResult::Passed;
    }

    // fail when expected diagnostics are missing from an otherwise clean run
    if !has_destack_errors {
        let mut message = format_expected_diagnostic_mismatch(
            phase,
            entrypoints.len(),
            files.len(),
            &expected_phase_diagnostics,
            &[],
        );

        append_tsc_context_maybe(&mut message, tsc_result.as_ref());

        return TestResult::Failed { message };
    }

    // match observed diagnostics against expectations when configured
    if !expected_phase_diagnostics.is_empty() {
        let observed_phase_diagnostics =
            collect_observed_phase_diagnostics(&program.files, package_dir, &errors);
        let outcome = match_expected_phase_diagnostics(
            &expected_phase_diagnostics,
            &observed_phase_diagnostics,
        );

        if outcome.missing_expected.is_empty() && outcome.unexpected_observed.is_empty() {
            return TestResult::Passed;
        }

        let mut message = format_expected_diagnostic_mismatch(
            phase,
            entrypoints.len(),
            files.len(),
            &outcome.missing_expected,
            &outcome.unexpected_observed,
        );

        let diagnostic_output =
            format_phase_error_diagnostics(&program, &errors, entrypoints.len());
        message.push_str("\n\n");
        message.push_str(diagnostic_output.trim_end());

        append_tsc_context_maybe(&mut message, tsc_result.as_ref());

        return TestResult::Failed { message };
    }

    // report full diagnostics when no expectation list is configured
    let diagnostic_output = format_phase_error_diagnostics(&program, &errors, entrypoints.len());

    let mut message = format!(
        "phase '{}' failed with {} errors across {} entrypoints and {} discovered files:\n\n{}",
        phase.name(),
        errors.len(),
        entrypoints.len(),
        files.len(),
        diagnostic_output.trim_end()
    );

    append_tsc_context_maybe(&mut message, tsc_result.as_ref());

    TestResult::Failed { message }
}

/// One observed compiler phase diagnostic used for expectation matching.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ObservedPhaseDiagnostic {
    /// The diagnostic code.
    code: String,
    /// The normalized file path fragment.
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
    files: &FileRegistry,
    package_dir: &Path,
    errors: &[Diagnostic],
) -> Vec<ObservedPhaseDiagnostic> {
    let mut observed = BTreeSet::new();

    // map diagnostics to stable code, file, and message triples
    for diagnostic in errors {
        let file = normalize_diagnostic_file(files, package_dir, diagnostic);
        observed.insert(ObservedPhaseDiagnostic {
            code: diagnostic.code.clone(),
            file,
            message: diagnostic.message.clone(),
        });
    }

    observed.into_iter().collect()
}

/// Normalize one diagnostic file path for stable path fragment matching.
fn normalize_diagnostic_file(
    files: &FileRegistry,
    package_dir: &Path,
    diagnostic: &Diagnostic,
) -> String {
    let file = files.get(diagnostic.file_id);

    // use package relative paths when available
    if let Some(path) = file.path.as_deref() {
        let relative = path.strip_prefix(package_dir).unwrap_or(path);
        return normalize_path_fragment(&relative.to_string_lossy());
    }

    // fall back to uri text when no path is attached
    normalize_path_fragment(&file.uri.to_string())
}

/// Normalize one path like fragment for cross-platform matching.
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
                && observed_diagnostic.file.contains(expected_file.as_str())
                && observed_diagnostic.message.contains(expected_message)
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
    program: &Arc<Program>,
    errors: &[Diagnostic],
    module_count: usize,
) -> String {
    let mut error_diagnostics = DiagnosticCollection::new();

    // render only error level diagnostics for deterministic phase output
    for diagnostic in errors.iter().cloned() {
        error_diagnostics.insert(diagnostic);
    }

    format_diagnostics(
        &program.files,
        &error_diagnostics,
        PrintOptions::new()
            .with_colorizer(source_colorizer())
            .with_module_count(module_count),
    )
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
    // skip packages and languages that do not opt into tsc execution
    if !manifest.tsc.enabled_for_language(manifest.package.language) {
        return None;
    }

    // skip phases outside the tsc phase allowlist
    if !manifest.tsc.includes_phase(phase) {
        return None;
    }

    let tsc_mode = tsc_mode_for_manifest(manifest, default_tsc_mode);

    // skip by mode when no tsc execution is requested
    if tsc_mode == EcosystemTscMode::Off {
        return None;
    }

    // run only on Destack failures when on-failure mode is selected
    if tsc_mode == EcosystemTscMode::OnFailure && !has_destack_errors {
        return None;
    }

    let tsc_tool = tsc_tool_for_manifest(manifest, default_tsc_tool);

    // resolve one available binary for the configured tool policy
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

    // build tsc arguments from selected entrypoints
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
            message.push_str("\n");
            message.push_str(&format!(
                "tsc: {}",
                color::dim(&format_tsc_command_for_display(tsc_run))
            ));

            if !tsc_run.output.is_empty() {
                message.push_str("\n");
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
            message.push_str("\n");
            message.push_str(&format!("tsc: {}", color::dim("not executed")));
            message.push_str("\n");
            message.push_str(&format!(" - {}", color::red(error)));
            message
        }
    }
}

/// Enqueue one compiler task that represents a phase boundary.
fn enqueue_phase_task(
    compiler: &Compiler,
    program: &Arc<Program>,
    module_id: ModuleId,
    phase: EcosystemPhase,
) {
    let module = compiler.module_stamp(module_id);

    match phase {
        EcosystemPhase::Parse => {}
        EcosystemPhase::Resolve => {
            let profile_id = program.default_profile_id_for_module(module_id);
            let profile = compiler.profile_stamp(profile_id);
            compiler.enqueue(ResolveTask::ResolveModule {
                module,
                profile,
                graph: compiler.module_graph_stamp(profile_id),
            });
        }
        EcosystemPhase::Analyze => {
            let profile_id = program.default_profile_id_for_module(module_id);
            let profile = compiler.profile_stamp(profile_id);
            compiler.enqueue(AnalyzeTask::AnalyzeModule { module, profile });
        }
        EcosystemPhase::Lower => {
            let target = program.ensure_target_for_module(module_id);
            let profile_id = program.profile_id_for_target_or_default(module_id, &target);
            let profile = compiler.profile_stamp(profile_id);
            compiler.enqueue(OptimizeTask::OptimizeModule {
                module,
                profile,
                target,
            });
        }
    }
}

/// Select phase entrypoints from package metadata, with deterministic fallback heuristics.
fn select_phase_entrypoints(package_dir: &Path, files: &[PathBuf]) -> Vec<PathBuf> {
    let mut candidates = files.to_vec();

    // filter declaration files for semantic and lowering phases
    candidates.retain(|path| !is_declaration_file(path.as_path()));

    candidates.sort_by_key(|path| entrypoint_sort_key(package_dir, path.as_path()));

    // prefer package.json entry fields when possible
    let manifest_entrypoints = select_manifest_entrypoints(package_dir, &candidates);
    let selected = if manifest_entrypoints.is_empty() {
        candidates
    } else {
        manifest_entrypoints
    };

    selected
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

/// Select candidate files that correspond to package.json entry fields.
fn select_manifest_entrypoints(package_dir: &Path, candidates: &[PathBuf]) -> Vec<PathBuf> {
    let entry_targets = load_package_entry_targets(package_dir);
    if entry_targets.is_empty() {
        return Vec::new();
    }
    select_manifest_entry_paths(package_dir, candidates, &entry_targets)
}

/// Load package entry target strings from package.json.
fn load_package_entry_targets(package_dir: &Path) -> Vec<String> {
    let package_json_path = package_dir.join("package.json");
    let Ok(content) = fs::read_to_string(&package_json_path) else {
        return Vec::new();
    };
    let Ok(package_json) = serde_json::from_str::<PackageJson>(&content) else {
        return Vec::new();
    };

    package_json.entry_targets()
}

/// Parse one source file and fail when parser diagnostics contain errors.
fn parse_file(path: &Path, manifest: &EcosystemManifest) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|error| format!("read error: {error}"))?;

    let file_type = FileType::from_path(path).unwrap_or(FileType::TypeScript);
    let language_type = language_type_for_parse(file_type, &manifest.compiler_options);

    let uri = Uri::from_path(path);
    let files = Arc::new(FileRegistry::new());
    let file_system: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::new());
    let cwd = path.parent().unwrap_or(Path::new(".")).to_path_buf();

    let program = Arc::new(Program::from_options(
        FormatterOptions::default(),
        LinterOptions::default(),
        cwd,
        file_system,
        files,
    ));

    let file_id = program.files.next_id();
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("input")
        .to_string();
    let file = File::from_text(
        file_id,
        name,
        uri,
        Some(path.to_path_buf()),
        file_type,
        content,
    );
    program.files.insert(file);
    let file = program.files.get(file_id);

    let mut parser = Parser::lex_file(file, language_type);
    let _ = parser.parse();
    program.diagnostics.merge_from(&parser.diagnostics);

    let errors = program
        .diagnostics
        .iter()
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
        &program.files,
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

    LanguageType::from(file_type)
}

#[cfg(test)]
mod tests {
    use super::{
        ExpectedDiagnosticConfig, ObservedPhaseDiagnostic, match_expected_phase_diagnostics,
        normalize_path_fragment,
    };
    use crate::ecosystem::manifest::EcosystemPhase;

    #[test]
    fn test_match_expected_phase_diagnostics_matches_path_and_message_fragments() {
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
}
