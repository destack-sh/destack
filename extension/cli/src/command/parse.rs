use dyst_ast::{Dumper, DumperOptions, NodeVisitor};
use dyst_dir::Session;
use dyst_parser::Parser;
use dyst_source::{DiagnosticOptions, FileRegistry, LanguageOptions};

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
    let diagnostic_options = DiagnosticOptions::parse(&ctx.flags);

    // read input source
    let mut files = FileRegistry::new();
    let file_id = match get_string_or_file(&mut files, &ctx) {
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
    let mut session = Session::new(LanguageOptions::default(), &files);
    let file = files.get(file_id).unwrap();
    let mut parser = Parser::lex_file(file, session.language, &mut session.diagnostics);
    let expressions = parser.parse();

    // dump AST to output
    if !silent {
        let dump_options = DumperOptions::default();
        let strings = parser.strings.clone().into_immutable();
        let mut dumper = Dumper::new(&strings, &parser.tree, dump_options);
        for expression in expressions {
            dumper.visit_expression(&parser.tree, expression, parser.tree.get(expression));
        }
        console::info(&dumper.finish());
    }

    // handle diagnostics
    let diagnostics = session.diagnostics.collect().map(&diagnostic_options);
    print_diagnostics(&session, &diagnostics);
    diagnostics.get_status_code()
}
