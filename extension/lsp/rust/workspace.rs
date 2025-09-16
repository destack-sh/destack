use std::collections::HashMap;

use dyst_language_ast::NodeTree;
use dyst_language_diagnostic::Diagnostic;
use dyst_language_session::Session;
use dyst_language_source::{Source, SourceId, Uri};

#[derive(Debug)]
pub struct Workspace {
    /// The root URI of the workspace.
    pub root: Uri,
    /// The language session.
    pub session: Session,

    /// The next source id to allocate.
    next_source_id: u32,
    /// The sources by URI.
    sources_by_uri: HashMap<Uri, Source>,

    /// The AST by URI.
    ast_by_uri: HashMap<Uri, NodeTree>,
    /// The diagnostics by URI.
    diagnostics_by_uri: HashMap<Uri, Vec<Diagnostic>>,
}

impl Workspace {
    pub fn new(root: Uri) -> Self {
        Self {
            root,
            session: Session::new(),
            next_source_id: 0,
            sources_by_uri: HashMap::new(),
            ast_by_uri: HashMap::new(),
            diagnostics_by_uri: HashMap::new(),
        }
    }

    /// Update a source in the workspace.
    pub fn upsert_source(&mut self, uri: Uri, content: String) {
        // update source if exists
        if self.sources_by_uri.contains_key(&uri) {
            let source = self.sources_by_uri.get_mut(&uri).unwrap();
            source.content = content;
            self.ast_by_uri.insert(uri.clone(), NodeTree::new());
        }
        // otherwise, create a new source
        else {
            let source =
                Source::from_string(SourceId::new(self.next_source_id), uri.clone(), content);
            self.next_source_id += 1;
            self.sources_by_uri.insert(uri.clone(), source);
            self.ast_by_uri.insert(uri.clone(), NodeTree::new());
        }

        // reset diagnostics
        self.diagnostics_by_uri.remove(&uri.clone());
    }

    /// Get a source from the workspace.
    pub fn get_source(&self, uri: Uri) -> Option<&Source> {
        self.sources_by_uri.get(&uri)
    }
}
