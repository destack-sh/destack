use super::core as input_core;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{InputDeviceInfo, InputReadMode};
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
    #[cfg(target_os = "macos")]
    input_core::release_macos_subscription(context, handle);

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
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one discoverable input backend.
/// Uses evdev device-node enumeration on Linux, global-session and terminal discovery on macOS, terminal input discovery on other Unix hosts, and console plus raw-state discovery on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
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
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one openable input backend.
/// Uses evdev device-node open on Linux, global-session or terminal-device open on macOS, terminal-device open on other Unix hosts, and duplicated console-input handles or raw-state handles on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
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
    let descriptor = if spec.path.is_empty() {
        None
    } else {
        Some(input_core::open_input_descriptor(&spec.path)?)
    };
    let terminal_original_mode = if spec.backend == input_core::UnixInputBackend::UnixTerminal {
        match descriptor {
            Some(descriptor) => Some(input_core::read_terminal_mode(descriptor)?),
            None => None,
        }
    } else {
        None
    };

    let binding = input_core::UnixInputBinding {
        descriptor,
        backend: spec.backend,
        read_mode: match spec.backend {
            input_core::UnixInputBackend::Platform => input_core::platform_default_read_mode(),
            input_core::UnixInputBackend::UnixTerminal => InputReadMode::Cooked,
        },
        device_id: spec.device_id,
        next_sequence: 1,
        #[cfg(target_os = "linux")]
        linux_modifiers: 0,
        terminal_original_mode,
        #[cfg(target_os = "macos")]
        macos_state: input_core::initial_macos_state(spec.backend),
    };

    let entry = ResourceEntry::new(resource::ResourceKind::Input)
        .with_label(input_core::INPUT_RESOURCE_LABEL)
        .with_payload(binding);
    let entry = if let Some(descriptor) = descriptor {
        entry.with_finalizer(input_core::InputDeviceFinalizer {
            fd: descriptor,
            restore_terminal_mode: terminal_original_mode,
        })
    } else {
        entry
    };
    let resource_id = context.runtime().resources.insert(entry);

    unsafe {
        *out = resource::InputDeviceHandle(resource_id);
    }

    Ok(())
}
