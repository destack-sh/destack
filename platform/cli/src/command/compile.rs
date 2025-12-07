use std::path::Path;

use clap::{ArgGroup, Args};
use destack_compiler::{AnalyzeTask, CompileOptions, Compiler};
use destack_dir::{Dumper, DumperOptions, NodeVisitor};
use destack_parser::colorize_source;
use destack_source::{DiagnosticOptions, FileType, Uri};

use crate::command::{DiagnosticArgs, DumpFormat, DumpKind, ProgramArgs, print_diagnostics};
use crate::console;

#[derive(Args, Debug, Clone)]
#[command(group(
    ArgGroup::new("source")
        .args(["file", "string"])
        .required(true)
        .multiple(false)
))]
pub struct CompileArgs {
    /// Compile a package.
    #[arg(long)]
    pub package: Option<String>,

    /// Compile a single module.
    #[arg(long)]
    pub module: Option<String>,

    /// Compile a single file.
    #[arg(long)]
    pub file: Option<String>,

    /// Compile a string.
    #[arg(long)]
    pub string: Option<String>,

    /// Compile a file with the given format (js|ts|jsx|tsx|ds, default: ds).
    #[arg(long = "type", alias = "format", value_name = "FORMAT")]
    pub format: Option<String>,

    /// Dump output format(s): file|f, node|n, symbol|s, all|a (comma-separated).
    #[arg(long, value_delimiter = ',', default_value = "all")]
    pub dump: Vec<DumpKind>,

    /// Don't print anything to the console (except errors).
    #[arg(long)]
    pub silent: bool,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// The diagnostic options.
    #[command(flatten)]
    pub diagnostics: DiagnosticArgs,
}

/// Compile source into its final DIR.
pub fn run(args: &CompileArgs) -> i32 {
    let silent = args.silent;
    let dump = DumpFormat::from(args.dump.clone());
    let diagnostic_options: DiagnosticOptions = args.diagnostics.clone().into();
    let program = args.program.setup();

    // create compiler
    let compiler = Compiler::new(
        program.clone(),
        CompileOptions {
            diagnostic: diagnostic_options.clone(),
            workers: args.program.workers,
            ..Default::default()
        },
    );

    // determine file type
    let format_name = args.format.as_deref().unwrap_or("ds");
    let file_type = FileType::from_extension_or_unknown(format_name);

    // resolve input source to module
    let module_id = if let Some(ref path_str) = args.file {
        // file input: resolve path to module
        let path = Path::new(path_str);
        match compiler.resolve_path_to_module(&path.to_path_buf()) {
            Ok(id) => id,
            Err(e) => {
                console::error(&format!("error: {e:?}"));
                return 1;
            }
        }
    } else if let Some(ref string) = args.string {
        // string input: register inline module on program
        let uri = Uri::from_string("<string>");
        program.register_inline_module(uri, string.clone(), file_type)
    } else {
        console::error("error: no source input provided");
        return 1;
    };

    // compile (import phase)
    compiler.enqueue(AnalyzeTask::AnalyzeModule { module: module_id });
    compiler.compile();
    drop(compiler);

    // dump DIR to output (iterate per module, dump all requested representations)
    if !silent {
        let dump_options = DumperOptions::default();
        let strings = program.strings.clone().into_immutable();

        for module in program.modules.iter() {
            let module = module.read();

            // dump file representation
            if dump.includes_file() {
                let file = program.files.get(module.file_id);
                console::info("=".repeat(80).as_str());
                console::info(format!("{} [FILE]", module.uri).as_str());
                console::info("=".repeat(80).as_str());
                console::info(&colorize_source(&file));
            }

            // dump node representation
            if dump.includes_node() {
                let tree = module.dir.tree.read();
                let mut dumper = Dumper::new(&strings, &tree, dump_options);
                console::info("=".repeat(80).as_str());
                console::info(format!("{} [NODE]", module.uri).as_str());
                console::info("=".repeat(80).as_str());
                for expression_id in &module.dir.roots {
                    let expression = tree.get(*expression_id);
                    dumper.visit_expression(&tree, *expression_id, expression);
                }
                console::info(&dumper.finish());
            }

            // dump symbol representation
            if dump.includes_symbol() {
                let tree = module.dir.tree.read();
                let symbols = module.dir.symbols.read();
                let mut dumper = Dumper::new(&strings, &tree, dump_options);
                console::info("=".repeat(80).as_str());
                console::info(format!("{} [SYMBOL]", module.uri).as_str());
                console::info("=".repeat(80).as_str());
                let scope = symbols.get_scope_by_id(module.dir.namespace_scope);
                dumper.visit_scope(&tree, &symbols, module.dir.namespace_scope, scope);
                console::info(&dumper.finish());
            }
        }
    }

    // handle diagnostics
    let diagnostics = program.diagnostics.collect().map(&diagnostic_options);
    print_diagnostics(&program, &diagnostics);
    diagnostics.get_status_code()
}
