use crate::{Access, Lifetime, Reference, Storage, Substitution, Tree, Type, TypeId};

    /// Return the object one type's value stands for, a managed handle's referent.
pub fn referent_of(tree: &Tree, ty: TypeId) -> TypeId {
    let resolved = Substitution::resolve(ty, tree);
    match tree.get(resolved).clone() {
        Type::Reference {
            kind: Reference::Managed,
            pointee,
            ..
        } => pointee,
        definition => match definition.reference_kind() {
            Some(Reference::Managed) => fat_handle_at(tree, 
                &definition,
                Reference::Unique,
                definition.reference_lifetime().cloned().unwrap_or_default(),
                definition.reference_storage().unwrap_or_default(),
                definition.reference_access().unwrap_or(Access::Mutable),
            ),
            _ => ty,
        },
    }
}
    /// Return one reference to the object a type's value stands for.
pub fn lend(
    tree: &Tree,
    kind: Reference,
    lifetime: Lifetime,
    storage: Storage,
    access: Access,
    ty: TypeId,
) -> TypeId {
    // fuse a fat handle into one descriptor at the borrow's terms, else address the object
    let resolved = Substitution::resolve(ty, tree);
    let definition = tree.get(resolved).clone();
    match definition.reference_kind() {
        Some(Reference::Managed) if !matches!(definition, Type::Reference { .. }) => {
            fat_handle_at(tree, &definition, kind, lifetime, storage, access)
        }
        _ => tree.intern_type(
            Type::Reference {
                kind,
                lifetime,
                storage,
                access,
                pointee: referent_of(tree, ty),
            },
            kind.copy(),
        ),
    }
}
    /// Intern one fat handle definition at other reference terms.
fn fat_handle_at(
    tree: &Tree,
    definition: &Type,
    kind: Reference,
    lifetime: Lifetime,
    storage: Storage,
    access: Access,
) -> TypeId {
    let requalified = match definition.clone() {
        Type::Slice { element, .. } => Type::Slice {
            kind,
            lifetime,
            element,
            storage,
            access,
        },
        Type::Dynamic { constraint, .. } => Type::Dynamic {
            kind,
            lifetime,
            constraint,
            storage,
            access,
        },
        Type::Function {
            signature,
            multiplicity,
            ..
        } => Type::Function {
            signature,
            multiplicity,
            kind,
            lifetime,
            storage,
            access,
        },
        definition => unreachable!("a fat handle of the form {definition:?}"),
    };

    tree.intern_type(requalified, kind.copy())
}
