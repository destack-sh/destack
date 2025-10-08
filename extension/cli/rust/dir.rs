//! AST parsing subcommand.

use destack_lsp::DocumentBody;
use destack_terminal::{CommandArguments, console};
use dyst_ast::{ModuleFormat, TokenType};
use dyst_compiler::{AstNodeId, Compiler, CompilerOptions};
use dyst_diagnostic::Severity;
use dyst_dir::{DumperOptions, NodeVisitor};
use dyst_package::Workspace;
use dyst_parser::Parser;
use dyst_session::Session;
use dyst_source::{AnnotateOptions, Color, Uri, annotate_source};

use crate::source::read_source;

pub const HELP: &str = r"Parse and compile source into DIR (implicit module).
	--file <path>      Read input from file
	--string <string>  Read input from provided string
    --package <path>   The package to compile (default: auto-detect)
    --standalone       Compile as standalone package (disable auto-detect)
    ";

/// Parse source into an DIR and dump the module.
pub fn run(ctx: CommandArguments) -> i32 {
    let mut session = Session::new();
    let file = ctx.option("file");
    let package = ctx.option("package");
    let standalone = ctx.flag("standalone");

    // load workspace
    let workspace: Workspace = {
        // package workspace
        if let Some(package) = package {
            let package_uri = Uri::from_string(package);
            if let Ok(Some(workspace)) = Workspace::load_containing(&package_uri) {
                workspace
            } else {
                console::error("Failed to load package workspace");
                return 1;
            }
        }
        // detect package workspace from file
        else if file.is_some()
            && !standalone
            && let Ok(Some(workspace)) =
                Workspace::load_containing(&Uri::from_string(file.unwrap()))
        {
            workspace
        }
        // standalone
        else if let Some(file) = file {
            Workspace::empty(Uri::from_string(file))
        }
        // no file or package
        else {
            Workspace::empty(Uri::from_string("<string>"))
        }
    };

    // read input source
    let source = match read_source(&ctx) {
        Ok(source) => source,
        Err(error) => {
            console::error(&format!("Read input error: {error}"));
            return 1;
        }
    };

    // get the definition
    let definition_id = {
        // get the definition from the workspace
        if let Some(file) = file
            && !workspace.has_document(&Uri::from_string(file))
        {
            let document = workspace
                .get_document(&Uri::from_string(file))
                .unwrap();
            match document.body {
                DocumentBody::Text {
                    root_definition_id, ..
                } => root_definition_id,
                DocumentBody::Binary { .. } => None,
            }
        }
        // parse as implicit module if file is not in workspace
        else {
            let mut parser = Parser::prepare(&source, &mut session);
            let module_name = source.uri.last_segment().unwrap_or("<string>");
            let module_name_id = parser.intern_string(module_name);
            let definition_id = parser.with_recovery(
                parser.mark(),
                |parser| {
                    parser
                        .eat_module_body(None, Some(module_name_id), ModuleFormat::Source)
                        .map(Some)
                },
                None,
                TokenType::End,
            );
            parser.finalize();
            definition_id
        }
    };

    // compile the AST to DIR
    let mut compiler = Compiler::new(&workspace, CompilerOptions::default());

    // dump DIR module to output
    let dump_options = DumperOptions::default();
    if let Some(definition_id) = definition_id {
        let definition_id = compiler.lower_definition(AstNodeId::new(definition_id, source.id));
        if let Some(definition_id) = definition_id {
            let mut dumper = compiler.dumper(dump_options);
            dumper.visit_definition(
                &compiler.tree,
                definition_id,
                compiler.tree.get(definition_id),
            );
            console::info(&dumper.finish());
        }
    }

    // print diagnostics
    for diagnostic in &session.diagnostics {
        let annotated = annotate_source(
            &source,
            &diagnostic.primary_span,
            AnnotateOptions {
                max_line_width: 100,
                prefix_lines: 1,
                suffix_lines: 1,
                use_color: true,
            },
        );
        let diagnostic_header =
            Color::Red.apply_bold(&format!("{}: {}", diagnostic.code, diagnostic.message));
        console::error(&diagnostic_header);
        console::info(&annotated);
    }

    // return error if any error diagnostics
    let has_errors = session
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error);
    if has_errors {
        return 1;
    }
    0
}
