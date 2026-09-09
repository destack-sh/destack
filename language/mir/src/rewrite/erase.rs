use crate::{GenericArgument, Lifetime, Tree, Type, TypeId};

/// Intern one type with every region erased, an identified declaration kept as it is.
pub fn erase_regions(tree: &mut Tree, ty: TypeId) -> TypeId {
    // keep a declaration as written
    if tree.is_identified_type(ty) {
        return ty;
    }

    // erase the region written on the type and on its region arguments
    let mut erased = tree.get(ty).clone();
    erased.set_lifetime(Lifetime::empty());
    if let Type::Application { arguments, .. } = &mut erased {
        for argument in arguments.iter_mut() {
            if let GenericArgument::Region { lifetime, .. } = argument {
                *lifetime = Lifetime::empty();
            }
        }
    }

    // erase every child type
    let mut children = Vec::new();
    erased.map_child_type_ids(&mut |child| {
        children.push(child);
        child
    });
    let mut erased_children = Vec::with_capacity(children.len());
    for child in children {
        erased_children.push(erase_regions(tree, child));
    }
    let mut erased_children = erased_children.into_iter();
    erased.map_child_type_ids(&mut |_| {
        erased_children
            .next()
            .unwrap_or_else(|| unreachable!("erased child count changed"))
    });

    tree.intern_type(erased)
}
