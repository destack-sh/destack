use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::c_void;
use std::sync::atomic::Ordering;

use super::{core as input_core, raw as raw_input, xinput as xinput_input};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{
    InputAxisMetadata, InputButtonMetadata, InputCapabilityMetadataFidelity,
    InputCapabilityMetadataOrigin, InputDeviceCapabilities, InputDeviceCapabilityKind,
    InputDeviceDescriptor, InputDeviceKind, InputReadMode, InputTextGeometry, InputTextInputType,
    InputTextRectangle, InputTextTransform2D,
};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Enumerate windows input devices and raw-input devices.
pub(super) fn list_devices(
    binding: &BindingCallContext,
) -> RuntimeResult<Vec<InputDeviceDescriptor>> {
    // append console input when available for this process
    let mut devices = Vec::new();

    if input_core::get_stdin_console_mode()?.is_some() {
        devices.push(InputDeviceDescriptor {
            id: binding.store_string(input_core::WINDOWS_INPUT_DEVICE_ID),
            instance_id: binding.store_string(input_core::WINDOWS_INPUT_DEVICE_ID),
            hardware_id: binding.store_string(input_core::WINDOWS_INPUT_DEVICE_ID),
            name: binding.store_string(input_core::WINDOWS_INPUT_DEVICE_NAME),
            transport: binding.store_string("console"),
            kind: InputDeviceKind::Keyboard,
            vendor_id: 0,
            product_id: 0,
            key_count: input_core::WINDOWS_CONSOLE_KEY_COUNT,
            button_count: input_core::WINDOWS_CONSOLE_BUTTON_COUNT,
            axis_count: input_core::WINDOWS_CONSOLE_AXIS_COUNT,
            connected: true,
            supports_exclusive_grab: true,
            supports_raw: true,
            supports_text: true,
            supports_rumble: false,
            supports_battery: false,
            supports_light: false,
            supports_raw_hid: false,
            is_virtual: false,
            is_system: true,
        });
    }

    // append enumerated per-device raw-input endpoints
    let raw_devices = raw_input::list_raw_input_devices("destack.input.device.list")?;
    for raw_device in raw_devices {
        devices.push(InputDeviceDescriptor {
            id: binding.store_string(&raw_device.id),
            instance_id: binding.store_string(&raw_device.instance_id),
            hardware_id: binding.store_string(&raw_device.hardware_id),
            name: binding.store_string(&raw_device.name),
            transport: binding.store_string("rawinput"),
            kind: raw_device.kind,
            vendor_id: raw_device.vendor_id,
            product_id: raw_device.product_id,
            key_count: raw_device.key_count,
            button_count: raw_device.button_count,
            axis_count: raw_device.axis_count,
            connected: true,
            supports_exclusive_grab: false,
            supports_raw: true,
            supports_text: raw_device.supports_text,
            supports_rumble: raw_device.supports_rumble,
            supports_battery: raw_device.supports_battery,
            supports_light: raw_device.supports_light,
            supports_raw_hid: raw_device.supports_raw_hid,
            is_virtual: false,
            is_system: false,
        });
    }

    // append connected xinput gamepads
    devices.extend(xinput_input::list_xinput_devices(binding));

    Ok(devices)
}

