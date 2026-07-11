use crate::EmitError;
use destack_artifact::DirParsed;
use destack_fir as fir;
use destack_js as js;
use destack_source::{File, NodeSpanType, Span};

/// One printed JS module payload.
#[derive(Debug, Clone)]
pub(crate) struct PrintedJsModule {
    /// The printed JS or TS text.
    pub code: String,
    /// The output to source markers.
    pub markers: Vec<fir::format::FileMarker>,
}

/// Print one emitted JS module with the target output policy.
pub(crate) fn print_js_module(
    options: js::JsFormatOptions,
    parsed: &DirParsed,
    source_file: &File,
    module: &js::Module,
) -> Result<PrintedJsModule, EmitError> {
    if options.mode == js::FormatMode::Minimal {
        return print_js_module_minified(options, parsed, source_file, module);
    }

    print_js_module_pretty(options, parsed, source_file, module)
}

/// Print one emitted JS module through the direct minified printer.
pub(crate) fn print_js_module_minified(
    options: js::JsFormatOptions,
    parsed: &DirParsed,
    source_file: &File,
    module: &js::Module,
) -> Result<PrintedJsModule, EmitError> {
    let source_map = SourceMap { parsed };
    let printed = js::print_roots_minified_with_source_map(
        options.format,
        &module.tree,
        &module.roots,
        &module.strings,
        &source_map,
    )
    .map_err(|error| match error {})?;

    let _ = source_file;

    Ok(PrintedJsModule::from_printed_script(printed))
}

/// One source span provider backed by source parts.
#[derive(Debug)]
struct SourceMap<'a> {
    /// The original parsed DIR artifact.
    parsed: &'a DirParsed,
}

impl SourceMap<'_> {
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

impl js::JsSourceMap for SourceMap<'_> {
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

impl PrintedJsModule {
    /// Build one printed JS module from one pure JS print result.
    fn from_printed_script(printed: js::PrintedScript) -> Self {
        Self {
            code: printed.code,
            markers: printed.markers,
        }
    }
}

/// Print one emitted JS module through the pure formatter.
fn print_js_module_pretty(
    options: js::JsFormatOptions,
    parsed: &DirParsed,
    source_file: &File,
    module: &js::Module,
) -> Result<PrintedJsModule, EmitError> {
    let source_map = SourceMap { parsed };
    let roots = module.roots.as_slice();
    let context = js::JsFormatContext {
        options,
        file: source_file,
        tree: &module.tree,
        roots,
        strings: &module.strings,
        source_map: &source_map,
    };
    let allocator = fir::format::Allocator::default();
    let roots = fir::format::format_with(|formatter| js::format_roots(formatter, roots));
    let formatted = fir::format!(&allocator, context, [roots]).map_err(|error| {
        print_internal_error(parsed, format!("failed to format JS module: {error}"))
    })?;

    // print the completed FIR document
    let printed = formatted.print().map_err(|error| {
        print_internal_error(parsed, format!("failed to print JS module: {error}"))
    })?;

    Ok(PrintedJsModule {
        code: printed.as_str().to_string(),
        markers: printed.sourcemap().to_vec(),
    })
}

/// Build one internal JS print error.
fn print_internal_error(parsed: &DirParsed, message: String) -> EmitError {
    EmitError::Internal {
        anchor: parsed.tree.module_id.into(),
        module: parsed.tree.module_id,
        message,
    }
}
