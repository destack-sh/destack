//! Format subcommand for Dyst source code.

use dyst_language_ast::{
    BlockFormat, DystFormatContext, DystFormatOptions, Module, ModuleFormat, NodeParentIndex,
    Parser,
};
use dyst_language_diagnostic::Severity;
use dyst_language_fir::format::{IndentStyle, LineEnding, format as format_document};
use dyst_language_fir::format_args;
use dyst_language_session::Session;
use dyst_language_source::{AnnotateOptions, Color, annotate_source};
use dyst_language_token::TokenType;

use crate::cli::source::{read_source, render_semantic_spans, semantic_spans_from_text};
use crate::console::console;
use crate::console::parse::CommandArguments;

pub const HELP: &str = r"Format Dyst source code.
	--file <path>        Read input from file
	--string <string>    Read input from provided string
	--line-width <n>     Set maximum line width (default 100)
	--indent-style <s>   Choose indent style: space or tab
	--indent-width <n>   Set spaces per indent (default 4)
	--line-ending <e>    Choose line ending: lf, crlf, cr";

/// Format input source and print the formatted output.
pub fn run(ctx: CommandArguments) -> i32 {
    // read and validate input source
    let source = match read_source(&ctx) {
        Ok(source) => source,
        Err(error) => {
            console::error(&format!("Read input error: {error}"));
            return 1;
        }
    };

    // parse command line options
    let options = match parse_options(&ctx) {
        Ok(options) => options,
        Err(error) => {
            console::error(&format!("Option error: {error}"));
            return 1;
        }
    };

    // parse source into AST
    let mut session = Session::new();
    let mut parser = Parser::from_source(&source, &mut session);
    let start = parser.mark();
    let statements = parser.with_recovery(
        parser.mark(),
        |parser| parser.eat_block_body(BlockFormat::Implicit),
        Vec::new(),
        TokenType::End,
    );
    let module = parser.tree.allocate(
        Module {
            format: ModuleFormat::Implicit,
            name: None,
            visibility: None,
            statements,
        },
        parser.get_span_from(start),
    );
    parser.finalize();

    // format the AST node
    let tree = parser.tree;
    let context = DystFormatContext {
        options,
        source: &source,
        session: &session,
        tree: &tree,
        spans: &tree.spans,
        parents: NodeParentIndex::from_tree(&tree),
    };
    let formatted = match format_document(context, format_args![module]) {
        Ok(formatted) => formatted,
        Err(error) => {
            console::error(&format!("format error: {error}"));
            return 1;
        }
    };

    // print formatted document
    let printed = match formatted.print() {
        Ok(printed) => printed,
        Err(error) => {
            console::error(&format!("print error: {error}"));
            return 1;
        }
    };

    // apply syntax highlighting and output
    let formatted_text = printed.into_str();
    let colored_output = match semantic_spans_from_text("<formatted>", &formatted_text) {
        Ok(spans) => render_semantic_spans(&spans),
        Err(error) => {
            console::warn(&format!("semantic highlighting error: {error}"));
            formatted_text.clone()
        }
    };
    console::write_line(&colored_output);

    // display any diagnostics
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

    // return error code if any errors occurred
    let has_errors = session
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error);
    if has_errors {
        return 1;
    }
    0
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
