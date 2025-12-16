use clap::Args;
use destack_ast::{Dumper, DumperOptions, NodeVisitor};
use destack_parser::{Parser, colorize_source};
use destack_source::{DiagnosticOptions, LanguageType};

use crate::common::{DiagnosticArgs, ProgramArgs, SingleInputArgs, load_source, print_diagnostics};
use crate::console;

/// What to dump from parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum DumpKind {
    /// Dump file/source representation.
    #[value(alias = "f")]
    File,
    /// Dump AST/node representation.
    #[value(alias = "n")]
    Node,
    /// Dump all representations.
    #[value(alias = "a")]
    All,
}

#[derive(Args, Debug, Clone)]
pub struct ParseArgs {
    /// Input arguments.
    #[command(flatten)]
    pub input: SingleInputArgs,

    /// Dump output format(s): file|f, node|n (comma-separated).
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

/// Parse source into an AST and dump the statements.
pub fn run(args: &ParseArgs) -> i32 {
    let silent = args.silent;
    let dump_file = args.dump.contains(&DumpKind::File) || args.dump.contains(&DumpKind::All);
    let dump_node = args.dump.contains(&DumpKind::Node) || args.dump.contains(&DumpKind::All);
    let diagnostic_options: DiagnosticOptions = args.diagnostics.clone().into();
    let session = args.program.setup();

    // get the program from the session
    let program = session
        .programs
        .iter()
        .next()
        .map(|entry| entry.value().clone())
        .expect("session should have a program after setup");

    // determine input source
    let source = match args.input.to_source() {
        Ok(s) => s,
        Err(e) => {
            console::error(&format!("error: {e}"));
            return 1;
        }
    };

    // load source
    let file = match load_source(&program, &source) {
        Ok(f) => f,
        Err(e) => {
            console::error(&format!("error: {e}"));
            return 1;
        }
    };

    // parse as implicit module
    let language_type = LanguageType::from(file.ty);
    let mut parser = Parser::lex_file(file.clone(), language_type);
    let expressions = parser.parse();
    program.diagnostics.merge_from(&parser.diagnostics);

    // dump output
    if !silent {
        // dump file representation (colorized source)
        if dump_file {
            console::info("=".repeat(80).as_str());
            console::info(format!("{} [FILE]", file.uri).as_str());
            console::info("=".repeat(80).as_str());
            console::info(&colorize_source(&file));
        }

        // dump AST node representation
        if dump_node {
            let dump_options = DumperOptions::default();
            let strings = parser.strings.clone().into_immutable();
            let mut dumper = Dumper::new(&strings, &parser.tree, dump_options);
            console::info("=".repeat(80).as_str());
            console::info(format!("{} [NODE]", file.uri).as_str());
            console::info("=".repeat(80).as_str());
            for expression in expressions {
                dumper.visit_expression(&parser.tree, expression, parser.tree.get(expression));
            }
            console::info(&dumper.finish());
        }
    }

    // handle diagnostics
    let diagnostics = program.diagnostics.collect().map(&diagnostic_options);
    print_diagnostics(&program, &diagnostics);
    diagnostics.get_status_code()
}
