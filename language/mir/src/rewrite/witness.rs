use crate::{Field, Static, StaticId, Tree, Type, TypeId, WitnessTable};

/// Resolve every closed associated type within one type through the witnesses answering for it.
pub fn resolve_witness_types(tree: &Tree, witnesses: &WitnessTable, ty: TypeId) -> TypeId {
    if tree.is_identified_type(ty) {
        return ty;
    }

    // resolve the projection the witness answers, leaving an open one in place
    let mut resolved = tree.get(ty).clone();
    if let Type::Witness {
        receiver,
        interface,
        member,
    } = resolved
        && let Some(answer) = witnesses.associated_type(tree, receiver, interface, member)
    {
        return resolve_witness_types(tree, witnesses, answer);
    }

    // resolve the children
    if let Type::Struct { fields, .. } | Type::Class { fields, .. } = &mut resolved {
        for field in fields {
            let declared = tree.get(*field).clone();
            let ty = resolve_witness_types(tree, witnesses, declared.ty);
            *field = tree.intern_field(Field { ty, ..declared });
        }
    }
    resolved.map_child_type_ids(&mut |child| resolve_witness_types(tree, witnesses, child));
    resolved.map_values(&mut |value| resolve_witness_value(tree, witnesses, value));

    tree.intern_type(resolved)
}

/// Resolve one closed associated const value through the witness answering for it.
fn resolve_witness_value(tree: &Tree, witnesses: &WitnessTable, value: StaticId) -> StaticId {
    let Static::Witness {
        receiver,
        interface,
        member,
    } = *tree.static_value(value)
    else {
        return value;
    };

    witnesses
        .associated_constant(tree, receiver, interface, member)
        .unwrap_or(value)
}
