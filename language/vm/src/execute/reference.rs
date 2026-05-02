use crate::diagnostic::Error;
use crate::interpreter::DispatchState;
use crate::{ReferenceAddressSpace, ReferenceMeta};
use destack_mir as mir;

/// Format a reference kind label for diagnostics.
#[cold]
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

/// Build an unsupported address-space error.
#[cold]
#[inline(never)]
fn unsupported_address_space(reference: ReferenceMeta) -> Error {
    let address_space = reference.address_space();

    Error::UnsupportedAddressSpace {
        address_space: address_space.label().to_string(),
    }
}

/// Build an immutable-reference store error.
#[cold]
#[inline(never)]
fn immutable_reference_write(reference: ReferenceMeta) -> Error {
    Error::ImmutableReferenceWrite {
        reference: reference_label(reference),
    }
}

/// Validate the declared reference address space.
#[inline(always)]
pub(crate) fn check_reference_address_space(
    state: &DispatchState<'_, '_>,
    reference: ReferenceMeta,
) -> Result<(), Error> {
    // skip validation when the runtime checks are disabled
    if !state.reference_kind_checks {
        return Ok(());
    }

    // local address spaces accept local runtime pointer values
    let address_space = reference.address_space();
    if !address_space.is_supported_by_vm() {
        return Err(unsupported_address_space(reference));
    }

    Ok(())
}

/// Validate reference mutability for stores.
#[inline(always)]
pub(crate) fn check_reference_mutability(
    state: &DispatchState<'_, '_>,
    reference: ReferenceMeta,
) -> Result<(), Error> {
    // skip validation when the runtime checks are disabled
    if !state.reference_mutability_checks {
        return Ok(());
    }

    // unspecified mutability does not constrain writes
    let Some(mutability) = reference.mutability() else {
        return Ok(());
    };

    // reject writes through immutable references
    if matches!(mutability, mir::Mutability::Immutable) {
        return Err(immutable_reference_write(reference));
    }

    Ok(())
}
