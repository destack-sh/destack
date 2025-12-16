use destack_source::{FileId, Span};

use crate::Session;

/// Kind of document symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    File,
    Module,
    Namespace,
    Package,
    Class,
    Method,
    Property,
    Field,
    Constructor,
    Enum,
    Interface,
    Function,
    Variable,
    Constant,
    String,
    Number,
    Boolean,
    Array,
    Object,
    Key,
    Null,
    EnumMember,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

/// A symbol in a document (for outline view).
#[derive(Debug, Clone)]
pub struct DocumentSymbol {
    /// The symbol's name.
    pub name: String,
    /// Additional detail (e.g., signature).
    pub detail: Option<String>,
    /// The kind of symbol.
    pub kind: SymbolKind,
    /// The full range of the symbol (including body).
    pub range: Span,
    /// The range of the symbol's name.
    pub selection_range: Span,
    /// Children symbols (for hierarchical outline).
    pub children: Vec<DocumentSymbol>,
}

impl DocumentSymbol {
    /// Create a new document symbol.
    pub fn new(name: impl Into<String>, kind: SymbolKind, range: Span) -> Self {
        Self {
            name: name.into(),
            detail: None,
            kind,
            range,
            selection_range: range,
            children: Vec::new(),
        }
    }

    /// Set the detail.
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Set the selection range.
    pub fn with_selection_range(mut self, selection_range: Span) -> Self {
        self.selection_range = selection_range;
        self
    }

    /// Add a child symbol.
    pub fn with_child(mut self, child: DocumentSymbol) -> Self {
        self.children.push(child);
        self
    }
}

/// Get all symbols in a document (for outline view).
pub fn document_symbols(_session: &Session, _file: FileId) -> Vec<DocumentSymbol> {
    // walk the AST/DIR and collect:
    // - function declarations
    // - class/struct declarations
    // - interface declarations
    // - enum declarations
    // - variable declarations (top-level)
    // - type aliases
    // - namespace declarations
    todo!("#Incomplete: document_symbols")
}

/// A symbol in the workspace (flat list for workspace symbol search).
#[derive(Debug, Clone)]
pub struct WorkspaceSymbol {
    /// The symbol's name.
    pub name: String,
    /// The kind of symbol.
    pub kind: SymbolKind,
    /// The file containing the symbol.
    pub file: FileId,
    /// The location of the symbol.
    pub range: Span,
    /// Container name (e.g., class name for methods).
    pub container: Option<String>,
}

/// Search for symbols across the workspace.
pub fn workspace_symbols(
    _session: &Session,
    _query: &str,
) -> Vec<WorkspaceSymbol> {
    // search all modules for symbols matching the query
    // support fuzzy matching
    todo!("#Incomplete: workspace_symbols")
}
