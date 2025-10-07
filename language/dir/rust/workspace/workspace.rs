use std::collections::HashMap;

use dyst_diagnostic::Diagnostic;
use dyst_session::Session;
use dyst_source::{Source, SourceFormat, SourceId, Uri};

use crate::Document;

pub const TRACKED_FORMATS: [SourceFormat; 4] = [
    SourceFormat::Dyst,
    SourceFormat::DystText,
    SourceFormat::DystBinary,
    SourceFormat::DystExecutable,
];

/// A workspace is a set of Documents in a shared Session.
#[derive(Debug)]
pub struct Workspace {
    /// The root URI of the workspace.
    pub root: Uri,
    /// The shared session for the workspace.
    pub session: Session,

    /// The next source ID to use for a new document.
    pub(super) next_source_id: u32,
    /// All the documents.
    pub(crate) documents: HashMap<Uri, Document>,
}

#[cfg(test)]
impl Default for Workspace {
    fn default() -> Self {
        Self::empty(Uri::from_string("test"))
    }
}

impl Workspace {
    /// Create a new empty workspace.
    pub fn empty(root_uri: Uri) -> Self {
        Self {
            root: root_uri,
            session: Session::new(),
            next_source_id: 0,
            documents: HashMap::new(),
        }
    }

    /// Create a new workspace from a set of Sources.
    pub fn from_sources(root_uri: Uri, sources: impl IntoIterator<Item = Source>) -> Self {
        let mut workspace = Self::empty(root_uri);
        for source in sources {
            workspace.upsert_text_document(
                &source.uri,
                source.format,
                false,
                source.content,
            );
        }
        workspace
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
}
