use std::path::Path;

use destack_lsp_server::{UriExt, jsonrpc};
use destack_lsp_types as lsp;
use destack_source::{File, Uri};

use crate::server::error::internal_error;

/// Build an LSP uri for a file system path.
pub(crate) fn path(path: impl AsRef<Path>) -> Option<lsp::Uri> {
    lsp::Uri::from_file_path(path)
}

/// Build the LSP URI for a source file.
pub(crate) fn file(file: &File) -> jsonrpc::Result<lsp::Uri> {
    if let Some(uri) = source(&file.uri) {
        return Ok(uri);
    }

    Err(internal_error(format!(
        "source file {:?} has no representable LSP URI: {}",
        file.id, file.uri
    )))
}

/// Parse one source uri into an LSP uri.
pub(crate) fn source(uri: &Uri) -> Option<lsp::Uri> {
    if let Ok(parsed) = uri.as_ref().parse::<lsp::Uri>() {
        return Some(parsed);
    }

    path(uri.as_ref())
}
