use destack_terminal::{CommandArguments, console};
use dyst_compiler::Compiler;
use dyst_javascript_transpiler::Transpiler;
use dyst_source::{DiagnosticCollector, DiagnosticSeverity, LanguageOptions};

use crate::diagnostic::print_diagnostics;
use crate::source::get_string_or_file;

pub const HELP: &str = r"Transpile source files.
	--file <path>      Read input from file
	--string <string>  Read input from provided string
    --silent           Don't print anything to the console (except errors)
";

/// Transpile source into its final JavaScript.
pub fn run(ctx: CommandArguments) -> i32 {
    let _silent = ctx.flag("silent");

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
    let mut diagnostics = DiagnosticCollector::new();
    let mut compiler = Compiler::from_file(file.clone(), language, &mut diagnostics);
    compiler.compile();
    let diagnostics = compiler.diagnostics.clone();

    // handle compiler diagnostics
    print_diagnostics(
        &diagnostics,
        language,
        DiagnosticSeverity::Note,
        |file_id| compiler.modules.get_file_by_file_id(file_id),
    );
    if diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
        return 1;
    }

    // transpile source
    let mut transpiler = Transpiler::from_compiler(&compiler);
    transpiler.transpile();

    todo!("print/write transpiler artifacts");

    0
}
