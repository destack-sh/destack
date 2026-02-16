use crate::diagnostic::RuntimeResult;
use crate::platform::resource;
use crate::platform::resource::native as resource_native;
use crate::runtime::RuntimeCallContext;

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
    context: &RuntimeCallContext,
    id: resource::ResourceId,
) -> RuntimeResult<()> {
    unsafe { resource_native::destack_resource_close(context, id) }
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
    context: &RuntimeCallContext,
    out: *mut resource::ResourceKind,
    id: resource::ResourceId,
) -> RuntimeResult<()> {
    unsafe { resource_native::destack_resource_kind(context, out, id) }
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
    context: &RuntimeCallContext,
    id: resource::ResourceId,
) -> RuntimeResult<()> {
    unsafe { resource_native::destack_resource_remove(context, id) }
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
    context: &RuntimeCallContext,
    id: resource::ResourceId,
    ownership: resource::ResourceOwnership,
) -> RuntimeResult<()> {
    unsafe { resource_native::destack_resource_transfer(context, id, ownership) }
}
