use dyst_compiler::{Compiler, CompilerOptions};
use dyst_dir::Session;
use dyst_javascript_transpiler::{Transpiler, TranspilerOptions, TranspilerTarget};
use dyst_source::{DiagnosticSeverity, FileContent, FileRegistry, LanguageOptions};

use crate::command::{get_string_or_file, print_diagnostics};
use crate::{CommandArguments, console};

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
    let mut files = FileRegistry::new();
    let file_id = match get_string_or_file(&mut files, &ctx) {
        Ok(Some(file_id)) => file_id,
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

    // handle compiler diagnostics
    if session.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
        print_diagnostics(&session, DiagnosticSeverity::Warning);
        return 1;
    }

    // transpile source
    let transpiler_options = TranspilerOptions {
        target,
        ..Default::default()
    };
    let mut transpiler = Transpiler::new(&session, transpiler_options);
    transpiler.transpile();

    // handle transpiler diagnostics
    print_diagnostics(&session, DiagnosticSeverity::Note);
    if session.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
        return 1;
    }

    // print/write transpiler artifacts
    if !silent {
        let line_width = session.language.formatting.line_width as usize;
        for (i, (uri, artifact)) in transpiler.artifacts.iter().enumerate() {
            console::print("=".repeat(line_width).as_str());
            console::print(uri.as_ref());
            console::print("=".repeat(line_width).as_str());
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

    session.get_diagnostics_status_code()
}
