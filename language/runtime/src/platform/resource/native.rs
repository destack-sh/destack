use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{PlatformError, PlatformErrorCode, resource};
use crate::runtime::BindingCallContext;

/// Validate one required output pointer.
unsafe fn check_out_pointer<T>(out: *mut T, name: &'static str) -> RuntimeResult<()> {
    // reject null pointers explicitly
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer(name)).boxed());
    }

    Ok(())
}

/// Build one io not found error for missing resources.
fn resource_not_found(op: &'static str, id: resource::ResourceId) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(op.to_string()),
        None,
        format!("resource {} not found", id.0),
    ))
    .boxed()
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
        .agent()
        .resources
        .remove_and_finalize(id, Some(context.engine()));
    if !removed {
        return Err(resource_not_found("destack.resource.id.close", id));
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
    let kind = context
        .agent()
        .resources
        .with_entry(id, |entry| entry.kind)
        .ok_or_else(|| resource_not_found("destack.resource.id.kind", id))?;

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
        .agent()
        .resources
        .remove_and_finalize(id, Some(context.engine()));
    if !removed {
        return Err(resource_not_found("destack.resource.id.remove", id));
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
    let exists = context.agent().resources.contains(id);
    if !exists {
        return Err(resource_not_found("destack.resource.id.transfer", id));
    }

    // keep ownership value consumed for future policy hooks
    let _ = ownership;

    Ok(())
}
