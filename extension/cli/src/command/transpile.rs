use clap::{ArgGroup, Args, ValueEnum};
use dyst_compiler::{CompileOptions, Compiler};
use dyst_dir::Session;
use dyst_javascript_transpiler::{TranspileOptions, Transpiler, TranspilerTarget};
use dyst_source::{
    DiagnosticOptions, DiagnosticSeverity, FileContent, FileRegistry, LanguageOptions,
};

use crate::command::{DiagnosticOptionsArgs, SourceArg, get_string_or_file, print_diagnostics};
use crate::console;

/// The target language to transpile to.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum TranspileTargetArg {
    /// Plain JavaScript (`.js`).
    Js,
    /// TypeScript (`.ts`).
    Ts,
    /// Plain JavaScript with TypeScript declarations (.js and .d.ts).
    Jsdts,
    /// All languages.
    All,
}

impl From<TranspileTargetArg> for TranspilerTarget {
    fn from(target: TranspileTargetArg) -> Self {
        match target {
            TranspileTargetArg::Js => TranspilerTarget::JavaScript,
            TranspileTargetArg::Ts => TranspilerTarget::TypeScript,
            TranspileTargetArg::Jsdts => TranspilerTarget::JavaScriptWithTypeScriptDeclarations,
            TranspileTargetArg::All => TranspilerTarget::All,
        }
    }
}

#[derive(Args, Debug, Clone)]
#[command(group(
    ArgGroup::new("source")
        .args(["file", "string"])
        .required(true)
        .multiple(false)
))]
pub struct TranspileArgs {
    /// Read input from file.
    #[arg(long)]
    pub file: Option<String>,

    /// Read input from provided string.
    #[arg(long)]
    pub string: Option<String>,

    /// Transpile to the given target (js|ts|jsdts|all, default: all).
    #[arg(long, default_value_t = TranspileTargetArg::All, value_enum)]
    pub target: TranspileTargetArg,

    /// Don't print anything to the console (except errors).
    #[arg(long)]
    pub silent: bool,

    #[command(flatten)]
    pub diagnostics: DiagnosticOptionsArgs,
}

/// Transpile source into its final JavaScript.
pub fn run(args: &TranspileArgs) -> i32 {
    let silent = args.silent;
    let target = args.target.into();
    let diagnostic_options: DiagnosticOptions = args.diagnostics.clone().into();

    // read input source
    let mut files = FileRegistry::new();
    let file_id = match get_string_or_file(
        &mut files,
        SourceArg {
            file: args.file.as_deref(),
            string: args.string.as_deref(),
            format: None,
        },
    ) {
        Ok(Some(file_id)) => file_id,
        Ok(None) => {
            console::error("error: failed to resolve source input");
            return 1;
        }
        Err(e) => {
            console::error(&format!("error: {e}"));
            return 1;
        }
    };

    // compile source
    let session = Session::new(LanguageOptions::default(), &files);
    let compiler = Compiler::from_file(&session, file_id, CompileOptions::default());
    compiler.compile();
    drop(compiler);

    // handle compiler diagnostics
    let diagnostics = session.diagnostics.collect().map(&diagnostic_options);
    if diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
        // bail early with diagnostics
        print_diagnostics(&session, &diagnostics);
        return 1;
    }

    // transpile source
    let transpiler_options = TranspileOptions {
        target,
        diagnostic: diagnostic_options.clone(),
        ..Default::default()
    };
    let transpiler = Transpiler::new(&session, transpiler_options);
    transpiler.transpile();

    // handle transpiler diagnostics
    let diagnostics = session.diagnostics.collect().map(&diagnostic_options);
    print_diagnostics(&session, &diagnostics);
    if diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
        return 1;
    }

    // print/write transpiler artifacts
    if !silent {
        let line_width = session.language.formatting.line_width as usize;
        let artifacts = transpiler.artifacts.read();
        for (i, (uri, artifact)) in artifacts.iter().enumerate() {
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
            if i < artifacts.len() - 1 {
                console::print("");
            }
        }
    }

    diagnostics.get_status_code()
}
