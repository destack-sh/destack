use destack_lsp_server::UriExt;
use destack_lsp_types as lsp;
use destack_source::File;

/// Build an LSP uri for a file.
pub(crate) fn lsp_uri_for_file(file: &File) -> Option<lsp::Uri> {
    // require canonical file paths for lsp locations
    let path = file.path.as_ref()?;

    lsp::Uri::from_file_path(path)
}
