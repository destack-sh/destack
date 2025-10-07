use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::{fs, io};

use destack_file::glob::glob;
use dyst_diagnostic::Diagnostic;
use dyst_session::Session;
use dyst_source::{SourceFormat, SourceId, Uri};
use std::str::FromStr;
use tower_lsp_server::{UriExt, lsp_types as lsp};

use crate::Document;

pub const TRACKED_FORMATS: [SourceFormat; 4] = [
    SourceFormat::Dyst,
    SourceFormat::DystText,
    SourceFormat::DystBinary,
    SourceFormat::DystExecutable,
];

/// The result of a workspace reindex.
#[derive(Debug, Default)]
pub struct WorkspaceReindex {
    /// URIs whose content was refreshed from disk.
    pub updated: Vec<Uri>,
    /// URIs removed from the workspace because the backing file no longer exists.
    pub removed: Vec<Uri>,
}

/// A workspace.
#[derive(Debug)]
pub struct Workspace {
    /// The root URI of the workspace.
    pub root: Uri,
    /// The shared session for the workspace.
    pub session: Session,

    /// The next source ID to use for a new document.
    next_source_id: u32,
    /// All the documents.
    documents: HashMap<Uri, Document>,

    /// The LSP registration ID for the file watcher.
    pub watch_registration_id: Option<String>,
}

impl Workspace {
    /// Create a new workspace.
    pub fn new(root_uri: Uri) -> Self {
        Self {
            root: root_uri,
            session: Session::new(),
            next_source_id: 0,
            documents: HashMap::new(),
            watch_registration_id: None,
        }
    }

    /// Check if a document exists in the workspace.
    pub fn has_document(&self, uri: &Uri) -> bool {
        self.documents.contains_key(uri)
    }

    /// Check if the workspace has a document and it is open.
    pub fn has_open_document(&self, uri: &Uri) -> bool {
        self.documents
            .get(uri)
            .map(|doc| doc.is_open)
            .unwrap_or(false)
    }

    /// Get a document from the workspace.
    pub fn get_document(&self, uri: &Uri) -> Option<&Document> {
        self.documents.get(uri)
    }

    /// Collect URIs for all tracked documents.
    pub fn document_uris(&self) -> Vec<Uri> {
        self.documents.keys().cloned().collect()
    }

