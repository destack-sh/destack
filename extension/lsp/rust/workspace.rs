use std::collections::{HashMap, HashSet};
use std::{fs, io};

use destack_library_file::glob::glob;
use dyst_language_diagnostic::Diagnostic;
use dyst_language_session::Session;
use dyst_language_source::{Source, SourceId, Uri};
use std::str::FromStr;
use tower_lsp_server::{UriExt, lsp_types as lsp};

use crate::Document;

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
    /// All the source states.
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

    /// Get a document from the workspace.
    pub fn get_document(&self, uri: &Uri) -> Option<&Document> {
        self.documents.get(uri)
    }

    /// Collect URIs for all tracked documents.
    pub fn document_uris(&self) -> Vec<Uri> {
        self.documents.keys().cloned().collect()
    }

    /// Get diagnostics for a source. If no source is provided, all diagnostics are returned.
    pub fn get_diagnostics(&self, source: Option<SourceId>) -> Vec<Diagnostic> {
        self.session.get_diagnostics(source)
    }

    /// Get all diagnostics.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.session.diagnostics
    }

    /// Get diagnostics for a predicate.
    pub fn get_diagnostics_for_predicate(
        &self,
        predicate: impl Fn(&Diagnostic) -> bool,
    ) -> Vec<Diagnostic> {
        self.session.get_diagnostics_for_predicate(predicate)
    }

    /// Reset diagnostics for a source. If no source is provided, all diagnostics are reset.
    pub fn reset_diagnostics(&mut self, source: Option<SourceId>) {
        self.session.reset_diagnostics(source);
    }

    /// Upsert and parse a document into the workspace.
    pub fn upsert_document(&mut self, uri: &Uri, content: String, is_open: bool) -> bool {
        let source_id = self
            .documents
            .get(uri)
            .map(|doc| doc.source.id)
            .unwrap_or_else(|| {
                let id = SourceId::new(self.next_source_id);
                self.next_source_id = self.next_source_id.saturating_add(1);
                id
            });

        if let Some(existing) = self.documents.get_mut(uri) {
            if existing.is_open && !is_open {
                return false;
            }
            existing.is_open = is_open;
        }

        let source = Source::from_string(source_id, uri.clone(), content);
        self.reset_diagnostics(Some(source_id));
        let document = Document::parse(source, &mut self.session, is_open);
        self.documents.insert(uri.clone(), document);

        true
    }

    /// Remove a document from the workspace.
    pub fn remove_document(&mut self, uri: &Uri) {
        if let Some(document) = self.documents.remove(uri) {
            self.reset_diagnostics(Some(document.source.id));
        }
    }

    /// Update the open flag for a document if it exists.
    pub fn set_document_is_open(&mut self, uri: &Uri, is_open: bool) {
        if let Some(document) = self.documents.get_mut(uri) {
            document.is_open = is_open;
        }
    }

    /// Refresh a single document from disk when it is not open.
    pub fn sync_document_from_disk(&mut self, uri: &Uri) -> io::Result<bool> {
        if self
            .documents
            .get(uri)
            .map(|document| document.is_open)
            .unwrap_or(false)
        {
            return Ok(false);
        }

        // read the document from disk
        let lsp_uri = uri_to_lsp_uri(uri);
        let path = lsp_uri
            .to_file_path()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "uri is not a file path"))?
            .into_owned();
        let content = fs::read_to_string(&path)?;

        // upsert the document
        Ok(self.upsert_document(uri, content, false))
    }

    /// Rebuild the workspace state from disk for all `.ds` sources.
    pub fn reindex_from_disk(&mut self) -> io::Result<WorkspaceReindex> {
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
        let glob_pattern = format!("{}/**/*.ds", root_path);

        // collect from workspace tree
        let mut seen_uris = HashSet::new();
        let mut index = WorkspaceReindex::default();
        let paths = glob(&glob_pattern);
        for path in &paths {
            let Some(lsp_uri) = lsp::Uri::from_file_path(path) else {
                continue;
            };
            let uri = lsp_uri_to_uri(&lsp_uri);
            seen_uris.insert(uri.clone());

            // read the document from disk
            let content = match fs::read_to_string(path) {
                Ok(value) => value,
                Err(_) => continue, // ignore?
            };

            // upsert the document
            if self.upsert_document(&uri, content, false) {
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
