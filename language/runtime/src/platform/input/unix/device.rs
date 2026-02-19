use super::core as input_core;
#[cfg(target_os = "linux")]
use super::linux as input_linux;
#[cfg(target_os = "macos")]
use super::macos as input_macos;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{
    InputAxisInfo, InputButtonInfo, InputDeviceCapabilities, InputDeviceCapabilityKind,
    InputDeviceInfo, InputDeviceKind, InputReadMode,
};
use crate::platform::resource::ResourceEntry;
use crate::platform::{NativeSlice, NativeStringRef, PlatformError, resource};
use crate::runtime::RuntimeCallContext;

/// Build one conservative capabilities payload from device metadata.
fn derive_capabilities_from_device_info(
    context: &RuntimeCallContext,
    device: InputDeviceInfo,
) -> InputDeviceCapabilities {
    let mut kinds = Vec::new();
    match device.kind {
        InputDeviceKind::Keyboard => {
            kinds.push(InputDeviceCapabilityKind::Keyboard);
        }
        InputDeviceKind::Mouse => {
            kinds.push(InputDeviceCapabilityKind::Pointer);
        }
        InputDeviceKind::Touch => {
            kinds.push(InputDeviceCapabilityKind::Touch);
            kinds.push(InputDeviceCapabilityKind::Pointer);
        }
        InputDeviceKind::Gamepad => {
            kinds.push(InputDeviceCapabilityKind::Gamepad);
        }
        InputDeviceKind::Pen => {
            kinds.push(InputDeviceCapabilityKind::Pen);
            kinds.push(InputDeviceCapabilityKind::Pointer);
        }
        InputDeviceKind::Raw => {}
    }
    if device.supports_rumble {
        kinds.push(InputDeviceCapabilityKind::Haptics);
    }
    if device.supports_text {
        kinds.push(InputDeviceCapabilityKind::TextInput);
    }

    let mut axes = Vec::with_capacity(device.axis_count as usize);
    for code in 0..u32::from(device.axis_count) {
        axes.push(InputAxisInfo {
            code,
            minimum: 0.0,
            maximum: 0.0,
            flat: 0.0,
            fuzz: 0.0,
            resolution: 0.0,
        });
    }

    let mut buttons = Vec::with_capacity(device.button_count as usize);
    for code in 0..u32::from(device.button_count) {
        buttons.push(InputButtonInfo {
            code,
            analog: false,
        });
    }

    InputDeviceCapabilities {
        kinds: context.store_array(kinds),
        axes: context.store_array(axes),
        buttons: context.store_array(buttons),
        supports_relative_pointer: matches!(
            device.kind,
            InputDeviceKind::Mouse | InputDeviceKind::Pen
        ),
        supports_pointer_grab: device.supports_exclusive_grab,
        supports_pointer_capture: false,
        supports_pointer_warp: false,
        supports_text_input: device.supports_text,
        supports_composition: device.supports_text,
        supports_rumble: device.supports_rumble,
        supports_trigger_rumble: false,
        supports_sensors: false,
        supports_battery_state: device.supports_battery,
        supports_light_control: device.supports_light,
        supports_raw_hid: device.supports_raw_hid,
        supports_player_index: matches!(device.kind, InputDeviceKind::Gamepad),
    }
}

