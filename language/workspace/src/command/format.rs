use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tspp_formatter::format_source;
use tspp_json::{JsonFormatOptions, format_json};
use tspp_parser::{colorize_source, source_colorizer};
use tspp_repository::{FormatterOptions, Repository, Revision, TraceView};
use tspp_serde::Reflect;
use tspp_source::{
    DiagnosticCollection, DiagnosticSeverity, File, FileId, FileSystem, FileType, IgnoreSet,
    PrintOptions, Uri, print_diagnostics,
};

use super::common::{
    CommandEnvVar, CommandInput, CommandOptions, CommandRevision, CommandTargetOverrides,
    ManifestOverride, impl_command_input_options,
};
use super::context::CommandContext;
use super::outcome::CommandOutcome;
use super::output::OutputBuffer;
use super::{CommandError, CommandResult};

/// File types that the formatter can process.
const FORMATTABLE_TYPES: &[FileType] = &[FileType::Tspp, FileType::TsppDeclaration];

/// Payload for format command output.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct FormatPayload {
    /// The number of files inspected.
    pub files: usize,
    /// The number of files that would change.
    pub changed: usize,
    /// Paths for files that changed or would change in check mode.
    #[serde(default)]
    pub changed_files: Vec<String>,
    /// The number of errors encountered.
    pub errors: usize,
    /// Paths for files that failed formatting.
    #[serde(default)]
    pub error_files: Vec<String>,
    /// Whether this was a check-only run.
    pub check: bool,
    /// Formatted output for eval mode.
    pub formatted: Option<String>,
}

/// Request to format source files or content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FormatInput {
    /// Revision selected for this format request.
    pub revision: CommandRevision,
    /// Input sources for the command.
    pub inputs: Vec<CommandInput>,
    /// Whether package.json should resolve inputs when none are provided.
    pub config_inputs: bool,
    /// Optional working directory for this command.
    pub cwd: Option<PathBuf>,
    /// Optional manifest path override.
    pub manifest: Option<PathBuf>,
    /// Optional target name override.
    pub target: Option<String>,
    /// Optional target overrides.
    pub target_overrides: Option<CommandTargetOverrides>,
    /// Optional profile name override.
    pub profile: Option<String>,
    /// Optional environment overrides.
    pub env: Vec<CommandEnvVar>,
    /// Optional manifest overrides.
    pub overrides: Vec<ManifestOverride>,
    /// Whether the command should watch for changes.
    pub watch: bool,
    /// Whether the command should skip writes.
    pub dry_run: bool,
    /// Trace detail returned for this command.
    pub trace: Option<TraceView>,
    /// Formatting source.
    pub source: FormatSource,
    /// Formatting mode.
    pub mode: FormatMode,
}

impl_command_input_options!(FormatInput {
    source: FormatSource::Files(Vec::new()),
    mode: FormatMode::Preview,
});

/// Formatting source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum FormatSource {
    /// Format files or directories.
    Files(Vec<PathBuf>),
    /// Format explicit text.
    Text {
        /// Input label.
        name: String,
        /// Input file type.
        file_type: FileType,
        /// Text to format.
        text: String,
    },
}

/// Formatting mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum FormatMode {
    /// Return formatted output without mutating source state.
    Preview,
    /// Return whether formatting would change source state.
    Check,
    /// Apply formatted output through the workspace.
    Write,
}

