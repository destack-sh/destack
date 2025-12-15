use std::path::{Path, PathBuf};
use std::sync::Arc;

use clap::Args;
use destack_ast::NodeParentIndex;
use destack_fir::format as fir_format;
use destack_formatter::{DestackFormatContext, DestackFormatOptions};
use destack_parser::{Parser, colorize_source};
use destack_source::{
    DiagnosticOptions, DiagnosticSeverity, File, FileId, FileType, LanguageType, Uri, glob,
};
use destack_workspace::{FormatterOptions, Program};
use serde::Deserialize;

use crate::common::{DiagnosticArgs, ProgramArgs, print_diagnostics};
use crate::console;

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
    line_width: Option<u8>,
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

/// Print diagnostics and return whether there were errors.
fn check_and_print_errors(program: &Arc<Program>, diagnostic_options: &DiagnosticOptions) -> bool {
    let diagnostics = program.diagnostics.collect().map(diagnostic_options);
    let has_errors = diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error);
    if has_errors {
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
        line_ending: formatter.line_ending,
        indent_style: formatter.indent_style,
        indent_width: formatter.indent_width,
        line_width: formatter.line_width,
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
    let check = args.check;
    let diagnostic_options: DiagnosticOptions = args.diagnostics.clone().into();
    let program = args.program.setup();
    let default_formatting = program.formatter;

    // case 1: format inline string
    if let Some(ref string) = args.eval {
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
        if check_and_print_errors(&program, &diagnostic_options) {
            return 1;
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
        let mut had_errors = false;

        for path in &args.files {
            if path.is_file() {
                // format single file
                let result = format_single_file(
                    &program,
                    path,
                    default_formatting,
                    &diagnostic_options,
                    check,
                );
                match result {
                    FormatResult::Unchanged => {}
                    FormatResult::Changed => did_any_change = true,
                    FormatResult::Error => had_errors = true,
                }
            } else if path.is_dir() {
                // format all .ds files in directory
                let pattern = format!("{}/**/*.ds", path.display());
                let paths = glob(&pattern);
                for file_path in paths {
                    let result = format_single_file(
                        &program,
                        &file_path,
                        default_formatting,
                        &diagnostic_options,
                        check,
                    );
                    match result {
                        FormatResult::Unchanged => {}
                        FormatResult::Changed => did_any_change = true,
                        FormatResult::Error => had_errors = true,
                    }
                }
            } else {
                console::error(&format!("not found: '{}'", path.display()));
                had_errors = true;
            }
        }

        if had_errors {
            return 1;
        }
        if check && did_any_change {
            return 1;
        }
        return 0;
    }

    // case 3: format all .ds files in current directory
    let base_directory = program.cwd.clone();
    if !base_directory.exists() {
        console::error(&format!(
            "directory not found: '{}'",
            base_directory.display()
        ));
        return 1;
    }

    // find all .ds files in directory
    let pattern = format!("{}/**/*.ds", base_directory.display());
    let paths = glob(&pattern);
    if paths.is_empty() {
        console::info("no .ds files found");
        return 0;
    }

    // phase 1: parse all files and collect results, checking for errors
    let mut files: Vec<(PathBuf, String, String)> = Vec::new();
    for path in &paths {
        // get formatting options from dsconfig
        let formatting_options = get_formatting_options(path, default_formatting);

        // read file
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                console::error(&format!("error reading '{}': {e}", path.display()));
                return 1;
            }
        };
        let file_id = program.files.next_id();
        let (name, uri) = Uri::from_path_with_name(path);
        let file = File::from_text(
            file_id,
            name,
            uri.clone(),
            Some(path.clone()),
            FileType::Destack,
            content.clone(),
        );
        program.files.insert(file);

        // format file
        let file = program.files.get_by_uri(&uri).expect("file not found");
        let formatted = format_file(file.clone(), formatting_options, program.clone());
        files.push((path.clone(), content, formatted));
    }

    // check for any parse errors across all files
    if check_and_print_errors(&program, &diagnostic_options) {
        return 1;
    }

    // phase 2: now that we know there are no errors, write/check files
    let mut did_any_change = false;
    for (path, original_content, formatted_content) in files {
        if check {
            // error if the file changed
            if original_content != formatted_content {
                console::error(&format!("{}", path.display()));
                did_any_change = true;
            }
        } else if original_content != formatted_content {
            // write the changed file, bail on error
            if let Err(e) = std::fs::write(&path, &formatted_content) {
                console::error(&format!("error writing '{}': {e}", path.display()));
                return 1;
            }
            // print the changed file path
            else {
                console::info(&format!("'{}'", path.display()));
            }
        }
    }

    if check && did_any_change {
        return 1;
    }

    0
}

enum FormatResult {
    Unchanged,
    Changed,
    Error,
}

fn format_single_file(
    program: &Arc<Program>,
    path: &Path,
    default_formatting: FormatterOptions,
    diagnostic_options: &DiagnosticOptions,
    check: bool,
) -> FormatResult {
    // get formatting options from dsconfig
    let formatting_options = get_formatting_options(path, default_formatting);

    // read file
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            console::error(&format!("error reading '{}': {e}", path.display()));
            return FormatResult::Error;
        }
    };

    let file_id = program.files.next_id();
    let (name, uri) = Uri::from_path_with_name(path);
    let file = File::from_text(
        file_id,
        name,
        uri.clone(),
        Some(path.to_path_buf()),
        FileType::Destack,
        content.clone(),
    );
    program.files.insert(file);

    // format file
    let file = program.files.get_by_uri(&uri).expect("file not found");
    let formatted = format_file(file.clone(), formatting_options, program.clone());

    // check for parse errors
    if check_and_print_errors(program, diagnostic_options) {
        return FormatResult::Error;
    }

    if check {
        if content != formatted {
            console::error(&format!("{}", path.display()));
            return FormatResult::Changed;
        }
        return FormatResult::Unchanged;
    }

    if content != formatted {
        if let Err(e) = std::fs::write(path, &formatted) {
            console::error(&format!("error writing '{}': {e}", path.display()));
            return FormatResult::Error;
        }
        console::info(&format!("'{}'", path.display()));
        return FormatResult::Changed;
    }

    FormatResult::Unchanged
}
