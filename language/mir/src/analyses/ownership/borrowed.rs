use destack_core::FxIndexSet;

use crate::{
    BorrowedPath, GenericArgument, Lifetime, Path, Projection, Reference, Substitution, Tree, Type,
    TypeId,
};

    /// Return the extents one type stores across its regions.
pub fn type_lifetime(tree: &Tree, ty: TypeId) -> Option<Lifetime> {
    let mut visited = FxIndexSet::default();

    type_lifetime_inner(tree, ty, &mut visited)
}
    /// Return the explicit lifetime carried by a type.
fn type_lifetime_inner(
    tree: &Tree,
    ty: TypeId,
    visited: &mut FxIndexSet<TypeId>,
) -> Option<Lifetime> {
    if !visited.insert(ty) {
        return None;
    }

    // inspect nominal arguments without expanding recursive definitions
    let definition = match tree.get(ty) {
        application @ Type::Application { .. } => application,
        _ => tree.type_definition(ty),
    };
    let lifetime = match definition {
        Type::Application { arguments, .. } => {
            let lifetimes = arguments.iter().filter_map(|argument| match argument {
                GenericArgument::Type(ty) => type_lifetime_inner(tree, *ty, visited),
                GenericArgument::Region { lifetime, .. } => Some(lifetime.clone()),
                _ => None,
            });

            Some(Lifetime::new(
                lifetimes.flat_map(|lifetime| lifetime.extents),
            ))
            .filter(|lifetime| !lifetime.is_empty())
        }
        Type::Dynamic {
            kind: Reference::Borrowed,
            lifetime,
            ..
        }
        | Type::Function {
            kind: Reference::Borrowed,
            lifetime,
            ..
        } if !lifetime.is_empty() => Some(lifetime.clone()),
        Type::Reference {
            kind,
            lifetime,
            pointee,
            ..
        } => {
            let own = (matches!(kind, Reference::Borrowed)).then(|| lifetime.clone());
            let nested = type_lifetime_inner(tree, *pointee, visited);
            let terms = own
                .into_iter()
                .chain(nested)
                .flat_map(|lifetime| lifetime.extents);

            Some(Lifetime::new(terms)).filter(|lifetime| !lifetime.is_empty())
        }
        Type::Slice {
            kind,
            lifetime,
            element,
            ..
        } => {
            let own = (matches!(kind, Reference::Borrowed)).then(|| lifetime.clone());
            let nested = type_lifetime_inner(tree, *element, visited);
            let terms = own
                .into_iter()
                .chain(nested)
                .flat_map(|lifetime| lifetime.extents);

            Some(Lifetime::new(terms)).filter(|lifetime| !lifetime.is_empty())
        }
        Type::Struct { fields, .. } => {
            let nested_lifetimes = fields.iter().filter_map(|field| {
                let field = tree.get(*field);
                type_lifetime_inner(tree, field.ty, visited)
            });

            Some(Lifetime::new(
                nested_lifetimes.flat_map(|lifetime| lifetime.extents.into_iter()),
            ))
            .filter(|lifetime| !lifetime.is_empty())
        }
        Type::Newtype { inner, .. } => type_lifetime_inner(tree, *inner, visited),
        Type::Uninit { value } | Type::ManuallyDrop { value } => {
            type_lifetime_inner(tree, *value, visited)
        }
        Type::Variant {
            discriminant,
            cases,
            ..
        } => {
            let discriminant = type_lifetime_inner(tree, *discriminant, visited);
            let nested_lifetimes = cases
                .iter()
                .filter_map(|case| type_lifetime_inner(tree, case.ty, visited));

            Some(Lifetime::new(
                discriminant
                    .into_iter()
                    .chain(nested_lifetimes)
                    .flat_map(|lifetime| lifetime.extents.into_iter()),
            ))
            .filter(|lifetime| !lifetime.is_empty())
        }
        Type::Tuple { elements, .. } => {
            let nested_lifetimes = elements
                .iter()
                .filter_map(|element| type_lifetime_inner(tree, *element, visited));

            Some(Lifetime::new(
                nested_lifetimes.flat_map(|lifetime| lifetime.extents.into_iter()),
            ))
            .filter(|lifetime| !lifetime.is_empty())
        }
        Type::FixedArray { element, .. } | Type::Vector { element, .. } => {
            type_lifetime_inner(tree, *element, visited)
        }
        _ => None,
    };
    visited.swap_remove(&ty);

    lifetime
}
    /// Return whether a type may contain borrowed references.
pub fn type_contains_borrowed_refs(tree: &Tree, ty: TypeId) -> bool {
    let mut visited = FxIndexSet::default();

    type_contains_borrowed_refs_inner(tree, ty, &mut visited)
}
    /// Return whether a type path contains borrowed references.
