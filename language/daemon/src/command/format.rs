use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_core::StringPool;
use destack_dir::NodeParentIndex;
use destack_formatter::{DestackFormatContext, DestackFormatOptions, statement_list};
use destack_json::{JsonFormatOptions, format_json};
use destack_parser::{Parser, colorize_source, source_colorizer};
use destack_source::{
    DiagnosticCollection, DiagnosticCollector, DiagnosticSeverity, File, FileId, FileSystem,
    FileType, LanguageType, PrintOptions, Uri, print_diagnostics,
};
use destack_workspace::{FormatterOptions, Repository, Revision};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use super::context::CommandContext;
use super::dispatch::{CommandOutcome, CommandOutputBuffer};
use super::{CommandResult, DaemonCommandError};

/// Options for the format command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommandFormatOptions {
    /// Files or directories to format.
    pub files: Vec<PathBuf>,
    /// Optional inline string to format.
    pub eval: Option<String>,
    /// Format check mode for the format command.
    pub check: bool,
}

/// Payload for format command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandFormatPayload {
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

impl CommandContext<'_> {
    /// Execute a format command.
    pub(super) fn run_format_command(
        &mut self,
        _root: &Path,
        options: &CommandFormatOptions,
    ) -> CommandResult<CommandOutcome> {
        // resolve formatting inputs
        let format_options = options.clone();
        let check = options.check;
        let suppress_output = false;
        let command_diagnostics = DiagnosticCollector::new();
        let revision = self.revision()?;

        // build formatter state
        let mut report = FormatReport::new(check);
        let default_formatting = workspace_formatting_options(&self.repository, revision)?;

        // format inline eval when provided
        if let Some(eval) = format_options.eval.as_ref() {
            report.files_total = 1;
            let file_id = FileId::from_logical_str("<eval>");
            let file = File::from_text(
                file_id,
                "<eval>".to_string(),
                Uri::from_string("<eval>"),
                None,
                FileType::Destack,
                eval.clone(),
            );
            let file = Arc::new(file);
            let file_for_id = |current_file_id| {
                if current_file_id == file_id {
                    Some(file.clone())
                } else {
                    None
                }
            };

            let strings = self.repository.string_pool().clone();
            let (formatted, diagnostics) = format_file(file.clone(), default_formatting, strings)?;
            command_diagnostics.merge_from(&diagnostics);
            if check_and_collect_errors(&file_for_id, &diagnostics, suppress_output, self.output) {
                report.errors += 1;
                report.error_files.push("<eval>".to_string());
                let diagnostics = command_diagnostics.collect();
                let data = format_payload(&report)?;
                return Ok(CommandOutcome::new(diagnostics, 1, 0, 0, 0).with_data(data));
            }

            report.formatted_output = Some(formatted.clone());
            self.output
                .push_stdout(colorize_formatted_output(&formatted).into_bytes());
            let diagnostics = command_diagnostics.collect();
            let data = format_payload(&report)?;
            return Ok(CommandOutcome::new(diagnostics, 0, 0, 0, 0).with_data(data));
        }

        // collect file paths to format
        let fs = self.repository.file_system().clone();
        let mut paths = Vec::new();
        if format_options.files.is_empty() {
            paths = collect_formattable_files(fs.as_ref(), self.repository.workspace_root());
        } else {
            for path in &format_options.files {
                paths.push(path.clone());
            }
        }

        // format each path or directory
        let mut did_any_change = false;
        let mut seen_files = HashSet::new();
        for path in paths {
            let metadata = fs.metadata(&path).ok();
            if matches!(metadata, Some(meta) if meta.is_file) {
                if !seen_files.insert(path.clone()) {
                    continue;
                }
                report.files_total += 1;
                let result = format_single_file(
                    fs.as_ref(),
                    &self.repository,
                    revision,
                    &path,
                    &command_diagnostics,
                    suppress_output,
                    check,
                    self.output,
                );
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

            if matches!(metadata, Some(meta) if meta.is_directory) {
                let files = collect_formattable_files(fs.as_ref(), &path);
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
                        &command_diagnostics,
                        suppress_output,
                        check,
                        self.output,
                    );
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

        let diagnostics = command_diagnostics.collect();
        let data = format_payload(&report)?;
        Ok(CommandOutcome::new(diagnostics, exit_code, 0, 0, 0).with_data(data))
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
}

/// Build a format payload from a report.
fn format_payload(report: &FormatReport) -> CommandResult<serde_json::Value> {
    let payload = CommandFormatPayload {
        files: report.files_total,
        changed: report.files_changed,
        changed_files: report.changed_files.clone(),
        errors: report.errors,
        error_files: report.error_files.clone(),
        check: report.check,
        formatted: report.formatted_output.clone(),
    };
    Ok(
        serde_json::to_value(payload)
            .map_err(|error| format!("invalid format payload: {error}"))?,
    )
}

/// Print diagnostics and return whether there were errors.
fn check_and_collect_errors(
    file_for_id: &impl Fn(FileId) -> Option<Arc<File>>,
    diagnostics: &DiagnosticCollector,
    suppress_output: bool,
    output: &mut CommandOutputBuffer,
) -> bool {
    // collect diagnostics and check for errors
    let diagnostics = diagnostics.collect();
    let has_errors = diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error);
    if has_errors
        && !suppress_output
        && let Err(error) = print_diagnostics_to_output(file_for_id, &diagnostics, output)
    {
        output.push_stderr(format!("failed to render diagnostics: {error}\n").into_bytes());
    }

    has_errors
}

/// Print diagnostics into the command output buffer.
fn print_diagnostics_to_output(
    file_for_id: &impl Fn(FileId) -> Option<Arc<File>>,
    diagnostics: &DiagnosticCollection,
    output: &mut CommandOutputBuffer,
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
        .map_err(|error| DaemonCommandError::internal(error.to_string()))?;

    // flush rendered diagnostics into output
    let mut lines = lines.lock();
    for line in lines.drain(..) {
        output.push_stderr(line);
    }

    Ok(())
}

/// Format a single file and return the formatted content.
fn format_file(
    file: Arc<File>,
    formatter: FormatterOptions,
    strings: Arc<StringPool>,
) -> CommandResult<(String, DiagnosticCollector)> {
    let language_type = LanguageType::try_from(file.ty).map_err(|_| {
        DaemonCommandError::internal(format!(
            "formatter received non-code file type: {:?}",
            file.ty
        ))
    })?;
    let mut parser = Parser::lex_file(file.clone(), language_type, strings);
    let expressions = parser.parse();
    let diagnostics = parser.diagnostics();
    let diagnostic_collector = DiagnosticCollector::new();
    for diagnostic in diagnostics.iter() {
        diagnostic_collector.insert(diagnostic.clone());
    }

    let side_span = parser.compute_side_span();
    let (tokens, side_tokens) = parser.take_tokens();
    let tokens = tokens
        .into_iter()
        .map(|token| token.with_file(file.id))
        .collect::<Vec<_>>();
    let side_tokens = side_tokens
        .into_iter()
        .map(|token| token.with_file(file.id))
        .collect::<Vec<_>>();
    let parents = NodeParentIndex::from_tree(&parser.tree);
    let format_options = DestackFormatOptions {
        language_type,
        ..formatter.into()
    };
    let context = DestackFormatContext::new(
        format_options,
        file.as_ref(),
        &parser.tree,
        &tokens,
        &side_tokens,
        &side_span,
        parser.strings.as_ref(),
        parents,
    );

    let mut result = if expressions.is_empty() {
        String::new()
    } else {
        let formatted = destack_fir::format!(context.clone(), [statement_list(&expressions)])
            .map_err(|error| DaemonCommandError::internal(error.to_string()))?;
        let printed = formatted
            .print()
            .map_err(|error| DaemonCommandError::internal(error.to_string()))?;
        printed.as_str().to_string()
    };

    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }

    Ok((result, diagnostic_collector))
}

