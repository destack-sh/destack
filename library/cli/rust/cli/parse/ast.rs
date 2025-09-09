use dyst_language_ast::{BlockFormat, DumperOptions, Parser};
use dyst_language_source::{AnnotateOptions, annotate_source};

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

    // parse as block
    let mut parser = Parser::from_source(&source);
    let dump_options = DumperOptions::default();
    match parser.eat_block_body(BlockFormat::Implicit) {
        Ok(statements) => {
            let mut dumper = parser.dumper(dump_options);
            dumper.dump_lines(&statements, None);
            console::info(&dumper.finish());
        }
        Err(e) => {
            let diagnostic = e.to_diagnostic();
            let annotated = annotate_source(
                &source,
                &diagnostic.primary_span.unwrap(),
                AnnotateOptions {
                    max_line_length: 100,
                    prefix_lines: 1,
                    suffix_lines: 1,
                    use_color: true,
                },
            );
            let diagnostic_header = format!("{}: {}", diagnostic.id, diagnostic.message);
            console::error(&diagnostic_header);
            console::info(&annotated);
            return 1;
        }
    }

    0
}
