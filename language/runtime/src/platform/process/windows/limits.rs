#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;

use crate::runtime::BindingCallContext;

use crate::platform::process::{ProcessLimit, ProcessLimitResource};
/// Read a process resource limit.
pub(crate) unsafe fn destack_process_get_limit(
    binding: &BindingCallContext,
    out: *mut ProcessLimit,
    resource: ProcessLimitResource,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, resource);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.limits.getLimit",
    ))
    .boxed())
}

/// Set a process resource limit.
pub(crate) unsafe fn destack_process_set_limit(
    binding: &BindingCallContext,
    resource: ProcessLimitResource,
    limit: ProcessLimit,
) -> RuntimeResult<()> {
    let _ = (binding, resource, limit);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.limits.setLimit",
    ))
    .boxed())
}
