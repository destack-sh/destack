use crate::EmitError;
use tspp_artifact::DirParsed;
use tspp_fir as fir;
use tspp_js as js;
use tspp_source::File;

/// One printed JS module payload.
#[derive(Debug, Clone)]
pub(crate) struct PrintedJsModule {
    /// The printed JavaScript text.
    pub code: String,
    /// The output to source markers.
    pub markers: Vec<fir::format::FileMarker>,
}

/// Print one emitted JS module with the target output policy.
pub(crate) fn print_js_module(
    options: js::Options,
    parsed: &DirParsed,
    source_file: &File,
    module: &js::Module,
) -> Result<PrintedJsModule, EmitError> {
    let roots = module.roots.as_slice();
    let context = js::Context {
        options,
        file: source_file,
        tree: &module.tree,
        roots,
        strings: &module.strings,
        source: Some(&parsed.tree),
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
