use std::collections::HashMap;

use destack_terminal::{CommandArguments, console};
use dyst_ast::{DefinitionMeta, Dumper, DumperOptions, Name, NodeVisitor};
use dyst_parser::Parser;
use dyst_source::{DiagnosticCollector, LanguageOptions, Severity};

use crate::diagnostic::print_diagnostics;
use crate::source::get_string_or_file;

pub const HELP: &str = r"Parse source into AST (implicit module).
	--file <path>      Read input from file
	--string <string>  Read input from provided string
    --type <format>  Parse a file with the given format (default: ds)
    --silent           Don't print anything to the console (except errors)
    ";

/// Parse source into an AST and dump the statements.
pub fn run(ctx: CommandArguments) -> i32 {
    let silent = ctx.flag("silent");

    // read input source
    let file = match get_string_or_file(&ctx) {
        Ok(Some(file)) => file,
        Ok(None) => {
            console::error("no source provided");
            return 1;
        }
        Err(e) => {
            console::error(&format!("failed to read file: {e}"));
            return 1;
        }
    };

    // parse as implicit module
    let language = LanguageOptions::default();
    let mut diagnostics = DiagnosticCollector::new();
    let mut parser = Parser::from_file(&file, language, &mut diagnostics);
    let module_name = file
        .uri
        .last_segment()
        .map(|s| Name::Identifier(parser.intern_string(s)))
        .unwrap_or(Name::String(parser.intern_string("<string>")));
    let definition_id =
        parser.eat_implicit_namespace_with_recovery(DefinitionMeta::new(module_name));
    parser.finish();

    // dump AST to output
    if !silent {
        let dump_options = DumperOptions::default();
        let mut dumper = Dumper::new(&parser.strings, &parser.tree, dump_options);
        if let Some(definition_id) = definition_id {
            dumper.visit_definition(&parser.tree, definition_id, parser.tree.get(definition_id));
        }
        console::info(&dumper.finish());
    }

    // handle diagnostics
    let source_by_id = HashMap::from([(file.id, &file)]);
    print_diagnostics(&source_by_id, language, &diagnostics);
    if diagnostics.has_diagnostics_of_severity(Severity::Error) {
        return 1;
    }
    0
}
