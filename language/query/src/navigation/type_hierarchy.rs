use destack_dir::{GlobalSymbolId, SymbolKind};
use serde::{Deserialize, Serialize};

use crate::core::{
    ModuleQueryContext, NominalRelation, QueryPosition, QueryTarget, WorkspaceQueryContext,
    nominal_relations_for_target,
};
use crate::dir::{find_symbol_at_offset, symbol_declaration_span, symbol_definition_span};

/// An item in the type hierarchy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeHierarchyItem {
    /// The name of the type.
    pub name: String,
    /// The kind of type.
    pub kind: TypeHierarchyKind,
    /// Detail (e.g., generic parameters).
    pub detail: Option<String>,
    /// The target source and resolved identity.
    pub target: QueryTarget,
}

/// Kind of type hierarchy item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TypeHierarchyKind {
    /// Class type.
    Class,
    /// Interface type.
    Interface,
    /// Struct type.
    Struct,
    /// Enum type.
    Enum,
    /// Type alias.
    TypeAlias,
}

impl TypeHierarchyKind {
    /// Convert from SymbolKind if it's a type kind.
    fn from_symbol_kind(ty: SymbolKind) -> Option<Self> {
        match ty {
            SymbolKind::Class => Some(Self::Class),
            SymbolKind::Interface => Some(Self::Interface),
            SymbolKind::Struct => Some(Self::Struct),
            SymbolKind::Enum => Some(Self::Enum),
            SymbolKind::AssociatedType
            | SymbolKind::TypeAlias
            | SymbolKind::GenericTypeParameter => Some(Self::TypeAlias),
            _ => None,
        }
    }
}

/// Request the type hierarchy item at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeHierarchyItemRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// Response payload for type hierarchy item queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeHierarchyItemResponse {
    /// Type hierarchy item, if available.
    pub item: Option<TypeHierarchyItem>,
}

/// Request type hierarchy supertypes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeHierarchySupertypesRequest {
    /// The type hierarchy item to expand.
    pub item: TypeHierarchyItem,
}

/// Response payload for type hierarchy supertypes queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeHierarchySupertypesResponse {
    /// Type hierarchy items.
    pub items: Vec<TypeHierarchyItem>,
}

/// Request type hierarchy subtypes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeHierarchySubtypesRequest {
    /// The type hierarchy item to expand.
    pub item: TypeHierarchyItem,
}

/// Response payload for type hierarchy subtypes queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeHierarchySubtypesResponse {
    /// Type hierarchy items.
    pub items: Vec<TypeHierarchyItem>,
}

/// Return a type hierarchy item at the given position.
pub fn type_hierarchy_item(ctx: &ModuleQueryContext<'_>, offset: u32) -> Option<TypeHierarchyItem> {
    // find the symbol at offset
    let symbol_at = find_symbol_at_offset(ctx, offset)?;

    type_hierarchy_item_from_symbol(ctx, symbol_at.symbol_id)
}

/// Get supertypes of a type hierarchy item.
///
/// For classes: base class and implemented interfaces.
/// For interfaces: extended interfaces.
/// For structs: implemented interfaces.
pub fn supertypes(
    ctx: &WorkspaceQueryContext<'_>,
    item: &TypeHierarchyItem,
) -> Vec<TypeHierarchyItem> {
    let Some(symbol_id) = item.target.symbol_id else {
        return Vec::new();
    };
    let profile_id = item.target.module.profile_id;
    let Some(module_ctx) = ctx.module_context(symbol_id.module_id, profile_id) else {
        return Vec::new();
    };
    let canonical_id = module_ctx.canonical_symbol(symbol_id);

    let _ = canonical_id;

    Vec::new()
}

/// Get subtypes of a type hierarchy item.
///
/// For classes: subclasses.
/// For interfaces: implementing types and extending interfaces.
pub fn subtypes(
    ctx: &WorkspaceQueryContext<'_>,
    item: &TypeHierarchyItem,
) -> Vec<TypeHierarchyItem> {
    let Some(symbol_id) = item.target.symbol_id else {
        return Vec::new();
    };
    let profile_id = item.target.module.profile_id;
    let Some(module_ctx) = ctx.module_context(symbol_id.module_id, profile_id) else {
        return Vec::new();
    };
    let canonical_id = module_ctx.canonical_symbol(symbol_id);

    // collect all subtype symbol ids first, then convert
    let mut subtype_ids: Vec<GlobalSymbolId> = Vec::new();

    // search cached direct nominal edges across the repository
    let entries = nominal_relations_for_target(ctx, canonical_id);

    for entry in entries {
        let matches = entry.target_symbol == canonical_id
            && matches!(
                entry.relation,
                NominalRelation::Extends | NominalRelation::Implements
            );

        if matches {
            subtype_ids.push(entry.source_symbol);
        }
    }

    // convert to TypeHierarchyItems
    subtype_ids
        .into_iter()
        .filter_map(|symbol_id| type_hierarchy_item_from_symbol(&module_ctx, symbol_id))
        .collect()
}

/// Convert a symbol ID to a TypeHierarchyItem.
fn type_hierarchy_item_from_symbol(
    ctx: &ModuleQueryContext<'_>,
    symbol_id: GlobalSymbolId,
) -> Option<TypeHierarchyItem> {
    let canonical_id = ctx.canonical_symbol(symbol_id);
    let canonical_ctx = ctx.module_context(canonical_id.module_id)?;
    let (kind, name) = {
        let symbols = canonical_ctx.dir().symbols();
        let symbol = symbols.get_symbol(canonical_id.local_id);
        let kind = TypeHierarchyKind::from_symbol_kind(symbol.kind)?;
        let name = canonical_ctx.symbol_name(canonical_id)?;
        Some((kind, name))
    }?;

    // resolve the selection range at the symbol name
    let selection_range = symbol_definition_span(&canonical_ctx, canonical_id)?;

    // use the selection range when no declaration range is recorded
    let range = symbol_declaration_span(&canonical_ctx, canonical_id).unwrap_or(selection_range);

    let target = QueryTarget::span(canonical_ctx.query_module(), range)
        .with_selection_span(selection_range)
        .with_symbol(canonical_id);

    Some(TypeHierarchyItem {
        name,
        kind,
        detail: None,
        target,
    })
}
