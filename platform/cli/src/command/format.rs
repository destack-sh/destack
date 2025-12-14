use std::path::{Path, PathBuf};
use std::sync::Arc;

use clap::Args;
use destack_ast::NodeParentIndex;
use destack_fir::format as fir_format;
use destack_formatter::{DestackFormatContext, DestackFormatOptions};
use destack_parser::{Parser, colorize_source};
use destack_source::{
    DiagnosticOptions, DiagnosticSeverity, File, FileId, FileType, IndentStyle, LineEnding, Uri,
    glob,
};
use destack_workspace::{FormatterOptions, LanguageOptions, Program};
use serde::Deserialize;

use crate::command::{
    DiagnosticArgs, ProgramArgs, SourceArg, get_string_or_file, print_diagnostics,
};
use crate::console;

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

impl From<LineEndingJson> for LineEnding {
    fn from(value: LineEndingJson) -> Self {
        match value {
            LineEndingJson::LineFeed => LineEnding::LineFeed,
            LineEndingJson::CarriageReturnLineFeed => LineEnding::CarriageReturnLineFeed,
            LineEndingJson::CarriageReturn => LineEnding::CarriageReturn,
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

impl From<IndentStyleJson> for IndentStyle {
    fn from(value: IndentStyleJson) -> Self {
        match value {
            IndentStyleJson::Tab => IndentStyle::Tab,
            IndentStyleJson::Space => IndentStyle::Space,
        }
    }
}

#[derive(Args, Debug, Clone)]
pub struct FormatArgs {
    /// Directory to format (default: current directory).
    #[arg(value_name = "DIR")]
    pub directory: Option<String>,

    /// Format a specific file.
    #[arg(long)]
    pub file: Option<String>,

    /// Format inline string (output to stdout).
    #[arg(long)]
    pub string: Option<String>,

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
fn format_file(file: Arc<File>, language: LanguageOptions, program: Arc<Program>) -> String {
    // parse file
    let mut parser = Parser::lex_file(file.clone(), language.ty);
    let expressions = parser.parse();
    parser.finish();
    program.diagnostics.merge_from(&parser.diagnostics);

    // format context
    let side_span = parser.compute_side_span();
    let strings = parser.strings.into_immutable();
    let parents = NodeParentIndex::from_tree(&parser.tree);
    let format_options = DestackFormatOptions {
        language_type: language.ty,
        line_ending: language.formatting.line_ending,
        indent_style: language.formatting.indent_style,
        indent_width: language.formatting.indent_width,
        line_width: language.formatting.line_width,
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
pub fn run(args: &FormatArgs) -> i32 {
    let check = args.check;
    let diagnostic_options: DiagnosticOptions = args.diagnostics.clone().into();
    let program = args.program.setup();
    let default_formatting = program.language.formatting;

    // case 1: format inline string
    if let Some(ref string) = args.string {
        let file_id = FileId::new(0);
        let file = Arc::new(File::from_text(
            file_id,
            "<string>".to_string(),
            Uri::from_string("<string>"),
            None,
            FileType::Destack,
            string.clone(),
        ));

        // format and check for parse errors
        let formatted = format_file(file.clone(), program.language, program.clone());
        if check_and_print_errors(&program, &diagnostic_options) {
            return 1;
        }

        // create a file from the formatted output for colorization
        let formatted_file = File::from_text(
            FileId::new(0),
            "<string>".to_string(),
            Uri::from_string("<string>"),
            None,
            FileType::Destack,
            formatted,
        );
        console::print(&colorize_source(&formatted_file));
        return 0;
    }

    // case 2: format single file
    if let Some(ref file_path) = args.file {
        let path = PathBuf::from(file_path);
        if !path.exists() {
            console::error(&std::format!("file not found: '{file_path}'"));
            return 1;
        }

        // get formatting options from dsconfig
        let formatting_options = get_formatting_options(&path, default_formatting);
        let language = program.language.with_formatting(formatting_options);

        // read and parse file
        let file = match get_string_or_file(
            &program,
            SourceArg {
                file: Some(file_path),
                string: None,
                format: None,
            },
        ) {
            Ok(file) => file,
            Err(e) => {
                console::error(&std::format!("error: {e}"));
                return 1;
            }
        };

        // format file
        let formatted = format_file(file.clone(), language, program.clone());
        if check_and_print_errors(&program, &diagnostic_options) {
            return 1;
        }

        // check if file changed
        if check {
            let original = file.text();
            if original != formatted {
                // error if the file changed
                console::error(&file_path.to_string());
                return 1;
            }
            return 0;
        }

        // write back to file
        if let Err(e) = std::fs::write(&path, &formatted) {
            console::error(&std::format!("error writing '{file_path}': {e}"));
            return 1;
        }
        return 0;
    }

    // case 3: format all .ds and .d.ds files in directory
    let base_directory = if let Some(ref directory) = args.directory {
        let path = PathBuf::from(directory);
        if path.is_absolute() {
            path
        } else {
            program.cwd.join(path)
        }
    } else {
        program.cwd.clone()
    };
    if !base_directory.exists() {
        console::error(&std::format!(
            "directory not found: '{}'",
            base_directory.display()
        ));
        return 1;
    }

    // find all .ds files in directory
    let pattern = std::format!("{}/**/*.ds", base_directory.display());
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
        let language = program.language.with_formatting(formatting_options);

        // read file
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                console::error(&std::format!("error reading '{}': {e}", path.display()));
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
        let formatted = format_file(file.clone(), language, program.clone());
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
                console::error(&std::format!("{}", path.display()));
                did_any_change = true;
            } else {
                // nothing to do
            }
        } else if original_content != formatted_content {
            // write the changed file, bail on error
            if let Err(e) = std::fs::write(&path, &formatted_content) {
                console::error(&std::format!("error writing '{}': {e}", path.display()));
                return 1;
            }
            // print the changed file path
            else {
                console::info(&std::format!("'{}'", path.display()));
            }
        }
    }

    if check && did_any_change {
        return 1;
    }

    0
}
