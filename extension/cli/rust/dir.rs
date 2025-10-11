//! AST parsing subcommand.

use destack_terminal::{CommandArguments, console};
use dyst_compiler::{Compiler, CompilerOptions};
use dyst_diagnostic::Severity;
use dyst_dir::{DumperOptions, NodeVisitor};
use dyst_package::{DocumentBody, Workspace};
use dyst_session::Session;
use dyst_source::{AnnotateOptions, Color, SourceFormat, Uri, annotate_source};

use crate::source::read_source;

pub const HELP: &str = r"Parse and compile source into DIR (implicit module).
	--file <path>      Read input from file
	--string <string>  Read input from provided string
    --package <path>   The package to compile (default: auto-detect)
    --standalone       Compile as standalone package (disable auto-detect)
    --verbose          Print verbose output
    ";

/// Parse source into an DIR and dump the module.
pub fn run(ctx: CommandArguments) -> i32 {
    let session = Session::new();
    let file = ctx.option("file");
    let package = ctx.option("package");
    let standalone = ctx.flag("standalone");
    let verbose = ctx.flag("verbose");

    // load workspace
    let mut workspace: Workspace = {
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
        // load workspace from file
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
    if verbose {
        console::debug(&format!("Workspace: {workspace:?}"));
    }

    // read input source
    let source = match read_source(&ctx) {
        Ok(source) => source,
        Err(error) => {
            console::error(&format!("Read input error: {error}"));
            return 1;
        }
    };

    // get the document & definition
    let (source_id, definition_id) = {
        // get the definition from the workspace
        if let Some(file) = file
            && workspace.has_document(&Uri::from_string(file))
        {
            let document = workspace.get_document(&Uri::from_string(file)).unwrap();
            match &document.body {
                DocumentBody::Text {
                    root_definition_id, ..
                } => (document.id, *root_definition_id),
                DocumentBody::Binary { .. } => panic!("binary document not supported"),
            }
        }
        // add source to workspace if file is not in workspace
        else {
            let uri = Uri::from_string(source.name.clone());
            let source_id = workspace.upsert_text_document(
                &uri,
                SourceFormat::DystText,
                true,
                source.content.clone(),
            );
            let document = workspace.get_document_by_id(source_id).unwrap();
            match &document.body {
                DocumentBody::Text {
                    root_definition_id, ..
                } => (source_id, *root_definition_id),
                DocumentBody::Binary { .. } => panic!("binary document not supported"),
            }
        }
    };

    // compile the AST to DIR
    let mut compiler = Compiler::new(&workspace, CompilerOptions::default());
    let dir_tree = workspace
        .get_document_ast_by_id(source_id)
        .unwrap_or_else(|| panic!("document ast not found: {source_id:?}"));
    let definition_id = compiler.lower_definition(source_id, dir_tree, definition_id);
    compiler.finalize();

    // dump the DIR
    let dump_options = DumperOptions::default();
    let mut dumper = compiler.dumper(dump_options);
    dumper.visit_definition(
        &compiler.tree,
        definition_id,
        compiler.tree.get(definition_id),
    );
    console::info(&dumper.finish());

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