/// Resolve one best-effort device kind for one normalized open spec.
fn resolve_device_kind_for_open(
    context: &RuntimeCallContext,
    backend: input_core::UnixInputBackend,
    device_id: &str,
) -> InputDeviceKind {
    if backend == input_core::UnixInputBackend::UnixTerminal {
        return InputDeviceKind::Keyboard;
    }

    let devices = match input_core::list_unix_devices(context) {
        Ok(devices) => devices,
        Err(_) => return InputDeviceKind::Raw,
    };
    for device in devices {
        let Ok(id) = (unsafe { device.id.as_str() }) else {
            continue;
        };
        if id == device_id {
            return device.kind;
        }
    }

    InputDeviceKind::Raw
}

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
/// Uses evdev device-node enumeration on Linux.
/// Uses global-session and terminal discovery on macOS.
/// Uses terminal input discovery on other Unix hosts.
/// Uses console and raw-state discovery on Windows.
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
/// Uses evdev device-node open on Linux.
/// Uses global-session or terminal-device open on macOS.
/// Uses terminal-device open on other Unix hosts.
/// Uses duplicated console-input handles or raw-state handles on Windows.
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
    let device_kind = resolve_device_kind_for_open(context, spec.backend, &spec.device_id);

    let binding = input_core::UnixInputBinding {
        descriptor,
        backend: spec.backend,
        read_mode: match spec.backend {
            input_core::UnixInputBackend::Platform => input_core::platform_default_read_mode(),
            input_core::UnixInputBackend::UnixTerminal => InputReadMode::Cooked,
        },
        device_id: spec.device_id,
        device_kind,
        next_sequence: 1,
        #[cfg(target_os = "linux")]
        linux_modifiers: 0,
        #[cfg(target_os = "linux")]
        linux_pointer_buttons: 0,
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

/// Query capabilities for one opened input device.
///
/// Return detailed axis, button, and feature capability metadata for one opened device.
/// Metadata values are backend-derived and may be partially unavailable.
///
/// # Platform
/// Unix and Windows.
/// Uses evdev and libinput-style capability tables on Linux.
/// Uses HID and raw-input capability queries on Windows.
/// Uses backend-specific capability synthesis on other Unix hosts.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_capabilities(
    context: &RuntimeCallContext,
    out: *mut InputDeviceCapabilities,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve one opened input binding
    let binding = input_core::resolve_unix_input_binding(
        context,
        handle,
        "destack.input.device.capabilities",
    )?;

    // find matching device metadata from current host enumeration
    let mut device_info = None;
    for device in input_core::list_unix_devices(context)? {
        let device_id = unsafe { device.id.as_str()? };
        if device_id == binding.device_id {
            device_info = Some(device);
            break;
        }
    }

    // synthesize fallback metadata when enumeration does not include this binding
    let device_info = match device_info {
        Some(device_info) => device_info,
        None => {
            let transport = match binding.backend {
                input_core::UnixInputBackend::UnixTerminal => "tty",
                input_core::UnixInputBackend::Platform => "platform",
            };

            InputDeviceInfo {
                id: context.store_string(&binding.device_id),
                instance_id: context.store_string(&binding.device_id),
                hardware_id: context.store_string(&binding.device_id),
                name: context.store_string(&binding.device_id),
                transport: context.store_string(transport),
                kind: binding.device_kind,
                vendor_id: 0,
                product_id: 0,
                key_count: 0,
                button_count: 0,
                axis_count: 0,
                connected: true,
                supports_exclusive_grab: binding.backend == input_core::UnixInputBackend::Platform,
                supports_raw: true,
                supports_text: binding.backend == input_core::UnixInputBackend::UnixTerminal,
                supports_rumble: false,
                supports_battery: false,
                supports_light: false,
                supports_raw_hid: false,
                is_virtual: false,
                is_system: true,
            }
        }
    };

    // query backend-derived linux capabilities for opened platform descriptors
    #[cfg(target_os = "linux")]
    let capabilities = if binding.backend == input_core::UnixInputBackend::Platform {
        if let Some(descriptor) = binding.descriptor {
            input_linux::query_linux_capabilities(
                context,
                descriptor,
                device_info.kind,
                device_info.supports_exclusive_grab,
                device_info.supports_text,
                device_info.supports_rumble,
                device_info.supports_battery,
                device_info.supports_light,
                device_info.supports_raw_hid,
            )
        } else {
            derive_capabilities_from_device_info(context, device_info)
        }
    } else {
        derive_capabilities_from_device_info(context, device_info)
    };

    // synthesize capabilities for non-linux unix backends
    #[cfg(not(target_os = "linux"))]
    let capabilities = derive_capabilities_from_device_info(context, device_info);

    // report explicit capability tables for the macOS global-session backend
    #[cfg(target_os = "macos")]
    let capabilities = if binding.backend == input_core::UnixInputBackend::Platform
        && binding.device_id == input_macos::MACOS_INPUT_SESSION_ID
    {
        input_macos::query_macos_session_capabilities(context)
    } else {
        capabilities
    };

    // write capabilities payload
    unsafe {
        *out = capabilities;
    }

    Ok(())
}
