use destack_dir::{GlobalSymbolId, SymbolType};
use destack_source::{FileId, Span, Uri};
use serde::{Deserialize, Serialize};

use crate::Session;
use crate::query::common::{
    find_symbol_at_offset, get_canonical_symbol, get_symbol_declaration_span,
    get_symbol_definition_span,
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
    /// Convert from SymbolType if it's a type kind.
    fn from_symbol_type(ty: SymbolType) -> Option<Self> {
        match ty {
            SymbolType::Class => Some(Self::Class),
            SymbolType::Interface => Some(Self::Interface),
            SymbolType::Struct => Some(Self::Struct),
            SymbolType::Enum => Some(Self::Enum),
            SymbolType::TypeAlias => Some(Self::TypeAlias),
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
    session: &Session,
    file: FileId,
    offset: u32,
) -> Option<TypeHierarchyItem> {
    // find the symbol at offset
    let symbol_at = find_symbol_at_offset(session, file, offset)?;
    let canonical_id = get_canonical_symbol(session, symbol_at.symbol_id);

    // check if it's a type
    let module = session.modules.get(canonical_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(canonical_id.local_id);

    let kind = TypeHierarchyKind::from_symbol_type(symbol.ty)?;

    // get the name
    let name = symbol
        .name()
        .map(|id| ctx.ast.strings.get(id).to_string())?;

    drop(symbols);
    drop(module);

    // resolve the selection range at the symbol name
    let selection_range = get_symbol_definition_span(session, canonical_id)?;

    // resolve the full declaration range, fall back to the selection range
    let range = get_symbol_declaration_span(session, canonical_id).unwrap_or(selection_range);

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
pub fn supertypes(session: &Session, item: &TypeHierarchyItem) -> Vec<TypeHierarchyItem> {
    let canonical_id = get_canonical_symbol(session, item.symbol_id);

    // get the lineage for this type
    let module = session.modules.get(canonical_id.module_id);
    let module = module.read();
    let Some(ctx) = session.query_context(&module) else {
        return Vec::new();
    };
    let types = ctx.types();
    let Some(lineage) = types.get_lineage_for_symbol(canonical_id) else {
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

    drop(types);
    drop(module);

    // convert to TypeHierarchyItems
    supertype_ids
        .into_iter()
        .filter_map(|symbol_id| type_hierarchy_item_from_symbol(session, symbol_id))
        .collect()
}

/// Get subtypes of a type hierarchy item.
///
/// For classes: subclasses.
/// For interfaces: implementing types and extending interfaces.
pub fn subtypes(session: &Session, item: &TypeHierarchyItem) -> Vec<TypeHierarchyItem> {
    let canonical_id = get_canonical_symbol(session, item.symbol_id);

    // collect all subtype symbol ids first, then convert
    let mut subtype_ids: Vec<GlobalSymbolId> = Vec::new();

    // search all modules for types that extend/implement this type
    for module in session.modules.iter() {
        let module = module.read();
        let Some(ctx) = session.query_context(&module) else {
            continue;
        };
        let types = ctx.types();

        for (symbol_id, lineage) in types.iter_lineages() {
            // check if this type extends or implements our target
            let is_subtype = lineage.directly_extends(canonical_id)
                || lineage.directly_implements(canonical_id)
                || lineage.directly_embeds(canonical_id);

            if is_subtype {
                subtype_ids.push(symbol_id);
            }
        }
    }

    // convert to TypeHierarchyItems
    subtype_ids
        .into_iter()
        .filter_map(|symbol_id| type_hierarchy_item_from_symbol(session, symbol_id))
        .collect()
}

/// Convert a symbol ID to a TypeHierarchyItem.
pub fn type_hierarchy_item_from_symbol(
    session: &Session,
    symbol_id: GlobalSymbolId,
) -> Option<TypeHierarchyItem> {
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    let kind = TypeHierarchyKind::from_symbol_type(symbol.ty)?;
    let name = symbol
        .name()
        .map(|id| ctx.ast.strings.get(id).to_string())?;

    drop(symbols);
    drop(module);

    // resolve the selection range at the symbol name
    let selection_range = get_symbol_definition_span(session, symbol_id)?;

    // resolve the full declaration range, fall back to the selection range
    let range = get_symbol_declaration_span(session, symbol_id).unwrap_or(selection_range);

    Some(TypeHierarchyItem {
        name,
        kind,
        detail: None,
        file: selection_range.file,
        range,
        selection_range,
        symbol_id,
    })
}
