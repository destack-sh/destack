use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::resource::{ResourceId, ResourceKindVm, ResourceOwnership};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.resource.close.
pub(super) fn destack_resource_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: ResourceId,
) -> RuntimeResult<()> {
    let _ = id;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.resource.close is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.resource.kind.
pub(super) fn destack_resource_kind(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: ResourceId,
) -> RuntimeResult<ResourceKindVm> {
    let _ = id;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.resource.kind is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.resource.remove.
pub(super) fn destack_resource_remove(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: ResourceId,
) -> RuntimeResult<()> {
    let _ = id;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.resource.remove is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.resource.transfer.
pub(super) fn destack_resource_transfer(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: ResourceId,
    ownership: ResourceOwnership,
) -> RuntimeResult<()> {
    let _ = (id, ownership);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.resource.transfer is not available in the VM yet",
    ))
    .boxed())
}
