use destack_dir::{Declaration, DynamicKey, EnumField, LocalNodeId, Member, NodeTree};
use destack_source::{FileId, Span, Uri};
use serde::{Deserialize, Serialize};

use crate::Session;
use crate::query::QueryContext;
use crate::query::common::with_query_context_for_file;

/// Kind of document symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// Request document symbols for a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentSymbolsRequest {
    /// The document URI.
    pub uri: Uri,
}

/// Response payload for document symbols queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentSymbolsResponse {
    /// Document symbols.
    pub symbols: Vec<DocumentSymbol>,
}

/// Get all symbols in a document (for outline view).
pub fn document_symbols(session: &Session, file: FileId) -> Vec<DocumentSymbol> {
    with_query_context_for_file(session, file, |ctx| {
        let dir_tree = ctx.tree();
        let mut symbols = Vec::new();

        // iterate through all declarations
        for (declaration_id, declaration) in dir_tree.iter_nodes_of_type::<Declaration>() {
            // get the kind based on declaration type
            let kind = match declaration {
                Declaration::Global { .. } => SymbolKind::Namespace,
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
            let name = match declaration {
                Declaration::Global { .. } => "global".to_string(),
                _ => descriptor
                    .name
                    .map(|name| session.strings.get(name.string()).to_string())
                    .unwrap_or_else(|| "<anonymous>".to_string()),
            };

            // resolve the full range and the main selection range
            let ast_node_id = dir_tree.get_source(declaration_id.id);
            let full_span = ctx.ast.tree.source_map.get(ast_node_id);
            let range = Span::new(ctx.file_id, full_span.start, full_span.end);

            let selection_range = ctx
                .ast
                .tree
                .source_map
                .get_main(ast_node_id)
                .map(|span| Span::new(ctx.file_id, span.start, span.end))
                .unwrap_or(range);

            let mut symbol =
                DocumentSymbol::new(name, kind, range).with_selection_range(selection_range);

            // add children for declarations with members
            if let Some(member_ids) = declaration.member_ids() {
                for member_id in member_ids {
                    if let Some(child) =
                        member_to_document_symbol(&dir_tree, *member_id, &ctx, session)
                    {
                        symbol = symbol.with_child(child);
                    }
                }
            }

            // add enum field children
            if let Declaration::Enum { fields, .. } = declaration {
                for field_id in fields {
                    if let Some(child) =
                        enum_field_to_document_symbol(&dir_tree, *field_id, &ctx, session)
                    {
                        symbol = symbol.with_child(child);
                    }
                }
            }

            symbols.push(symbol);
        }

        symbols
    })
    .unwrap_or_default()
}

/// Convert a member to a document symbol.
fn member_to_document_symbol(
    dir_tree: &NodeTree,
    member_id: LocalNodeId<Member>,
    ctx: &QueryContext<'_>,
    session: &Session,
) -> Option<DocumentSymbol> {
    let member = dir_tree.get::<Member>(member_id);
    let ast_node_id = dir_tree.get_source(member_id.id);
    let full_span = ctx.ast.tree.source_map.get(ast_node_id);

    // get the member name from the key
    let name = match member.key() {
        Some(DynamicKey::Name(string_id)) => session.strings.get(*string_id).to_string(),
        Some(DynamicKey::Number(string_id)) => session.strings.get(*string_id).to_string(),
        _ => return None, // skip members without static names
    };

    // get kind based on member type
    let kind = match member {
        Member::Type { .. } => SymbolKind::TypeParameter,
        Member::Field { .. } => SymbolKind::Field,
        Member::Method { .. } => SymbolKind::Method,
        Member::Embed { .. } => return None, // skip embedded types
        Member::StaticBlock { .. } => return None, // skip static blocks
        Member::ComptimeBlock { .. } => return None, // skip comptime blocks
    };

    // skip synthetic function keyword fields for methods
    let full_len = full_span.end.saturating_sub(full_span.start);
    if matches!(member, Member::Field { .. }) && name == "function" && full_len == 8 {
        return None;
    }

    // get spans
    let range = Span::new(ctx.file_id, full_span.start, full_span.end);

    // get main span (name span) if available
    let selection_range = ctx
        .ast
        .tree
        .source_map
        .get_main(ast_node_id)
        .map(|span| Span::new(ctx.file_id, span.start, span.end))
        .unwrap_or(range);

    Some(DocumentSymbol::new(name, kind, range).with_selection_range(selection_range))
}

/// Convert an enum field to a document symbol.
fn enum_field_to_document_symbol(
    dir_tree: &NodeTree,
    field_id: LocalNodeId<EnumField>,
    ctx: &QueryContext<'_>,
    session: &Session,
) -> Option<DocumentSymbol> {
    let field = dir_tree.get::<EnumField>(field_id);

    // get the field name
    let name = session.strings.get(field.name).to_string();

    // get spans
    let ast_node_id = dir_tree.get_source(field_id.id);
    let full_span = ctx.ast.tree.source_map.get(ast_node_id);
    let range = Span::new(ctx.file_id, full_span.start, full_span.end);

    // get main span (name span) if available
    let selection_range = ctx
        .ast
        .tree
        .source_map
        .get_main(ast_node_id)
        .map(|span| Span::new(ctx.file_id, span.start, span.end))
        .unwrap_or(range);

    Some(
        DocumentSymbol::new(name, SymbolKind::EnumMember, range)
            .with_selection_range(selection_range),
    )
}
