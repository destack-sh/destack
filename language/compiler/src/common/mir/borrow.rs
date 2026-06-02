use destack_mir as mir;

/// Check whether a function signature returns borrowed references.
pub fn signature_return_contains_borrowed_refs(
    signature_type: &mir::TypeReference,
    tree: &mir::Tree,
) -> bool {
    let Some(signature_type) = signature_type.ty() else {
        return true;
    };

    // default to borrowed for unknown signatures
    let signature_type = tree.get(signature_type);
    let Some((_, result)) = mir::function_signature_parts(signature_type) else {
        return true;
    };

    tree.type_reference_contains_borrowed_refs(&result)
}

/// Collect borrowed parameter indices from a function signature.
pub fn borrowed_parameter_indices_for_signature(
    signature_type: &mir::TypeReference,
    tree: &mir::Tree,
) -> Option<Vec<usize>> {
    let signature_type = signature_type.ty()?;
    let signature_type = tree.get(signature_type);

    let (parameters, _) = mir::function_signature_parts(signature_type)?;

    let indices = parameters
        .iter()
        .enumerate()
        .filter(|(_, ty)| tree.type_reference_contains_borrowed_refs(ty))
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
        .filter(|(_, param)| tree.type_reference_contains_borrowed_refs(&param.ty))
        .map(|(index, _)| index as u32)
        .collect()
}
