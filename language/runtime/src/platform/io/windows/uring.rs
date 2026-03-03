use super::core::require_out;
use crate::diagnostic::RuntimeResult;
use crate::platform::io::{UringFeatures, UringParameters, core as io_core};
use crate::platform::resource;
use crate::runtime::{BindingCallContext, NativeSlice};

/// Close one io_uring ring.
///
/// Tear down one io_uring instance and unmap ring memory.
/// Pending entries are canceled or drained by kernel behavior.
///
/// # Platform
/// Linux.
/// Uses io_uring ring teardown and unmap operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.uring`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_uring_close(
    context: &BindingCallContext,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    io_core::uring_close(context, handle)
}

/// Query io_uring feature support.
///
/// Probe one ring instance for normalized feature support metadata.
/// Feature flags are derived from host kernel capability bits.
///
/// # Platform
/// Linux.
/// Uses io_uring register and probe primitives.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.uring`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_uring_features(
    context: &BindingCallContext,
    out: *mut UringFeatures,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    require_out(out)?;
    let value = io_core::uring_features(context, handle)?;

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Open one io_uring ring.
///
/// Create one io_uring instance with explicit setup parameters.
/// Kernel feature availability is validated during setup.
///
/// # Platform
/// Linux.
/// Uses io_uring_setup and associated ring mappings.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.uring`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_uring_open(
    context: &BindingCallContext,
    out: *mut resource::UringHandle,
    parameters: UringParameters,
) -> RuntimeResult<()> {
    require_out(out)?;
    let value = io_core::uring_open(context, parameters)?;

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Register fixed buffers with a ring.
///
/// Register one fixed-buffer table from address and length lanes.
/// Buffer registration semantics follow io_uring fixed-buffer contracts.
///
/// # Platform
/// Linux.
/// Uses io_uring register buffers operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.register`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_uring_register_buffers(
    context: &BindingCallContext,
    handle: resource::UringHandle,
    addresses: NativeSlice<u64>,
    lengths: NativeSlice<u32>,
) -> RuntimeResult<()> {
    io_core::uring_register_buffers(context, handle, addresses, lengths)
}

/// Register fixed files with a ring.
///
/// Register one fixed-file table from runtime resource identifiers.
/// File registration semantics follow io_uring fixed-file contracts.
///
/// # Platform
/// Linux.
/// Uses io_uring register files operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.register`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_uring_register_files(
    context: &BindingCallContext,
    handle: resource::UringHandle,
    files: NativeSlice<resource::ResourceId>,
) -> RuntimeResult<()> {
    io_core::uring_register_files(context, handle, files)
}

/// Unregister fixed buffers for a ring.
///
/// Remove the fixed-buffer table for one ring.
/// Pending operations that reference fixed buffers follow kernel cancellation semantics.
///
/// # Platform
/// Linux.
/// Uses io_uring unregister buffers operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.register`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_uring_unregister_buffers(
    context: &BindingCallContext,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    io_core::uring_unregister_buffers(context, handle)
}

/// Unregister fixed files for a ring.
///
/// Remove the fixed-file table for one ring.
/// Pending operations that reference fixed files follow kernel cancellation semantics.
///
/// # Platform
/// Linux.
/// Uses io_uring unregister files operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.register`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_uring_unregister_files(
    context: &BindingCallContext,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    io_core::uring_unregister_files(context, handle)
}
