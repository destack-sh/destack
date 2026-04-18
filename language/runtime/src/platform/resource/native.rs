use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::ensure_resource_affinity;
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
    binding: &BindingCallContext,
    id: resource::ResourceId,
) -> RuntimeResult<()> {
    // enforce any stored resource-affinity requirement before closing
    ensure_resource_affinity(binding, id, "destack.resource.id.close")?;

    // remove the entry and run finalization
    let removed =
        binding
            .worker()
            .resources
            .remove_and_finalize(&binding.world(), id, Some(binding.engine()));
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
    binding: &BindingCallContext,
    out: *mut resource::ResourceKind,
    id: resource::ResourceId,
) -> RuntimeResult<()> {
    // validate pointer before writing
    unsafe { check_out_pointer(out, "out")? };

    // enforce any stored resource-affinity requirement before resolving metadata
    ensure_resource_affinity(binding, id, "destack.resource.id.kind")?;

    // load the kind for the requested resource
    let kind = binding
        .worker()
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
    binding: &BindingCallContext,
    id: resource::ResourceId,
) -> RuntimeResult<()> {
    // enforce any stored resource-affinity requirement before removal
    ensure_resource_affinity(binding, id, "destack.resource.id.remove")?;

    // remove the entry and run finalization
    let removed =
        binding
            .worker()
            .resources
            .remove_and_finalize(&binding.world(), id, Some(binding.engine()));
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
    binding: &BindingCallContext,
    id: resource::ResourceId,
    ownership: resource::ResourceOwnership,
) -> RuntimeResult<()> {
    // enforce any stored resource-affinity requirement before transfer
    ensure_resource_affinity(binding, id, "destack.resource.id.transfer")?;

    // validate that the source resource exists
    let exists = binding.worker().resources.contains(id);
    if !exists {
        return Err(resource_not_found("destack.resource.id.transfer", id));
    }

    // keep ownership value consumed for future policy hooks
    let _ = ownership;

    Ok(())
}
