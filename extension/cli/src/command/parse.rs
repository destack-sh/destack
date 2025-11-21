use clap::{ArgGroup, Args};
use dyst_ast::{Dumper, DumperOptions, NodeVisitor};
use dyst_dir::Session;
use dyst_parser::Parser;
use dyst_source::{DiagnosticOptions, FileRegistry, LanguageOptions};

use crate::command::{DiagnosticOptionsArgs, SourceArg, get_string_or_file, print_diagnostics};
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

    /// Don't print anything to the console (except errors).
    #[arg(long)]
    pub silent: bool,

    #[command(flatten)]
    pub diagnostics: DiagnosticOptionsArgs,
}

/// Parse source into an AST and dump the statements.
pub fn run(args: &ParseArgs) -> i32 {
    let silent = args.silent;
    let diagnostic_options: DiagnosticOptions = args.diagnostics.clone().into();

    // read input source
    let mut files = FileRegistry::new();
    let file_id = match get_string_or_file(
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

    // parse as implicit module
    let mut session = Session::new(LanguageOptions::default(), &files);
    let file = files.get(file_id).unwrap();
    let mut parser = Parser::lex_file(file, session.language, &mut session.diagnostics);
    let expressions = parser.parse();

    // dump AST to output
    if !silent {
        let dump_options = DumperOptions::default();
        let strings = parser.strings.clone().into_immutable();
        let mut dumper = Dumper::new(&strings, &parser.tree, dump_options);
        for expression in expressions {
            dumper.visit_expression(&parser.tree, expression, parser.tree.get(expression));
        }
        console::info(&dumper.finish());
    }

    // handle diagnostics
    let diagnostics = session.diagnostics.collect().map(&diagnostic_options);
    print_diagnostics(&session, &diagnostics);
    diagnostics.get_status_code()
}
