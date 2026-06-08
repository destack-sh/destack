use destack_parser::source_colorizer;
use destack_repository::{Repository, Revision};
use destack_source::{
    DiagnosticCollection, DiagnosticRenderError, PrintOptions,
    print_diagnostics as print_diagnostics_impl,
};

use clap::Args;

/// Diagnostic command line arguments.
#[derive(Args, Debug, Clone, Default)]
pub struct DiagnosticArgs {}

/// Print diagnostics (and suggestions) to the console.
pub fn print_diagnostics(
    repository: &Repository,
    revision: Revision,
    diagnostics: &DiagnosticCollection,
) -> Result<(), DiagnosticRenderError> {
    let module_count = repository
        .module_ids(revision)
        .map(|modules| modules.len())
        .unwrap_or(0);
    let options = PrintOptions::new()
        .with_line_width(100)
        .with_module_count(module_count)
        .with_colorizer(source_colorizer());
    let file_for_id = |file_id| repository.file(revision, file_id).ok().flatten();
    print_diagnostics_impl(&file_for_id, diagnostics, options)
}
