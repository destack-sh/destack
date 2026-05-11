use super::core::require_out;
use crate::diagnostic::RuntimeResult;
use crate::platform::io::{
    DescriptorControlCommand, DescriptorControlFlags, DescriptorRequest, DescriptorResult,
    host as io_host,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

/// Execute one fcntl-style descriptor command.
pub(crate) unsafe fn destack_io_control_fcntl(
    binding: &BindingCallContext,
    out: *mut i64,
    handle: resource::ResourceId,
    command: DescriptorControlCommand,
    argument: u64,
    flags: DescriptorControlFlags,
) -> RuntimeResult<()> {
    require_out(out)?;
    let value = io_host::host_control_fcntl(binding, handle, command, argument, flags)?;

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Execute one ioctl-style descriptor request.
pub(crate) unsafe fn destack_io_control_ioctl(
    binding: &BindingCallContext,
    out: *mut DescriptorResult,
    handle: resource::ResourceId,
    request: DescriptorRequest,
) -> RuntimeResult<()> {
    require_out(out)?;
    let value = io_host::host_control_ioctl(binding, handle, request)?;

    unsafe {
        out.write(value);
    }

    Ok(())
}