/// Collect all formattable files in a directory.
fn collect_formattable_files(fs: &dyn FileSystem, directory: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_formattable_files_in_dir(fs, directory, &mut files);
    files.sort();
    files
}

/// Walk a directory to collect formattable files.
fn collect_formattable_files_in_dir(
    fs: &dyn FileSystem,
    directory: &Path,
    files: &mut Vec<PathBuf>,
) {
    let Ok(entries) = fs.read_dir(directory) else {
        return;
    };
    let mut entries = entries;
    entries.sort();

    for entry in entries {
        let Ok(metadata) = fs.metadata(&entry) else {
            continue;
        };
        if metadata.is_directory {
            if should_ignore_directory(&entry) {
                continue;
            }
            collect_formattable_files_in_dir(fs, &entry, files);
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
}

/// Get formatting options for a file, checking for destack.json.
fn formatting_options_for_path(
    repository: &Repository,
    revision: Revision,
    path: &Path,
) -> FormatterOptions {
    let package = repository.nearest_package(revision, path).ok().flatten();
    if let Some(package) = package
        && let Ok(Some(config)) = repository.destack_for_package_id(revision, package.id)
    {
        return config.formatter;
    }

    workspace_formatting_options(repository, revision).unwrap_or_default()
}

/// Format a single file, dispatching by file type.
// keep formatter inputs explicit
#[allow(clippy::too_many_arguments)]
fn format_single_file(
    fs: &dyn FileSystem,
    repository: &Arc<Repository>,
    revision: Revision,
    path: &Path,
    command_diagnostics: &DiagnosticCollector,
    suppress_output: bool,
    check: bool,
    output: &mut CommandOutputBuffer,
) -> FormatResult {
    // dispatch by file type
    let file_type = FileType::from_path(path).unwrap_or(FileType::Unknown);
    if !is_formattable_file_type(file_type) {
        if !suppress_output {
            output.push_stderr(
                format!(
                    "error formatting '{}': unsupported file type\n",
                    path.display()
                )
                .into_bytes(),
            );
        }
        return FormatResult::Error;
    }

    // get formatting options from destack.json
    let formatting_options = formatting_options_for_path(repository.as_ref(), revision, path);

    // read file
    let content = match fs.read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            if !suppress_output {
                output
                    .push_stderr(format!("error reading '{}': {e}\n", path.display()).into_bytes());
            }
            return FormatResult::Error;
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
                return FormatResult::Error;
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
            let file = Arc::new(file);
            let file_for_id = |current_file_id| {
                if current_file_id == file_id {
                    Some(file.clone())
                } else {
                    None
                }
            };
            let strings = repository.string_pool().clone();
            let (result, diagnostics) = match format_file(file.clone(), formatting_options, strings)
            {
                Ok(result) => result,
                Err(error) => {
                    if !suppress_output {
                        output.push_stderr(
                            format!("error formatting '{}': {error}\n", path.display())
                                .into_bytes(),
                        );
                    }
                    return FormatResult::Error;
                }
            };
            command_diagnostics.merge_from(&diagnostics);

            if check_and_collect_errors(&file_for_id, &diagnostics, suppress_output, output) {
                return FormatResult::Error;
            }

            result
        }
    };

    let formatted = if !formatted.is_empty() && !formatted.ends_with('\n') {
        format!("{formatted}\n")
    } else {
        formatted
    };

    if check {
        if content != formatted {
            if !suppress_output {
                output.push_stderr(format!("{}\n", path.display()).into_bytes());
            }
            return FormatResult::Changed;
        }
        return FormatResult::Unchanged;
    }

    if content != formatted {
        if let Err(e) = fs.write(path, formatted.as_bytes()) {
            if !suppress_output {
                output
                    .push_stderr(format!("error writing '{}': {e}\n", path.display()).into_bytes());
            }
            return FormatResult::Error;
        }
        if !suppress_output {
            output.push_stdout(format!("{}\n", path.display()).into_bytes());
        }
        return FormatResult::Changed;
    }

    FormatResult::Unchanged
}

