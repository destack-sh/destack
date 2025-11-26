use clap::{ArgGroup, Args, ValueEnum};
use dyst_compiler::{CompileOptions, Compiler, ImportTask};
use dyst_javascript_transpiler::{TranspileOptions, TranspileTarget, Transpiler};
use dyst_source::{DiagnosticOptions, DiagnosticSeverity, FileContent};

use crate::command::{
    DiagnosticOptionsArgs, ProgramArgs, SourceArg, get_string_or_file, print_diagnostics,
};
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

impl From<TranspileTargetArg> for TranspileTarget {
    fn from(target: TranspileTargetArg) -> Self {
        match target {
            TranspileTargetArg::Js => TranspileTarget::JavaScript,
            TranspileTargetArg::Ts => TranspileTarget::TypeScript,
            TranspileTargetArg::Jsdts => TranspileTarget::JavaScriptWithTypeScriptDeclarations,
            TranspileTargetArg::All => TranspileTarget::All,
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
    pub program: ProgramArgs,

    #[command(flatten)]
    pub diagnostics: DiagnosticOptionsArgs,
}

/// Transpile source into its final JavaScript.
pub fn run(args: &TranspileArgs) -> i32 {
    let silent = args.silent;
    let target = args.target.into();
    let diagnostic_options: DiagnosticOptions = args.diagnostics.clone().into();
    let program = args.program.setup();

    // read input source
    let file = match get_string_or_file(
        &program,
        SourceArg {
            file: args.file.as_deref(),
            string: args.string.as_deref(),
            format: None,
        },
    ) {
        Ok(file) => file,
        Err(e) => {
            console::error(&format!("error: {e}"));
            return 1;
        }
    };

    // compile source
    let compiler = Compiler::new(program.clone(), CompileOptions::default());
    compiler.enqueue(ImportTask::ImportModuleFromFile { file: file.id });
    compiler.compile();
    drop(compiler);

    // handle compiler diagnostics
    let diagnostics = program.diagnostics.collect().map(&diagnostic_options);
    if diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
        print_diagnostics(&program, &diagnostics);
        return 1;
    }

    // transpile source
    let transpiler_options = TranspileOptions {
        target,
        diagnostic: diagnostic_options.clone(),
        ..Default::default()
    };
    let transpiler = Transpiler::new(program.clone(), transpiler_options);
    transpiler.transpile();

    // handle transpiler diagnostics
    let diagnostics = program.diagnostics.collect().map(&diagnostic_options);
    print_diagnostics(&program, &diagnostics);
    if diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
        return 1;
    }

    // print/write transpiler artifacts
    if !silent {
        let line_width = program.language.formatting.line_width as usize;
        let artifact_count = transpiler.artifacts.len();
        for (i, entry) in transpiler.artifacts.iter().enumerate() {
            let (uri, artifact) = (entry.key(), entry.value());
            console::print("=".repeat(line_width).as_str());
            console::print(uri.as_ref());
            console::print("=".repeat(line_width).as_str());
            match &artifact.content {
                FileContent::Text { content } => {
                    console::print(content);
                }
                FileContent::Json { content, .. } => {
                    console::print(content);
                }
                FileContent::Binary { content } => {
                    console::error(&format!("<binary {} bytes>", content.len()));
                }
                FileContent::Unloaded => {
                    console::error("<unloaded>");
                }
            }
            if i < artifact_count - 1 {
                console::print("");
            }
        }
    }

    diagnostics.get_status_code()
}
