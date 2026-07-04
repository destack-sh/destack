use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::{FileId, ModuleId, ProfileId, Span};
use serde::{Deserialize, Serialize};

/// One module in one query profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Module {
    /// The queried module.
    pub module_id: ModuleId,
    /// The queried profile.
    pub profile_id: ProfileId,
}

/// One byte position in a module source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Position {
    /// The queried module profile.
    pub module: Module,
    /// The source file.
    pub file_id: FileId,
    /// The byte offset in the source file.
    pub offset: u32,
}

/// One source range in a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Range {
    /// The queried module profile.
    pub module: Module,
    /// The source range.
    pub span: Span,
}

/// One source-backed target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Target {
    /// The target module profile.
    pub module: Module,
    /// The full source range.
    pub span: Span,
    /// The primary selection range.
    pub selection_span: Option<Span>,
    /// The target symbol when known.
    pub symbol_id: Option<dir::GlobalSymbolId>,
    /// The target node when known.
    pub node_id: Option<dir::GlobalNodeIdAny>,
}

impl Target {
    /// Create a source target with no resolved identity.
    pub fn new(module: Module, span: Span) -> Self {
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
    pub fn with_symbol_id(mut self, symbol_id: dir::GlobalSymbolId) -> Self {
        self.symbol_id = Some(symbol_id);

        self
    }

    /// Return this target with a node id.
    pub fn with_node_id(mut self, node_id: dir::GlobalNodeIdAny) -> Self {
        self.node_id = Some(node_id);

        self
    }
}

/// Text in display formats understood by clients.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Text {
    /// Plain text.
    pub plain: Option<String>,
    /// Markdown text.
    pub markdown: Option<String>,
}
