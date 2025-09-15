use std::collections::HashMap;

use dyst_language_ast::NodeTree;
use dyst_language_session::Session;
use dyst_language_source::{Source, SourceId};
use ls_types::Uri;

#[derive(Debug)]
pub struct Workspace {
    pub root: Uri,
    pub session: Session,

    next_source_id: u32,
    sources_by_uri: HashMap<String, Source>,

    ast_by_uri: HashMap<Uri, NodeTree>,

    // diagnostics: Vec<Diagnostic>,
}

impl Workspace {
    pub fn new(root: Uri) -> Self {
        Self {
            root,
            session: Session::new(),

            next_source_id: 0,
            sources_by_uri: HashMap::new(),
            ast_by_uri: HashMap::new(),
            // diagnostics: Vec::new(),
        }
    }

    /// Allocate and add a source to the workspace.
    pub fn allocate_source(&mut self, uri: Uri, name: String, content: String) -> SourceId {
        let source_id = SourceId::new(self.next_source_id);
        self.next_source_id += 1;
        self.add_source(uri, Source::from_string(source_id, name, content));
        source_id
    }

    /// Add a source to the workspace.
    pub fn add_source(&mut self, uri: Uri, source: Source) {
        self.sources_by_uri.insert(uri.to_string(), source);
        self.ast_by_uri.insert(uri, NodeTree::new());
    }

    /// Get a source from the workspace.
    pub fn get_source(&self, uri: Uri) -> Option<&Source> {
        self.sources_by_uri.get(&uri.to_string())
    }
}
