#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::device::bindings_generated as bindings;
use crate::platform::{NativeArray, NativeSlice, PlatformError};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::device::DeviceControlOperation;
use crate::platform::{fs, resource};

/// Run a device control request.
///
/// Execute one host device-control operation with input and output buffers.
/// Request meaning and binary payload layout are device-specific by design.
///
/// # Platform
/// Unix and Windows.
/// Uses ioctl(2) on Unix and DeviceIoControl on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_device_control(
    _context: &RuntimeCallContext,
    out: *mut u64,
    handle: resource::DeviceHandle,
    operation: DeviceControlOperation,
    input: NativeSlice<u8>,
    output: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, operation, input, output);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.control.request",
    ))
    .boxed())
}

/// Close a device endpoint.
///
/// Close one previously opened device handle.
/// Close semantics for pending I O follow host kernel behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses close(2) on Unix and CloseHandle on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.read`, `device.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_device_close(
    _context: &RuntimeCallContext,
    handle: resource::DeviceHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.device.io.close")).boxed())
}

/// Open a device endpoint.
///
/// Open one host device node with the requested open flags.
/// Access checks and device availability are enforced by the host kernel.
///
/// # Platform
/// Unix and Windows.
/// Uses open(2) on Unix and CreateFileW on Windows device paths.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.read`, `device.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_device_open(
    _context: &RuntimeCallContext,
    out: *mut resource::DeviceHandle,
    path: fs::OsPath,
    flags: u32,
    mode: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, path, flags, mode);

    Err(RuntimeError::from(PlatformError::not_supported("destack.device.io.open")).boxed())
}

/// Read bytes from a device endpoint.
///
/// Read bytes into caller-provided memory and return the number of bytes transferred.
/// Partial reads are preserved exactly as reported by the host.
///
/// # Platform
/// Unix and Windows.
/// Uses read(2) on Unix and ReadFile on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_device_read(
    _context: &RuntimeCallContext,
    out: *mut u64,
    handle: resource::DeviceHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, buffer);

    Err(RuntimeError::from(PlatformError::not_supported("destack.device.io.read")).boxed())
}

/// Write bytes to a device endpoint.
///
/// Write bytes from caller-provided memory and return the number of bytes transferred.
/// Partial writes are preserved exactly as reported by the host.
///
/// # Platform
/// Unix and Windows.
/// Uses write(2) on Unix and WriteFile on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_device_write(
    _context: &RuntimeCallContext,
    out: *mut u64,
    handle: resource::DeviceHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, buffer);

    Err(RuntimeError::from(PlatformError::not_supported("destack.device.io.write")).boxed())
}
