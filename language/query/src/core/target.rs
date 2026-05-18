use destack_dir as dir;
use destack_source::{FileId, ModuleId, ProfileId, Span};
use serde::{Deserialize, Serialize};

/// One module in one query profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QueryModule {
    /// The queried module.
    pub module_id: ModuleId,
    /// The queried profile.
    pub profile_id: ProfileId,
}

/// One byte position in a module source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QueryPosition {
    /// The queried module profile.
    pub module: QueryModule,
    /// The source file.
    pub file_id: FileId,
    /// The byte offset in the source file.
    pub offset: u32,
}

/// One source range in a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QueryRange {
    /// The queried module profile.
    pub module: QueryModule,
    /// The source range.
    pub span: Span,
}

/// One source-backed query target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QueryTarget {
    /// The target module profile.
    pub module: QueryModule,
    /// The full source range.
    pub span: Span,
    /// The primary selection range.
    pub selection_span: Option<Span>,
    /// The target symbol when known.
    pub symbol_id: Option<dir::GlobalSymbolId>,
    /// The target node when known.
    pub node_id: Option<dir::GlobalNodeIdAny>,
}

impl QueryTarget {
    /// Create a source target with no resolved identity.
    pub fn span(module: QueryModule, span: Span) -> Self {
        Self {
            module,
            span,
            selection_span: None,
            symbol_id: None,
            node_id: None,
        }
    }

    /// Return this target with a selection span.
    pub fn with_selection_span(mut self, selection_span: Span) -> Self {
        self.selection_span = Some(selection_span);

        self
    }

    /// Return this target with a symbol id.
    pub fn with_symbol(mut self, symbol_id: dir::GlobalSymbolId) -> Self {
        self.symbol_id = Some(symbol_id);

        self
    }

    /// Return this target with a node id.
    pub fn with_node(mut self, node_id: dir::GlobalNodeIdAny) -> Self {
        self.node_id = Some(node_id);

        self
    }
}

/// Query text in display formats understood by clients.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryText {
    /// Plain text.
    pub plain: Option<String>,
    /// Markdown text.
    pub markdown: Option<String>,
}
