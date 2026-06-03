use std::path::Path;

use destack_lsp_server::UriExt;
use destack_lsp_types as lsp;
use destack_source::{File, Uri};

/// Build an LSP uri for a file system path.
pub(crate) fn lsp_uri_for_path(path: impl AsRef<Path>) -> Option<lsp::Uri> {
    lsp::Uri::from_file_path(path)
}

/// Build an LSP uri for a file.
pub(crate) fn lsp_uri_for_file(file: &File) -> Option<lsp::Uri> {
    // prefer the file uri identity because it preserves the original path variant
    if let Some(uri) = lsp_uri_for_source_uri(&file.uri) {
        return Some(uri);
    }

    // otherwise accept the source-uri path fallback used by older file snapshots
    if let Some(path) = file.uri.to_path_buf() {
        return lsp_uri_for_path(path);
    }

    file.path.as_ref().and_then(lsp_uri_for_path)
}

/// Parse one source uri into an LSP uri.
pub(crate) fn lsp_uri_for_source_uri(uri: &Uri) -> Option<lsp::Uri> {
    uri.as_ref().parse().ok()
}
