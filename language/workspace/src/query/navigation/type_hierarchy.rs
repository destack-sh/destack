use destack_source::{FileId, Span};

use crate::Session;

/// An item in the type hierarchy.
#[derive(Debug, Clone)]
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
}

/// Kind of type hierarchy item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeHierarchyKind {
    Class,
    Interface,
    Struct,
    Enum,
    TypeAlias,
}

/// Prepare a type hierarchy item at the given position.
///
/// Returns the item if the position is on a type.
pub fn prepare_type_hierarchy(
    _session: &Session,
    _file: FileId,
    _offset: u32,
) -> Option<TypeHierarchyItem> {
    // 1. find the symbol at offset
    // 2. check if it's a type (class, interface, struct, enum)
    // 3. return TypeHierarchyItem with its info
    todo!("#Incomplete: prepare_type_hierarchy")
}

/// Get supertypes of a type hierarchy item.
///
/// For classes: base class and implemented interfaces.
/// For interfaces: extended interfaces.
/// For structs: implemented interfaces.
pub fn supertypes(_session: &Session, _item: &TypeHierarchyItem) -> Vec<TypeHierarchyItem> {
    // 1. get the type's declaration
    // 2. collect:
    //    - for class: extends clause, implements clause
    //    - for interface: extends clause
    //    - for struct: implements clause
    // 3. resolve each to its declaration
    todo!("#Incomplete: supertypes")
}

/// Get subtypes of a type hierarchy item.
///
/// For classes: subclasses.
/// For interfaces: implementing types and extending interfaces.
pub fn subtypes(_session: &Session, _item: &TypeHierarchyItem) -> Vec<TypeHierarchyItem> {
    // 1. search all types in the workspace
    // 2. for each type, check if it extends/implements this type
    // 3. return matching types
    todo!("#Incomplete: subtypes")
}
