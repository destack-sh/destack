use destack_dir::{GlobalSymbolId, SymbolForm};
use destack_source::{FileId, Span, Uri};
use destack_workspace::{Repository, Revision};
use serde::{Deserialize, Serialize};

use crate::core::{NominalRelation, nominal_relations_for_target, query_context};
use crate::dir::{
    find_symbol_at_offset, get_canonical_symbol, get_symbol_declaration_span,
    get_symbol_definition_span, resolve_symbol_name,
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
    /// The file containing this type.
    pub file: FileId,
    /// The full range of the type declaration.
    pub range: Span,
    /// The range of the type's name.
    pub selection_range: Span,
    /// The symbol ID (internal use for follow-up queries).
    pub symbol_id: GlobalSymbolId,
}

/// Kind of type hierarchy item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TypeHierarchyKind {
    Class,
    Interface,
    Struct,
    Enum,
    TypeAlias,
}

impl TypeHierarchyKind {
    /// Convert from SymbolForm if it's a type kind.
    fn from_symbol_form(ty: SymbolForm) -> Option<Self> {
        match ty {
            SymbolForm::Class => Some(Self::Class),
            SymbolForm::Interface => Some(Self::Interface),
            SymbolForm::Struct => Some(Self::Struct),
            SymbolForm::Enum => Some(Self::Enum),
            SymbolForm::TypeAlias => Some(Self::TypeAlias),
            _ => None,
        }
    }
}

/// Request prepare type hierarchy at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrepareTypeHierarchyRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
}

/// Response payload for prepare type hierarchy queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrepareTypeHierarchyResponse {
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

/// Prepare a type hierarchy item at the given position.
///
/// Returns the item if the position is on a type.
pub fn prepare_type_hierarchy(
    repository: &Repository,
    revision: Revision,
    file: FileId,
    offset: u32,
) -> Option<TypeHierarchyItem> {
    // find the symbol at offset
    let symbol_at = find_symbol_at_offset(repository, revision, file, offset)?;
    let canonical_id = get_canonical_symbol(repository, revision, symbol_at.symbol_id);

    // check if it's a type
    let ctx = query_context(repository, revision, canonical_id.module_id)?;
    let (kind, name) = {
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(canonical_id.local_id);
        let kind = TypeHierarchyKind::from_symbol_form(symbol.form)?;
        let name = resolve_symbol_name(repository, revision, canonical_id)?;
        Some((kind, name))
    }?;

    // resolve the selection range at the symbol name
    let selection_range = get_symbol_definition_span(repository, revision, canonical_id)?;

    // resolve the full declaration range, fall back to the selection range
    let range =
        get_symbol_declaration_span(repository, revision, canonical_id).unwrap_or(selection_range);

    Some(TypeHierarchyItem {
        name,
        kind,
        detail: None,
        file: selection_range.file,
        range,
        selection_range,
        symbol_id: canonical_id,
    })
}

/// Get supertypes of a type hierarchy item.
///
/// For classes: base class and implemented interfaces.
/// For interfaces: extended interfaces.
/// For structs: implemented interfaces.
pub fn supertypes(
    repository: &Repository,
    revision: Revision,
    item: &TypeHierarchyItem,
) -> Vec<TypeHierarchyItem> {
    let canonical_id = get_canonical_symbol(repository, revision, item.symbol_id);

    // get the lineage for this type
    let Some(ctx) = query_context(repository, revision, canonical_id.module_id) else {
        return Vec::new();
    };
    let supertype_ids: Vec<GlobalSymbolId> = {
        let types = ctx.dir().types();
        let Some(lineage) = types.symbol_lineage(canonical_id) else {
            return Vec::new();
        };

        // collect supertype symbol ids
        let mut supertype_ids: Vec<GlobalSymbolId> = Vec::new();

        // add extended type (parent class or extended interface)
        if let Some(extends_id) = lineage.extends {
            supertype_ids.push(extends_id);
        }

        // add implemented interfaces
        supertype_ids.extend(lineage.implements.iter().copied());

        // add embedded types (for structs with composition)
        supertype_ids.extend(lineage.embedded.iter().copied());

        supertype_ids
    };

    // convert to TypeHierarchyItems
    supertype_ids
        .into_iter()
        .filter_map(|symbol_id| type_hierarchy_item_from_symbol(repository, revision, symbol_id))
        .collect()
}

/// Get subtypes of a type hierarchy item.
///
/// For classes: subclasses.
/// For interfaces: implementing types and extending interfaces.
pub fn subtypes(
    repository: &Repository,
    revision: Revision,
    item: &TypeHierarchyItem,
) -> Vec<TypeHierarchyItem> {
    let canonical_id = get_canonical_symbol(repository, revision, item.symbol_id);

    // collect all subtype symbol ids first, then convert
    let mut subtype_ids: Vec<GlobalSymbolId> = Vec::new();

    // search cached direct nominal edges across the repository
    let entries = nominal_relations_for_target(repository, revision, canonical_id);

    for entry in entries {
        let matches = entry.target_symbol == canonical_id
            && matches!(
                entry.relation,
                NominalRelation::Extends | NominalRelation::Implements | NominalRelation::Embeds
            );

        if matches {
            subtype_ids.push(entry.source_symbol);
        }
    }

    // convert to TypeHierarchyItems
    subtype_ids
        .into_iter()
        .filter_map(|symbol_id| type_hierarchy_item_from_symbol(repository, revision, symbol_id))
        .collect()
}

/// Convert a symbol ID to a TypeHierarchyItem.
fn type_hierarchy_item_from_symbol(
    repository: &Repository,
    revision: Revision,
    symbol_id: GlobalSymbolId,
) -> Option<TypeHierarchyItem> {
    let canonical_id = get_canonical_symbol(repository, revision, symbol_id);
    let ctx = query_context(repository, revision, canonical_id.module_id)?;
    let (kind, name) = {
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(canonical_id.local_id);
        let kind = TypeHierarchyKind::from_symbol_form(symbol.form)?;
        let name = resolve_symbol_name(repository, revision, canonical_id)?;
        Some((kind, name))
    }?;

    // resolve the selection range at the symbol name
    let selection_range = get_symbol_definition_span(repository, revision, canonical_id)?;

    // resolve the full declaration range, fall back to the selection range
    let range =
        get_symbol_declaration_span(repository, revision, canonical_id).unwrap_or(selection_range);

    Some(TypeHierarchyItem {
        name,
        kind,
        detail: None,
        file: selection_range.file,
        range,
        selection_range,
        symbol_id: canonical_id,
    })
}
