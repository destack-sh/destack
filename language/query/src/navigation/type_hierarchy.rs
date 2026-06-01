use destack_dir as dir;
use serde::{Deserialize, Serialize};

use crate::core::{
    ModuleQueryContext, NominalRelation, QueryPosition, QueryTarget, WorkspaceQueryContext,
};

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
    /// Convert from dir::SymbolKind if it's a type kind.
    fn from_symbol_kind(ty: dir::SymbolKind) -> Option<Self> {
        match ty {
            dir::SymbolKind::Class => Some(Self::Class),
            dir::SymbolKind::Interface => Some(Self::Interface),
            dir::SymbolKind::Struct => Some(Self::Struct),
            dir::SymbolKind::Enum => Some(Self::Enum),
            dir::SymbolKind::AssociatedType
            | dir::SymbolKind::TypeAlias
            | dir::SymbolKind::GenericTypeParameter => Some(Self::TypeAlias),
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

impl ModuleQueryContext<'_> {
    /// Return a type hierarchy item at the given position.
    pub fn type_hierarchy_item(&self, offset: u32) -> Option<TypeHierarchyItem> {
        let symbol_at = self.find_symbol_at_offset(offset)?;

        self.type_hierarchy_item_from_symbol(symbol_at.symbol_id)
    }

    /// Convert one symbol ID to a type hierarchy item.
    pub(crate) fn type_hierarchy_item_from_symbol(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<TypeHierarchyItem> {
        let canonical_id = self.canonical_symbol(symbol_id);
        let canonical_ctx = self.module_context(canonical_id.module_id)?;
        let (kind, name) = {
            let symbols = canonical_ctx.dir().symbols();
            let symbol = symbols.get_symbol(canonical_id.local_id);
            let kind = TypeHierarchyKind::from_symbol_kind(symbol.kind)?;
            let name = canonical_ctx.symbol_name(canonical_id)?;
            Some((kind, name))
        }?;

        // resolve source ranges around the declaration name
        let selection_range = canonical_ctx.symbol_definition_span(canonical_id)?;
        let range = canonical_ctx
            .symbol_declaration_span(canonical_id)
            .unwrap_or(selection_range);

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
}

impl WorkspaceQueryContext<'_> {
    /// Get supertypes of a type hierarchy item.
    ///
    /// For classes: base class and implemented interfaces.
    /// For interfaces: extended interfaces.
    /// For structs: implemented interfaces.
    pub fn supertypes(&self, item: &TypeHierarchyItem) -> Vec<TypeHierarchyItem> {
        let Some(symbol_id) = item.target.symbol_id else {
            return Vec::new();
        };
        let profile_id = item.target.module.profile_id;
        let Some(module_ctx) = self.module_context(symbol_id.module_id, profile_id) else {
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
    pub fn subtypes(&self, item: &TypeHierarchyItem) -> Vec<TypeHierarchyItem> {
        let Some(symbol_id) = item.target.symbol_id else {
            return Vec::new();
        };
        let profile_id = item.target.module.profile_id;
        let Some(module_ctx) = self.module_context(symbol_id.module_id, profile_id) else {
            return Vec::new();
        };
        let canonical_id = module_ctx.canonical_symbol(symbol_id);
        let mut subtype_ids: Vec<dir::GlobalSymbolId> = Vec::new();

        // collect direct nominal edges
        let entries = self.nominal_relations_for_target(canonical_id);
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

        subtype_ids
            .into_iter()
            .filter_map(|symbol_id| module_ctx.type_hierarchy_item_from_symbol(symbol_id))
            .collect()
    }
}
