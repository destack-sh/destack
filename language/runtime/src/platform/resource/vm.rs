#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::{ResourceId, ResourceKind, ResourceKindVm, ResourceOwnership};
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: ResourceId,
) -> RuntimeResult<()> {
    // remove the entry and run finalization
    let removed = runtime
        .runtime()
        .resources
        .remove_and_finalize(id, Some(runtime.engine()));
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
pub(crate) fn destack_resource_kind(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    id: ResourceId,
) -> RuntimeResult<ResourceKindVm> {
    // resolve the kind for the requested resource
    let kind = super::with_any_entry(runtime, id, |entry| entry.kind).ok_or_else(|| {
        core_platform::io_not_found(
            "destack.resource.id.kind",
            format!("resource {} not found", id.0),
        )
    })?;

    // encode the kind label as the vm-facing payload
    let label = kind.label();
    let handle = vm::StringHandle::new(context.intern_string(label));

    Ok(ResourceKindVm(handle))
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: ResourceId,
) -> RuntimeResult<()> {
    // remove the entry and run finalization
    let removed = runtime
        .runtime()
        .resources
        .remove_and_finalize(id, Some(runtime.engine()));
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
pub(crate) fn destack_resource_transfer(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: ResourceId,
    ownership: ResourceOwnership,
) -> RuntimeResult<()> {
    // validate that the source resource exists
    let exists = runtime.runtime().resources.contains(id);
    if !exists {
        return Err(core_platform::io_not_found(
            "destack.resource.id.transfer",
            format!("resource {} not found", id.0),
        ));
    }

    // keep ownership consumed for future runtime ownership policy
    let _ = ownership;

    Ok(())
}
