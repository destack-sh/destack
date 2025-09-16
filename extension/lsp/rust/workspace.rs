use std::collections::HashMap;

use dyst_language_ast::{BlockFormat, Module, NodeId, NodeTree, Parser};
use dyst_language_session::Session;
use dyst_language_source::{Source, SourceId, Uri as DystUri};
use dyst_language_token::{TokenSpan, TokenType};
use tower_lsp_server::lsp_types as lsp;

use crate::semantic;

#[derive(Debug)]
pub struct Workspace {
    root_uri: lsp::Uri,
    session: Session,
    next_source_id: u32,
    documents: HashMap<lsp::Uri, DocumentState>,
    watch_registration_id: Option<String>,
}

#[derive(Debug, Clone)]
struct DocumentState {
    source: Source,
    tokens: Vec<TokenSpan>,
    tree: NodeTree,
    root_module: Option<NodeId<Module>>,
}

impl Workspace {
    pub fn new(root_uri: lsp::Uri) -> Self {
        Self {
            root_uri,
            session: Session::new(),
            next_source_id: 0,
            documents: HashMap::new(),
            watch_registration_id: None,
        }
    }

    pub fn root_uri(&self) -> &lsp::Uri {
        &self.root_uri
    }

    pub fn watch_registration_id(&self) -> Option<&str> {
        self.watch_registration_id.as_deref()
    }

    pub fn set_watch_registration_id(&mut self, id: String) {
        self.watch_registration_id = Some(id);
    }

    pub fn clear_watch_registration_id(&mut self) {
        self.watch_registration_id = None;
    }

    pub fn upsert_document(&mut self, uri: &lsp::Uri, content: String) {
        let dyst_uri = DystUri::from_string(uri.as_str());
        let source_id = self
            .documents
            .get(uri)
            .map(|doc| doc.source.id)
            .unwrap_or_else(|| {
                let id = SourceId::new(self.next_source_id);
                self.next_source_id = self.next_source_id.saturating_add(1);
                id
            });

        let source = Source::from_string(source_id, dyst_uri, content);
        let (tokens, tree, root_module) = self.parse(&source);

        let state = DocumentState {
            source,
            tokens,
            tree,
            root_module,
        };
        self.documents.insert(uri.clone(), state);
    }

    pub fn remove_document(&mut self, uri: &lsp::Uri) {
        self.documents.remove(uri);
    }

    pub fn has_document(&self, uri: &lsp::Uri) -> bool {
        self.documents.contains_key(uri)
    }

    pub fn semantic_tokens_full(&self, uri: &lsp::Uri) -> Option<Vec<lsp::SemanticToken>> {
        let doc = self.documents.get(uri)?;
        let tree = doc.root_module.map(|root| (&doc.tree, root));
        semantic::collect_full_tokens(&doc.source, &doc.tokens, tree)
    }

    pub fn semantic_tokens_range(
        &self,
        uri: &lsp::Uri,
        range: &lsp::Range,
    ) -> Option<Vec<lsp::SemanticToken>> {
        let doc = self.documents.get(uri)?;
        let tree = doc.root_module.map(|root| (&doc.tree, root));
        semantic::collect_range_tokens(&doc.source, &doc.tokens, tree, range)
    }

    fn parse(&mut self, source: &Source) -> (Vec<TokenSpan>, NodeTree, Option<NodeId<Module>>) {
        let mut parser = Parser::from_source(source, &mut self.session);
        let root_module = parser.with_recovery(
            parser.mark(),
            |parser| {
                parser
                    .eat_module_body(None, BlockFormat::Implicit)
                    .map(Some)
            },
            None,
            TokenType::End,
        );
        let tokens = parser.tokens.clone();
        let tree = parser.tree.clone();
        (tokens, tree, root_module)
    }
}
