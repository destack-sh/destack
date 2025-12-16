use std::panic;

use destack_dir::Declaration;
use destack_source::{FileId, Span};

use crate::Session;
use crate::query::common::get_module_by_file_id;

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
pub fn document_symbols(session: &Session, file: FileId) -> Vec<DocumentSymbol> {
    let Some(module) = get_module_by_file_id(session, file) else {
        return Vec::new();
    };

    let module_guard = module.read();
    let dir_tree = module_guard.dir.tree.read();

    let mut symbols = Vec::new();

    // iterate through all declarations
    for (declaration_id, declaration) in dir_tree.iter_nodes_of_type::<Declaration>() {
        // get the kind based on declaration type
        let kind = match declaration {
            Declaration::Function { .. } => SymbolKind::Function,
            Declaration::Struct { .. } => SymbolKind::Struct,
            Declaration::Class { .. } => SymbolKind::Class,
            Declaration::Interface { .. } => SymbolKind::Interface,
            Declaration::Enum { .. } => SymbolKind::Enum,
            Declaration::Namespace { .. } => SymbolKind::Namespace,
            Declaration::Type { .. } => SymbolKind::TypeParameter,
            Declaration::Extension { .. } => SymbolKind::Class,
        };

        // get the declaration name using the descriptor method
        let descriptor = declaration.descriptor();
        let name = descriptor
            .name
            .map(|string_id| module_guard.ast.strings.get(string_id).to_string())
            .unwrap_or_else(|| "<anonymous>".to_string());

        // get spans
        let ast_node_id = dir_tree.get_source(declaration_id.id);
        let full_span = module_guard.ast.tree.source_map.get(ast_node_id);
        let range = Span::new(file, full_span.start, full_span.end);

        // get main span (name span) if available
        let selection_range = module_guard
            .ast
            .tree
            .source_map
            .get_main(ast_node_id)
            .map(|span| Span::new(file, span.start, span.end))
            .unwrap_or(range);

        symbols.push(DocumentSymbol::new(name, kind, range).with_selection_range(selection_range));
    }

    symbols
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
///
/// Returns symbols whose names contain the query string (case-insensitive).
pub fn workspace_symbols(session: &Session, query: &str) -> Vec<WorkspaceSymbol> {
    let mut symbols = Vec::new();
    let query_lower = query.to_lowercase();

    // search all modules
    for module in session.modules.iter() {
        let module_guard = module.read();
        let dir_tree = module_guard.dir.tree.read();
        let file = module_guard.file_id;

        // iterate through all declarations
        for (declaration_id, declaration) in dir_tree.iter_nodes_of_type::<Declaration>() {
            // get the declaration name
            let descriptor = declaration.descriptor();
            let Some(string_id) = descriptor.name else {
                continue;
            };

            let name = module_guard.ast.strings.get(string_id).to_string();

            // check if name matches query (case-insensitive substring match)
            if !query.is_empty() && !name.to_lowercase().contains(&query_lower) {
                continue;
            }

            // get the kind based on declaration type
            let kind = match declaration {
                Declaration::Function { .. } => SymbolKind::Function,
                Declaration::Struct { .. } => SymbolKind::Struct,
                Declaration::Class { .. } => SymbolKind::Class,
                Declaration::Interface { .. } => SymbolKind::Interface,
                Declaration::Enum { .. } => SymbolKind::Enum,
                Declaration::Namespace { .. } => SymbolKind::Namespace,
                Declaration::Type { .. } => SymbolKind::TypeParameter,
                Declaration::Extension { .. } => SymbolKind::Class,
            };

            // get span (safely, skipping if out of bounds)
            let ast_node_id = dir_tree.get_source(declaration_id.id);
            let full_span = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                module_guard.ast.tree.source_map.get(ast_node_id)
            }));

            let Ok(full_span) = full_span else {
                continue;
            };

            symbols.push(WorkspaceSymbol {
                name,
                kind,
                file,
                range: full_span,
                container: None,
            });
        }
    }

    symbols
}
