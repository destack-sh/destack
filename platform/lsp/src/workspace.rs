use destack_source::{FileType, Uri};
use std::str::FromStr;
use tower_lsp_server::lsp_types as lsp;

pub const TRACKED_FILE_TYPES: [FileType; 3] = [
    FileType::Destack,
    FileType::DestackText,
    FileType::DestackBinary,
];

/// Convert an LSP URI to a URI.
pub fn lsp_uri_to_uri(uri: &lsp::Uri) -> Uri {
    Uri::from_string(uri.to_string())
}

/// Convert a URI to an LSP URI. Panics on invalid Uri.
pub fn uri_to_lsp_uri(uri: &Uri) -> lsp::Uri {
    lsp::Uri::from_str(uri.as_ref()).unwrap()
}
