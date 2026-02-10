use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::device::DeviceControlOperationVm;
use crate::platform::{PlatformError, VmSlice, fs, resource};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.device.control.control.
pub(super) fn destack_device_control(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::DeviceHandle,
    operation: DeviceControlOperationVm,
    input: VmSlice<u8>,
    output: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, operation, input, output);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.control.control is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.device.io.close.
pub(super) fn destack_device_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::DeviceHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.io.close is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.device.io.open.
pub(super) fn destack_device_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: fs::OsPathVm,
    flags: u32,
    mode: u32,
) -> RuntimeResult<resource::DeviceHandle> {
    let _ = (path, flags, mode);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.io.open is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.device.io.read.
pub(super) fn destack_device_read(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::DeviceHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.io.read is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.device.io.write.
pub(super) fn destack_device_write(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::DeviceHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.io.write is not available in the VM yet",
    ))
    .boxed())
}
