use dyst_language_ast::{BlockFormat, DumperOptions, Parser};
use dyst_language_diagnostic::Severity;
use dyst_language_session::Session;
use dyst_language_source::{AnnotateOptions, Color, annotate_source};
use dyst_language_token::TokenType;

use crate::cli::parse::read_source;
use crate::console::console;
use crate::console::parse::CommandArguments;

/// Parse source into AST.
pub(crate) fn parse_ast(ctx: CommandArguments) -> i32 {
    // read input
    let source = match read_source(&ctx) {
        Ok(s) => s,
        Err(e) => {
            console::error(&format!("Read input error: {e}"));
            return 1;
        }
    };

    // parse as (implicit) block of statements
    let mut session = Session::new();
    let mut parser = Parser::from_source(&source, &mut session);
    let statements = parser.with_recovery(
        parser.mark(),
        |parser| parser.eat_block_body(BlockFormat::Implicit),
        Vec::new(),
        TokenType::End,
    );
    parser.process_annotations();

    // print statements
    let dump_options = DumperOptions::default();
    let mut dumper = parser.dumper(dump_options);
    dumper.dump_nodes(&statements, None);
    console::info(&dumper.finish());

    // print errors
    for diagnostic in &session.diagnostics {
        let annotated = annotate_source(
            &source,
            &diagnostic.primary_span,
            AnnotateOptions {
                max_line_length: 100,
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

    let has_errors = session
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error);
    if has_errors {
        return 1;
    }
    0
}
