use dyst_compiler::{Compiler, CompilerOptions};
use dyst_dir::{Dumper, DumperOptions, NodeVisitor, Session};
use dyst_source::{DiagnosticSeverity, FileRegistry, LanguageOptions};

use crate::command::{get_string_or_file, print_diagnostics};
use crate::{CommandArguments, console};

pub const HELP: &str = r"
Compile source files.
    --package <path>   Compile a package
    --file <path>      Compile a single file
    --string <string>  Compile a string
    --type <format>    Compile a file with the given format (default: ds)
    --silent           Don't print anything to the console (except errors)
";

/// Compile source into its final DIR.
pub fn run(ctx: CommandArguments) -> i32 {
    let silent = ctx.flag("silent");

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
    let mut compiler = Compiler::from_file(&session, file_id, CompilerOptions::default());
    compiler.compile();
    drop(compiler);

    // dump DIR to output
    if !silent {
        let dump_options = DumperOptions::default();
        let tree = session.tree.read();
        let strings = session.strings.clone().into_immutable();
        let mut dumper = Dumper::new(&strings, &tree, dump_options);
        for module in session.modules.iter() {
            console::info("=".repeat(80).as_str());
            console::info(module.uri.to_string().as_str());
            console::info("=".repeat(80).as_str());
            for expression_id in &module.expressions {
                let expression = tree.get(*expression_id);
                dumper.visit_expression(&tree, *expression_id, expression);
            }
        }
        console::info(&dumper.finish());
    }

    // handle diagnostics
    print_diagnostics(&session, DiagnosticSeverity::Note);
    session.get_diagnostics_status_code()
}
