use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_ast::NodeParentIndex;
use destack_fir::format as fir_format;
use destack_formatter::{DestackFormatContext, DestackFormatOptions, statement_list};
use destack_json::{JsonFormatOptions, format_json, parse as parse_json};
use destack_parser::{Parser, colorize_source, source_colorizer};
use destack_resolver::{CachePolicy, ResolveOptions, Resolver};
use destack_source::{
    DiagnosticCollection, DiagnosticCollector, DiagnosticOptions, DiagnosticSeverity, File, FileId,
    FileStore, FileSystem, FileType, LanguageType, PrintOptions, Uri, print_diagnostics,
};
use destack_workspace::{FormatterOptions, Repository};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use super::context::CommandContext;
use super::dispatch::{CommandOutcome, CommandOutputBuffer};

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
    ) -> super::CommandResult<CommandOutcome> {
        // resolve formatting inputs
        let format_options = options.clone();
        let check = options.check;
        let suppress_output = false;
        let diagnostic_options = self.diagnostic_options.clone();
        let command_diagnostics = DiagnosticCollector::new();

        // build formatter state
        let mut summary = FmtSummary::new(check);
        let default_formatting = self.repository.formatter;
        let revision = self.revision()?;
        let workspace_options = self
            .daemon
            .repository
            .workspace_options(revision)
            .map_err(|error| format!("failed to derive workspace options: {error}"))?;
        let resolver = Resolver::from_repository(
            self.repository.as_ref(),
            ResolveOptions::default_for_workspace(
                self.repository.cwd.clone(),
                workspace_options.as_ref(),
            ),
        );

        // format inline eval when provided
        if let Some(eval) = format_options.eval.as_ref() {
            summary.files_total = 1;
            let file_id = FileId::new(0);
            let file = File::from_text(
                file_id,
                "<eval>".to_string(),
                Uri::from_string("<eval>"),
                None,
                FileType::Destack,
                eval.clone(),
            );
            let file = Arc::new(file);
            let files = FileStore::new();
            files.insert((*file).clone());

            let (formatted, diagnostics) = format_file(file.clone(), default_formatting);
            command_diagnostics.merge_from(&diagnostics);
            if check_and_collect_errors(
                &files,
                &diagnostics,
                &diagnostic_options,
                suppress_output,
                self.output,
            ) {
                summary.errors += 1;
                summary.error_files.push("<eval>".to_string());
                let diagnostics = command_diagnostics.collect().map(&diagnostic_options);
                let data = summary_payload(&summary)?;
                return Ok(CommandOutcome::new(diagnostics, 1, 0, 0, 0, None).with_data(data));
            }

            summary.formatted_output = Some(formatted.clone());
            self.output
                .push_stdout(colorize_formatted_output(&formatted).into_bytes());
            let diagnostics = command_diagnostics.collect().map(&diagnostic_options);
            let data = summary_payload(&summary)?;
            return Ok(CommandOutcome::new(diagnostics, 0, 0, 0, 0, None).with_data(data));
        }

        // collect file paths to format
        let fs = self.repository.file_system().clone();
        let mut paths = Vec::new();
        if format_options.files.is_empty() {
            paths = collect_formattable_files(fs.as_ref(), &self.repository.cwd);
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
                summary.files_total += 1;
                let result = format_single_file(
                    fs.as_ref(),
                    &resolver,
                    &self.repository,
                    &path,
                    &self.repository.cwd,
                    self.common.config_path.as_deref(),
                    default_formatting,
                    &command_diagnostics,
                    &diagnostic_options,
                    suppress_output,
                    check,
                    self.output,
                );
                match result {
                    FormatResult::Unchanged => {}
                    FormatResult::Changed => {
                        did_any_change = true;
                        summary.files_changed += 1;
                        summary.changed_files.push(path.display().to_string());
                    }
                    FormatResult::Error => {
                        summary.errors += 1;
                        summary.error_files.push(path.display().to_string());
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
                    summary.files_total += 1;
                    let result = format_single_file(
                        fs.as_ref(),
                        &resolver,
                        &self.repository,
                        &file_path,
                        &self.repository.cwd,
                        self.common.config_path.as_deref(),
                        default_formatting,
                        &command_diagnostics,
                        &diagnostic_options,
                        suppress_output,
                        check,
                        self.output,
                    );
                    match result {
                        FormatResult::Unchanged => {}
                        FormatResult::Changed => {
                            did_any_change = true;
                            summary.files_changed += 1;
                            summary.changed_files.push(file_path.display().to_string());
                        }
                        FormatResult::Error => {
                            summary.errors += 1;
                            summary.error_files.push(file_path.display().to_string());
                        }
                    }
                }
                continue;
            }

            summary.errors += 1;
            summary.error_files.push(path.display().to_string());
            self.output
                .push_stderr(format!("invalid path: {}\n", path.display()).into_bytes());
        }

        let mut exit_code = if summary.errors == 0 { 0 } else { 1 };
        if check && did_any_change {
            exit_code = 1;
        }

        let diagnostics = command_diagnostics.collect().map(&diagnostic_options);
        let data = summary_payload(&summary)?;
        Ok(CommandOutcome::new(diagnostics, exit_code, 0, 0, 0, None).with_data(data))
    }
}

