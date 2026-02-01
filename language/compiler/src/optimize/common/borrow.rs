use destack_mir as mir;

/// Check whether a type contains any borrowed references.
pub fn type_contains_borrowed_refs(ty: &mir::Type, tree: &mir::NodeTree) -> bool {
    // check direct borrowed references
    if ty.is_borrowed_reference() {
        return true;
    }

    match ty {
        mir::Type::Struct { fields, .. } => fields.iter().any(|field_id| {
            let field = tree.get(*field_id);
            let field_ty = tree.get(field.ty);
            type_contains_borrowed_refs(field_ty, tree)
        }),
        mir::Type::Newtype { inner, .. } => {
            let inner_ty = tree.get(*inner);
            type_contains_borrowed_refs(inner_ty, tree)
        }
        mir::Type::Tuple { elements, .. } => elements.iter().any(|elem_id| {
            let elem = tree.get(*elem_id);
            type_contains_borrowed_refs(elem, tree)
        }),
        mir::Type::Array { element, .. } => {
            let elem_ty = tree.get(*element);
            type_contains_borrowed_refs(elem_ty, tree)
        }
        _ => false,
    }
}

/// Check whether a function signature returns borrowed references.
pub fn signature_return_contains_borrowed_refs(
    signature_type: &mir::Type,
    tree: &mir::NodeTree,
) -> bool {
    // default to borrowed for unknown signatures
    let mir::Type::FunctionPointer { result, .. } = signature_type else {
        return true;
    };

    let result_type = tree.get(*result);
    type_contains_borrowed_refs(result_type, tree)
}

/// Collect borrowed parameter indices from a function signature.
pub fn borrowed_parameter_indices_for_signature(
    signature_type: &mir::Type,
    tree: &mir::NodeTree,
) -> Option<Vec<usize>> {
    let mir::Type::FunctionPointer { parameters, .. } = signature_type else {
        return None;
    };

    let indices = parameters
        .iter()
        .enumerate()
        .filter(|(_, ty_id)| tree.get(**ty_id).is_borrowed_reference())
        .map(|(index, _)| index)
        .collect();

    Some(indices)
}

/// Collect borrowed parameter indices from a function definition.
pub fn borrowed_parameter_indices_for_function(
    function: &mir::Function,
    tree: &mir::NodeTree,
) -> Vec<u32> {
    function
        .parameters
        .iter()
        .enumerate()
        .filter(|(_, param)| tree.get(param.ty).is_borrowed_reference())
        .map(|(index, _)| index as u32)
        .collect()
}
