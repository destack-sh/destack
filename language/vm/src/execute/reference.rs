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
    state: &ExecutionState<'_, '_>,
    reference: ReferenceMeta,
    pointer: Value,
) -> Result<(), Error> {
    // skip validation when reference-kind checks are disabled
    if !state.options().checks.enforce_reference_kinds {
        return Ok(());
    }

    // untyped references do not constrain the runtime pointer
    let Some(kind) = reference.kind() else {
        return Ok(());
    };

    // compare the declared kind against the runtime pointer kind
    let is_typed = matches!(
        pointer.tag(),
        ValueTag::HeapReference | ValueTag::SharedHeapReference
    );
    match kind {
        mir::ReferenceKind::Managed | mir::ReferenceKind::Owned if !is_typed => {
            Err(Error::InvalidReferenceKind {
                reference: reference_label(reference),
                actual: format!("{pointer:?}"),
            })
        }
        mir::ReferenceKind::Raw if is_typed => Err(Error::InvalidReferenceKind {
            reference: reference_label(reference),
            actual: format!("{pointer:?}"),
        }),
        _ => Ok(()),
    }?;

    check_reference_address_space(state, reference, pointer)
}

/// Validate reference address space against the pointer value.
pub(crate) fn check_reference_address_space(
    state: &ExecutionState<'_, '_>,
    reference: ReferenceMeta,
    pointer: Value,
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

    // map the runtime pointer value to one VM address space
    let actual_space = match pointer.tag() {
        ValueTag::StackPointer => ReferenceAddressSpace::Stack,
        ValueTag::FramePointer => ReferenceAddressSpace::Frame,
        ValueTag::StaticPointer => pointer.reference_meta().address_space(),
        ValueTag::HeapReference => ReferenceAddressSpace::Local,
        ValueTag::SharedHeapReference => ReferenceAddressSpace::Shared,
        ValueTag::RawPointer => ReferenceAddressSpace::Local,
        ValueTag::SharedRawPointer => ReferenceAddressSpace::Shared,
        _ => {
            return Err(Error::InvalidPointerType {
                actual: format!("{pointer:?}"),
            });
        }
    };

    // require the declared address space to match the runtime storage
    let is_match = match address_space {
        ReferenceAddressSpace::Local => matches!(actual_space, ReferenceAddressSpace::Local),
        ReferenceAddressSpace::Shared => {
            matches!(actual_space, ReferenceAddressSpace::Shared)
        }
        ReferenceAddressSpace::Stack => matches!(actual_space, ReferenceAddressSpace::Stack),
        ReferenceAddressSpace::Frame => matches!(actual_space, ReferenceAddressSpace::Frame),
        ReferenceAddressSpace::Static => {
            matches!(actual_space, ReferenceAddressSpace::Static)
        }
        ReferenceAddressSpace::Named => false,
    };

    if !is_match {
        return Err(Error::InvalidAddressSpace {
            expected: address_space.label().to_string(),
            actual: actual_space.label().to_string(),
        });
    }

    Ok(())
}

/// Validate reference mutability for stores.
pub(crate) fn check_reference_mutability(
    state: &ExecutionState<'_, '_>,
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
