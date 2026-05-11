use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::{
    ResourceId, ResourceKind, ResourceKindVm, ResourceOwnership, ensure_resource_affinity,
};
use crate::platform::{PlatformError, PlatformErrorCode};
use crate::runtime::BindingCallContext;
use destack_vm;

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
pub(crate) fn destack_resource_close(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    id: ResourceId,
) -> RuntimeResult<()> {
    // enforce any stored resource-affinity requirement before closing
    ensure_resource_affinity(binding, id, "destack.resource.id.close")?;

    // remove the entry and run finalization
    let removed = binding.worker().resources.remove_and_finalize(
        &binding.world(),
        id,
        Some(binding.engine()),
    );
    if !removed {
        return Err(resource_not_found("destack.resource.id.close", id));
    }

    Ok(())
}

/// Describe a resource kind.
pub(crate) fn destack_resource_kind(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    id: ResourceId,
) -> RuntimeResult<ResourceKindVm> {
    // enforce any stored resource-affinity requirement before resolving metadata
    ensure_resource_affinity(binding, id, "destack.resource.id.kind")?;

    // resolve the kind for the requested resource
    let kind = binding
        .worker()
        .resources
        .with_entry(id, |entry| entry.kind)
        .ok_or_else(|| resource_not_found("destack.resource.id.kind", id))?;

    // encode the kind label as the vm-facing payload
    let label = resource_kind_label(kind);
    context
        .string_handle(label)
        .map_err(Box::<RuntimeError>::from)
}

/// Remove a resource from the table.
pub(crate) fn destack_resource_remove(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    id: ResourceId,
) -> RuntimeResult<()> {
    // enforce any stored resource-affinity requirement before removal
    ensure_resource_affinity(binding, id, "destack.resource.id.remove")?;

    // remove the entry and run finalization
    let removed = binding.worker().resources.remove_and_finalize(
        &binding.world(),
        id,
        Some(binding.engine()),
    );
    if !removed {
        return Err(resource_not_found("destack.resource.id.remove", id));
    }

    Ok(())
}

/// Transfer resource ownership.
pub(crate) fn destack_resource_transfer(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    id: ResourceId,
    ownership: ResourceOwnership,
) -> RuntimeResult<()> {
    // enforce any stored resource-affinity requirement before transfer
    ensure_resource_affinity(binding, id, "destack.resource.id.transfer")?;

    // validate that the source resource exists
    let exists = binding.worker().resources.contains(id);
    if !exists {
        return Err(resource_not_found("destack.resource.id.transfer", id));
    }

    // keep ownership consumed for future binding ownership policy
    let _ = ownership;

    Ok(())
}
