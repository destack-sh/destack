#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::device::bindings_generated as bindings;
use crate::platform::{NativeSlice, PlatformError};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::device::DeviceControlOperation;
use crate::platform::{fs, resource};

/// Stub for destack.device.control.control.
pub unsafe fn destack_device_control(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: resource::DeviceHandle,
    operation: DeviceControlOperation,
    input: NativeSlice<u8>,
    output: NativeSlice<u8>,
) -> RuntimeResult<()> {
    context.check_policy(DEVICE_CONTROL_CONTROL)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, operation, input, output);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.control.control",
    ))
    .boxed())
}

/// Stub for destack.device.io.close.
pub unsafe fn destack_device_close(
    context: &RuntimeCallContext,
    handle: resource::DeviceHandle,
) -> RuntimeResult<()> {
    context.check_policy(DEVICE_IO_CLOSE)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.device.io.close")).boxed())
}

/// Stub for destack.device.io.open.
pub unsafe fn destack_device_open(
    context: &RuntimeCallContext,
    out: *mut resource::DeviceHandle,
    path: fs::OsPath,
    flags: u32,
    mode: u32,
) -> RuntimeResult<()> {
    context.check_policy(DEVICE_IO_OPEN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, path, flags, mode);

    Err(RuntimeError::from(PlatformError::not_supported("destack.device.io.open")).boxed())
}

/// Stub for destack.device.io.read.
pub unsafe fn destack_device_read(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: resource::DeviceHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    context.check_policy(DEVICE_IO_READ)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, buffer);

    Err(RuntimeError::from(PlatformError::not_supported("destack.device.io.read")).boxed())
}

/// Stub for destack.device.io.write.
pub unsafe fn destack_device_write(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: resource::DeviceHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    context.check_policy(DEVICE_IO_WRITE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, buffer);

    Err(RuntimeError::from(PlatformError::not_supported("destack.device.io.write")).boxed())
}