/// Open one windows input endpoint by identifier.
pub(super) fn open_device(
    binding: &BindingCallContext,
    id: &str,
) -> RuntimeResult<resource::InputDeviceHandle> {
    // normalize the input identifier into one backend selector
    let spec = input_core::normalize_input_id(id)?;

    match spec {
        input_core::WindowsInputOpenSpec::Console => {
            // reserve the singleton console stream lane before opening
            input_core::acquire_console_stream("destack.input.device.open")?;

            // ensure any open failure releases the reserved stream lane
            let open_result = (|| {
                let (pointer_x, pointer_y) = input_core::current_pointer_position();

                // resolve and duplicate console input for resource ownership
                let Some((stdin, mode)) = input_core::get_stdin_console_mode()? else {
                    return Err(RuntimeError::from(PlatformError::io_with(
                        Some(PlatformErrorCode::IoNotFound),
                        None,
                        None,
                        Some("destack.input.device.open".to_string()),
                        None,
                        "console input is not available for this process",
                    ))
                    .boxed());
                };
                let duplicated = input_core::duplicate_console_handle(stdin)?;

                // insert console binding with restore finalizer
                let entry = ResourceEntry::new(ResourceKind::InputDevice)
                    .with_label(input_core::INPUT_RESOURCE_LABEL)
                    .with_handle(duplicated as *mut c_void)
                    .with_payload(input_core::WindowsInputBinding {
                        backend: input_core::WindowsInputBackend::Console,
                        read_mode: input_core::read_mode_from_console_mode(mode),
                        next_sequence: 1,
                        console_button_state: 0,
                        pending_console_button_transitions: VecDeque::new(),
                        pending_console_records: VecDeque::new(),
                        original_mode: Some(mode),
                        raw_device: None,
                        xinput_user_index: None,
                        xinput_packet_number: 0,
                        xinput_player_index_override: None,
                        last_pointer_x: pointer_x,
                        last_pointer_y: pointer_y,
                        relative_mode_enabled: false,
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
                        sensor_enabled_kinds: HashSet::new(),
                        sensor_effective_configs: HashMap::new(),
                    })
                    .with_finalizer(input_core::WindowsInputFinalizer {
                        handle: duplicated,
                        restore_mode: Some(mode),
                        release_console_lane: true,
                    });
                let resource_id = binding.worker().resources.insert(
                    binding.world(),
                    entry,
                    Some(binding.engine()),
                );
                Ok(resource::InputDeviceHandle(resource_id))
            })();

            if open_result.is_err() {
                input_core::WINDOWS_CONSOLE_STREAMS.fetch_sub(1, Ordering::AcqRel);
            }

            open_result
        }
        input_core::WindowsInputOpenSpec::RawDevice(raw_device) => {
            let (pointer_x, pointer_y) = input_core::current_pointer_position();

            // register this stream so the worker filters queueing per opened device
            let raw_runtime_state = raw_input::register_input_stream(
                binding,
                &raw_device.id,
                "destack.input.device.open",
            )?;

            let entry = ResourceEntry::new(ResourceKind::InputDevice)
                .with_label(input_core::INPUT_RESOURCE_LABEL)
                .with_payload(input_core::WindowsInputBinding {
                    backend: input_core::WindowsInputBackend::RawDevice,
                    read_mode: InputReadMode::Raw,
                    next_sequence: 1,
                    console_button_state: 0,
                    pending_console_button_transitions: VecDeque::new(),
                    pending_console_records: VecDeque::new(),
                    original_mode: None,
                    raw_device: Some(raw_device.clone()),
                    xinput_user_index: None,
                    xinput_packet_number: 0,
                    xinput_player_index_override: None,
                    last_pointer_x: pointer_x,
                    last_pointer_y: pointer_y,
                    relative_mode_enabled: false,
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
                    sensor_enabled_kinds: HashSet::new(),
                    sensor_effective_configs: HashMap::new(),
                })
                .with_finalizer(input_core::RawInputDeviceFinalizer {
                    device_id: raw_device.id.clone(),
                    runtime_state: raw_runtime_state,
                });
            let resource_id =
                binding
                    .worker()
                    .resources
                    .insert(binding.world(), entry, Some(binding.engine()));
            Ok(resource::InputDeviceHandle(resource_id))
        }
        input_core::WindowsInputOpenSpec::XInput(user_index) => {
            let packet =
                xinput_input::xinput_packet_number(user_index, "destack.input.device.open")?;
            let (pointer_x, pointer_y) = input_core::current_pointer_position();

            // insert xinput binding
            let entry = ResourceEntry::new(ResourceKind::InputDevice)
                .with_label(input_core::INPUT_RESOURCE_LABEL)
                .with_payload(input_core::WindowsInputBinding {
                    backend: input_core::WindowsInputBackend::XInput,
                    read_mode: InputReadMode::Raw,
                    next_sequence: 1,
                    console_button_state: 0,
                    pending_console_button_transitions: VecDeque::new(),
                    pending_console_records: VecDeque::new(),
                    original_mode: None,
                    raw_device: None,
                    xinput_user_index: Some(user_index),
                    xinput_packet_number: packet,
                    xinput_player_index_override: None,
                    last_pointer_x: pointer_x,
                    last_pointer_y: pointer_y,
                    relative_mode_enabled: false,
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
                    sensor_enabled_kinds: HashSet::new(),
                    sensor_effective_configs: HashMap::new(),
                });
            let resource_id =
                binding
                    .worker()
                    .resources
                    .insert(binding.world(), entry, Some(binding.engine()));
            Ok(resource::InputDeviceHandle(resource_id))
        }
    }
}

