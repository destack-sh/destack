//! AST parsing subcommand.

use destack_terminal::{CommandArguments, console};
use dyst_ast::{
    DefinitionMeta, DumperOptions, ModuleFormat, ModuleStyle, Name, NodeVisitor, TokenType,
};
use dyst_diagnostic::Severity;
use dyst_parser::Parser;
use dyst_source::{AnnotateOptions, Color, LanguageOptions, annotate_source};

use crate::source::read_source;

pub const HELP: &str = r"Parse source into AST (implicit module).
	--file <path>      Read input from file
	--string <string>  Read input from provided string
    --silent           Don't print anything to the console (except errors)
    ";

/// Parse source into an AST and dump the statements.
pub fn run(ctx: CommandArguments) -> i32 {
    let silent = ctx.flag("silent");
    let mut session = Session::new();

    // read input source
    let source = match read_source(&ctx) {
        Ok(source) => source,
        Err(error) => {
            console::error(&format!("Read input error: {error}"));
            return 1;
        }
    };

    // parse as implicit module
    let language = LanguageOptions::default();
    let mut parser = Parser::from_file(&source, language, &mut session);
    let module_name = source
        .uri
        .last_segment()
        .map(|s| Name::Identifier(parser.intern_string(s)))
        .unwrap_or(Name::String(parser.intern_string("<string>")));
    let definition_id = parser.with_recovery(
        parser.mark(),
        |parser| {
            parser
                .eat_module_body(
                    DefinitionMeta::new(module_name),
                    ModuleFormat::Inline,
                    ModuleStyle::Module,
                )
                .map(Some)
        },
        None,
        TokenType::End,
    );
    parser.finalize();

    // dump AST module to output
    if !silent {
        let dump_options = DumperOptions::default();
        let mut dumper = parser.dumper(dump_options);
        if let Some(definition_id) = definition_id {
            dumper.visit_definition(&parser.tree, definition_id, parser.tree.get(definition_id));
        }
        console::info(&dumper.finish());
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