fn type_contains_borrowed_refs_inner(
    tree: &Tree,
    ty: TypeId,
    visited: &mut FxIndexSet<TypeId>,
) -> bool {
    if !visited.insert(ty) {
        return false;
    }

    let type_id = ty;
    let ty = match tree.get(ty) {
        application @ Type::Application { .. } => application,
        _ => tree.type_definition(ty),
    };
    if ty.is_borrowed_reference() {
        visited.swap_remove(&type_id);

        return true;
    }

    // inspect nominal arguments without expanding recursive definitions
    let contains = match ty {
        Type::Application { arguments, .. } => {
            arguments.iter().any(|argument| match argument {
                GenericArgument::Type(ty) => {
                    type_contains_borrowed_refs_inner(tree, *ty, visited)
                }
                GenericArgument::Region { .. } => true,
                _ => false,
            })
        }
        Type::Struct { fields, .. } => fields.iter().any(|field| {
            let field = tree.get(*field);
            type_contains_borrowed_refs_inner(tree, field.ty, visited)
        }),
        Type::Newtype { inner, .. } => type_contains_borrowed_refs_inner(tree, *inner, visited),
        Type::Uninit { value } | Type::ManuallyDrop { value } => {
            type_contains_borrowed_refs_inner(tree, *value, visited)
        }
        Type::Variant {
            discriminant,
            cases,
            ..
        } => {
            type_contains_borrowed_refs_inner(tree, *discriminant, visited)
                || cases
                    .iter()
                    .any(|case| type_contains_borrowed_refs_inner(tree, case.ty, visited))
        }
        Type::Tuple { elements, .. } => elements
            .iter()
            .any(|element| type_contains_borrowed_refs_inner(tree, *element, visited)),
        Type::Reference {
            pointee: element, ..
        }
        | Type::FixedArray { element, .. }
        | Type::Slice { element, .. }
        | Type::Vector { element, .. } => {
            type_contains_borrowed_refs_inner(tree, *element, visited)
        }
        _ => false,
    };
    visited.swap_remove(&type_id);

    contains
}
    /// Return borrowed reference-like paths carried by one type.
pub fn type_borrowed_paths(tree: &Tree, ty: TypeId) -> Vec<BorrowedPath> {
    collect_borrowed_paths(tree, ty, false)
}
    /// Return borrowed paths used for origin tracking.
pub fn type_origin_paths(tree: &Tree, ty: TypeId) -> Vec<BorrowedPath> {
    collect_borrowed_paths(tree, ty, true)
}
    /// Return borrowed reference-like paths carried by one type.
fn collect_borrowed_paths(tree: &Tree, ty: TypeId, is_tracking: bool) -> Vec<BorrowedPath> {
    let mut borrowed_paths = Vec::new();

    collect_type_borrowed_paths(tree, ty, is_tracking, Path::root(), &mut borrowed_paths);

    borrowed_paths
}
    /// Collect borrowed paths carried by one type into an output vector.
fn collect_type_borrowed_paths(
    tree: &Tree,
    ty: TypeId,
    is_tracking: bool,
    path: Path,
    borrowed_paths: &mut Vec<BorrowedPath>,
) {
    let resolved = Substitution::resolve(ty, tree);
    let definition = tree.get(resolved).clone();

    match &definition {
        // record reference-like leaves
        Type::Dynamic {
            kind,
            lifetime,
            access,
            ..
        }
        | Type::Reference {
            kind,
            lifetime,
            access,
            ..
        }
        | Type::Slice {
            kind,
            lifetime,
            access,
            ..
        }
        | Type::Function {
            kind,
            lifetime,
            access,
            ..
        } => {
            let lifetime = lifetime.clone();
            // track managed handles and empty borrows only for origin
            let is_included = match kind {
                Reference::Borrowed => is_tracking || !lifetime.is_empty(),
                Reference::Managed => is_tracking,
                Reference::Unique | Reference::Raw => false,
            };
            if is_included {
                borrowed_paths.push(BorrowedPath {
                    path,
                    lifetime,
                    access: *access,
                    kind: *kind,
                });
            }
        }
        // descend into named fields
        Type::Struct { fields, .. } => {
            for (index, field) in fields.iter().enumerate() {
                let ty = tree.get(*field).ty;
                let path = path.clone().with_projection(Projection::Field {
                    index: index as u32,
                });

                collect_type_borrowed_paths(tree, ty, is_tracking, path, borrowed_paths);
            }
        }
        // descend into positional fields
        Type::Tuple { elements, .. } => {
            for (index, element) in elements.iter().enumerate() {
                let path = path.clone().with_projection(Projection::Field {
                    index: index as u32,
                });

                collect_type_borrowed_paths(tree, *element, is_tracking, path, borrowed_paths);
            }
        }
        // select the stored field of a newtype
        Type::Newtype { inner, .. } => {
            let path = path.with_projection(Projection::Field { index: 0 });

            collect_type_borrowed_paths(tree, *inner, is_tracking, path, borrowed_paths);
        }
        // preserve paths through initialization and destruction modifiers
        Type::Uninit { value: inner } | Type::ManuallyDrop { value: inner } => {
            collect_type_borrowed_paths(tree, *inner, is_tracking, path, borrowed_paths);
        }
        // descend into each possible storage shape
        Type::Variant { cases, .. } => {
            for (index, case) in cases.iter().enumerate() {
                let path = path
                    .clone()
                    .with_projection(Projection::Variant { case: index as u32 });

                collect_type_borrowed_paths(tree, case.ty, is_tracking, path, borrowed_paths);
            }
        }
        // retain each reference path within repeated elements
        Type::FixedArray { element, .. } | Type::Vector { element, .. } => {
            let path = path.with_projection(Projection::Elements);
            collect_type_borrowed_paths(tree, *element, is_tracking, path, borrowed_paths);
        }
        _ => {}
    }
}