/// Close one windows input handle and run any finalizer.
pub(super) fn close_device(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    // validate handle before attempting removal
    input_core::resolve_input(binding, handle, operation)?;

    // remove from resource table and run finalizer
    let removed = binding.worker().resources.remove_and_finalize(
        binding.world(),
        handle.0,
        Some(binding.engine()),
    );
    if !removed {
        return Err(input_core::input_not_found(operation, handle));
    }

    Ok(())
}

/// Return capability metadata for one opened windows input handle.
pub(super) fn device_capabilities(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<InputDeviceCapabilities> {
    let resolved = input_core::resolve_input(binding, handle, operation)?;

    let capabilities = match resolved.backend {
        input_core::WindowsInputBackend::Console => {
            let kinds = vec![
                InputDeviceCapabilityKind::Keyboard,
                InputDeviceCapabilityKind::Pointer,
                InputDeviceCapabilityKind::TextInput,
            ];
            let axes = vec![
                InputAxisMetadata {
                    code: 0,
                    minimum: 0.0,
                    maximum: 0.0,
                    flat: 0.0,
                    fuzz: 0.0,
                    resolution: 0.0,
                },
                InputAxisMetadata {
                    code: 1,
                    minimum: 0.0,
                    maximum: 0.0,
                    flat: 0.0,
                    fuzz: 0.0,
                    resolution: 0.0,
                },
                InputAxisMetadata {
                    code: windows_sys::Win32::System::Console::MOUSE_WHEELED,
                    minimum: 0.0,
                    maximum: 0.0,
                    flat: 0.0,
                    fuzz: 0.0,
                    resolution: 0.0,
                },
                InputAxisMetadata {
                    code: windows_sys::Win32::System::Console::MOUSE_HWHEELED,
                    minimum: 0.0,
                    maximum: 0.0,
                    flat: 0.0,
                    fuzz: 0.0,
                    resolution: 0.0,
                },
            ];
            let mut buttons = Vec::new();
            for code in 0..u32::from(input_core::WINDOWS_CONSOLE_BUTTON_COUNT) {
                buttons.push(InputButtonMetadata {
                    code,
                    analog: false,
                });
            }

            InputDeviceCapabilities {
                kinds: binding.store_array(kinds),
                axes: binding.store_array(axes),
                buttons: binding.store_array(buttons),
                metadata_origin: InputCapabilityMetadataOrigin::Mixed,
                axis_metadata_fidelity: InputCapabilityMetadataFidelity::Partial,
                button_metadata_fidelity: InputCapabilityMetadataFidelity::Partial,
                supports_relative_pointer: true,
                supports_pointer_grab: false,
                supports_pointer_capture: false,
                supports_pointer_warp: true,
                supports_text_input: true,
                supports_composition: false,
                supports_rumble: false,
                supports_trigger_rumble: false,
                supports_sensors: false,
                supports_battery_state: false,
                supports_light_control: false,
                supports_raw_hid: false,
                supports_player_index: false,
            }
        }
        input_core::WindowsInputBackend::Window => {
            return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
        }
        input_core::WindowsInputBackend::RawDevice => {
            let Some(raw_device) = resolved.raw_device.as_ref() else {
                return Err(input_core::input_not_found(operation, handle));
            };
            raw_input::capabilities_for_raw_input_device(binding, raw_device)
        }
        input_core::WindowsInputBackend::XInput => {
            let Some(user_index) = resolved.xinput_user_index else {
                return Err(input_core::input_not_found(operation, handle));
            };
            xinput_input::capabilities_for_xinput_device(binding, user_index, operation)?
        }
    };

    Ok(capabilities)
}

/// Close one input device.
pub(crate) unsafe fn destack_input_close(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    close_device(binding, handle, "destack.input.device.close")
}

/// List available input devices.
pub(crate) unsafe fn destack_input_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<InputDeviceDescriptor>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let devices = list_devices(binding)?;
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
    let handle = open_device(binding, id)?;
    unsafe {
        *out = handle;
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

    // resolve one opened input handle into backend-derived capability metadata
    let capabilities = device_capabilities(binding, handle, "destack.input.device.capabilities")?;

    // write one capabilities payload
    unsafe {
        *out = capabilities;
    }

    Ok(())
}
