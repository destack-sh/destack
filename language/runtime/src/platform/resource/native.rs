use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{PlatformError, resource};
use crate::runtime::RuntimeCallContext;

/// Return one not supported error for a resource binding.
fn missing_binding(binding_name: &'static str) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(binding_name)).boxed())
}

/// Validate one required output pointer.
unsafe fn check_out_pointer<T>(out: *mut T, name: &'static str) -> RuntimeResult<()> {
    // reject null pointers explicitly
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer(name)).boxed());
    }

    Ok(())
}

/// Close one resource by identifier.
pub(crate) unsafe fn destack_resource_close(
    _context: &RuntimeCallContext,
    id: resource::ResourceId,
) -> RuntimeResult<()> {
    // keep arguments used
    let _ = id;

    missing_binding("destack.resource.close")
}

/// Read the kind of one resource.
pub(crate) unsafe fn destack_resource_kind(
    _context: &RuntimeCallContext,
    out: *mut resource::ResourceKind,
    id: resource::ResourceId,
) -> RuntimeResult<()> {
    // validate pointer and keep arguments used
    unsafe { check_out_pointer(out, "out")? };
    let _ = (out, id);

    missing_binding("destack.resource.kind")
}

/// Remove one resource by identifier.
pub(crate) unsafe fn destack_resource_remove(
    _context: &RuntimeCallContext,
    id: resource::ResourceId,
) -> RuntimeResult<()> {
    // keep arguments used
    let _ = id;

    missing_binding("destack.resource.remove")
}

/// Transfer ownership of one resource.
pub(crate) unsafe fn destack_resource_transfer(
    _context: &RuntimeCallContext,
    id: resource::ResourceId,
    ownership: resource::ResourceOwnership,
) -> RuntimeResult<()> {
    // keep arguments used
    let _ = (id, ownership);

    missing_binding("destack.resource.transfer")
}
