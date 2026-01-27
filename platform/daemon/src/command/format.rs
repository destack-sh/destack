use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_ast::NodeParentIndex;
use destack_fir::format as fir_format;
use destack_formatter::{DestackFormatContext, DestackFormatOptions};
use destack_json::{JsonFormatOptions, format_json, parse as parse_json};
use destack_parser::{Parser, colorize_source, source_colorizer};
use destack_source::{
    DiagnosticCollection, DiagnosticOptions, DiagnosticSeverity, File, FileId, FileSystem,
    FileType, LanguageType, PrintOptions, Uri, print_diagnostics,
};
use destack_workspace::{FormatterOptions, Program};
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
    /// The number of errors encountered.
    pub errors: usize,
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
    ) -> Result<CommandOutcome, String> {
        // reset diagnostics before formatting
        self.reset_diagnostics();

        // resolve formatting inputs
        let format_options = options.clone();
        let check = options.check;
        let suppress_output = false;
        let diagnostic_options = self.diagnostic_options.clone();

        // build formatter state
        let mut summary = FmtSummary::new(check);
        let default_formatting = self.program.formatter;

        // format inline eval when provided
        if let Some(eval) = format_options.eval.as_ref() {
            summary.files_total = 1;
            let file_id = FileId::new(0);
            let file = Arc::new(File::from_text(
                file_id,
                "<eval>".to_string(),
                Uri::from_string("<eval>"),
                None,
                FileType::Destack,
                eval.clone(),
            ));

            let formatted = format_file(file.clone(), default_formatting, &self.program);
            if check_and_collect_errors(
                &self.program,
                &diagnostic_options,
                suppress_output,
                self.output,
            ) {
                summary.errors += 1;
                let diagnostics = self.collect_diagnostics();
                let data = summary_payload(&summary)?;
                return Ok(CommandOutcome::new(diagnostics, 1, 0, 0, 0, None).with_data(data));
            }

            summary.formatted_output = Some(formatted.clone());
            self.output
                .push_stdout(colorize_formatted_output(&formatted).into_bytes());
            let diagnostics = self.collect_diagnostics();
            let data = summary_payload(&summary)?;
            return Ok(CommandOutcome::new(diagnostics, 0, 0, 0, 0, None).with_data(data));
        }

        // collect file paths to format
        let fs = self.program.fs.clone();
        let mut paths = Vec::new();
        if format_options.files.is_empty() {
            paths = collect_formattable_files(fs.as_ref(), &self.program.cwd);
        } else {
            for path in &format_options.files {
                paths.push(path.clone());
            }
        }

        // format each path or directory
        let mut did_any_change = false;
        for path in paths {
            let metadata = fs.metadata(&path).ok();
            if matches!(metadata, Some(meta) if meta.is_file) {
                summary.files_total += 1;
                let result = format_single_file(
                    fs.as_ref(),
                    &self.program,
                    &path,
                    default_formatting,
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
                    }
                    FormatResult::Error => summary.errors += 1,
                }
                continue;
            }

            if matches!(metadata, Some(meta) if meta.is_directory) {
                let files = collect_formattable_files(fs.as_ref(), &path);
                for file_path in files {
                    summary.files_total += 1;
                    let result = format_single_file(
                        fs.as_ref(),
                        &self.program,
                        &file_path,
                        default_formatting,
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
                        }
                        FormatResult::Error => summary.errors += 1,
                    }
                }
                continue;
            }

            summary.errors += 1;
            self.output
                .push_stderr(format!("invalid path: {}\n", path.display()).into_bytes());
        }

        let mut exit_code = if summary.errors == 0 { 0 } else { 1 };
        if check && did_any_change {
            exit_code = 1;
        }

        let diagnostics = self.collect_diagnostics();
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
    /// The number of errors encountered.
    errors: usize,
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
            check,
            formatted_output: None,
        }
    }
}

/// Build a summary payload for format responses.
fn summary_payload(summary: &FmtSummary) -> Result<serde_json::Value, String> {
    let payload = CommandFormatPayload {
        files: summary.files_total,
        changed: summary.files_changed,
        errors: summary.errors,
        check: summary.check,
        formatted: summary.formatted_output.clone(),
    };
    serde_json::to_value(payload).map_err(|error| format!("invalid format payload: {error}"))
}

