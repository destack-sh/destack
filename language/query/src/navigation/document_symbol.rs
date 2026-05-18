use destack_dir as dir;
use destack_dir::{Declaration, EnumField, LocalNodeId, Member, TypeMember};
use destack_source::Span;
use serde::{Deserialize, Serialize};

pub use crate::SymbolKind;
use crate::core::{ModuleQueryContext, QueryModule};
use crate::dir::{
    declaration_display_name, declaration_symbol_kind, is_synthetic_function_keyword_field,
    member_key_name, member_symbol_kind, type_member_symbol_kind,
};
use crate::source::{main_span_for_dir_node, span_for_dir_node};

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
    /// The queried module.
    pub module: QueryModule,
}

/// Response payload for document symbols queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentSymbolsResponse {
    /// Document symbols.
    pub symbols: Vec<DocumentSymbol>,
}

/// Get all symbols in a document.
pub fn document_symbols(ctx: &ModuleQueryContext<'_>) -> Vec<DocumentSymbol> {
    let dir_tree = ctx.dir().view();
    let mut symbols = Vec::new();

    // collect declarations and their child symbols
    for (declaration_id, declaration) in dir_tree.iter_nodes_of_type::<Declaration>() {
        let kind = declaration_symbol_kind(declaration);
        let name = declaration_display_name(ctx.dir().strings(), declaration);
        let range = span_for_dir_node(ctx.dir(), dir_tree, declaration_id.into());
        let selection_range =
            main_span_for_dir_node(ctx.dir(), dir_tree, declaration_id.into()).unwrap_or(range);

        let mut symbol =
            DocumentSymbol::new(name, kind, range).with_selection_range(selection_range);

        if let Some(member_ids) = declaration.member_ids() {
            for member_id in member_ids {
                if let Some(child) = member_to_document_symbol(dir_tree, *member_id, ctx) {
                    symbol = symbol.with_child(child);
                }
            }
        }

        if let Some(member_ids) = declaration.type_member_ids() {
            for member_id in member_ids {
                if let Some(child) = type_member_to_document_symbol(dir_tree, *member_id, ctx) {
                    symbol = symbol.with_child(child);
                }
            }
        }

        if let Declaration::Enum(declaration) = declaration {
            for field_id in &declaration.fields {
                if let Some(child) = enum_field_to_document_symbol(dir_tree, *field_id, ctx) {
                    symbol = symbol.with_child(child);
                }
            }
        }

        symbols.push(symbol);
    }

    symbols
}

/// Convert a member to a document symbol.
fn member_to_document_symbol(
    dir_tree: dir::View<'_>,
    member_id: LocalNodeId<Member>,
    ctx: &ModuleQueryContext<'_>,
) -> Option<DocumentSymbol> {
    let member = dir_tree.get::<Member>(member_id);
    let range = span_for_dir_node(ctx.dir(), dir_tree, member_id.into());

    // get the member name and symbol kind
    let (name, kind) = match member {
        Member::AssociatedType { name, .. } => {
            let name = ctx.dir().strings().get(*name).to_string();
            (name, SymbolKind::TypeParameter)
        }
        Member::AssociatedConst { name, .. } => {
            let name = ctx.dir().strings().get(*name).to_string();
            (name, SymbolKind::Constant)
        }
        _ => {
            let key = member.key()?;
            let name = member_key_name(ctx.dir().strings(), key)?;
            let kind = member_symbol_kind(member)?;
            (name, kind)
        }
    };

    // skip synthetic function keyword fields for methods
    if is_synthetic_function_keyword_field(member, &name, range) {
        return None;
    }

    // get spans
    // get main span (name span) if available
    let selection_range =
        main_span_for_dir_node(ctx.dir(), dir_tree, member_id.into()).unwrap_or(range);

    Some(DocumentSymbol::new(name, kind, range).with_selection_range(selection_range))
}

/// Convert a type member to a document symbol.
fn type_member_to_document_symbol(
    dir_tree: dir::View<'_>,
    member_id: LocalNodeId<TypeMember>,
    ctx: &ModuleQueryContext<'_>,
) -> Option<DocumentSymbol> {
    let member = dir_tree.get::<TypeMember>(member_id);
    let range = span_for_dir_node(ctx.dir(), dir_tree, member_id.into());

    // resolve the type member name and symbol kind
    let kind = type_member_symbol_kind(member)?;
    let name = if let Some(name) = member.name() {
        ctx.dir().strings().get(name).to_string()
    } else {
        let key = member.key()?;
        member_key_name(ctx.dir().strings(), key)?
    };

    // get spans
    let selection_range =
        main_span_for_dir_node(ctx.dir(), dir_tree, member_id.into()).unwrap_or(range);

    Some(DocumentSymbol::new(name, kind, range).with_selection_range(selection_range))
}

/// Convert an enum field to a document symbol.
fn enum_field_to_document_symbol(
    dir_tree: dir::View<'_>,
    field_id: LocalNodeId<EnumField>,
    ctx: &ModuleQueryContext<'_>,
) -> Option<DocumentSymbol> {
    let field = dir_tree.get::<EnumField>(field_id);

    // get the field name
    let name = ctx.dir().strings().get(field.name.string()).to_string();

    // get spans
    let range = span_for_dir_node(ctx.dir(), dir_tree, field_id.into());

    // get main span (name span) if available
    let selection_range =
        main_span_for_dir_node(ctx.dir(), dir_tree, field_id.into()).unwrap_or(range);

    Some(
        DocumentSymbol::new(name, SymbolKind::EnumMember, range)
            .with_selection_range(selection_range),
    )
}
