//! AST parsing subcommand.

use dyst_language_ast::{BlockFormat, DumperOptions, NodeVisitor, Parser};
use dyst_language_diagnostic::Severity;
use dyst_language_session::Session;
use dyst_language_source::{AnnotateOptions, Color, annotate_source};
use dyst_language_token::TokenType;

use crate::cli::source::read_source;
use crate::console::console;
use crate::console::parse::CommandArguments;

pub const HELP: &str = r"Parse source into AST (implicit block).
	--file <path>      Read input from file
	--string <string>  Read input from provided string";

/// Parse source into an AST and dump the statements.
pub fn run(ctx: CommandArguments) -> i32 {
    // read input source
    let source = match read_source(&ctx) {
        Ok(source) => source,
        Err(error) => {
            console::error(&format!("Read input error: {error}"));
            return 1;
        }
    };

    // parse as implicit block of statements
    let mut session = Session::new();
    let mut parser = Parser::from_source(&source, &mut session);
    let statements = parser.with_recovery(
        parser.mark(),
        |parser| parser.eat_block_body(BlockFormat::Implicit),
        Vec::new(),
        TokenType::End,
    );
    parser.process_annotations();

    // dump AST statements to output
    let dump_options = DumperOptions::default();
    let mut dumper = parser.dumper(dump_options);
    for statement in statements {
        dumper.visit_statement(&parser.tree, statement, parser.tree.get(statement));
    }
    console::info(&dumper.finish());

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
        let diagnostic_header =
            Color::Red.apply_bold(&format!("{}: {}", diagnostic.code, diagnostic.message));
        console::error(&diagnostic_header);
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
