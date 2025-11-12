use dyst_compiler::{Compiler, CompilerOptions};
use dyst_dir::{Dumper, DumperOptions, NodeVisitor};
use dyst_source::{DiagnosticSeverity, LanguageOptions};

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
    let file = match get_string_or_file(&ctx) {
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
    let language = LanguageOptions::default();
    let compiler_options = CompilerOptions::default();
    let mut compiler = Compiler::from_file(file.clone(), language, compiler_options);
    compiler.compile();
    let compiler_diagnostics = compiler.diagnostics.clone();

    // dump DIR to output
    if !silent {
        let dump_options = DumperOptions::default();
        let mut dumper = Dumper::new(&compiler.strings, &compiler.tree, dump_options);
        for module in compiler.modules.iter() {
            console::info("=".repeat(80).as_str());
            console::info(module.file.uri.to_string().as_str());
            console::info("=".repeat(80).as_str());
            for expression_id in &module.expressions {
                let expression = compiler.tree.get(*expression_id);
                dumper.visit_expression(&compiler.tree, *expression_id, expression);
            }
        }
        console::info(&dumper.finish());
    }

    // handle diagnostics
    print_diagnostics(
        &compiler_diagnostics,
        language,
        DiagnosticSeverity::Note,
        |file_id| compiler.modules.get_file_by_file_id(file_id),
    );
    if compiler_diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
        return 1;
    }
    0
}