impl CommandContext<'_> {
    /// Execute a format command.
    pub(crate) fn run_format_command(
        &mut self,
        _root: &Path,
        source: &FormatSource,
        mode: FormatMode,
    ) -> CommandResult<CommandOutcome<FormatPayload>> {
        // resolve formatting inputs
        let check = matches!(mode, FormatMode::Check);
        let suppress_output = false;
        let mut command_diagnostics = DiagnosticCollection::new();
        let mut diagnostic_files = Vec::new();
        let revision = self.revision();

        // build formatter state
        let mut report = FormatReport::new(check);
        let default_formatting = workspace_formatting_options(&self.repository, revision)?;

        // format explicit text when provided
        if let FormatSource::Text {
            name,
            file_type,
            text,
        } = source
        {
            report.files_total = 1;
            let file_id = FileId::from_logical_str(name);
            let file = File::from_text(
                file_id,
                name.clone(),
                Uri::from_string(name),
                None,
                *file_type,
                text.clone(),
            )
            .map_err(|error| CommandError::invalid_input(error.to_string()))?;
            let file = Arc::new(file);
            let file_for_id = |current_file_id| {
                if current_file_id == file_id {
                    Some(file.clone())
                } else {
                    None
                }
            };

            let formatted = format_source(file.as_ref(), file.text(), default_formatting)
                .map_err(|error| CommandError::internal(error.to_string()))?;
            if !formatted.diagnostics.is_empty() {
                diagnostic_files.push(file.clone());
            }
            command_diagnostics.merge_from(&formatted.diagnostics);
            if check_and_collect_errors(
                &file_for_id,
                &formatted.diagnostics,
                suppress_output,
                self.output,
            ) {
                report.errors += 1;
                report.error_files.push("<eval>".to_string());
                let payload = report.payload();

                return Ok(CommandOutcome::new(command_diagnostics, 1, 0, 0, 0)
                    .with_files(diagnostic_files)
                    .with_data(payload));
            }

            if check && file.text() != formatted.text {
                report.files_changed = 1;
                report.changed_files.push(name.clone());
            }

            report.formatted_output = Some(formatted.text.clone());
            if !check {
                self.output
                    .push_stdout(colorize_formatted_output(&formatted.text)?.into_bytes());
            }
            let payload = report.payload();
            let exit_code = if check && report.files_changed > 0 {
                1
            } else {
                0
            };

            return Ok(CommandOutcome::new(command_diagnostics, exit_code, 0, 0, 0)
                .with_files(diagnostic_files)
                .with_data(payload));
        }

        // collect file paths to format
        let fs = self.repository.file_system().clone();
        let mut paths = Vec::new();
        match source {
            FormatSource::Files(files) if files.is_empty() => {
                paths = collect_formattable_files(
                    fs.as_ref(),
                    self.repository.path(),
                    self.repository.path(),
                )?;
            }
            FormatSource::Files(files) => {
                for path in files {
                    paths.push(path.clone());
                }
            }
            FormatSource::Text { .. } => {}
        }

        // format each path or directory
        let mut did_any_change = false;
        let mut seen_files = HashSet::new();
        for path in paths {
            let metadata = fs.metadata(&path).map_err(|error| {
                CommandError::source(format!("failed to inspect {}: {error}", path.display()))
            })?;
            if metadata.is_file {
                if !seen_files.insert(path.clone()) {
                    continue;
                }
                report.files_total += 1;
                let result = format_single_file(
                    fs.as_ref(),
                    &self.repository,
                    revision,
                    &path,
                    &mut command_diagnostics,
                    suppress_output,
                    mode,
                    self.output,
                    &mut diagnostic_files,
                )?;
                match result {
                    FormatResult::Unchanged => {}
                    FormatResult::Changed => {
                        did_any_change = true;
                        report.files_changed += 1;
                        report.changed_files.push(path.display().to_string());
                    }
                    FormatResult::Error => {
                        report.errors += 1;
                        report.error_files.push(path.display().to_string());
                    }
                }
                continue;
            }

            if metadata.is_directory {
                let files = collect_formattable_files(fs.as_ref(), &path, self.repository.path())?;
                for file_path in files {
                    if !seen_files.insert(file_path.clone()) {
                        continue;
                    }
                    report.files_total += 1;
                    let result = format_single_file(
                        fs.as_ref(),
                        &self.repository,
                        revision,
                        &file_path,
                        &mut command_diagnostics,
                        suppress_output,
                        mode,
                        self.output,
                        &mut diagnostic_files,
                    )?;
                    match result {
                        FormatResult::Unchanged => {}
                        FormatResult::Changed => {
                            did_any_change = true;
                            report.files_changed += 1;
                            report.changed_files.push(file_path.display().to_string());
                        }
                        FormatResult::Error => {
                            report.errors += 1;
                            report.error_files.push(file_path.display().to_string());
                        }
                    }
                }
                continue;
            }

            report.errors += 1;
            report.error_files.push(path.display().to_string());
            self.output
                .push_stderr(format!("invalid path: {}\n", path.display()).into_bytes());
        }

        let mut exit_code = if report.errors == 0 { 0 } else { 1 };
        if check && did_any_change {
            exit_code = 1;
        }

        let payload = report.payload();

        Ok(CommandOutcome::new(command_diagnostics, exit_code, 0, 0, 0)
            .with_files(diagnostic_files)
            .with_data(payload))
    }
}