/// Format JSON content.
fn format_json_content(content: &str, formatter: FormatterOptions) -> CommandResult<String> {
    let file_id = FileId::from_logical_str("<json>");
    let doc = destack_json::parse(content, file_id).map_err(|e| e.to_string())?;
    let options: JsonFormatOptions = formatter.into();

    Ok(format_json(&doc, &options))
}

/// Render formatted output for eval mode.
fn colorize_formatted_output(formatted: &str) -> String {
    let formatted_file = File::from_text(
        FileId::from_logical_str("<eval:formatted>"),
        "<eval>".to_string(),
        Uri::from_string("<eval>"),
        None,
        FileType::Destack,
        formatted.to_string(),
    );
    colorize_source(&formatted_file)
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
        .destack_for_workspace(revision)
        .map_err(|error| format!("failed to derive workspace options: {error}"))?;

    Ok(workspace_config
        .map(|options| options.formatter)
        .unwrap_or_default())
}

/// Return whether a file type should be formatted by default.
fn is_formattable_file_type(file_type: FileType) -> bool {
    FORMATTABLE_TYPES.contains(&file_type)
}

/// Return whether a directory should be skipped by formatter traversal.
fn should_ignore_directory(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    IGNORED_DIRECTORIES.contains(&name)
}

/// File types that the formatter can process.
const FORMATTABLE_TYPES: &[FileType] = &[
    FileType::Destack,
    FileType::DestackDeclaration,
    FileType::JavaScript,
    FileType::JavaScriptXml,
    FileType::TypeScript,
    FileType::TypeScriptXml,
    FileType::TypeScriptDeclaration,
];

/// Directory names skipped by formatter traversal.
const IGNORED_DIRECTORIES: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "target",
    "dist",
    "build",
    "coverage",
    ".next",
    ".nuxt",
    ".parcel-cache",
    ".turbo",
    ".cache",
];
