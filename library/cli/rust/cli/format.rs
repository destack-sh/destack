//! Format subcommand for Dyst source code.

use std::path::{Path, PathBuf};
use std::{env, fs};

use destack_file::walk::{WalkOptions, walk};
use dyst_ast::{DystFormatContext, DystFormatOptions, ModuleFormat, NodeParentIndex, Parser};
use dyst_diagnostic::Severity;
use dyst_fir::format::{IndentStyle, LineEnding, format as format_fir};
use dyst_fir::format_args;
use dyst_session::Session;
use dyst_source::{AnnotateOptions, Color, Source, SourceId, Uri, annotate_source};
use dyst_token::TokenType;

use crate::cli::source::{render_semantic_spans, semantic_spans_from_text};
use crate::console::console;
use crate::console::parse::CommandArguments;

pub const HELP: &str = r"Format Dyst source code.
	--line-width <n>     Set maximum line width (default 100)
	--indent-style <s>   Choose indent style: space or tab
	--indent-width <n>   Set spaces per indent (default 4)
	--line-ending <e>    Choose line ending: lf, crlf, cr
	--dry-run            Preview formatting without writing files
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

/// Format all .ds files in the current directory tree.
fn run_for_all_files(options: &DystFormatOptions, dry_run: bool) -> i32 {
    // get current working directory
    let root = match env::current_dir() {
        Ok(dir) => dir,
        Err(error) => {
            console::error(&format!("failed to read current directory: {error}"));
            return 1;
        }
    };
    let files = collect_ds_files(&root);

    // process each file and track overall success
    let mut exit_code = 0;
    for path in &files {
        if dry_run {
            console::write_line(&canonical_display(path));
        }

        match format_file(path, options, dry_run, false) {
            Ok(FormattedFlags {
                did_change,
                did_error,
            }) => {
                if !dry_run && did_change {
                    console::write_line(&canonical_display(path));
                }
                if did_error {
                    exit_code = 1;
                }
            }
            Err(message) => {
                console::error(&message);
                exit_code = 1;
            }
        }
    }

    exit_code
}

/// Format the provided list of files.
fn run_for_files(paths: &[PathBuf], options: &DystFormatOptions, dry_run: bool) -> i32 {
    let mut exit_code = 0;

    // process each file and track overall success
    for path in paths {
        match format_file(path, options, dry_run, true) {
            Ok(FormattedFlags { did_error, .. }) => {
                if did_error {
                    exit_code = 1;
                }
            }
            Err(message) => {
                console::error(&message);
                exit_code = 1;
            }
        }
    }
    exit_code
}

/// Format inline source provided via --string.
fn run_for_string(body: &str, options: &DystFormatOptions) -> i32 {
    // create source from string input
    let source = Source::from_string(
        SourceId::new(0),
        Uri::from_string("<string>"),
        body.to_string(),
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
        Err(message) => {
            console::error(&message);
            1
        }
    }
}

/// Collect all .ds files under the given root using the shared glob walker.
fn collect_ds_files(root: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();

    // configure walker to find .ds files while ignoring common directories
    let walk_options = WalkOptions {
        root: root.to_path_buf(),
        ignore: Some(DEFAULT_IGNORE_PATHS.iter().map(|s| s.to_string()).collect()),
        glob: Some(vec!["**/*.ds".to_string()]),
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
) -> Result<FormattedFlags, String> {
    let path_buf = path.to_path_buf();

    // read original file content
    let original_text = fs::read_to_string(&path_buf)
        .map_err(|error| format!("failed to read {}: {error}", path_buf.display()))?;

    // create source and format it
    let source = Source::from_string(
        SourceId::new(0),
        Uri::from(&path_buf),
        original_text.clone(),
    );
    let FormattedSource { formatted, session } = format_source(&source, options)
        .map_err(|error| format!("{error} ({})", path_buf.display()))?;

    // output formatted result if requested
    if emit_output {
        print_formatted_output(&canonical_display(&path_buf), &formatted);
    }
    print_diagnostics(&source, &session);

    // check if formatting did_change the file or had errors
    let did_error = session.has_diagnostics_of_severity(Severity::Error);
    let did_change = formatted != original_text;

    // write back to disk if not dry run and content did_change
    if !dry_run && did_change {
        fs::write(&path_buf, formatted)
            .map_err(|error| format!("failed to write {}: {error}", path_buf.display()))?;
    }

    Ok(FormattedFlags {
        did_change,
        did_error,
    })
}

/// Format the provided source into a string along with diagnostics.
fn format_source(source: &Source, options: &DystFormatOptions) -> Result<FormattedSource, String> {
    let mut session = Session::new();
    let module_name = source.uri.last_segment().unwrap_or("<string>");
    let module_name_id = session.intern_string(module_name);

    // parse the source into an AST
    let mut parser = Parser::prepare(source, &mut session);
    let module_id = parser.with_recovery(
        parser.mark(),
        |parser| {
            parser
                .eat_module_body(None, Some(module_name_id), ModuleFormat::Implicit)
                .map(Some)
        },
        None,
        TokenType::End,
    );
    parser.finalize();

    // create format context and format the AST
    let side_span = parser.get_side_span();
    let tokens = parser.tokens;
    let side_tokens = parser.side_tokens;
    let tree = parser.tree;
    let context = DystFormatContext {
        options: options.clone(),
        source,
        tree: &tree,
        tokens: &tokens,
        side_tokens: &side_tokens,
        side_span: &side_span,
        spans: &tree.spans,
        parents: NodeParentIndex::from_tree(&tree),
        session: &session,
    };

    // format and print the document
    let formatted = format_fir(context, format_args![module_id])
        .map_err(|error| format!("format error: {error}"))?;
    let printed = formatted
        .print()
        .map_err(|error| format!("print error: {error}"))?;

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

    Ok(options)
}