/// Formatting work report.
struct FormatReport {
    /// The number of files inspected.
    files_total: usize,
    /// The number of files that would change.
    files_changed: usize,
    /// The list of changed file paths.
    changed_files: Vec<String>,
    /// The number of errors encountered.
    errors: usize,
    /// The list of file paths with formatting errors.
    error_files: Vec<String>,
    /// Whether this is a check-only run.
    check: bool,
    /// Formatted output for eval mode.
    formatted_output: Option<String>,
}

impl FormatReport {
    /// Create a new formatting report.
    fn new(check: bool) -> Self {
        Self {
            files_total: 0,
            files_changed: 0,
            errors: 0,
            changed_files: Vec::new(),
            error_files: Vec::new(),
            check,
            formatted_output: None,
        }
    }

    /// Build a command payload from this report.
    fn payload(&self) -> FormatPayload {
        FormatPayload {
            files: self.files_total,
            changed: self.files_changed,
            changed_files: self.changed_files.clone(),
            errors: self.errors,
            error_files: self.error_files.clone(),
            check: self.check,
            formatted: self.formatted_output.clone(),
        }
    }
}

/// Print diagnostics and return whether there were errors.
fn check_and_collect_errors(
    file_for_id: &impl Fn(FileId) -> Option<Arc<File>>,
    diagnostics: &DiagnosticCollection,
    suppress_output: bool,
    output: &mut OutputBuffer,
) -> bool {
    // collect diagnostics and check for errors
    let has_errors = diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error);
    if has_errors
        && !suppress_output
        && let Err(error) = print_diagnostics_to_output(file_for_id, diagnostics, output)
    {
        output.push_stderr(format!("failed to render diagnostics: {error}\n").into_bytes());
    }

    has_errors
}

/// Print diagnostics into the command output buffer.
fn print_diagnostics_to_output(
    file_for_id: &impl Fn(FileId) -> Option<Arc<File>>,
    diagnostics: &DiagnosticCollection,
    output: &mut OutputBuffer,
) -> CommandResult<()> {
    // collect diagnostic lines
    let lines = Arc::new(Mutex::new(Vec::new()));
    let writer_lines = Arc::clone(&lines);
    let line_writer = Arc::new(move |line: &str| {
        writer_lines.lock().push(format!("{line}\n").into_bytes());
    });

    // configure diagnostic printer
    let options = PrintOptions::new()
        .with_line_width(100)
        .with_colorizer(source_colorizer())
        .with_line_writer(line_writer);

    // render diagnostics into the line buffer
    print_diagnostics(file_for_id, diagnostics, options)
        .map_err(|error| CommandError::internal(error.to_string()))?;

    // flush rendered diagnostics into output
    let mut lines = lines.lock();
    for line in lines.drain(..) {
        output.push_stderr(line);
    }

    Ok(())
}

/// Collect all formattable files in a directory.
fn collect_formattable_files(
    fs: &dyn FileSystem,
    directory: &Path,
    ignore_root: &Path,
) -> CommandResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut ignore_set = IgnoreSet::new();
    ignore_set.load_root(fs, ignore_root).map_err(|error| {
        CommandError::source(format!(
            "failed to read ignore rules in {}: {error}",
            ignore_root.display()
        ))
    })?;

    collect_formattable_files_in_dir(fs, directory, &mut ignore_set, &mut files)?;
    files.sort();

    Ok(files)
}

