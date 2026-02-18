use super::core as input_core;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::InputDeviceInfo;
use crate::platform::resource::ResourceEntry;
use crate::platform::{NativeSlice, NativeStringRef, PlatformError, resource};
use crate::runtime::RuntimeCallContext;

/// Close one input device.
///
/// Close one opened input device endpoint and release host resources.
/// Pending unread events are discarded according to host backend behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses close(2) on Unix and CloseHandle on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_close(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    input_core::resolve_unix_input_binding(context, handle, "destack.input.device.close")?;

    let removed = context.runtime().resources.remove_and_finalize(handle.0);
    if !removed {
        return Err(input_core::input_not_found(
            "destack.input.device.close",
            handle,
        ));
    }

    Ok(())
}

/// List available input devices.
///
/// Enumerate host input devices and return stable identifiers and typed device metadata.
/// Device ordering and hotplug visibility follow host input subsystem semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses evdev device-node enumeration on Linux, terminal input discovery on other Unix hosts, and console-input availability checks on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_list(
    context: &RuntimeCallContext,
    out: *mut NativeSlice<InputDeviceInfo>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let devices = input_core::list_unix_devices(context)?;
    unsafe {
        *out = context.store_slice(devices);
    }

    Ok(())
}

/// Open one input device.
///
/// Open one input device endpoint for event reads and optional control operations.
/// Exclusive-grab behavior and permission checks are host-defined.
///
/// # Platform
/// Unix and Windows.
/// Uses evdev device-node open on Linux, terminal-device open on other Unix hosts, and duplicated console-input handles on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_open(
    context: &RuntimeCallContext,
    out: *mut resource::InputDeviceHandle,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let id = unsafe { id.as_str()? };
    let spec = input_core::normalize_unix_input_spec(id)?;
    let descriptor = input_core::open_input_descriptor(&spec.path)?;

    let entry = ResourceEntry::new(resource::ResourceKind::Input)
        .with_label(input_core::INPUT_RESOURCE_LABEL)
        .with_fd(descriptor)
        .with_payload(input_core::UnixInputBinding {
            backend: spec.backend,
        })
        .with_finalizer(input_core::InputDeviceFinalizer { fd: descriptor });
    let resource_id = context.runtime().resources.insert(entry);

    unsafe {
        *out = resource::InputDeviceHandle(resource_id);
    }

    Ok(())
}
