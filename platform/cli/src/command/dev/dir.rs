use clap::Args;
use destack_dir::{Dumper, DumperOptions, NodeVisitor};
use destack_parser::colorize_source;

use crate::common::{CompileContext, DiagnosticArgs, ProgramArgs, SingleInputArgs};
use crate::console;

/// What to dump from DIR compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum DumpKind {
    /// Dump file/source representation.
    #[value(alias = "f")]
    File,
    /// Dump DIR node representation.
    #[value(alias = "n")]
    Node,
    /// Dump symbol table.
    #[value(alias = "s")]
    Symbol,
    /// Dump all representations.
    #[value(alias = "a")]
    All,
}

#[derive(Args, Debug, Clone)]
pub struct DirArgs {
    /// Input arguments.
    #[command(flatten)]
    pub input: SingleInputArgs,

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

/// Compile source and dump DIR representation.
pub fn run(args: &DirArgs) -> i32 {
    let silent = args.silent;
    let dump_file = args.dump.contains(&DumpKind::File) || args.dump.contains(&DumpKind::All);
    let dump_node = args.dump.contains(&DumpKind::Node) || args.dump.contains(&DumpKind::All);
    let dump_symbol = args.dump.contains(&DumpKind::Symbol) || args.dump.contains(&DumpKind::All);
    let context = CompileContext::for_lint(&args.program, &args.diagnostics);

    // determine input source
    let source = match args.input.to_source() {
        Ok(s) => s,
        Err(e) => {
            console::error(&format!("error: {e}"));
            return 1;
        }
    };

    // resolve to module and enqueue
    if let Err(e) = context.enqueue_source(&source) {
        console::error(&format!("error: {e}"));
        return 1;
    }
    let result = context.compile();

    // dump DIR to output
    if !silent {
        let dump_options = DumperOptions::default();
        let strings = result.program.strings.clone().into_immutable();

        for module in result.program.modules.iter() {
            let module = module.read();

            // dump file representation
            if dump_file {
                let file = result.program.files.get(module.file_id);
                console::info("=".repeat(80).as_str());
                console::info(format!("{} [FILE]", module.uri).as_str());
                console::info("=".repeat(80).as_str());
                console::info(&colorize_source(&file));
            }

            // dump node representation
            if dump_node {
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
            if dump_symbol {
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

    result.finish()
}
