use super::core as input_core;
#[cfg(target_os = "linux")]
use super::linux as input_linux;
#[cfg(target_os = "macos")]
use super::macos as input_macos;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::input::{
    InputAxisMetadata, InputButtonMetadata, InputCapabilityMetadataFidelity,
    InputCapabilityMetadataOrigin, InputDeviceCapabilities, InputDeviceCapabilityKind,
    InputDeviceDescriptor, InputDeviceKind, InputReadMode, InputTextGeometry, InputTextInputType,
    InputTextRectangle, InputTextTransform2D,
};
use crate::platform::resource::ResourceEntry;
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Build one capabilities payload from available device summary metadata.
fn derive_capabilities_from_device_summary(
    binding: &BindingCallContext,
    device: InputDeviceDescriptor,
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
        axes.push(InputAxisMetadata {
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
        buttons.push(InputButtonMetadata {
            code,
            analog: false,
        });
    }

    InputDeviceCapabilities {
        kinds: binding.store_array(kinds),
        axes: binding.store_array(axes),
        buttons: binding.store_array(buttons),
        metadata_origin: InputCapabilityMetadataOrigin::DeviceSummary,
        axis_metadata_fidelity: InputCapabilityMetadataFidelity::Minimal,
        button_metadata_fidelity: InputCapabilityMetadataFidelity::Minimal,
        supports_relative_pointer: matches!(
            device.kind,
            InputDeviceKind::Mouse | InputDeviceKind::Pen
        ),
        supports_pointer_grab: false,
        supports_pointer_capture: false,
        supports_pointer_warp: false,
        supports_text_input: device.supports_text,
        supports_composition: false,
        supports_edit_intents: false,
        supports_rumble: device.supports_rumble,
        supports_trigger_rumble: false,
        supports_sensors: false,
        supports_battery_state: device.supports_battery,
        supports_light_control: device.supports_light,
        supports_raw_hid: device.supports_raw_hid,
        supports_player_index: matches!(device.kind, InputDeviceKind::Gamepad),
        supports_pointer_coalescing: false,
        supports_pointer_prediction: false,
        supports_pen_hover_distance: false,
        supports_pen_orientation_angles: false,
    }
}

/// Resolve one best-effort device kind for one normalized open spec.
fn resolve_device_kind_for_open(
    binding: &BindingCallContext,
    backend: input_core::UnixInputBackend,
    device_id: &str,
) -> InputDeviceKind {
    if backend == input_core::UnixInputBackend::UnixTerminal {
        return InputDeviceKind::Keyboard;
    }

    let devices = match input_core::list_unix_devices(binding) {
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
pub(crate) unsafe fn destack_input_close(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    input_core::resolve_unix_input_binding(binding, handle, "destack.input.device.close")?;
    #[cfg(target_os = "macos")]
    input_core::release_macos_subscription(binding, handle);

    let removed = binding.worker().resources.remove_and_finalize(
        binding.world(),
        handle.0,
        Some(binding.engine()),
    );
    if !removed {
        return Err(input_core::input_not_found(
            "destack.input.device.close",
            handle,
        ));
    }

    Ok(())
}

/// List available input devices.
pub(crate) unsafe fn destack_input_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<InputDeviceDescriptor>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let devices = input_core::list_unix_devices(binding)?;
    unsafe {
        *out = binding.store_slice(devices);
    }

    Ok(())
}

/// Open one input device.
pub(crate) unsafe fn destack_input_open(
    binding: &BindingCallContext,
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
    let device_kind = resolve_device_kind_for_open(binding, spec.backend, &spec.device_id);

    let resolved_binding = input_core::UnixInputBinding {
        descriptor,
        backend: spec.backend,
        read_mode: match spec.backend {
            input_core::UnixInputBackend::Platform => input_core::platform_default_read_mode(),
            input_core::UnixInputBackend::UnixTerminal => InputReadMode::Cooked,
        },
        text_active: false,
        text_input_type: InputTextInputType::Text,
        text_area: Some(InputTextGeometry {
            local_to_target_transform: InputTextTransform2D {
                xx: 1.0,
                xy: 0.0,
                yx: 0.0,
                yy: 1.0,
                tx: 0.0,
                ty: 0.0,
            },
            editor_rectangle: InputTextRectangle {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
            caret_rectangle: None,
            composing_rectangle: None,
        }),
        gamepad_player_index_override: None,
        relative_mode_enabled: false,
        last_pointer_x: 0.0,
        last_pointer_y: 0.0,
        sensor_enabled_kinds: std::collections::HashSet::new(),
        sensor_effective_configs: std::collections::HashMap::new(),
        device_id: spec.device_id,
        device_kind,
        next_sequence: 1,
        #[cfg(target_os = "linux")]
        linux_modifiers: 0,
        #[cfg(target_os = "linux")]
        linux_pointer_buttons: 0,
        #[cfg(target_os = "linux")]
        linux_active_rumble_effect_id: None,
        terminal_original_mode,
        #[cfg(target_os = "macos")]
        macos_state: input_core::initial_macos_state(spec.backend),
    };

    let entry = ResourceEntry::new(resource::ResourceKind::InputDevice)
        .with_label(input_core::INPUT_RESOURCE_LABEL)
        .with_payload(resolved_binding);
    let entry = if let Some(descriptor) = descriptor {
        entry.with_finalizer(input_core::InputDeviceFinalizer {
            fd: descriptor,
            restore_terminal_mode: terminal_original_mode,
        })
    } else {
        entry
    };
    let resource_id =
        binding
            .worker()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));

    unsafe {
        *out = resource::InputDeviceHandle(resource_id);
    }

    Ok(())
}

/// Query capabilities for one opened input device.
pub(crate) unsafe fn destack_input_capabilities(
    binding: &BindingCallContext,
    out: *mut InputDeviceCapabilities,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve one opened input resolved_binding
    let resolved_binding = input_core::resolve_unix_input_binding(
        binding,
        handle,
        "destack.input.device.capabilities",
    )?;

    // find matching device metadata from current host enumeration
    let mut device_info = None;
    for device in input_core::list_unix_devices(binding)? {
        let device_id = unsafe { device.id.as_str()? };
        if device_id == resolved_binding.device_id {
            device_info = Some(device);
            break;
        }
    }

    // construct fallback metadata from the open resolved_binding when enumeration misses this handle
    let device_info = match device_info {
        Some(device_info) => device_info,
        None => {
            let transport = match resolved_binding.backend {
                input_core::UnixInputBackend::UnixTerminal => "tty",
                input_core::UnixInputBackend::Platform => "platform",
            };

            InputDeviceDescriptor {
                id: binding.store_string(&resolved_binding.device_id),
                instance_id: binding.store_string(&resolved_binding.device_id),
                hardware_id: binding.store_string(&resolved_binding.device_id),
                serial_number: None,
                location_id: None,
                guid: None,
                name: binding.store_string(&resolved_binding.device_id),
                transport: binding.store_string(transport),
                kind: resolved_binding.device_kind,
                vendor_id: 0,
                product_id: 0,
                key_count: 0,
                button_count: 0,
                axis_count: 0,
                connected: true,
                supports_exclusive_grab: resolved_binding.backend
                    == input_core::UnixInputBackend::Platform,
                supports_raw: true,
                supports_text: resolved_binding.backend
                    == input_core::UnixInputBackend::UnixTerminal,
                supports_rumble: false,
                supports_battery: false,
                supports_light: false,
                supports_raw_hid: {
                    #[cfg(target_os = "linux")]
                    {
                        input_linux::is_linux_hidraw_runtime_id(&resolved_binding.device_id)
                            || resolved_binding.device_id.starts_with("/dev/hidraw")
                    }

                    #[cfg(not(target_os = "linux"))]
                    {
                        resolved_binding.device_id.starts_with("/dev/hidraw")
                    }
                },
                is_virtual: false,
                is_system: true,
            }
        }
    };

    // query backend-derived linux capabilities for opened platform descriptors
    #[cfg(target_os = "linux")]
    let capabilities = if resolved_binding.backend == input_core::UnixInputBackend::Platform {
        if let Some(descriptor) = resolved_binding.descriptor {
            input_linux::query_linux_capabilities(
                binding,
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
            derive_capabilities_from_device_summary(binding, device_info)
        }
    } else {
        derive_capabilities_from_device_summary(binding, device_info)
    };

    // derive capabilities from available summary metadata for non-linux unix backends
    #[cfg(not(target_os = "linux"))]
    let capabilities = derive_capabilities_from_device_summary(binding, device_info);

    // report explicit capability tables for the macOS global-session backend
    #[cfg(target_os = "macos")]
    let capabilities = if resolved_binding.backend == input_core::UnixInputBackend::Platform
        && resolved_binding.device_id == input_macos::MACOS_INPUT_SESSION_ID
    {
        input_macos::query_macos_session_capabilities(binding)
    } else {
        capabilities
    };

    // write capabilities payload
    unsafe {
        *out = capabilities;
    }

    Ok(())
}
