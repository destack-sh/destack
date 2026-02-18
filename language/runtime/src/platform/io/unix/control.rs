use super::core::require_out;
use crate::diagnostic::RuntimeResult;
use crate::platform::io::{
    DescriptorControlCommand, DescriptorControlFlags, DescriptorRequest, DescriptorResult,
    host as io_host,
};
use crate::platform::resource;
use crate::runtime::RuntimeCallContext;

/// Execute one fcntl-style descriptor command.
///
/// Forward one descriptor control command to the host kernel for the target resource.
/// Command semantics and valid arguments follow the active host ABI.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` when host mapping is unavailable.
/// Uses fcntl(2) style controls on Unix and host descriptor control adapters on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `io.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_control_fcntl(
    context: &RuntimeCallContext,
    out: *mut i64,
    handle: resource::ResourceId,
    command: DescriptorControlCommand,
    argument: u64,
    flags: DescriptorControlFlags,
) -> RuntimeResult<()> {
    require_out(out)?;
    let value = io_host::host_control_fcntl(context, handle, command, argument, flags)?;

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Execute one ioctl-style descriptor request.
///
/// Forward one ioctl request with opaque payload bytes to the host kernel for the target resource.
/// Request code semantics and payload layout follow the active host ABI.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` when host mapping is unavailable.
/// Uses ioctl(2) style controls on Unix and DeviceIoControl or ioctlsocket adapters on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_control_ioctl(
    context: &RuntimeCallContext,
    out: *mut DescriptorResult,
    handle: resource::ResourceId,
    request: DescriptorRequest,
) -> RuntimeResult<()> {
    require_out(out)?;
    let value = io_host::host_control_ioctl(context, handle, request)?;

    unsafe {
        out.write(value);
    }

    Ok(())
}
