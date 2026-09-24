use crate::{Field, StaticId, Tree, Type, TypeId};

/// Erase lifetimes while preserving declaration identities and memory spaces.
pub fn erase_lifetimes(tree: &Tree, ty: TypeId) -> TypeId {
    // preserve declaration identities and terminate recursive definitions
    if tree.is_identified_type(ty) {
        return ty;
    }

    // erase the lifetimes written directly on this type
    let mut erased = tree.get(ty).erased_lifetimes();

    // erase field types while preserving names and attributes
    if let Type::Struct { fields, .. } = &mut erased {
        for field in fields {
            let declared = tree.get(*field).clone();
            let ty = erase_lifetimes(tree, declared.ty);
            *field = tree.intern_field(Field { ty, ..declared });
        }
    }

    // erase the remaining child types
    erased.map_values(&mut |value| erase_value(tree, value));
    erased.map_child_type_ids(&mut |child| erase_lifetimes(tree, child));

    tree.intern_type(erased)
}

/// Erase lifetime requirements from types reflected in a compile-time value.
fn erase_value(tree: &Tree, id: StaticId) -> StaticId {
    let mut value = tree.static_value(id).clone();
    value.map_values(&mut |value| erase_value(tree, value));
    value.map_types(&mut |ty| erase_lifetimes(tree, ty));

    tree.intern_static(value)
}