/// Summary of formatting work for reports.
struct FmtSummary {
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

impl FmtSummary {
    /// Create a new formatting summary.
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

/// Build a summary payload for format responses.
fn summary_payload(summary: &FmtSummary) -> super::CommandResult<serde_json::Value> {
    let payload = CommandFormatPayload {
        files: summary.files_total,
        changed: summary.files_changed,
        changed_files: summary.changed_files.clone(),
        errors: summary.errors,
        error_files: summary.error_files.clone(),
        check: summary.check,
        formatted: summary.formatted_output.clone(),
    };
    Ok(
        serde_json::to_value(payload)
            .map_err(|error| format!("invalid format payload: {error}"))?,
    )
}

/// Print diagnostics and return whether there were errors.
fn check_and_collect_errors(
    files: &FileStore,
    diagnostics: &DiagnosticCollector,
    diagnostic_options: &DiagnosticOptions,
    suppress_output: bool,
    output: &mut CommandOutputBuffer,
) -> bool {
    // collect diagnostics and check for errors
    let diagnostics = diagnostics.collect().map(diagnostic_options);
    let has_errors = diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error);
    if has_errors && !suppress_output {
        print_diagnostics_to_output(files, &diagnostics, output);
    }
    has_errors
}

/// Print diagnostics into the command output buffer.
fn print_diagnostics_to_output(
    files: &FileStore,
    diagnostics: &DiagnosticCollection,
    output: &mut CommandOutputBuffer,
) {
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
    print_diagnostics(files, diagnostics, options);

    // flush rendered diagnostics into output
    let mut lines = lines.lock();
    for line in lines.drain(..) {
        output.push_stderr(line);
    }
}

/// Format a single file and return the formatted content.
fn format_file(file: Arc<File>, formatter: FormatterOptions) -> (String, DiagnosticCollector) {
    let language_type = LanguageType::from(file.ty);
    let mut parser = Parser::lex_file(file.clone(), language_type);
    let expressions = parser.parse();
    let diagnostics = parser.diagnostics.clone();

    let side_span = parser.compute_side_span();
    let (tokens, side_tokens) = parser.take_tokens();
    let strings = parser.strings.into_immutable();
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
        &strings,
        parents,
    );

    let mut result = if expressions.is_empty() {
        String::new()
    } else {
        let formatted = fir_format!(context.clone(), [statement_list(&expressions)]).unwrap();
        let printed = formatted.print().unwrap();
        printed.as_str().to_string()
    };

    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }

    (result, diagnostics)
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
fn get_formatting_options(
    fs: &dyn FileSystem,
    resolver: &Resolver,
    path: &Path,
    cwd: &Path,
    config_override: Option<&Path>,
    default: FormatterOptions,
) -> FormatterOptions {
    let Some(destack_config_path) = find_destack_config_json(fs, path, cwd, config_override) else {
        return default;
    };
    if let Some(options) = load_destack_config_formatting(resolver, &destack_config_path) {
        return merge_formatter_options(options, default);
    }
    default
}

