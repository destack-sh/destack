use destack_dir as dir;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{ModuleQueryContext, Position, ProgramQueryContext, Target};

/// An item in the type hierarchy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TypeItem {
    /// The name of the type.
    pub name: String,
    /// The kind of type.
    pub kind: dir::SymbolKind,
    /// Detail (e.g., generic parameters).
    pub detail: Option<String>,
    /// The target source and resolved identity.
    pub target: Target,
}

impl TypeItem {
    /// Return the resolved symbol identity carried by this item.
    pub fn symbol_id(&self) -> Option<dir::GlobalSymbolId> {
        self.target.symbol_id
    }
}

/// Request the type item at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TypeItemRequest {
    /// The queried position.
    pub position: Position,
}

/// Response payload for type item queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TypeItemResponse {
    /// Type hierarchy item, if available.
    pub item: Option<TypeItem>,
}

/// Request supertypes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SupertypesRequest {
    /// The type item to expand.
    pub item: TypeItem,
}

/// Response payload for supertypes queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SupertypesResponse {
    /// Type hierarchy items.
    pub items: Vec<TypeItem>,
}

/// Request subtypes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SubtypesRequest {
    /// The type item to expand.
    pub item: TypeItem,
}

/// Response payload for subtypes queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SubtypesResponse {
    /// Type hierarchy items.
    pub items: Vec<TypeItem>,
}

impl ModuleQueryContext<'_> {
    /// Return a type item at the given position.
    pub fn type_item(&self, offset: u32) -> Option<TypeItem> {
        let symbol_at = self.find_symbol_at_offset(offset)?;

        self.type_item_from_symbol(symbol_at.symbol_id)
    }

    /// Return the type item for one symbol.
    pub(crate) fn type_item_from_symbol(&self, symbol_id: dir::GlobalSymbolId) -> Option<TypeItem> {
        let canonical_id = self.canonical_symbol(symbol_id);
        let canonical_module = self.module_context(canonical_id.module_id);
        let (kind, name) = {
            let symbols = canonical_module.symbols();
            let symbol = symbols.get_symbol(canonical_id.local_id);
            if !symbol.kind.is_type_definition() {
                return None;
            }

            let kind = symbol.kind;
            let name = canonical_module.symbol_name(canonical_id)?;
            Some((kind, name))
        }?;

        // resolve source ranges around the declaration name
        let selection_range = canonical_module.symbol_definition_span(canonical_id)?;
        let range = canonical_module.symbol_declaration_span(canonical_id)?;

        let target = Target::new(canonical_module.module(), range)
            .with_selection_span(selection_range)
            .with_symbol_id(canonical_id);

        Some(TypeItem {
            name,
            kind,
            detail: None,
            target,
        })
    }
}

impl ProgramQueryContext<'_> {
    /// Return supertypes of a type item.
    ///
    /// For classes: base class and implemented interfaces.
    /// For interfaces: extended interfaces.
    /// For structs: implemented interfaces.
    pub fn supertypes(&self, item: &TypeItem) -> Vec<TypeItem> {
        let Some(symbol_id) = item.symbol_id() else {
            return Vec::new();
        };
        let profile_id = item.target.module.profile_id;
        let module = self.module_context(symbol_id.module_id, profile_id);
        let canonical_id = module.canonical_symbol(symbol_id);
        let mut supertype_ids = Vec::new();

        // collect direct nominal edges declared by the target symbol
        for entry in self.derived_heritage(canonical_id) {
            if entry.derived == canonical_id {
                supertype_ids.push(entry.base);
            }
        }

        supertype_ids
            .into_iter()
            .filter_map(|symbol_id| module.type_item_from_symbol(symbol_id))
            .collect()
    }

    /// Return subtypes of a type item.
    ///
    /// For classes: subclasses.
    /// For interfaces: implementing types and extending interfaces.
    pub fn subtypes(&self, item: &TypeItem) -> Vec<TypeItem> {
        let Some(symbol_id) = item.symbol_id() else {
            return Vec::new();
        };
        let profile_id = item.target.module.profile_id;
        let module = self.module_context(symbol_id.module_id, profile_id);
        let canonical_id = module.canonical_symbol(symbol_id);
        let mut subtype_ids: Vec<dir::GlobalSymbolId> = Vec::new();

        // collect direct nominal edges
        let entries = self.base_heritage(canonical_id);
        for entry in entries {
            let matches = entry.base == canonical_id
                && matches!(
                    entry.kind,
                    dir::HeritageKind::Extends | dir::HeritageKind::Implements
                );

            if matches {
                subtype_ids.push(entry.derived);
            }
        }

        subtype_ids
            .into_iter()
            .filter_map(|symbol_id| module.type_item_from_symbol(symbol_id))
            .collect()
    }
}