    /// Get all diagnostics.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.session.diagnostics
    }

    /// Get diagnostics for a source.
    pub fn get_diagnostics_for_source(&self, source: SourceId) -> Vec<Diagnostic> {
        self.session.get_diagnostics_for_source(source)
    }

    /// Get diagnostics for a predicate.
    pub fn get_diagnostics_for_predicate(
        &self,
        predicate: impl Fn(&Diagnostic) -> bool,
    ) -> Vec<Diagnostic> {
        self.session.get_diagnostics_for_predicate(predicate)
    }

    /// Reset all diagnostics.
    pub fn reset_diagnostics(&mut self) {
        self.session.reset_diagnostics();
    }

    /// Reset diagnostics for a source.
    pub fn reset_diagnostics_for_source(&mut self, source: SourceId) {
        self.session.reset_diagnostics_for_source(source);
    }

    /// Get or create a source ID for a URI.
    fn get_or_create_source_id(&mut self, uri: &Uri) -> SourceId {
        self.documents
            .get(uri)
            .map(|doc| doc.id)
            .unwrap_or_else(|| {
                let id = SourceId::new(self.next_source_id);
                self.next_source_id = self.next_source_id.wrapping_add(1);
                id
            })
    }

    /// Upsert any document into the workspace.
    pub fn upsert_document(
        &mut self,
        uri: &Uri,
        format: SourceFormat,
        is_open: bool,
        content: Vec<u8>,
    ) -> SourceId {
        match format {
            SourceFormat::Dyst | SourceFormat::DystText => {
                let content = String::from_utf8_lossy(&content).to_string();
                self.upsert_text_document(uri, format, is_open, content)
            }
            SourceFormat::DystBinary | SourceFormat::DystExecutable => {
                self.upsert_binary_document(uri, format, is_open, content)
            }
        }
    }

    /// Upsert and parse a document into the workspace.
    pub fn upsert_text_document(
        &mut self,
        uri: &Uri,
        format: SourceFormat,
        is_open: bool,
        content: String,
    ) -> SourceId {
        // get id
        let source_id = self.get_or_create_source_id(uri);

        // reset diagnostics
        self.reset_diagnostics_for_source(source_id);

        // create document
        let document = Document::from_text(
            source_id,
            uri.clone(),
            format,
            is_open,
            content,
            &mut self.session,
        );
        self.documents.insert(uri.clone(), document);

        source_id
    }

    /// Upsert and parse a binary document into the workspace.
    pub fn upsert_binary_document(
        &mut self,
        uri: &Uri,
        format: SourceFormat,
        is_open: bool,
        content: Vec<u8>,
    ) -> SourceId {
        // get id
        let source_id = self.get_or_create_source_id(uri);

        // create document
        let document = Document::from_binary(source_id, uri.clone(), format, is_open, content);
        self.documents.insert(uri.clone(), document);

        source_id
    }

    /// Remove a document from the workspace.
    pub fn remove_document(&mut self, uri: &Uri) {
        if let Some(document) = self.documents.remove(uri) {
            self.reset_diagnostics_for_source(document.id);
        }
    }

    /// Refresh a single document from disk when it is not open.
    pub fn sync_document_from_disk(&mut self, uri: &Uri) -> io::Result<()> {
        // infer the format
        let format = infer_source_format_from_uri(uri).unwrap_or(SourceFormat::Dyst);

        // read the document from disk
        let lsp_uri = uri_to_lsp_uri(uri);
        let path = lsp_uri
            .to_file_path()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "uri is not a file path"))?
            .into_owned();
        let content = fs::read(path)?;

        // upsert the document
        self.upsert_document(uri, format, false, content);

        Ok(())
    }

    /// Rebuild the workspace state from disk for all sources.
    pub fn reindex_all_from_disk(&mut self) -> io::Result<WorkspaceReindex> {
        // build pattern
        let root_uri = uri_to_lsp_uri(&self.root);
        let root_path = root_uri
            .to_file_path()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "workspace root is not a file path",
                )
            })?
            .to_string_lossy()
            .into_owned();

        // collect from workspace tree
        let mut seen_uris = HashSet::new();
        let mut index = WorkspaceReindex::default();
        for format in TRACKED_FORMATS {
            let glob_pattern = format!("{}/{}", root_path, format.glob());
            let paths = glob(&glob_pattern);
            for path in &paths {
                let Some(lsp_uri) = lsp::Uri::from_file_path(path) else {
                    continue;
                };
                let uri = lsp_uri_to_uri(&lsp_uri);
                seen_uris.insert(uri.clone());

                // read the document from disk
                let content = match fs::read(path) {
                    Ok(value) => value,
                    Err(_) => continue,
                };

                // upsert the document
                self.upsert_document(&uri, format, false, content);
                index.updated.push(uri.clone());
            }
        }

        // drop documents that disappeared from disk
        let stale_uris: Vec<Uri> = self
            .documents
            .iter()
            .filter(|(uri, document)| !document.is_open && !seen_uris.contains(uri.as_ref()))
            .map(|(uri, _)| uri.clone())
            .collect();
        for uri in stale_uris {
            self.remove_document(&uri);
            index.removed.push(uri);
        }

        Ok(index)
    }
}

/// Convert an LSP URI to a URI.
pub fn lsp_uri_to_uri(uri: &lsp::Uri) -> Uri {
    Uri::from_string(uri.to_string())
}

/// Convert a URI to an LSP URI. Panics on invalid Uri.
pub fn uri_to_lsp_uri(uri: &Uri) -> lsp::Uri {
    lsp::Uri::from_str(uri.as_ref()).unwrap()
}

/// Infer a source format from a URI.
pub fn infer_source_format_from_uri(uri: &Uri) -> Option<SourceFormat> {
    let lsp_uri = uri_to_lsp_uri(uri);
    infer_source_format_from_lsp_uri(&lsp_uri)
        .or_else(|| infer_source_format_from_str(uri.as_ref()))
}

/// Infer a source format from an LSP URI.
pub fn infer_source_format_from_lsp_uri(uri: &lsp::Uri) -> Option<SourceFormat> {
    infer_source_format_from_path_uri(uri).or_else(|| infer_source_format_from_str(uri.as_str()))
}

/// Infer a source format from a path URI.
fn infer_source_format_from_path_uri(uri: &lsp::Uri) -> Option<SourceFormat> {
    uri.to_file_path()
        .and_then(|path| infer_source_format_from_path(path.as_ref()))
}

/// Infer a source format from a path.
fn infer_source_format_from_path(path: &Path) -> Option<SourceFormat> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    format_from_extension(&extension)
}

/// Infer a source format from a string.
fn infer_source_format_from_str(value: &str) -> Option<SourceFormat> {
    let trimmed = value.split(['?', '#']).next().unwrap_or(value);
    let extension = trimmed.rsplit('.').next()?;
    if extension.contains('/') || extension.contains('\\') {
        return None;
    }
    format_from_extension(&extension.to_ascii_lowercase())
}

/// Infer a source format from a file extension.
fn format_from_extension(extension: &str) -> Option<SourceFormat> {
    match extension {
        "ds" => Some(SourceFormat::Dyst),
        "dst" => Some(SourceFormat::DystText),
        "dsb" => Some(SourceFormat::DystBinary),
        "dsx" => Some(SourceFormat::DystExecutable),
        _ => None,
    }
}