/// Format a single file, dispatching by file type.
// keep formatter inputs explicit
#[allow(clippy::too_many_arguments)]
fn format_single_file(
    fs: &dyn FileSystem,
    resolver: &Resolver,
    repository: &Arc<Repository>,
    path: &Path,
    cwd: &Path,
    config_override: Option<&Path>,
    default_formatting: FormatterOptions,
    command_diagnostics: &DiagnosticCollector,
    diagnostic_options: &DiagnosticOptions,
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
    let formatting_options =
        get_formatting_options(fs, resolver, path, cwd, config_override, default_formatting);

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
            let file_id = repository.file_id_for_workspace_path(path);
            let file = File::from_text(
                file_id,
                name,
                uri.clone(),
                Some(path.to_path_buf()),
                file_type,
                content.clone(),
            );
            let file = Arc::new(file);
            let files = FileStore::new();
            files.insert((*file).clone());
            let (result, diagnostics) = format_file(file.clone(), formatting_options);
            command_diagnostics.merge_from(&diagnostics);

            if check_and_collect_errors(
                &files,
                &diagnostics,
                diagnostic_options,
                suppress_output,
                output,
            ) {
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
fn format_json_content(content: &str, formatter: FormatterOptions) -> super::CommandResult<String> {
    let file_id = FileId::new(0);
    let doc = parse_json(content, file_id).map_err(|e| e.to_string())?;
    let options: JsonFormatOptions = formatter.into();

    Ok(format_json(&doc, &options))
}

/// Render formatted output for eval mode.
fn colorize_formatted_output(formatted: &str) -> String {
    let formatted_file = File::from_text(
        FileId::new(0),
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

/// Find the nearest destack.json by walking up parent directories.
fn find_destack_config_json(
    fs: &dyn FileSystem,
    path: &Path,
    cwd: &Path,
    config_override: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(config_override) = config_override {
        let resolved = if config_override.is_absolute() {
            config_override.to_path_buf()
        } else {
            cwd.join(config_override)
        };
        let metadata = fs.metadata(&resolved).ok()?;
        return if metadata.is_directory {
            Some(resolved.join("destack.json"))
        } else {
            Some(resolved)
        };
    }

    let metadata = fs.metadata(path).ok();
    let is_file = matches!(metadata, Some(meta) if meta.is_file);
    let mut current = if is_file {
        path.parent().map(|p| p.to_path_buf())
    } else {
        Some(path.to_path_buf())
    };

    while let Some(dir) = current {
        let destack_config_path = dir.join("destack.json");
        if fs.exists(&destack_config_path).unwrap_or(false) {
            return Some(destack_config_path);
        }
        current = dir.parent().map(|p| p.to_path_buf());
    }

    None
}

/// Load formatting options from a destack.json file.
fn load_destack_config_formatting(
    resolver: &Resolver,
    config_path: &Path,
) -> Option<FormatterOptions> {
    let config = resolver
        .read_destack_config(config_path, CachePolicy::UseCache)
        .ok()?;
    Some(config.package_options().formatter)
}

/// Merge formatter options with CLI precedence.
///
/// Uses non-default CLI option values to override config-derived values.
fn merge_formatter_options(
    config_options: FormatterOptions,
    cli_and_default_options: FormatterOptions,
) -> FormatterOptions {
    let default_options = FormatterOptions::default();
    let mut merged_options = config_options;

    if cli_and_default_options.line_ending != default_options.line_ending {
        merged_options.line_ending = cli_and_default_options.line_ending;
    }
    if cli_and_default_options.indent_style != default_options.indent_style {
        merged_options.indent_style = cli_and_default_options.indent_style;
    }
    if cli_and_default_options.indent_width != default_options.indent_width {
        merged_options.indent_width = cli_and_default_options.indent_width;
    }
    if cli_and_default_options.line_width != default_options.line_width {
        merged_options.line_width = cli_and_default_options.line_width;
    }
    if cli_and_default_options.quote_style != default_options.quote_style {
        merged_options.quote_style = cli_and_default_options.quote_style;
    }
    if cli_and_default_options.trailing_comma != default_options.trailing_comma {
        merged_options.trailing_comma = cli_and_default_options.trailing_comma;
    }
    if cli_and_default_options.bracket_spacing != default_options.bracket_spacing {
        merged_options.bracket_spacing = cli_and_default_options.bracket_spacing;
    }
    if cli_and_default_options.arrow_parentheses != default_options.arrow_parentheses {
        merged_options.arrow_parentheses = cli_and_default_options.arrow_parentheses;
    }
    if cli_and_default_options.quote_property != default_options.quote_property {
        merged_options.quote_property = cli_and_default_options.quote_property;
    }
    if cli_and_default_options.bracket_same_line != default_options.bracket_same_line {
        merged_options.bracket_same_line = cli_and_default_options.bracket_same_line;
    }
    if cli_and_default_options.single_attribute_per_line
        != default_options.single_attribute_per_line
    {
        merged_options.single_attribute_per_line =
            cli_and_default_options.single_attribute_per_line;
    }
    if cli_and_default_options.organize_imports != default_options.organize_imports {
        merged_options.organize_imports = cli_and_default_options.organize_imports;
    }
    if cli_and_default_options.import_sort_order != default_options.import_sort_order {
        merged_options.import_sort_order = cli_and_default_options.import_sort_order;
    }

    merged_options
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
    FileType::Json,
];

/// Directory names skipped by formatter traversal.
const IGNORED_DIRECTORIES: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "node_modules",
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
