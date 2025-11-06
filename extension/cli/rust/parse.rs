use destack_terminal::{CommandArguments, console};
use dyst_ast::{DefinitionMeta, Dumper, DumperOptions, Name, NodeVisitor};
use dyst_parser::Parser;
use dyst_source::{
    AnnotateOptions, Color, DiagnosticCollector, LanguageOptions, Severity, annotate_source,
};

use crate::source::get_file_from_arguments;

pub const HELP: &str = r"Parse source into AST (implicit module).
	--file <path>      Read input from file
	--string <string>  Read input from provided string
    --silent           Don't print anything to the console (except errors)
    ";

/// Parse source into an AST and dump the statements.
pub fn run(ctx: CommandArguments) -> i32 {
    let silent = ctx.flag("silent");
    let mut diagnostics = DiagnosticCollector::new();

    // read input source
    let source = match get_file_from_arguments(&ctx) {
        Ok(source) => source,
        Err(error) => {
            console::error(&format!("Read input error: {error}"));
            return 1;
        }
    };

    // parse as implicit module
    let language = LanguageOptions::default();
    let mut parser = Parser::from_file(&source, language, &mut diagnostics);
    let module_name = source
        .uri
        .last_segment()
        .map(|s| Name::Identifier(parser.intern_string(s)))
        .unwrap_or(Name::String(parser.intern_string("<string>")));
    let definition_id = parser.eat_implicit_module_with_recovery(DefinitionMeta::new(module_name));
    parser.finish();

    // dump module to output
    if !silent {
        let dump_options = DumperOptions::default();
        let mut dumper = Dumper::new(&parser.strings, &parser.tree, dump_options);
        if let Some(definition_id) = definition_id {
            dumper.visit_definition(&parser.tree, definition_id, parser.tree.get(definition_id));
        }
        console::info(&dumper.finish());
    }

    // print diagnostics
    for diagnostic in diagnostics.iter() {
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
    if diagnostics.has_diagnostics_of_severity(Severity::Error) {
        return 1;
    }
    0
}
