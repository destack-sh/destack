use super::core::require_out;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
use crate::platform::io::{UringFeatures, UringParameters};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Close one io_uring ring.
pub(crate) unsafe fn destack_io_uring_close(
    _binding: &BindingCallContext,
    _handle: resource::UringHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.io.uring.close")).boxed())
}

/// Query io_uring feature support.
pub(crate) unsafe fn destack_io_uring_features(
    binding: &BindingCallContext,
    out: *mut UringFeatures,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    require_out(out)?;
    let _ = (binding, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.uring.features")).boxed())
}

/// Open one io_uring ring.
pub(crate) unsafe fn destack_io_uring_open(
    binding: &BindingCallContext,
    out: *mut resource::UringHandle,
    parameters: UringParameters,
) -> RuntimeResult<()> {
    require_out(out)?;
    let _ = (binding, parameters);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.uring.open")).boxed())
}

/// Register fixed buffers with a ring.
pub(crate) unsafe fn destack_io_uring_register_buffers(
    binding: &BindingCallContext,
    handle: resource::UringHandle,
    addresses: NativeSlice<u64>,
    lengths: NativeSlice<u32>,
) -> RuntimeResult<()> {
    let _ = (binding, handle, addresses, lengths);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.uring.registerBuffers",
    ))
    .boxed())
}

/// Register fixed files with a ring.
pub(crate) unsafe fn destack_io_uring_register_files(
    binding: &BindingCallContext,
    handle: resource::UringHandle,
    files: NativeSlice<resource::ResourceId>,
) -> RuntimeResult<()> {
    let _ = (binding, handle, files);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.uring.registerFiles",
    ))
    .boxed())
}

/// Unregister fixed buffers for a ring.
pub(crate) unsafe fn destack_io_uring_unregister_buffers(
    _binding: &BindingCallContext,
    _handle: resource::UringHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.uring.unregisterBuffers",
    ))
    .boxed())
}

/// Unregister fixed files for a ring.
pub(crate) unsafe fn destack_io_uring_unregister_files(
    _binding: &BindingCallContext,
    _handle: resource::UringHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.uring.unregisterFiles",
    ))
    .boxed())
}
