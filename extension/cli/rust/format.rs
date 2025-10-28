//! Format subcommand for Dyst source code.

use std::path::{Path, PathBuf};
use std::{env, fs};

use destack_file::walk::{WalkOptions, walk};
use destack_terminal::{CommandArguments, console};
use dyst_ast::{DefinitionMeta, ModuleFormat, Name, NodeParentIndex, TokenType};
use dyst_diagnostic::Severity;
use dyst_fir::format::{
    FormatError as FirFormatError, IndentStyle, LineEnding, PrintError as FirPrintError,
    format as format_fir,
};
use dyst_fir::format_args;
use dyst_formatter::{DystFormatContext, DystFormatOptions};
use dyst_parser::Parser;
use dyst_session::Session;
use dyst_source::{AnnotateOptions, Color, Source, SourceFormat, SourceId, Uri, annotate_source};

use crate::source::{render_semantic_spans, semantic_spans_from_text};

pub const HELP: &str = r"Format Dyst source code.
	--line-width <n>     Set maximum line width (default 100)
	--indent-style <s>   Choose indent style: space or tab
	--indent-width <n>   Set spaces per indent (default 4)
	--line-ending <e>    Choose line ending: lf, crlf, cr
	--dry-run            Preview formatting without writing files
    --format <f>         Choose format: ds, dst, dsb, dsx
	<path>               Format the provided file (omit to format all .ds files)";

const DEFAULT_IGNORE_PATHS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".venv",
    "venv",
    "env",
    "ENV",
    "env.bak",
    "venv.bak",
    "dist",
    "build",
    ".idea",
    ".vscode",
    ".vscode-test",
    "out",
    ".next",
    ".nuxt",
    "coverage",
    "htmlcov",
    ".cache",
    ".parcel-cache",
    ".pytest_cache",
    ".mypy_cache",
    ".tox",
    ".nox",
    ".hypothesis",
    ".yarn",
    ".temp",
    "hfuzz_target",
    "fuzz",
];

/// Outcome of formatting some source.
#[derive(Debug)]
struct FormattedSource {
    formatted: String,
    session: Session,
}

/// Result of processing a single file.
#[derive(Debug)]
struct FormattedFlags {
    did_change: bool,
    did_error: bool,
}

#[derive(Debug)]
enum FormatSourceError {
    Diagnostics(Session),
    Formatter(FirFormatError),
    Printer(FirPrintError),
}

/// Run the format command using parsed CLI arguments.
pub fn run(ctx: CommandArguments) -> i32 {
    let dry_run = ctx.flag("dry-run");

    // parse formatting options
    let options = match parse_options(&ctx) {
        Ok(options) => options,
        Err(error) => {
            console::error(&format!("Option error: {error}"));
            return 1;
        }
    };

    // handle inline string formatting first
    if let Some(body) = ctx.option("string") {
        return run_for_string(body, &options);
    }

    // collect explicit file arguments from --file and positionals
    let mut file_arguments: Vec<PathBuf> = Vec::new();
    if let Some(file_flag) = ctx.option("file") {
        file_arguments.push(PathBuf::from(file_flag));
    }
    if !ctx.positionals.is_empty() {
        file_arguments.extend(ctx.positionals.iter().map(PathBuf::from));
    }

    // route to appropriate handler based on file arguments
    if !file_arguments.is_empty() {
        return run_for_files(&file_arguments, &options, dry_run);
    }

    run_for_all_files(&options, dry_run)
}

/// Format all Dyst files in the current directory tree.
fn run_for_all_files(options: &DystFormatOptions, dry_run: bool) -> i32 {
    // get current working directory
    let root = match env::current_dir() {
        Ok(dir) => dir,
        Err(error) => {
            console::error(&format!("failed to read current directory: {error}"));
            return 1;
        }
    };
    let files = collect_dyst_files(&root);

    // process each file and track overall success
    let mut exit_code = 0;
    for path in &files {
        if dry_run {
            console::write_line(&canonical_display(path));
        }

        let flags = format_file(path, options, dry_run, false);
        if !dry_run && flags.did_change {
            console::write_line(&canonical_display(path));
        }
        if flags.did_error {
            exit_code = 1;
        }
    }

    exit_code
}

/// Format the provided list of files.
fn run_for_files(paths: &[PathBuf], options: &DystFormatOptions, dry_run: bool) -> i32 {
    let mut exit_code = 0;

    // process each file and track overall success
    for path in paths {
        let flags = format_file(path, options, dry_run, true);
        if flags.did_error {
            exit_code = 1;
        }
    }
    exit_code
}