/// Print diagnostics and return whether there were errors.
fn check_and_collect_errors(
    program: &Arc<Program>,
    diagnostic_options: &DiagnosticOptions,
    suppress_output: bool,
    output: &mut CommandOutputBuffer,
) -> bool {
    // collect diagnostics and check for errors
    let diagnostics = program.diagnostics.collect().map(diagnostic_options);
    let has_errors = diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error);
    if has_errors && !suppress_output {
        print_diagnostics_to_output(program, &diagnostics, output);
    }
    has_errors
}

/// Print diagnostics into the command output buffer.
fn print_diagnostics_to_output(
    program: &Arc<Program>,
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
        .with_module_count(program.modules.len())
        .with_colorizer(source_colorizer())
        .with_line_writer(line_writer);

    // render diagnostics into the line buffer
    print_diagnostics(&program.files, diagnostics, options);

    // flush rendered diagnostics into output
    let mut lines = lines.lock();
    for line in lines.drain(..) {
        output.push_stderr(line);
    }
}

/// Format a single file and return the formatted content.
fn format_file(file: Arc<File>, formatter: FormatterOptions, program: &Arc<Program>) -> String {
    let language_type = LanguageType::from(file.ty);
    let mut parser = Parser::lex_file(file.clone(), language_type);
    let expressions = parser.parse();
    parser.finish();
    program.diagnostics.merge_from(&parser.diagnostics);

    let side_span = parser.compute_side_span();
    let strings = parser.strings.into_immutable();
    let parents = NodeParentIndex::from_tree(&parser.tree);
    let format_options = DestackFormatOptions {
        language_type,
        ..formatter.into()
    };
    let context = DestackFormatContext {
        options: format_options,
        file: file.as_ref(),
        tree: &parser.tree,
        source_map: &parser.tree.source_map,
        parents,
        tokens: &parser.tokens,
        side_tokens: &parser.side_tokens,
        side_span: &side_span,
        strings: &strings,
        current_argument_group_id: None,
    };

    let mut result = String::new();
    for (i, expr) in expressions.iter().enumerate() {
        let formatted = fir_format!(context.clone(), [expr]).unwrap();
        let printed = formatted.print().unwrap();
        result.push_str(printed.as_str());
        if i < expressions.len() - 1 {
            result.push('\n');
        }
    }

    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }

    result
}

/// Collect all formattable files in a directory.
fn collect_formattable_files(fs: &dyn FileSystem, directory: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_formattable_files_in_dir(fs, directory, &mut files);
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

    for entry in entries {
        let Ok(metadata) = fs.metadata(&entry) else {
            continue;
        };
        if metadata.is_directory {
            collect_formattable_files_in_dir(fs, &entry, files);
            continue;
        }
        if !metadata.is_file {
            continue;
        }

        let Some(file_type) = FileType::from_path(&entry) else {
            continue;
        };
        if FORMATTABLE_TYPES.contains(&file_type) {
            files.push(entry);
        }
    }
}

/// Get formatting options for a file, checking for dsconfig.json.
fn get_formatting_options(
    fs: &dyn FileSystem,
    path: &Path,
    default: FormatterOptions,
) -> FormatterOptions {
    if let Some(dsconfig_path) = find_dsconfig_json(fs, path)
        && let Some(options) = load_dsconfig_formatting(fs, &dsconfig_path)
    {
        return options;
    }
    default
}

/// Format a single file, dispatching by file type.
// keep formatter inputs explicit
#[allow(clippy::too_many_arguments)]
fn format_single_file(
    fs: &dyn FileSystem,
    program: &Arc<Program>,
    path: &Path,
    default_formatting: FormatterOptions,
    diagnostic_options: &DiagnosticOptions,
    suppress_output: bool,
    check: bool,
    output: &mut CommandOutputBuffer,
) -> FormatResult {
    // get formatting options from dsconfig
    let formatting_options = get_formatting_options(fs, path, default_formatting);

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

    // dispatch by file type
    let file_type = FileType::from_path(path).unwrap_or(FileType::Unknown);
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
            let file_id = program.files.next_id();
            let (name, uri) = Uri::from_path_with_name(path);
            let file = File::from_text(
                file_id,
                name,
                uri.clone(),
                Some(path.to_path_buf()),
                file_type,
                content.clone(),
            );
            program.files.insert(file);

            let file = program.files.get_by_uri(&uri).expect("file not found");
            let result = format_file(file.clone(), formatting_options, program);

            if check_and_collect_errors(program, diagnostic_options, suppress_output, output) {
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
            output.push_stdout(format!("'{}'\n", path.display()).into_bytes());
        }
        return FormatResult::Changed;
    }

    FormatResult::Unchanged
}

