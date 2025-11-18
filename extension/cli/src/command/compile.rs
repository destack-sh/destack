use dyst_compiler::{CompileOptions, Compiler};
use dyst_dir::{Dumper, DumperOptions, NodeVisitor, Session};
use dyst_source::{DiagnosticOptions, DiagnosticSeverity, FileRegistry, LanguageOptions};

use crate::command::{get_string_or_file, print_diagnostics};
use crate::{CommandArguments, console};

pub const HELP: &str = r"
Compile source files.
    --package <path>     Compile a package
    --module <module>    Compile a single module
    --file <path>        Compile a single file
    --string <string>    Compile a string
    --type <format>      Compile a file with the given format (js|ts|jsx|tsx|ds, default: ds)
    --dump <format>      Dump the compiled DIR in the given format (node|symbol|all, default: node)
    --silent             Don't print anything to the console (except errors)
    --error-warnings     Error on the given warning codes (like W001)
    --suppress-errors    Suppress the given error codes (like E001) as warnings
    --suppress-warnings  Suppress the given warning codes (like W001)
";

/// Compile source into its final DIR.
pub fn run(ctx: CommandArguments) -> i32 {
    let silent = ctx.flag("silent");
    let dump = ctx.option("dump").unwrap_or("node");
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
            console::error(&format!("failed to read file {e}"));
            return 1;
        }
    };

    // compile source
    let session = Session::new(LanguageOptions::default(), &files);
    let mut compiler = Compiler::from_file(
        &session,
        file_id,
        CompileOptions {
            diagnostic: diagnostic_options,
            ..Default::default()
        },
    );
    compiler.compile();
    drop(compiler);

    // dump DIR to output
    if !silent {
        let dump_options = DumperOptions::default();
        let tree = session.tree.read();
        let strings = session.strings.clone().into_immutable();
        // dump node representation
        if dump == "node" || dump == "all" {
            let mut dumper = Dumper::new(&strings, &tree, dump_options);
            for module in session.modules.iter() {
                console::info("=".repeat(80).as_str());
                console::info(format!("{} [NODE]", module.uri).as_str());
                console::info("=".repeat(80).as_str());
                for expression_id in &module.expressions {
                    let expression = tree.get(*expression_id);
                    dumper.visit_expression(&tree, *expression_id, expression);
                }
            }
            console::info(&dumper.finish());
        }
        // dump symbol representation
        if dump == "symbol" || dump == "all" {
            let mut dumper = Dumper::new(&strings, &tree, dump_options);
            for module in session.modules.iter() {
                console::info("=".repeat(80).as_str());
                console::info(format!("{} [SYMBOL]", module.uri).as_str());
                console::info("=".repeat(80).as_str());
                if let Some(scope_id) = module.scope {
                    let scope = tree.get_scope_by_id(scope_id);
                    dumper.visit_scope(&tree, scope_id, scope);
                }
            }
            console::info(&dumper.finish());
        }
    }

    // handle diagnostics
    print_diagnostics(&session, DiagnosticSeverity::Note);
    session.get_diagnostics_status_code()
}
