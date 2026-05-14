use destack_mir as mir;

/// Check whether a type contains any borrowed references.
pub fn type_contains_borrowed_refs(ty: &mir::Type, tree: &mir::Tree) -> bool {
    // check direct borrowed references
    if ty.is_borrowed_reference() {
        return true;
    }

    match ty {
        mir::Type::Struct { fields, .. } => fields.iter().any(|field_id| {
            let field = tree.get(*field_id);
            let Some(field_ty) = field.ty.ty() else {
                return true;
            };
            let field_ty = tree.get(field_ty);
            type_contains_borrowed_refs(field_ty, tree)
        }),
        mir::Type::Newtype { inner, .. } => {
            let Some(inner_ty) = inner.ty() else {
                return true;
            };
            let inner_ty = tree.get(inner_ty);
            type_contains_borrowed_refs(inner_ty, tree)
        }
        mir::Type::Tuple { elements, .. } => elements.iter().any(|element| {
            let Some(element) = element.ty() else {
                return true;
            };

            let elem = tree.get(element);
            type_contains_borrowed_refs(elem, tree)
        }),
        mir::Type::Array { element, .. } => {
            let Some(elem_ty) = element.ty() else {
                return true;
            };
            let elem_ty = tree.get(elem_ty);
            type_contains_borrowed_refs(elem_ty, tree)
        }
        _ => false,
    }
}

/// Check whether a function signature returns borrowed references.
pub fn signature_return_contains_borrowed_refs(
    signature_type: impl Into<mir::TypeReference>,
    tree: &mir::Tree,
) -> bool {
    let Some(signature_type) = signature_type.into().ty() else {
        return true;
    };

    // default to borrowed for unknown signatures
    let signature_type = tree.get(signature_type);
    let Some((_, result)) = mir::function_signature_parts(signature_type) else {
        return true;
    };

    let Some(result_type) = result.ty() else {
        return true;
    };
    let result_type = tree.get(result_type);
    type_contains_borrowed_refs(result_type, tree)
}

/// Collect borrowed parameter indices from a function signature.
pub fn borrowed_parameter_indices_for_signature(
    signature_type: impl Into<mir::TypeReference>,
    tree: &mir::Tree,
) -> Option<Vec<usize>> {
    let signature_type = signature_type.into().ty()?;
    let signature_type = tree.get(signature_type);

    let Some((parameters, _)) = mir::function_signature_parts(signature_type) else {
        return None;
    };

    let indices = parameters
        .iter()
        .enumerate()
        .filter(|(_, ty_id)| {
            ty_id
                .ty()
                .is_none_or(|ty_id| tree.get(ty_id).is_borrowed_reference())
        })
        .map(|(index, _)| index)
        .collect();

    Some(indices)
}

/// Collect borrowed parameter indices from a function definition.
pub fn borrowed_parameter_indices_for_function(
    function: &mir::Function,
    tree: &mir::Tree,
) -> Vec<u32> {
    function
        .parameters
        .iter()
        .enumerate()
        .filter(|(_, param)| {
            param
                .ty
                .ty()
                .is_none_or(|ty| tree.get(ty).is_borrowed_reference())
        })
        .map(|(index, _)| index as u32)
        .collect()
}
