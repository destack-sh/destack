use super::prelude::*;

/// Format a reference kind label for diagnostics.
pub(crate) fn reference_label(reference: ReferenceMeta) -> String {
    match reference.kind() {
        Some(kind) => {
            let address_space = reference.address_space();
            if matches!(address_space, ReferenceAddressSpace::Local) {
                format!("{kind:?}")
            } else {
                format!("{kind:?} space({})", address_space.label())
            }
        }
        None => "unknown".to_string(),
    }
}

/// Validate reference kind against the pointer value.
pub(crate) fn check_reference_kind(
    state: &DispatchState<'_, '_>,
    reference: ReferenceMeta,
    _pointer: Word,
) -> Result<(), Error> {
    // skip validation when reference-kind checks are disabled
    if !state.options().checks.enforce_reference_kinds {
        return Ok(());
    }

    check_reference_address_space(state, reference)
}

/// Validate the declared reference address space.
pub(crate) fn check_reference_address_space(
    state: &DispatchState<'_, '_>,
    reference: ReferenceMeta,
) -> Result<(), Error> {
    // skip validation when the runtime checks are disabled
    if !state.options().checks.enforce_reference_kinds {
        return Ok(());
    }

    // local address spaces accept local runtime pointer values
    let address_space = reference.address_space();
    if !address_space.is_supported_by_vm() {
        return Err(Error::UnsupportedAddressSpace {
            address_space: address_space.label().to_string(),
        });
    }

    Ok(())
}

/// Validate reference mutability for stores.
pub(crate) fn check_reference_mutability(
    state: &DispatchState<'_, '_>,
    reference: ReferenceMeta,
) -> Result<(), Error> {
    // skip validation when the runtime checks are disabled
    if !state.options().checks.enforce_reference_mutability {
        return Ok(());
    }

    // unspecified mutability does not constrain writes
    let Some(mutability) = reference.mutability() else {
        return Ok(());
    };

    // reject writes through immutable references
    if matches!(mutability, mir::Mutability::Immutable) {
        return Err(Error::ImmutableReferenceWrite {
            reference: reference_label(reference),
        });
    }

    Ok(())
}
