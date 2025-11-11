use destack_terminal::{CommandArguments, console};
use dyst_compiler::{Compiler, CompilerOptions};
use dyst_javascript_transpiler::{Transpiler, TranspilerOptions, TranspilerTarget};
use dyst_source::{DiagnosticSeverity, FileContent, LanguageOptions};

use crate::diagnostic::print_diagnostics;
use crate::source::get_string_or_file;

pub const HELP: &str = r"
Transpile source files.
    --file <path>      Read input from file
    --string <string>  Read input from provided string
    --target <target>  Transpile to the given target (js|ts|jsdts|all, default: all)
    --silent           Don't print anything to the console (except errors)
";

/// Transpile source into its final JavaScript.
pub fn run(ctx: CommandArguments) -> i32 {
    let silent = ctx.flag("silent");
    let target = ctx.option("target");
    let target = match target {
        Some("js") => TranspilerTarget::JavaScript,
        Some("ts") => TranspilerTarget::TypeScript,
        Some("jsdts") => TranspilerTarget::JavaScriptWithTypeScriptDeclarations,
        Some("all") => TranspilerTarget::All,
        Some(target) => {
            console::error(&format!("invalid target {target}"));
            return 1;
        }
        None => TranspilerTarget::All,
    };

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
    let mut compiler = Compiler::from_file(file, language, compiler_options);
    compiler.compile();

    // handle compiler diagnostics
    print_diagnostics(
        &compiler.diagnostics,
        language,
        DiagnosticSeverity::Note,
        |file_id| compiler.modules.get_file_by_file_id(file_id),
    );
    if compiler
        .diagnostics
        .has_diagnostics_of_severity(DiagnosticSeverity::Error)
    {
        return 1;
    }

    // transpile source
    let transpiler_options = TranspilerOptions {
        target,
        ..Default::default()
    };
    let mut transpiler = Transpiler::from_compiled(&compiler, language, transpiler_options);
    transpiler.transpile();

    // handle transpiler diagnostics
    print_diagnostics(
        &transpiler.diagnostics,
        language,
        DiagnosticSeverity::Note,
        |file_id| transpiler.modules.get_file_by_file_id(file_id),
    );
    if transpiler
        .diagnostics
        .has_diagnostics_of_severity(DiagnosticSeverity::Error)
    {
        return 1;
    }

    // print/write transpiler artifacts
    if !silent {
        for (i, artifact) in transpiler.artifacts.iter().enumerate() {
            console::print("=".repeat(80).as_str());
            console::print(artifact.file.uri.as_ref());
            console::print("=".repeat(80).as_str());
            match &artifact.content {
                FileContent::Text(text) => {
                    console::print(text);
                }
                FileContent::Binary(bytes) => {
                    console::error(&format!("<binary {} bytes>", bytes.len()));
                }
                FileContent::Unloaded => {
                    console::error("<unloaded>");
                }
            }
            if i < transpiler.artifacts.len() - 1 {
                console::print("");
            }
        }
    }

    0
}
