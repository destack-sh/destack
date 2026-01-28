use destack_dir::{Declaration, EnumField, LocalNodeId, Member, NodeTree};
use destack_source::{FileId, Span, Uri};
use serde::{Deserialize, Serialize};

use crate::Session;
use crate::query::QueryContext;
use crate::query::common::{
    is_synthetic_function_keyword_field, main_span_for_dir_node, member_key_name,
    span_for_dir_node, with_query_context_for_file,
};

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
            let kind = declaration_symbol_kind(declaration);

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
            let range = span_for_dir_node(&ctx, &dir_tree, declaration_id.into());
            let selection_range =
                main_span_for_dir_node(&ctx, &dir_tree, declaration_id.into()).unwrap_or(range);

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
    let range = span_for_dir_node(ctx, dir_tree, member_id.into());

    // get the member name from the key
    let key = member.key()?;
    let name = member_key_name(session, key)?;

    // get kind based on member type
    let kind = member_symbol_kind(member)?;

    // skip synthetic function keyword fields for methods
    if is_synthetic_function_keyword_field(member, &name, range) {
        return None;
    }

    // get spans
    // get main span (name span) if available
    let selection_range = main_span_for_dir_node(ctx, dir_tree, member_id.into()).unwrap_or(range);

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
    let range = span_for_dir_node(ctx, dir_tree, field_id.into());

    // get main span (name span) if available
    let selection_range = main_span_for_dir_node(ctx, dir_tree, field_id.into()).unwrap_or(range);

    Some(
        DocumentSymbol::new(name, SymbolKind::EnumMember, range)
            .with_selection_range(selection_range),
    )
}

/// Map a declaration to its document symbol kind.
pub(crate) fn declaration_symbol_kind(declaration: &Declaration) -> SymbolKind {
    match declaration {
        Declaration::Global { .. } => SymbolKind::Namespace,
        Declaration::Function { .. } => SymbolKind::Function,
        Declaration::Struct { .. } => SymbolKind::Struct,
        Declaration::Class { .. } => SymbolKind::Class,
        Declaration::Interface { .. } => SymbolKind::Interface,
        Declaration::Enum { .. } => SymbolKind::Enum,
        Declaration::Namespace { .. } => SymbolKind::Namespace,
        Declaration::Type { .. } => SymbolKind::TypeParameter,
        Declaration::Extension { .. } => SymbolKind::Class,
    }
}

/// Map a member to its document symbol kind.
pub(crate) fn member_symbol_kind(member: &Member) -> Option<SymbolKind> {
    match member {
        Member::Type { .. } => Some(SymbolKind::TypeParameter),
        Member::Field { .. } => Some(SymbolKind::Field),
        Member::Method { .. } => Some(SymbolKind::Method),
        Member::Embed { .. } => None,
        Member::StaticBlock { .. } => None,
        Member::ComptimeBlock { .. } => None,
    }
}
