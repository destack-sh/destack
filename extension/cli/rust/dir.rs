//! AST parsing subcommand.

use destack_terminal::{CommandArguments, console};
use dyst_ast::{ModuleFormat, TokenType};
use dyst_compiler::{AstNodeId, Compiler};
use dyst_diagnostic::Severity;
use dyst_dir::{DumperOptions, NodeVisitor};
use dyst_parser::Parser;
use dyst_session::Session;
use dyst_source::{AnnotateOptions, Color, annotate_source};

use crate::source::read_source;

pub const HELP: &str = r"Parse source into DIR (implicit module).
	--file <path>      Read input from file
	--string <string>  Read input from provided string
    --module <path>    The module to compile (default: auto-detect)
    --standalone       Compile as standalone package (disable auto-detect)
    ";

/// Parse source into an DIR and dump the module.
pub fn run(ctx: CommandArguments) -> i32 {
    let mut session = Session::new();
    let module = ctx.option("module");
    let standalone = ctx.flag("standalone");

    // read input source
    let source = match read_source(&ctx) {
        Ok(source) => source,
        Err(error) => {
            console::error(&format!("Read input error: {error}"));
            return 1;
        }
    };
    let module_name = source.uri.last_segment().unwrap_or("<string>");

    // parse as implicit module
    let mut parser = Parser::prepare(&source, &mut session);
    let module_name_id = parser.intern_string(module_name);
    let definition_id = parser.with_recovery(
        parser.mark(),
        |parser| {
            parser
                .eat_module_body(None, Some(module_name_id), ModuleFormat::Source)
                .map(Some)
        },
        None,
        TokenType::End,
    );
    parser.finalize();

    // parse rest of module
    

    // compile the AST to DIR
    let mut compiler = Compiler::new(&parser.tree, &mut session);

    // dump DIR module to output
    let dump_options = DumperOptions::default();
    let mut dumper = compiler.dumper(dump_options);
    if let Some(definition_id) = definition_id {
        let definition_id = compiler.lower_definition(AstNodeId::new(definition_id, source.id));
        if let Some(definition_id) = definition_id {
            dumper.visit_definition(
                &compiler.tree,
                definition_id,
                compiler.tree.get(definition_id),
            );
        }
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
