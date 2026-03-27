use super::prelude::*;

/// Format a reference kind label for diagnostics.
pub(crate) fn reference_label(reference: ReferenceMeta) -> String {
    match reference.kind() {
        Some(kind) => {
            let address_space = reference.address_space();
            if matches!(address_space, ReferenceAddressSpace::Generic) {
                format!("{kind:?}")
            } else {
                format!("{kind:?} addrspace({})", address_space.label())
            }
        }
        None => "unknown".to_string(),
    }
}

/// Validate reference kind against the pointer storage.
pub(crate) fn check_reference_kind(
    state: &StepState<'_, '_>,
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

    // compare the declared kind against the runtime storage class
    let is_managed = matches!(pointer.tag(), ValueTag::ManagedReference);
    match kind {
        mir::ReferenceKind::Managed if !is_managed => Err(Error::InvalidReferenceKind {
            reference: reference_label(reference),
            actual: format!("{pointer:?}"),
        }),
        mir::ReferenceKind::Owned | mir::ReferenceKind::Borrowed | mir::ReferenceKind::Raw
            if is_managed =>
        {
            Err(Error::InvalidReferenceKind {
                reference: reference_label(reference),
                actual: format!("{pointer:?}"),
            })
        }
        _ => Ok(()),
    }?;

    check_reference_address_space(state, reference, pointer)
}

/// Validate reference address space against the pointer storage.
pub(crate) fn check_reference_address_space(
    state: &StepState<'_, '_>,
    reference: ReferenceMeta,
    pointer: Value,
) -> Result<(), Error> {
    // skip validation when the runtime checks are disabled
    if !state.options().checks.enforce_reference_kinds {
        return Ok(());
    }

    // generic address spaces accept any supported pointer storage
    let address_space = reference.address_space();
    if matches!(address_space, ReferenceAddressSpace::Generic) {
        return Ok(());
    }

    // reject address spaces the VM does not model yet
    if !address_space.is_supported_by_vm() {
        return Err(Error::UnsupportedAddressSpace {
            address_space: address_space.label().to_string(),
        });
    }

    // map the runtime pointer storage to one VM address space
    let actual_space = match pointer.tag() {
        ValueTag::StackPointer => ReferenceAddressSpace::Stack,
        ValueTag::LocalPointer => ReferenceAddressSpace::Stack,
        ValueTag::GlobalPointer => ReferenceAddressSpace::Global,
        ValueTag::ManagedReference => ReferenceAddressSpace::Generic,
        ValueTag::RawPointer => ReferenceAddressSpace::Generic,
        ValueTag::SharedPointer => ReferenceAddressSpace::Shared,
        _ => {
            return Err(Error::InvalidPointerType {
                actual: format!("{pointer:?}"),
            });
        }
    };

    // require the declared address space to match the runtime storage
    let is_match = match address_space {
        ReferenceAddressSpace::Stack => matches!(actual_space, ReferenceAddressSpace::Stack),
        ReferenceAddressSpace::Global | ReferenceAddressSpace::Constant => {
            matches!(actual_space, ReferenceAddressSpace::Global)
        }
        ReferenceAddressSpace::Generic => true,
        ReferenceAddressSpace::Shared => {
            matches!(actual_space, ReferenceAddressSpace::Shared)
        }
        ReferenceAddressSpace::Local | ReferenceAddressSpace::Target => false,
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
    state: &StepState<'_, '_>,
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