/// Format JSON content.
fn format_json_content(content: &str, formatter: FormatterOptions) -> Result<String, String> {
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

/// Find the nearest dsconfig.json by walking up parent directories.
fn find_dsconfig_json(fs: &dyn FileSystem, path: &Path) -> Option<PathBuf> {
    let metadata = fs.metadata(path).ok();
    let is_file = matches!(metadata, Some(meta) if meta.is_file);
    let mut current = if is_file {
        path.parent().map(|p| p.to_path_buf())
    } else {
        Some(path.to_path_buf())
    };

    while let Some(dir) = current {
        let dsconfig_path = dir.join("dsconfig.json");
        if fs.exists(&dsconfig_path).unwrap_or(false) {
            return Some(dsconfig_path);
        }
        current = dir.parent().map(|p| p.to_path_buf());
    }
    None
}

/// Formatting options from dsconfig.json.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DsConfigFormatting {
    /// Line ending style for formatted files.
    line_ending: Option<LineEndingJson>,
    /// Indentation style for formatted files.
    indent_style: Option<IndentStyleJson>,
    /// Indentation width for formatted files.
    indent_width: Option<u8>,
    /// Maximum line width for formatted files.
    line_width: Option<u16>,
}

/// Minimal dsconfig.json structure for formatting.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DsConfigJson {
    /// Formatting section from the config file.
    #[serde(default)]
    formatter: DsConfigFormatting,
}

/// Line ending style for JSON deserialization.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum LineEndingJson {
    #[serde(alias = "lf")]
    /// Line feed endings.
    LineFeed,
    #[serde(alias = "crlf")]
    /// Carriage return and line feed endings.
    CarriageReturnLineFeed,
    #[serde(alias = "cr")]
    /// Carriage return endings.
    CarriageReturn,
}

impl From<LineEndingJson> for destack_source::LineEnding {
    fn from(value: LineEndingJson) -> Self {
        match value {
            LineEndingJson::LineFeed => Self::LineFeed,
            LineEndingJson::CarriageReturnLineFeed => Self::CarriageReturnLineFeed,
            LineEndingJson::CarriageReturn => Self::CarriageReturn,
        }
    }
}

/// Indent style for JSON deserialization.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum IndentStyleJson {
    #[serde(alias = "tabs")]
    /// Tab indentation.
    Tab,
    #[serde(alias = "spaces")]
    /// Space indentation.
    Space,
}

impl From<IndentStyleJson> for destack_source::IndentStyle {
    fn from(value: IndentStyleJson) -> Self {
        match value {
            IndentStyleJson::Tab => Self::Tab,
            IndentStyleJson::Space => Self::Space,
        }
    }
}

/// Load formatting options from a dsconfig.json file.
fn load_dsconfig_formatting(fs: &dyn FileSystem, dsconfig_path: &Path) -> Option<FormatterOptions> {
    let content = fs.read_to_string(dsconfig_path).ok()?;
    let dsconfig: DsConfigJson = serde_json::from_str(&content).ok()?;
    let fmt = &dsconfig.formatter;

    let mut options = FormatterOptions::default();
    if let Some(line_ending) = fmt.line_ending {
        options.line_ending = line_ending.into();
    }
    if let Some(indent_style) = fmt.indent_style {
        options.indent_style = indent_style.into();
    }
    if let Some(indent_width) = fmt.indent_width {
        options.indent_width = indent_width;
    }
    if let Some(line_width) = fmt.line_width {
        options.line_width = line_width;
    }

    Some(options)
}

/// File types that the formatter can process.
/// File types that the formatter can process.
const FORMATTABLE_TYPES: &[FileType] = &[FileType::Destack, FileType::Json];
