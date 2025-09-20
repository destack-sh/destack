//! Format subcommand for Dyst source code.

use dyst_language_ast::{BlockFormat, DystFormatContext, DystFormatOptions, Parser};
use dyst_language_diagnostic::Severity;
use dyst_language_fir::format::{
    Argument as FormatArgument, Arguments as FormatArguments, IndentStyle, LineEnding,
    format as format_document,
};
use dyst_language_session::Session;
use dyst_language_source::{AnnotateOptions, Color, annotate_source};
use dyst_language_token::TokenType;

use crate::cli::source::read_source;
use crate::console::console;
use crate::console::parse::CommandArguments;

pub const HELP: &str = "Format Dyst source code.\n\t--file <path>        Read input from file\n\t--string <string>    Read input from provided string\n\t--line-width <n>     Set maximum line width (default 100)\n\t--indent-style <s>   Choose indent style: space or tab\n\t--indent-width <n>   Set spaces per indent (default 4)\n\t--line-ending <e>    Choose line ending: lf, crlf, cr";

/// Format input source and print the formatted output.
pub fn run(ctx: CommandArguments) -> i32 {
    // read input source
    let source = match read_source(&ctx) {
        Ok(source) => source,
        Err(error) => {
            console::error(&format!("Read input error: {error}"));
            return 1;
        }
    };

    // parse formatting options from command arguments
    let options = match parse_options(&ctx) {
        Ok(options) => options,
        Err(error) => {
            console::error(&format!("Option error: {error}"));
            return 1;
        }
    };

    // parse source into AST statements
    let mut session = Session::new();
    let mut parser = Parser::from_source(&source, &mut session);
    let statements = parser.with_recovery(
        parser.mark(),
        |parser| parser.eat_block_body(BlockFormat::Implicit),
        Vec::new(),
        TokenType::End,
    );
    parser.process_annotations();

    // create format context with options and parsed tree
    let tree = parser.tree;
    let context = DystFormatContext {
        options,
        source: &source,
        session: &session,
        tree: &tree,
    };

    // convert statements to format arguments
    let arguments: Vec<_> = statements.iter().map(FormatArgument::new).collect();

    // format the document
    let formatted = match format_document(context, FormatArguments::new(&arguments)) {
        Ok(formatted) => formatted,
        Err(error) => {
            console::error(&format!("format error: {error}"));
            return 1;
        }
    };

    // print formatted output
    match formatted.print() {
        Ok(printed) => console::write_line(printed.as_str()),
        Err(error) => {
            console::error(&format!("print error: {error}"));
            return 1;
        }
    }

    // print diagnostics
    for diagnostic in &session.diagnostics {
        let annotated = annotate_source(
            &source,
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

    // return error if any error diagnostics
    let has_errors = session
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error);
    if has_errors {
        return 1;
    }
    0
}

/// Parse formatting options from command arguments.
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
