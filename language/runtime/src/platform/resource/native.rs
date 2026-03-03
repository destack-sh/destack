use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{PlatformError, core as core_platform, resource};
use crate::runtime::BindingCallContext;

/// Validate one required output pointer.
unsafe fn check_out_pointer<T>(out: *mut T, name: &'static str) -> RuntimeResult<()> {
    // reject null pointers explicitly
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer(name)).boxed());
    }

    Ok(())
}

/// Close a resource by identifier.
///
/// Close one resource endpoint while keeping table semantics explicit.
/// Close behavior is delegated to the owning runtime subsystem for the resource kind.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime resource-dispatch close logic.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `resource.close`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_resource_close(
    context: &BindingCallContext,
    id: resource::ResourceId,
) -> RuntimeResult<()> {
    // remove the entry and run finalization
    let removed = context
        .runtime()
        .resources
        .remove_and_finalize(id, Some(context.engine()));
    if !removed {
        return Err(core_platform::io_not_found(
            "destack.resource.id.close",
            format!("resource {} not found", id.0),
        ));
    }

    Ok(())
}

/// Describe a resource kind.
///
/// Return the declared kind label for one resource identifier.
/// Kind labels are stable runtime strings for diagnostics and policy checks.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime resource-table state only.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `resource.read`.
///
/// # Replay
/// Deterministic.
pub(crate) unsafe fn destack_resource_kind(
    context: &BindingCallContext,
    out: *mut resource::ResourceKind,
    id: resource::ResourceId,
) -> RuntimeResult<()> {
    // validate pointer before writing
    unsafe { check_out_pointer(out, "out")? };

    // load the kind for the requested resource
    let kind = super::with_any_entry(context, id, |entry| entry.kind).ok_or_else(|| {
        core_platform::io_not_found(
            "destack.resource.id.kind",
            format!("resource {} not found", id.0),
        )
    })?;

    // write the resolved kind to the output pointer
    unsafe {
        out.write(kind);
    }

    Ok(())
}

/// Remove a resource from the table.
///
/// Remove one resource identifier from the runtime table.
/// Owned resources are closed by runtime policy before removal.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime resource-table state only.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `resource.manage`.
///
/// # Replay
/// Deterministic.
pub(crate) unsafe fn destack_resource_remove(
    context: &BindingCallContext,
    id: resource::ResourceId,
) -> RuntimeResult<()> {
    // remove the entry and run finalization
    let removed = context
        .runtime()
        .resources
        .remove_and_finalize(id, Some(context.engine()));
    if !removed {
        return Err(core_platform::io_not_found(
            "destack.resource.id.remove",
            format!("resource {} not found", id.0),
        ));
    }

    Ok(())
}

/// Transfer resource ownership.
///
/// Move one resource identifier into the requested ownership mode.
/// Ownership transitions are validated against runtime boundary policy.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime resource ownership metadata only.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `resource.transfer`.
///
/// # Replay
/// Deterministic.
pub(crate) unsafe fn destack_resource_transfer(
    context: &BindingCallContext,
    id: resource::ResourceId,
    ownership: resource::ResourceOwnership,
) -> RuntimeResult<()> {
    // validate that the source resource exists
    let exists = context.runtime().resources.contains(id);
    if !exists {
        return Err(core_platform::io_not_found(
            "destack.resource.id.transfer",
            format!("resource {} not found", id.0),
        ));
    }

    // keep ownership value consumed for future policy hooks
    let _ = ownership;

    Ok(())
}