/// Walk a directory to collect formattable files.
fn collect_formattable_files_in_dir(
    fs: &dyn FileSystem,
    directory: &Path,
    ignore_set: &mut IgnoreSet,
    files: &mut Vec<PathBuf>,
) -> CommandResult<()> {
    let mut entries = fs.read_dir(directory).map_err(|error| {
        CommandError::source(format!("failed to read {}: {error}", directory.display()))
    })?;
    entries.sort();
    ignore_set.load(fs, directory).map_err(|error| {
        CommandError::source(format!(
            "failed to read ignore rules in {}: {error}",
            directory.display()
        ))
    })?;

    for entry in entries {
        let metadata = fs.metadata(&entry).map_err(|error| {
            CommandError::source(format!("failed to inspect {}: {error}", entry.display()))
        })?;
        if ignore_set.is_ignored(&entry, metadata.is_directory) {
            continue;
        }
        if metadata.is_directory {
            collect_formattable_files_in_dir(fs, &entry, ignore_set, files)?;
            continue;
        }
        if !metadata.is_file {
            continue;
        }

        let Some(file_type) = FileType::from_path(&entry) else {
            continue;
        };
        if is_formattable_file_type(file_type) {
            files.push(entry);
        }
    }

    Ok(())
}

/// Get formatting options for a file, checking for package.json.
fn formatting_options_for_path(
    repository: &Repository,
    revision: Revision,
    path: &Path,
) -> CommandResult<FormatterOptions> {
    let package = repository
        .nearest_package(revision, path)
        .map_err(|error| CommandError::internal(format!("failed to resolve package: {error}")))?;
    if let Some(package) = package
        && let Some(config) = repository
            .manifest_for_package_id(revision, package.id)
            .map_err(|error| {
                CommandError::internal(format!("failed to load package formatter options: {error}"))
            })?
    {
        return Ok(config.formatter);
    }

    workspace_formatting_options(repository, revision)
}

