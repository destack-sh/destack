use destack_lsp_server::UriExt;
use destack_lsp_types as lsp;
use destack_source::File;

/// Build an LSP uri for a file.
pub(crate) fn lsp_uri_for_file(file: &File) -> Option<lsp::Uri> {
    // prefer a canonical file uri from the path
    if let Some(path) = file.path.as_ref() {
        return lsp::Uri::from_file_path(path);
    }

    // only accept fallback uris that still map back to a file path
    let uri = file.uri.as_ref().parse::<lsp::Uri>().ok()?;
    if uri.to_file_path().is_some() {
        return Some(uri);
    }

    None
}
