use std::collections::HashMap;

use dyst_language_diagnostic::Diagnostic;
use dyst_language_session::Session;
use dyst_language_source::{Source, SourceId, Uri};
use std::str::FromStr;
use tower_lsp_server::lsp_types as lsp;

use crate::Document;

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

    /// Get diagnostics for a source. If no source is provided, all diagnostics are returned.
    pub fn get_diagnostics(&self, source: Option<SourceId>) -> Vec<Diagnostic> {
        self.session.get_diagnostics(source)
    }

    /// Reset diagnostics for a source. If no source is provided, all diagnostics are reset.
    pub fn reset_diagnostics(&mut self, source: Option<SourceId>) {
        self.session.reset_diagnostics(source);
    }

    /// Upsert and parse a document into the workspace.
    pub fn upsert_document(&mut self, uri: &Uri, content: String) -> SourceId {
        let source_id = self
            .documents
            .get(uri)
            .map(|doc| doc.source.id)
            .unwrap_or_else(|| {
                let id = SourceId::new(self.next_source_id);
                self.next_source_id = self.next_source_id.saturating_add(1);
                id
            });

        let source = Source::from_string(source_id, uri.clone(), content);
        self.reset_diagnostics(Some(source_id));
        let document = Document::parse(source, &mut self.session);
        self.documents.insert(uri.clone(), document);

        source_id
    }

    /// Remove a document from the workspace.
    pub fn remove_document(&mut self, uri: &Uri) {
        if let Some(document) = self.documents.remove(uri) {
            self.reset_diagnostics(Some(document.source.id));
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
}

/// Convert an LSP URI to a URI.
pub fn lsp_uri_to_uri(uri: &lsp::Uri) -> Uri {
    Uri::from_string(uri.to_string())
}

/// Convert a URI to an LSP URI. Panics on invalid Uri.
pub fn uri_to_lsp_uri(uri: &Uri) -> lsp::Uri {
    lsp::Uri::from_str(uri.as_ref()).unwrap()
}
