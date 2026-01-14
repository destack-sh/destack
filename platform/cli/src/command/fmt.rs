use std::path::{Path, PathBuf};
use std::sync::Arc;

use clap::Args;
use destack_ast::NodeParentIndex;
use destack_fir::format as fir_format;
use destack_formatter::{DestackFormatContext, DestackFormatOptions};
use destack_json::{JsonFormatOptions, format_json, parse as parse_json};
use destack_parser::{Parser, colorize_source};
use destack_source::{
    DiagnosticOptions, DiagnosticSeverity, File, FileId, FileType, LanguageType, Uri, glob,
};
use destack_workspace::{FormatterOptions, Program};
use serde::Deserialize;
use serde_json::json;

use crate::common::{
    CommandReport, DiagnosticArgs, ProgramArgs, ReportArgs, ensure_no_watch_or_dev,
    print_diagnostics, print_report,
};
use crate::console;

/// File types that the formatter can process.
///
/// Add new file types here to enable formatting support.
const FORMATTABLE_TYPES: &[FileType] = &[FileType::Destack, FileType::Json];

#[derive(Args, Debug, Clone)]
pub struct FmtArgs {
    /// Input files or directories to format.
    #[arg(value_name = "FILES")]
    pub files: Vec<PathBuf>,

    /// Format inline string (output to stdout).
    #[arg(short = 'e', long = "eval")]
    pub eval: Option<String>,

    /// Check if files are formatted (exit 1 if not, don't write).
    #[arg(long)]
    pub check: bool,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// The diagnostic options.
    #[command(flatten)]
    pub diagnostics: DiagnosticArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Find the nearest dsconfig.json by walking up parent directories.
fn find_dsconfig_json(path: &Path) -> Option<PathBuf> {
    let mut current = if path.is_file() {
        path.parent().map(|p| p.to_path_buf())
    } else {
        Some(path.to_path_buf())
    };

    while let Some(dir) = current {
        let dsconfig_path = dir.join("dsconfig.json");
        if dsconfig_path.exists() {
            return Some(dsconfig_path);
        }
        current = dir.parent().map(|p| p.to_path_buf());
    }
    None
}

/// Formatting options from dsconfig.json.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DsConfigFormatting {
    line_ending: Option<LineEndingJson>,
    indent_style: Option<IndentStyleJson>,
    indent_width: Option<u8>,
    line_width: Option<u16>,
}

/// Minimal dsconfig.json structure for formatting.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DsConfigJson {
    #[serde(default)]
    formatter: DsConfigFormatting,
}

/// Line ending style for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum LineEndingJson {
    #[serde(alias = "lf")]
    LineFeed,
    #[serde(alias = "crlf")]
    CarriageReturnLineFeed,
    #[serde(alias = "cr")]
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
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum IndentStyleJson {
    #[serde(alias = "tabs")]
    Tab,
    #[serde(alias = "spaces")]
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
fn load_dsconfig_formatting(dsconfig_path: &Path) -> Option<FormatterOptions> {
    let content = std::fs::read_to_string(dsconfig_path).ok()?;
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

/// Get formatting options for a file, checking for dsconfig.json.
fn get_formatting_options(path: &Path, default: FormatterOptions) -> FormatterOptions {
    if let Some(dsconfig_path) = find_dsconfig_json(path)
        && let Some(options) = load_dsconfig_formatting(&dsconfig_path)
    {
        return options;
    }
    default
}

/// Collect all formattable files in a directory.
fn collect_formattable_files(directory: &Path) -> Vec<PathBuf> {
    FORMATTABLE_TYPES
        .iter()
        .filter_map(|ty| ty.glob())
        .flat_map(|pattern| {
            let full_pattern = format!("{}/{pattern}", directory.display());
            glob(&full_pattern)
        })
        .collect()
}

/// Format JSON content.
fn format_json_content(content: &str, formatter: FormatterOptions) -> Result<String, String> {
    let file_id = FileId::new(0);
    let doc = parse_json(content, file_id).map_err(|e| e.to_string())?;
    let options: JsonFormatOptions = formatter.into();

    Ok(format_json(&doc, &options))
}

/// Print diagnostics and return whether there were errors.
fn check_and_print_errors(
    program: &Arc<Program>,
    diagnostic_options: &DiagnosticOptions,
    suppress_output: bool,
) -> bool {
    let diagnostics = program.diagnostics.collect().map(diagnostic_options);
    let has_errors = diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error);
    if has_errors && !suppress_output {
        print_diagnostics(program, &diagnostics);
    }
    has_errors
}

/// Format a single file and return the formatted content.
fn format_file(file: Arc<File>, formatter: FormatterOptions, program: Arc<Program>) -> String {
    // parse file
    let language_type = LanguageType::from(file.ty);
    let mut parser = Parser::lex_file(file.clone(), language_type);
    let expressions = parser.parse();
    parser.finish();
    program.diagnostics.merge_from(&parser.diagnostics);

    // format context
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
    };

    // format
    let mut result = String::new();
    for (i, expr) in expressions.iter().enumerate() {
        let formatted = fir_format!(context.clone(), [expr]).unwrap();
        let printed = formatted.print().unwrap();
        result.push_str(printed.as_str());
        if i < expressions.len() - 1 {
            result.push('\n');
        }
    }

    // ensure trailing newline
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }

    result
}

