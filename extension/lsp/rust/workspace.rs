use std::path::Path;

use dyst_source::{FileType, Uri};
use std::str::FromStr;
use tower_lsp_server::{UriExt, lsp_types as lsp};

pub const TRACKED_FORMATS: [FileType; 4] = [
    FileType::Dyst,
    FileType::DystText,
    FileType::DystBinary,
    FileType::DystExecutable,
];

/// Convert an LSP URI to a URI.
pub fn lsp_uri_to_uri(uri: &lsp::Uri) -> Uri {
    Uri::from_string(uri.to_string())
}

/// Convert a URI to an LSP URI. Panics on invalid Uri.
pub fn uri_to_lsp_uri(uri: &Uri) -> lsp::Uri {
    lsp::Uri::from_str(uri.as_ref()).unwrap()
}

/// Infer a source format from a URI.
pub fn infer_source_format_from_uri(uri: &Uri) -> Option<FileType> {
    let lsp_uri = uri_to_lsp_uri(uri);
    infer_source_format_from_lsp_uri(&lsp_uri)
        .or_else(|| infer_source_format_from_str(uri.as_ref()))
}

/// Infer a source format from an LSP URI.
pub fn infer_source_format_from_lsp_uri(uri: &lsp::Uri) -> Option<FileType> {
    infer_source_format_from_path_uri(uri).or_else(|| infer_source_format_from_str(uri.as_str()))
}

/// Infer a source format from a path URI.
fn infer_source_format_from_path_uri(uri: &lsp::Uri) -> Option<FileType> {
    uri.to_file_path()
        .and_then(|path| infer_source_format_from_path(path.as_ref()))
}

/// Infer a source format from a path.
fn infer_source_format_from_path(path: &Path) -> Option<FileType> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    format_from_extension(&extension)
}

/// Infer a source format from a string.
fn infer_source_format_from_str(value: &str) -> Option<FileType> {
    let trimmed = value.split(['?', '#']).next().unwrap_or(value);
    let extension = trimmed.rsplit('.').next()?;
    if extension.contains('/') || extension.contains('\\') {
        return None;
    }
    format_from_extension(&extension.to_ascii_lowercase())
}

/// Infer a source format from a file extension.
fn format_from_extension(extension: &str) -> Option<FileType> {
    match extension {
        "ds" => Some(FileType::Dyst),
        "dst" => Some(FileType::DystText),
        "dsb" => Some(FileType::DystBinary),
        "dsx" => Some(FileType::DystExecutable),
        _ => None,
    }
}
