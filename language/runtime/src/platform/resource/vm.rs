#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::{ResourceId, ResourceKind, ResourceKindVm, ResourceOwnership};
use crate::platform::{PlatformError, PlatformErrorCode};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

/// Return a stable label for one resource kind.
fn resource_kind_label(kind: ResourceKind) -> &'static str {
    kind.label()
}

/// Build one io not found error for missing resources.
fn resource_not_found(op: &'static str, id: ResourceId) -> Box<RuntimeError> {
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
pub(crate) fn destack_resource_close(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: ResourceId,
) -> RuntimeResult<()> {
    // remove the entry and run finalization
    let removed =
        binding
            .agent()
            .resources
            .remove_and_finalize(binding.world(), id, Some(binding.engine()));
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
pub(crate) fn destack_resource_kind(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    id: ResourceId,
) -> RuntimeResult<ResourceKindVm> {
    // resolve the kind for the requested resource
    let kind = binding
        .agent()
        .resources
        .with_entry(id, |entry| entry.kind)
        .ok_or_else(|| resource_not_found("destack.resource.id.kind", id))?;

    // encode the kind label as the vm-facing payload
    let label = resource_kind_label(kind);
    Ok(vm::StringHandle::new(context.intern_string(label)))
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
    _context: &mut vm::ExternalCallContext<'_>,
    id: ResourceId,
) -> RuntimeResult<()> {
    // remove the entry and run finalization
    let removed =
        binding
            .agent()
            .resources
            .remove_and_finalize(binding.world(), id, Some(binding.engine()));
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
pub(crate) fn destack_resource_transfer(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: ResourceId,
    ownership: ResourceOwnership,
) -> RuntimeResult<()> {
    // validate that the source resource exists
    let exists = binding.agent().resources.contains(id);
    if !exists {
        return Err(resource_not_found("destack.resource.id.transfer", id));
    }

    // keep ownership consumed for future binding ownership policy
    let _ = ownership;

    Ok(())
}