/// Format source files.
pub fn run(args: &FmtArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("fmt", &args.program, &args.report) {
        return code;
    }

    let check = args.check;
    let mut summary = FmtSummary::new(check);
    let suppress_output = args.report.is_json();
    let diagnostic_options: DiagnosticOptions = args.diagnostics.clone().into();
    let session = args.program.setup();
    let default_formatting = session.formatter;

    // get the program from the session
    let program = session
        .programs
        .iter()
        .next()
        .map(|entry| entry.value().clone())
        .expect("session should have a program after setup");

    // case 1: format inline string
    if let Some(ref string) = args.eval {
        summary.files_total = 1;
        let file_id = FileId::new(0);
        let file = Arc::new(File::from_text(
            file_id,
            "<eval>".to_string(),
            Uri::from_string("<eval>"),
            None,
            FileType::Destack,
            string.clone(),
        ));

        // format and check for parse errors
        let formatted = format_file(file.clone(), program.formatter, program.clone());
        if check_and_print_errors(&program, &diagnostic_options, suppress_output) {
            summary.errors += 1;
            return finish_fmt(args, summary, 1);
        }

        if args.report.is_json() {
            summary.formatted_output = Some(formatted);
            return finish_fmt(args, summary, 0);
        }

        // create a file from the formatted output for colorization
        let formatted_file = File::from_text(
            FileId::new(0),
            "<eval>".to_string(),
            Uri::from_string("<eval>"),
            None,
            FileType::Destack,
            formatted,
        );
        console::print(&colorize_source(&formatted_file));
        return 0;
    }

    // case 2: format specific files
    if !args.files.is_empty() {
        let mut did_any_change = false;

        for path in &args.files {
            if path.is_file() {
                summary.files_total += 1;
                let result = format_single_file(
                    &program,
                    path,
                    default_formatting,
                    &diagnostic_options,
                    suppress_output,
                    check,
                );
                match result {
                    FormatResult::Unchanged => {}
                    FormatResult::Changed => {
                        did_any_change = true;
                        summary.files_changed += 1;
                    }
                    FormatResult::Error => summary.errors += 1,
                }
            } else if path.is_dir() {
                let paths = collect_formattable_files(path);
                for file_path in paths {
                    summary.files_total += 1;
                    let result = format_single_file(
                        &program,
                        &file_path,
                        default_formatting,
                        &diagnostic_options,
                        suppress_output,
                        check,
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
            } else {
                if !suppress_output {
                    console::error(&format!("not found: '{}'", path.display()));
                }
                summary.errors += 1;
            }
        }

        if summary.errors > 0 {
            return finish_fmt(args, summary, 1);
        }
        if check && did_any_change {
            return finish_fmt(args, summary, 1);
        }
        return finish_fmt(args, summary, 0);
    }

    // case 3: format all formattable files in current directory
    let base_directory = program.cwd.clone();
    if !base_directory.exists() {
        if !suppress_output {
            console::error(&format!(
                "directory not found: '{}'",
                base_directory.display()
            ));
        }
        summary.errors += 1;
        return finish_fmt(args, summary, 1);
    }

    // find all formattable files in directory
    let paths = collect_formattable_files(&base_directory);
    if paths.is_empty() {
        if !suppress_output {
            console::info("no formattable files found");
        }
        return finish_fmt(args, summary, 0);
    }
    summary.files_total = paths.len();

    // phase 1: parse all files and collect results, checking for errors
    let mut files: Vec<(PathBuf, String, String)> = Vec::new();
    for path in &paths {
        // get formatting options from dsconfig
        let formatting_options = get_formatting_options(path, default_formatting);

        // read file
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                if !suppress_output {
                    console::error(&format!("error reading '{}': {e}", path.display()));
                }
                summary.errors += 1;
                return finish_fmt(args, summary, 1);
            }
        };

        // dispatch by file type
        let file_type = FileType::from_path(path).unwrap_or(FileType::Unknown);
        let formatted = match file_type {
            FileType::Json => match format_json_content(&content, formatting_options) {
                Ok(f) => f,
                Err(e) => {
                    if !suppress_output {
                        console::error(&format!("error parsing '{}': {e}", path.display()));
                    }
                    summary.errors += 1;
                    return finish_fmt(args, summary, 1);
                }
            },
            _ => {
                // destack formatting
                let file_id = program.files.next_id();
                let (name, uri) = Uri::from_path_with_name(path);
                let file = File::from_text(
                    file_id,
                    name,
                    uri.clone(),
                    Some(path.clone()),
                    file_type,
                    content.clone(),
                );
                program.files.insert(file);

                let file = program.files.get_by_uri(&uri).expect("file not found");
                format_file(file.clone(), formatting_options, program.clone())
            }
        };

        // ensure trailing newline
        let formatted = if !formatted.is_empty() && !formatted.ends_with('\n') {
            format!("{formatted}\n")
        } else {
            formatted
        };

        files.push((path.clone(), content, formatted));
    }

    // check for any parse errors across all files (destack only)
    if check_and_print_errors(&program, &diagnostic_options, suppress_output) {
        summary.errors += 1;
        return finish_fmt(args, summary, 1);
    }

    // phase 2: now that we know there are no errors, write/check files
    let mut did_any_change = false;
    for (path, original_content, formatted_content) in files {
        if check {
            if original_content != formatted_content {
                if !suppress_output {
                    console::error(&format!("{}", path.display()));
                }
                did_any_change = true;
                summary.files_changed += 1;
            }
        } else if original_content != formatted_content {
            if let Err(e) = std::fs::write(&path, &formatted_content) {
                if !suppress_output {
                    console::error(&format!("error writing '{}': {e}", path.display()));
                }
                summary.errors += 1;
                return finish_fmt(args, summary, 1);
            } else {
                if !suppress_output {
                    console::info(&format!("'{}'", path.display()));
                }
                summary.files_changed += 1;
            }
        }
    }

    if check && did_any_change {
        return finish_fmt(args, summary, 1);
    }

    finish_fmt(args, summary, 0)
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

/// Finalize formatting with a report payload when requested.
fn finish_fmt(args: &FmtArgs, summary: FmtSummary, exit_code: i32) -> i32 {
    if args.report.is_json() {
        let mut report = if exit_code == 0 {
            CommandReport::success("fmt", 0)
        } else {
            CommandReport::failure("fmt", exit_code)
        };
        report.data = Some(json!({
            "files": summary.files_total,
            "changed": summary.files_changed,
            "errors": summary.errors,
            "check": summary.check,
            "formatted": summary.formatted_output,
        }));
        print_report(&report, args.report.format());
    }

    exit_code
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

/// Format a single file, dispatching by file type.
fn format_single_file(
    program: &Arc<Program>,
    path: &Path,
    default_formatting: FormatterOptions,
    diagnostic_options: &DiagnosticOptions,
    suppress_output: bool,
    check: bool,
) -> FormatResult {
    // get formatting options from dsconfig
    let formatting_options = get_formatting_options(path, default_formatting);

    // read file
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            if !suppress_output {
                console::error(&format!("error reading '{}': {e}", path.display()));
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
                    console::error(&format!("error parsing '{}': {e}", path.display()));
                }
                return FormatResult::Error;
            }
        },
        _ => {
            // destack formatting
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
            let result = format_file(file.clone(), formatting_options, program.clone());

            // check for parse errors
            if check_and_print_errors(program, diagnostic_options, suppress_output) {
                return FormatResult::Error;
            }

            result
        }
    };

    // ensure trailing newline
    let formatted = if !formatted.is_empty() && !formatted.ends_with('\n') {
        format!("{formatted}\n")
    } else {
        formatted
    };

    // check or write
    if check {
        if content != formatted {
            if !suppress_output {
                console::error(&format!("{}", path.display()));
            }
            return FormatResult::Changed;
        }
        return FormatResult::Unchanged;
    }

    if content != formatted {
        if let Err(e) = std::fs::write(path, &formatted) {
            if !suppress_output {
                console::error(&format!("error writing '{}': {e}", path.display()));
            }
            return FormatResult::Error;
        }
        if !suppress_output {
            console::info(&format!("'{}'", path.display()));
        }
        return FormatResult::Changed;
    }

    FormatResult::Unchanged
}