/// Format inline source provided via --string.
fn run_for_string(string: &str, options: &DystFormatOptions) -> i32 {
    // create source from string input
    let source = Source::from_string(
        SourceId::new(0),
        "<string>".to_string(),
        Uri::from_string("<string>"),
        options.format,
        string.to_string(),
    );

    // format and output result
    match format_source(&source, options) {
        Ok(FormattedSource { formatted, session }) => {
            print_formatted_output("<formatted>", &formatted);
            print_diagnostics(&source, &session);
            if session.has_diagnostics_of_severity(Severity::Error) {
                1
            } else {
                0
            }
        }
        Err(error) => {
            print_format_source_error("<string>", &source, error);
            1
        }
    }
}

/// Collect all .ds and .d.ds files under the given root using the shared glob walker.
fn collect_dyst_files(root: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();

    // configure walker to find .ds files while ignoring common directories
    let walk_options = WalkOptions {
        root: root.to_path_buf(),
        ignore: Some(DEFAULT_IGNORE_PATHS.iter().map(|s| s.to_string()).collect()),
        glob: Some(vec!["**/*.ds".to_string(), "**/*.d.ds".to_string()]),
    };

    // collect all matching files
    walk(&walk_options, |path| files.push(path.to_path_buf()));
    files.sort();
    files
}

/// Format a single file, optionally writing it back to disk.
fn format_file(
    path: &Path,
    options: &DystFormatOptions,
    dry_run: bool,
    emit_output: bool,
) -> FormattedFlags {
    let path_buf = path.to_path_buf();
    let display_path = canonical_display(&path_buf);
    let format = path
        .extension()
        .and_then(|ext| ext.to_str())
        .and_then(SourceFormat::from_extension)
        .unwrap_or(options.format);

    // read original file content
    let original_text = match fs::read_to_string(&path_buf) {
        Ok(content) => content,
        Err(error) => {
            console::error(&format!("failed to read {display_path}: {error}"));
            return FormattedFlags {
                did_change: false,
                did_error: true,
            };
        }
    };

    // create source and format it
    let name = path_buf
        .iter()
        .next_back()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "<file>".to_string());
    let uri = Uri::from(&path_buf);
    let source = Source::from_string(SourceId::new(0), name, uri, format, original_text.clone());
    let formatted_source = match format_source(&source, options) {
        Ok(result) => result,
        Err(error) => {
            print_format_source_error(&display_path, &source, error);
            return FormattedFlags {
                did_change: false,
                did_error: true,
            };
        }
    };

    // print
    let FormattedSource { formatted, session } = formatted_source;
    print_diagnostics(&source, &session);
    if emit_output {
        print_formatted_output(&display_path, &formatted);
    }

    // bail on errors
    let did_change = formatted != original_text;
    let did_error = session.has_diagnostics_of_severity(Severity::Error);
    if !dry_run
        && did_change
        && let Err(error) = fs::write(&path_buf, &formatted)
    {
        console::error(&format!("failed to write {display_path}: {error}"));
        return FormattedFlags {
            did_change: false,
            did_error: true,
        };
    }

    FormattedFlags {
        did_change,
        did_error,
    }
}

/// Format the provided source into a string along with diagnostics.
fn format_source(
    source: &Source,
    options: &DystFormatOptions,
) -> Result<FormattedSource, FormatSourceError> {
    let mut session = Session::new();
    let module_name = source.uri.last_segment().unwrap_or("<string>");

    // parse the source into an AST
    let (module_id, tokens, side_tokens, side_span, tree, strings) = {
        let mut parser = Parser::prepare(source, &mut session);
        let module_name_id = parser.intern_string(module_name);
        let module_id = parser.with_recovery(
            parser.mark(),
            |parser| {
                parser
                    .eat_module_body(
                        DefinitionMeta::new(Name::Identifier(module_name_id)),
                        ModuleFormat::Source,
                    )
                    .map(Some)
            },
            None,
            TokenType::End,
        );
        parser.finalize();

        let side_span = parser.get_side_span();
        let tokens = parser.tokens;
        let side_tokens = parser.side_tokens;
        let tree = parser.tree;
        let strings = parser.strings;

        (module_id, tokens, side_tokens, side_span, tree, strings)
    };

    // bail on errors
    if session.has_diagnostics_of_severity(Severity::Error) {
        return Err(FormatSourceError::Diagnostics(session));
    }
    let Some(definition_id) = module_id else {
        return Err(FormatSourceError::Diagnostics(session));
    };

    // format the AST
    let parents = NodeParentIndex::from_tree(&tree);
    let context = DystFormatContext {
        options: options.clone(),
        source,
        tree: &tree,
        tokens: &tokens,
        side_tokens: &side_tokens,
        side_span: &side_span,
        spans: &tree.spans,
        parents,
        session: &session,
        strings: &strings,
    };
    let printed = {
        let formatted = format_fir(context, format_args![definition_id])
            .map_err(FormatSourceError::Formatter)?;
        formatted.print().map_err(FormatSourceError::Printer)?
    };

    Ok(FormattedSource {
        formatted: printed.into_str(),
        session,
    })
}

