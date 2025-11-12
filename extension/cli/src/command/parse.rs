use dyst_ast::{Dumper, DumperOptions, NodeVisitor};
use dyst_parser::Parser;
use dyst_source::{DiagnosticCollector, DiagnosticSeverity, LanguageOptions};

use crate::command::{get_string_or_file, print_diagnostics};
use crate::{CommandArguments, console};

pub const HELP: &str = r"
Parse source into AST (implicit module).
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
    let mut parser = Parser::lex_file(&file, language, &mut diagnostics);
    let expressions = parser.parse();

    // dump AST to output
    if !silent {
        let dump_options = DumperOptions::default();
        let mut dumper = Dumper::new(&parser.strings, &parser.tree, dump_options);
        for expression in expressions {
            dumper.visit_expression(&parser.tree, expression, parser.tree.get(expression));
        }
        console::info(&dumper.finish());
    }

    // handle diagnostics
    print_diagnostics(&diagnostics, language, DiagnosticSeverity::Error, |_| {
        Some(&file)
    });
    if diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
        return 1;
    }
    0
}
