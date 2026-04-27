use crate::{CodegenJsError, CodegenJsResult};
use destack_artifact::{Ast, DirPatched, ScriptModule};
use destack_fir::format::{Document, FormatState, Formatter, VecBuffer};
use destack_fir::print::Printer as FirPrinter;
use destack_js as js;
use destack_source::{File, NodeSpanType, Span};

/// One printed script module payload.
#[derive(Debug, Clone)]
pub struct PrintedScriptModule {
    /// The printed JS or TS text.
    pub code: String,
    /// The output to source markers.
    pub markers: Vec<destack_fir::format::FileMarker>,
}

/// Print one generated script module with the target output policy.
pub fn print_script_module(
    options: js::JsFormatOptions,
    ast: &Ast,
    dir: &DirPatched,
    source_file: &File,
    module: &ScriptModule,
) -> CodegenJsResult<PrintedScriptModule> {
    if options.mode == js::FormatMode::Minimal {
        return print_script_module_minified(options, ast, dir, source_file, module);
    }

    print_script_module_pretty(options, ast, dir, source_file, module)
}

/// Print one generated script module through the direct minified printer.
pub fn print_script_module_minified(
    options: js::JsFormatOptions,
    ast: &Ast,
    dir: &DirPatched,
    source_file: &File,
    module: &ScriptModule,
) -> CodegenJsResult<PrintedScriptModule> {
    let source_map = CodegenJsSourceMap { ast, dir };
    let strings = module.strings.clone().into_immutable();
    let printed = js::print_roots_minified_with_source_map(
        options.file_type,
        &module.tree,
        &module.roots,
        &strings,
        &source_map,
    )
    .map_err(|error| match error {})?;

    let _ = source_file;

    Ok(PrintedScriptModule::from_printed_script(printed))
}

/// One source span provider backed by AST source parts.
#[derive(Debug)]
struct CodegenJsSourceMap<'a> {
    /// The original source AST.
    ast: &'a Ast,
    /// The patched DIR artifact.
    dir: &'a DirPatched,
}

impl CodegenJsSourceMap<'_> {
    /// Return the AST source id for one lowered JS node when one exists.
    fn source_id(&self, tree: &js::Tree, node_id: u32) -> Option<u32> {
        let (module_id, source_id) = tree.get_source(node_id);

        // skip nodes lowered from another source module
        if module_id != self.ast.id {
            return None;
        }

        // JS nodes carry DIR ids, so resolve them back to AST ids first
        if !self.dir.tree.has_node_id(source_id) {
            return None;
        }

        Some(self.dir.tree.get_source(source_id))
    }
}

impl js::JsSourceMap for CodegenJsSourceMap<'_> {
    fn source_span(&self, tree: &js::Tree, node_id: u32) -> Option<Span> {
        let source_id = self.source_id(tree, node_id)?;

        Some(self.ast.tree.get_span_by_id(source_id))
    }

    fn source_part_span(
        &self,
        tree: &js::Tree,
        node_id: u32,
        span_type: NodeSpanType,
    ) -> Option<Span> {
        let source_id = self.source_id(tree, node_id)?;

        match span_type {
            NodeSpanType::Enclosing => Some(self.ast.tree.get_span_by_id(source_id)),
            NodeSpanType::Main => self.ast.tree.get_main_span_by_id(source_id),
            other => self.ast.tree.get_side_span_by_id(source_id, other),
        }
    }
}

impl PrintedScriptModule {
    /// Build one printed script module from one pure JS print result.
    fn from_printed_script(printed: js::PrintedScript) -> Self {
        Self {
            code: printed.code,
            markers: printed.markers,
        }
    }
}

/// Print one generated script module through the pure formatter.
fn print_script_module_pretty(
    options: js::JsFormatOptions,
    ast: &Ast,
    dir: &DirPatched,
    source_file: &File,
    module: &ScriptModule,
) -> CodegenJsResult<PrintedScriptModule> {
    let source_map = CodegenJsSourceMap { ast, dir };
    let strings = module.strings.clone().into_immutable();
    let roots = module.roots.as_slice();
    let context = js::JsFormatContext {
        options,
        file: source_file,
        tree: &module.tree,
        roots,
        strings: &strings,
        source_map: &source_map,
    };
    let mut state = FormatState::new(context);
    let mut buffer = VecBuffer::new(&mut state);

    // format the root list through the pure JS formatter
    {
        let mut formatter = Formatter::new(&mut buffer);
        js::format_roots(&mut formatter, roots).map_err(|error| CodegenJsError::Internal {
            message: format!("failed to format script module: {error}"),
        })?;
    }

    // build and print the fir document
    let document = Document::from(buffer.into_vec());

    let printed = FirPrinter::new(source_file, state.context().options.as_print_options())
        .print(&document)
        .map_err(|error| CodegenJsError::Internal {
            message: format!("failed to print script module: {error}"),
        })?;

    Ok(PrintedScriptModule {
        code: printed.as_str().to_string(),
        markers: printed.sourcemap().to_vec(),
    })
}
