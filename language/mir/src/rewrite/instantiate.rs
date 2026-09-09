use crate::{GenericArgument, Lifetime, LifetimeTerm, Tree, Type, TypeId};

/// Intern one type with its late-bound region slots instantiated at the given lifetimes.
pub fn instantiate_slots(tree: &mut Tree, ty: TypeId, slots: &[Lifetime]) -> TypeId {
    if tree.is_identified_type(ty) || matches!(tree.get(ty), Type::FunctionSignature { .. }) {
        return ty;
    }

    // instantiate the region written on the type and on its region arguments
    let mut instantiated = tree.get(ty).clone();
    match &mut instantiated {
        Type::Dynamic { lifetime, .. }
        | Type::Reference { lifetime, .. }
        | Type::Slice { lifetime, .. }
        | Type::Function { lifetime, .. } => *lifetime = instantiate_lifetime(lifetime, slots),
        Type::Application { arguments, .. } => {
            for argument in arguments.iter_mut() {
                if let GenericArgument::Region { lifetime, .. } = argument {
                    *lifetime = instantiate_lifetime(lifetime, slots);
                }
            }
        }
        _ => {}
    }

    // instantiate every child type
    let mut children = Vec::new();
    instantiated.map_child_type_ids(&mut |child| {
        children.push(child);
        child
    });
    let mut instantiated_children = Vec::with_capacity(children.len());
    for child in children {
        instantiated_children.push(instantiate_slots(tree, child, slots));
    }
    let mut instantiated_children = instantiated_children.into_iter();
    instantiated.map_child_type_ids(&mut |_| {
        instantiated_children
            .next()
            .unwrap_or_else(|| unreachable!("instantiated child count changed"))
    });

    tree.intern_type(instantiated)
}

/// Return one lifetime with each slot replaced by the lifetime bound at it.
fn instantiate_lifetime(lifetime: &Lifetime, slots: &[Lifetime]) -> Lifetime {
    Lifetime::new(lifetime.terms.iter().flat_map(|term| {
        match term {
            LifetimeTerm::Slot(slot) => slots
                .get(slot.0 as usize)
                .unwrap_or_else(|| {
                    unreachable!("a region slot {} outside its instantiation", slot.0)
                })
                .terms
                .clone(),
            term => vec![*term],
        }
    }))
}
