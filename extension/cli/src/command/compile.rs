use clap::{ArgGroup, Args, ValueEnum};
use dyst_compiler::{CompileOptions, Compiler};
use dyst_dir::{Dumper, DumperOptions, NodeVisitor, Program};
use dyst_source::{
    DiagnosticOptions, FileRegistry, FileSystem, LanguageOptions, PhysicalFileSystem,
};

use crate::command::{DiagnosticOptionsArgs, SourceArg, get_string_or_file, print_diagnostics};
use crate::console;

/// The format to dump the compiled DIR.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum DumpFormatArg {
    /// Dump the node representation.
    #[value(alias = "n")]
    Node,
    /// Dump the symbol representation.
    #[value(alias = "s")]
    Symbol,
    /// Dump both the node and symbol representations.
    #[value(alias = "a")]
    All,
}

impl DumpFormatArg {
    /// Whether the format includes the node representation.
    pub fn includes_node(self) -> bool {
        matches!(self, Self::Node | Self::All)
    }

    /// Whether the format includes the symbol representation.
    pub fn includes_symbol(self) -> bool {
        matches!(self, Self::Symbol | Self::All)
    }
}

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

    /// Dump the compiled DIR in the given format (node|symbol|all, default: node).
    #[arg(long, default_value_t = DumpFormatArg::All, value_enum)]
    pub dump: DumpFormatArg,

    /// Don't print anything to the console (except errors).
    #[arg(long)]
    pub silent: bool,

    #[command(flatten)]
    pub diagnostics: DiagnosticOptionsArgs,
}

/// Compile source into its final DIR.
pub fn run(args: &CompileArgs) -> i32 {
    let silent = args.silent;
    let dump = args.dump;
    let diagnostic_options: DiagnosticOptions = args.diagnostics.clone().into();

    // read input source
    let mut fs = PhysicalFileSystem::new();
    let mut files = FileRegistry::new();
    let file_id = match get_string_or_file(
        &mut fs,
        &mut files,
        SourceArg {
            file: args.file.as_deref(),
            string: args.string.as_deref(),
            format: args.format.as_deref(),
        },
    ) {
        Ok(Some(file)) => file,
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
    let program = Program::new(LanguageOptions::default(), &fs, &files);
    let compiler = Compiler::from_file(
        &program,
        file_id,
        CompileOptions {
            diagnostic: diagnostic_options.clone(),
            ..Default::default()
        },
    );
    compiler.compile();
    drop(compiler);

    // dump DIR to output
    if !silent {
        let dump_options = DumperOptions::default();
        let strings = program.strings.clone().into_immutable();
        // dump node representation
        if dump.includes_node() {
            for module in program.modules.iter() {
                let module = module.read();
                let tree = module.tree.read();
                let mut dumper = Dumper::new(&strings, &tree, dump_options);
                console::info("=".repeat(80).as_str());
                console::info(format!("{} [NODE]", module.uri).as_str());
                console::info("=".repeat(80).as_str());
                for expression_id in &module.roots {
                    let expression = tree.get(*expression_id);
                    dumper.visit_expression(&tree, *expression_id, expression);
                }
                console::info(&dumper.finish());
            }
        }
        // dump symbol representation
        if dump.includes_symbol() {
            for module in program.modules.iter() {
                let module = module.read();
                let tree = module.tree.read();
                let symbols = module.symbols.read();
                let mut dumper = Dumper::new(&strings, &tree, dump_options);
                console::info("=".repeat(80).as_str());
                console::info(format!("{} [SYMBOL]", module.uri).as_str());
                console::info("=".repeat(80).as_str());
                let scope = symbols.get_scope_by_id(module.scope);
                dumper.visit_scope(&tree, &symbols, module.scope, scope);
                console::info(&dumper.finish());
            }
        }
    }

    // handle diagnostics
    let diagnostics = program.diagnostics.collect().map(&diagnostic_options);
    print_diagnostics(&program, &diagnostics);
    diagnostics.get_status_code()
}
