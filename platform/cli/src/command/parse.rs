use clap::{ArgGroup, Args};
use dyst_ast::{Dumper, DumperOptions, NodeVisitor};
use dyst_parser::{Parser, colorize_source};
use dyst_source::DiagnosticOptions;

use crate::command::{
    DiagnosticArgs, DumpFormat, DumpKind, ProgramArgs, SourceArg, get_string_or_file,
    print_diagnostics,
};
use crate::console;

#[derive(Args, Debug, Clone)]
#[command(group(
    ArgGroup::new("source")
        .args(["file", "string"])
        .required(true)
        .multiple(false)
))]
pub struct ParseArgs {
    /// Read input from file.
    #[arg(long)]
    pub file: Option<String>,

    /// Read input from provided string.
    #[arg(long)]
    pub string: Option<String>,

    /// Parse a file with the given format (default: ds).
    #[arg(long = "type", alias = "format", value_name = "FORMAT")]
    pub format: Option<String>,

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
    let dump = DumpFormat::from(args.dump.clone());
    let diagnostic_options: DiagnosticOptions = args.diagnostics.clone().into();
    let program = args.program.setup();

    // read input source
    let file = match get_string_or_file(
        &program,
        SourceArg {
            file: args.file.as_deref(),
            string: args.string.as_deref(),
            format: args.format.as_deref(),
        },
    ) {
        Ok(file) => file,
        Err(e) => {
            console::error(&format!("error: {e}"));
            return 1;
        }
    };

    // parse as implicit module
    let mut parser = Parser::lex_file(file.clone(), program.language);
    let expressions = parser.parse();

    // dump output
    if !silent {
        // dump file representation (colorized source)
        if dump.includes_file() {
            console::info("=".repeat(80).as_str());
            console::info(format!("{} [FILE]", file.uri).as_str());
            console::info("=".repeat(80).as_str());
            console::info(&colorize_source(&file));
        }

        // dump AST node representation
        if dump.includes_node() {
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
