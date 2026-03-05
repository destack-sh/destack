use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::resource::{ResourceId, ResourceKindVm, ResourceOwnership, vm as resource_vm};
use crate::runtime::BindingCallContext;

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
pub(crate) fn destack_resource_close(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    id: ResourceId,
) -> RuntimeResult<()> {
    resource_vm::destack_resource_close(binding, context, id)
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
pub(crate) fn destack_resource_kind(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    id: ResourceId,
) -> RuntimeResult<ResourceKindVm> {
    resource_vm::destack_resource_kind(binding, context, id)
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
pub(crate) fn destack_resource_remove(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    id: ResourceId,
) -> RuntimeResult<()> {
    resource_vm::destack_resource_remove(binding, context, id)
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
pub(crate) fn destack_resource_transfer(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    id: ResourceId,
    ownership: ResourceOwnership,
) -> RuntimeResult<()> {
    resource_vm::destack_resource_transfer(binding, context, id, ownership)
}