/// Format a single file, dispatching by file type.
// keep formatter inputs explicit
#[allow(clippy::too_many_arguments)]
fn format_single_file(
    fs: &dyn FileSystem,
    repository: &Arc<Repository>,
    revision: Revision,
    path: &Path,
    command_diagnostics: &mut DiagnosticCollection,
    suppress_output: bool,
    mode: FormatMode,
    output: &mut OutputBuffer,
    diagnostic_files: &mut Vec<Arc<File>>,
) -> CommandResult<FormatResult> {
    // dispatch by file type
    let file_type =
        FileType::from_path(path).filter(|file_type| is_formattable_file_type(*file_type));
    let Some(file_type) = file_type else {
        if !suppress_output {
            output.push_stderr(
                format!(
                    "error formatting '{}': unsupported file type\n",
                    path.display()
                )
                .into_bytes(),
            );
        }
        return Ok(FormatResult::Error);
    };

    // get formatting options from package.json
    let formatting_options = formatting_options_for_path(repository.as_ref(), revision, path)?;

    // read file
    let content = match fs.read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            if !suppress_output {
                output.push_stderr(
                    format!("error reading '{}': {error}\n", path.display()).into_bytes(),
                );
            }
            return Ok(FormatResult::Error);
        }
    };

    let formatted = match file_type {
        FileType::Json => match format_json_content(&content, formatting_options) {
            Ok(f) => f,
            Err(e) => {
                if !suppress_output {
                    output.push_stderr(
                        format!("error parsing '{}': {e}\n", path.display()).into_bytes(),
                    );
                }
                return Ok(FormatResult::Error);
            }
        },
        _ => {
            let (name, uri) = Uri::from_path_with_name(path);
            let file_id = repository.file_id(path);
            let file = File::from_text(
                file_id,
                name,
                uri.clone(),
                Some(path.to_path_buf()),
                file_type,
                content.clone(),
            );
            let file = match file {
                Ok(file) => file,
                Err(error) => {
                    if !suppress_output {
                        output.push_stderr(
                            format!("error formatting '{}': {error}\n", path.display())
                                .into_bytes(),
                        );
                    }

                    return Ok(FormatResult::Error);
                }
            };
            let file = Arc::new(file);
            let file_for_id = |current_file_id| {
                if current_file_id == file_id {
                    Some(file.clone())
                } else {
                    None
                }
            };
            let formatted = match format_source(file.as_ref(), file.text(), formatting_options) {
                Ok(result) => result,
                Err(error) => {
                    if !suppress_output {
                        output.push_stderr(
                            format!("error formatting '{}': {error}\n", path.display())
                                .into_bytes(),
                        );
                    }
                    return Ok(FormatResult::Error);
                }
            };
            if !formatted.diagnostics.is_empty() {
                diagnostic_files.push(file.clone());
            }
            command_diagnostics.merge_from(&formatted.diagnostics);

            if check_and_collect_errors(
                &file_for_id,
                &formatted.diagnostics,
                suppress_output,
                output,
            ) {
                return Ok(FormatResult::Error);
            }

            formatted.text
        }
    };

    let formatted = if !formatted.is_empty() && !formatted.ends_with('\n') {
        format!("{formatted}\n")
    } else {
        formatted
    };

    if mode == FormatMode::Check {
        if content != formatted {
            if !suppress_output {
                output.push_stderr(format!("{}\n", path.display()).into_bytes());
            }
            return Ok(FormatResult::Changed);
        }

        return Ok(FormatResult::Unchanged);
    }

    if mode == FormatMode::Preview {
        if !suppress_output {
            output.push_stdout(colorize_formatted_output(&formatted)?.into_bytes());
        }

        return if content == formatted {
            Ok(FormatResult::Unchanged)
        } else {
            Ok(FormatResult::Changed)
        };
    }

    if content != formatted {
        if let Err(e) = fs.write(path, formatted.as_bytes()) {
            if !suppress_output {
                output
                    .push_stderr(format!("error writing '{}': {e}\n", path.display()).into_bytes());
            }
            return Ok(FormatResult::Error);
        }
        if !suppress_output {
            output.push_stdout(format!("{}\n", path.display()).into_bytes());
        }
        return Ok(FormatResult::Changed);
    }

    Ok(FormatResult::Unchanged)
}

/// Format JSON content.
fn format_json_content(content: &str, formatter: FormatterOptions) -> CommandResult<String> {
    let file_id = FileId::from_logical_str("<json>");
    let doc = tspp_json::parse(content, file_id).map_err(|e| e.to_string())?;
    let options: JsonFormatOptions = formatter.into();

    format_json(&doc, &options).map_err(|error| CommandError::internal(error.to_string()))
}

/// Render formatted output for eval mode.
fn colorize_formatted_output(formatted: &str) -> CommandResult<String> {
    let formatted_file = File::from_text(
        FileId::from_logical_str("<eval:formatted>"),
        "<eval>".to_string(),
        Uri::from_string("<eval>"),
        None,
        FileType::Tspp,
        formatted.to_string(),
    )
    .map_err(|error| CommandError::internal(error.to_string()))?;

    Ok(colorize_source(&formatted_file))
}

/// Result of formatting a single file.
enum FormatResult {
    /// File content unchanged.
    Unchanged,
    /// File content changed (formatted or would be formatted in check mode).
    Changed,
    /// Error occurred during formatting.
    Error,
}

/// Return workspace scoped formatting options for one revision.
fn workspace_formatting_options(
    repository: &Repository,
    revision: Revision,
) -> CommandResult<FormatterOptions> {
    let workspace_config = repository
        .manifest_for_workspace(revision)
        .map_err(|error| format!("failed to derive workspace options: {error}"))?;

    Ok(workspace_config
        .map(|options| options.formatter)
        .unwrap_or_default())
}

/// Return whether a file type should be formatted by default.
fn is_formattable_file_type(file_type: FileType) -> bool {
    FORMATTABLE_TYPES.contains(&file_type)
}
