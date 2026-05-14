use crate::{CodegenJsError, CodegenJsResult};
use destack_artifact::DirParsed;
use destack_source::{File, NodeSpanType, Span};
use {destack_fir as fir, destack_js as js};

/// One printed script module payload.
#[derive(Debug, Clone)]
pub struct PrintedScriptModule {
    /// The printed JS or TS text.
    pub code: String,
    /// The output to source markers.
    pub markers: Vec<fir::format::FileMarker>,
}

/// Print one generated script module with the target output policy.
pub fn print_script_module(
    options: js::JsFormatOptions,
    parsed: &DirParsed,
    source_file: &File,
    module: &js::Module,
) -> CodegenJsResult<PrintedScriptModule> {
    if options.mode == js::FormatMode::Minimal {
        return print_script_module_minified(options, parsed, source_file, module);
    }

    print_script_module_pretty(options, parsed, source_file, module)
}

/// Print one generated script module through the direct minified printer.
pub fn print_script_module_minified(
    options: js::JsFormatOptions,
    parsed: &DirParsed,
    source_file: &File,
    module: &js::Module,
) -> CodegenJsResult<PrintedScriptModule> {
    let source_map = CodegenJsSourceMap { parsed };
    let printed = js::print_roots_minified_with_source_map(
        options.file_type,
        &module.tree,
        &module.roots,
        &module.strings,
        &source_map,
    )
    .map_err(|error| match error {})?;

    let _ = source_file;

    Ok(PrintedScriptModule::from_printed_script(printed))
}

/// One source span provider backed by source parts.
#[derive(Debug)]
struct CodegenJsSourceMap<'a> {
    /// The original parsed DIR artifact.
    parsed: &'a DirParsed,
}

impl CodegenJsSourceMap<'_> {
    /// Return the source id for one lowered JS node when one exists.
    fn source_id(&self, tree: &js::Tree, node_id: u32) -> Option<u32> {
        let origin = tree.get_origin(node_id)?;

        // skip nodes lowered from another source module
        if origin.module_id != self.parsed.tree.module_id {
            return None;
        }

        // JS nodes carry DIR ids, so resolve them back to source ids first
        if !self.parsed.tree.has_node_id(origin.node_id) {
            return None;
        }

        Some(self.parsed.tree.get_source(origin.node_id))
    }
}

impl js::JsSourceMap for CodegenJsSourceMap<'_> {
    fn source_span(&self, tree: &js::Tree, node_id: u32) -> Option<Span> {
        let source_id = self.source_id(tree, node_id)?;

        self.parsed.tree.get_span_by_id(source_id)
    }

    fn source_part_span(
        &self,
        tree: &js::Tree,
        node_id: u32,
        span_type: NodeSpanType,
    ) -> Option<Span> {
        let source_id = self.source_id(tree, node_id)?;

        match span_type {
            NodeSpanType::Enclosing => self.parsed.tree.get_span_by_id(source_id),
            NodeSpanType::Main => self.parsed.tree.get_main_span_by_id(source_id),
            other => self.parsed.tree.get_side_span_by_id(source_id, other),
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
    parsed: &DirParsed,
    source_file: &File,
    module: &js::Module,
) -> CodegenJsResult<PrintedScriptModule> {
    let source_map = CodegenJsSourceMap { parsed };
    let roots = module.roots.as_slice();
    let context = js::JsFormatContext {
        options,
        file: source_file,
        tree: &module.tree,
        roots,
        strings: &module.strings,
        source_map: &source_map,
    };
    let mut state = fir::format::FormatState::new(context);
    let mut buffer = fir::format::VecBuffer::new(&mut state);

    // format the root list through the pure JS formatter
    {
        let mut formatter = fir::format::Formatter::new(&mut buffer);
        js::format_roots(&mut formatter, roots).map_err(|error| CodegenJsError::Internal {
            message: format!("failed to format script module: {error}"),
        })?;
    }

    // build and print the fir document
    let document = fir::format::Document::from(buffer.into_vec());

    let printed = fir::print::Printer::new(source_file, state.context().options.as_print_options())
        .print(&document)
        .map_err(|error| CodegenJsError::Internal {
            message: format!("failed to print script module: {error}"),
        })?;

    Ok(PrintedScriptModule {
        code: printed.as_str().to_string(),
        markers: printed.sourcemap().to_vec(),
    })
}