/// Print formatted output with syntax highlighting if available.
fn print_formatted_output(label: &str, formatted: &str) {
    let colored_output = match semantic_spans_from_text(label, formatted) {
        Ok(spans) => render_semantic_spans(&spans),
        Err(error) => {
            console::warn(&format!("semantic highlighting error: {error}"));
            formatted.to_string()
        }
    };
    console::write_line(&colored_output);
}

/// Print diagnostics collected during formatting.
fn print_diagnostics(source: &Source, session: &Session) {
    for diagnostic in &session.diagnostics {
        let annotated = annotate_source(
            source,
            &diagnostic.primary_span,
            AnnotateOptions {
                max_line_width: 100,
                prefix_lines: 1,
                suffix_lines: 1,
                use_color: true,
            },
        );
        let header = Color::Red.apply_bold(&format!("{}: {}", diagnostic.code, diagnostic.message));
        console::error(&header);
        console::info(&annotated);
    }
}

/// Print a format source error.
fn print_format_source_error(path_label: &str, source: &Source, error: FormatSourceError) {
    match error {
        FormatSourceError::Diagnostics(session) => {
            print_diagnostics(source, &session);
            console::error(&format!("failed to format {path_label}"));
        }
        FormatSourceError::Formatter(inner) => {
            console::error(&format!(
                "formatter error while formatting {path_label}: {inner}"
            ));
        }
        FormatSourceError::Printer(inner) => {
            console::error(&format!(
                "printer error while formatting {path_label}: {inner}"
            ));
        }
    }
}

/// Compute a display string for a path.
fn canonical_display(path: &Path) -> String {
    match fs::canonicalize(path) {
        Ok(full) => full.to_string_lossy().to_string(),
        Err(_) => path.to_string_lossy().to_string(),
    }
}

/// Parse command line options into DystFormatOptions.
fn parse_options(ctx: &CommandArguments) -> Result<DystFormatOptions, String> {
    let mut options = DystFormatOptions::default();

    // --line-width
    if let Some(value) = ctx.option("line-width") {
        let width: u8 = value
            .parse()
            .map_err(|_| format!("invalid line width: {value}"))?;
        options = options.with_line_width(width);
    }

    // --indent-width
    if let Some(value) = ctx.option("indent-width") {
        let width: u8 = value
            .parse()
            .map_err(|_| format!("invalid indent width: {value}"))?;
        options = options.with_indent_width(width);
    }

    // --indent-style
    if let Some(value) = ctx.option("indent-style") {
        let normalized = value.to_ascii_lowercase();
        let style = match normalized.as_str() {
            "space" => IndentStyle::Space,
            "tab" => IndentStyle::Tab,
            other => return Err(format!("invalid indent style: {other}")),
        };
        options = options.with_indent_style(style);
    }

    // --line-ending
    if let Some(value) = ctx.option("line-ending") {
        let normalized = value.to_ascii_lowercase();
        let ending = match normalized.as_str() {
            "lf" => LineEnding::LineFeed,
            "crlf" => LineEnding::CarriageReturnLineFeed,
            "cr" => LineEnding::CarriageReturn,
            other => return Err(format!("invalid line ending: {other}")),
        };
        options = options.with_line_ending(ending);
    }

    // --format
    if let Some(value) = ctx.option("format") {
        let format = match SourceFormat::from_extension(value) {
            Some(format) => format,
            None => return Err(format!("invalid format: {value}")),
        };
        options = options.with_format(format);
    }

    Ok(options)
}
