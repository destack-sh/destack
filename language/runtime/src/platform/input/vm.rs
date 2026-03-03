use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{
    InputCompositionEvent, InputCompositionEventPayloadVm, InputCompositionEventVm,
    InputDeviceCapabilities, InputDeviceCapabilitiesVm, InputDeviceDescriptor,
    InputDeviceDescriptorVm, InputDeviceEventVm, InputEvent, InputEventMetadata,
    InputEventMetadataVm, InputEventVm, InputGamepadEventVm, InputGamepadState,
    InputGamepadStateVm, InputHapticEffectParametersVm, InputHapticEffectType, InputHapticsResult,
    InputKeyEventVm, InputKeyboardState, InputKeyboardStateVm, InputMonitorChangeEventVm,
    InputMonitorConnectEventVm, InputMonitorDisconnectEventVm, InputMonitorEvent,
    InputMonitorEventMetadata, InputMonitorEventMetadataVm, InputMonitorEventVm,
    InputPointerButtonEventVm, InputPointerGrabMode, InputPointerMotionEventVm,
    InputPointerStateVm, InputRawHidReport, InputRawHidReportVm, InputReadMode, InputScrollEventVm,
    InputSensorConfigVm, InputSensorDescriptorVm, InputSensorEffectiveConfigVm, InputSensorEventVm,
    InputSensorKind, InputSensorSampleVm, InputTextEventPayloadVm, InputTextEventVm,
    InputTextInputAreaVm, InputTextInputType, InputTouchEventVm, InputTouchState,
    InputTouchStateVm, InputWindowTargetVm, host as host_input,
};
use crate::platform::{NativeArray, VmAggregateCodec, VmArray, VmSlice, resource};
use crate::runtime::{BindingCallContext, NativeSlice, NativeStringRef};

/// Invoke one host call that writes through an out pointer.
fn call_out<T>(call: impl FnOnce(*mut T) -> RuntimeResult<()>) -> RuntimeResult<T> {
    // allocate one uninitialized output slot for the host call
    let mut out = std::mem::MaybeUninit::<T>::uninit();

    // execute call and assume initialization on success
    call(out.as_mut_ptr())?;
    Ok(unsafe { out.assume_init() })
}

/// Convert one VM string handle into one runtime native string reference.
fn string_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: vm::StringHandle,
) -> RuntimeResult<NativeStringRef> {
    // resolve VM string payload and copy into runtime storage
    let value = context
        .string_ref(value)
        .map_err(|error| RuntimeError::from(error).boxed())?;
    Ok(runtime.store_string(value.as_str()))
}

/// Convert one native device-info payload into its VM shape.
fn device_info_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: InputDeviceDescriptor,
) -> RuntimeResult<InputDeviceDescriptorVm> {
    // decode borrowed native strings for vm interning
    let id = unsafe { value.id.as_str()? };
    let instance_id = unsafe { value.instance_id.as_str()? };
    let hardware_id = unsafe { value.hardware_id.as_str()? };
    let name = unsafe { value.name.as_str()? };
    let transport = unsafe { value.transport.as_str()? };

    // build one vm device-info record
    Ok(InputDeviceDescriptorVm {
        id: vm::StringHandle::new(context.intern_string(id)),
        instance_id: vm::StringHandle::new(context.intern_string(instance_id)),
        hardware_id: vm::StringHandle::new(context.intern_string(hardware_id)),
        name: vm::StringHandle::new(context.intern_string(name)),
        transport: vm::StringHandle::new(context.intern_string(transport)),
        kind: value.kind,
        vendor_id: value.vendor_id,
        product_id: value.product_id,
        key_count: value.key_count,
        button_count: value.button_count,
        axis_count: value.axis_count,
        connected: value.connected,
        supports_exclusive_grab: value.supports_exclusive_grab,
        supports_raw: value.supports_raw,
        supports_text: value.supports_text,
        supports_rumble: value.supports_rumble,
        supports_battery: value.supports_battery,
        supports_light: value.supports_light,
        supports_raw_hid: value.supports_raw_hid,
        is_virtual: value.is_virtual,
        is_system: value.is_system,
    })
}

/// Convert one native input event payload into its VM shape.
fn event_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: InputEvent,
) -> RuntimeResult<InputEventVm> {
    match value {
        InputEvent::InputCompositionEvent(value) => {
            let kind = native_string_to_vm(context, value.kind)?;
            let metadata = event_metadata_to_vm(context, value.metadata)?;
            let text = native_string_to_vm(context, value.payload.text)?;
            Ok(InputEventVm::InputCompositionEvent(
                InputCompositionEventVm {
                    kind,
                    metadata,
                    payload: InputCompositionEventPayloadVm {
                        action: value.payload.action,
                        text,
                        selection_start: value.payload.selection_start,
                        selection_end: value.payload.selection_end,
                    },
                },
            ))
        }
        InputEvent::InputDeviceEvent(value) => {
            let kind = native_string_to_vm(context, value.kind)?;
            let metadata = event_metadata_to_vm(context, value.metadata)?;
            Ok(InputEventVm::InputDeviceEvent(InputDeviceEventVm {
                kind,
                metadata,
                payload: value.payload,
            }))
        }
        InputEvent::InputGamepadEvent(value) => {
            let kind = native_string_to_vm(context, value.kind)?;
            let metadata = event_metadata_to_vm(context, value.metadata)?;
            Ok(InputEventVm::InputGamepadEvent(InputGamepadEventVm {
                kind,
                metadata,
                payload: value.payload,
            }))
        }
        InputEvent::InputKeyEvent(value) => {
            let kind = native_string_to_vm(context, value.kind)?;
            let metadata = event_metadata_to_vm(context, value.metadata)?;
            Ok(InputEventVm::InputKeyEvent(InputKeyEventVm {
                kind,
                metadata,
                payload: value.payload,
            }))
        }
        InputEvent::InputPointerButtonEvent(value) => {
            let kind = native_string_to_vm(context, value.kind)?;
            let metadata = event_metadata_to_vm(context, value.metadata)?;
            Ok(InputEventVm::InputPointerButtonEvent(
                InputPointerButtonEventVm {
                    kind,
                    metadata,
                    payload: value.payload,
                },
            ))
        }
        InputEvent::InputPointerMotionEvent(value) => {
            let kind = native_string_to_vm(context, value.kind)?;
            let metadata = event_metadata_to_vm(context, value.metadata)?;
            Ok(InputEventVm::InputPointerMotionEvent(
                InputPointerMotionEventVm {
                    kind,
                    metadata,
                    payload: value.payload,
                },
            ))
        }
        InputEvent::InputScrollEvent(value) => {
            let kind = native_string_to_vm(context, value.kind)?;
            let metadata = event_metadata_to_vm(context, value.metadata)?;
            Ok(InputEventVm::InputScrollEvent(InputScrollEventVm {
                kind,
                metadata,
                payload: value.payload,
            }))
        }
        InputEvent::InputSensorEvent(value) => {
            let kind = native_string_to_vm(context, value.kind)?;
            let metadata = event_metadata_to_vm(context, value.metadata)?;
            Ok(InputEventVm::InputSensorEvent(InputSensorEventVm {
                kind,
                metadata,
                payload: value.payload,
            }))
        }
        InputEvent::InputTextEvent(value) => {
            let kind = native_string_to_vm(context, value.kind)?;
            let metadata = event_metadata_to_vm(context, value.metadata)?;
            let text = native_string_to_vm(context, value.payload.text)?;
            Ok(InputEventVm::InputTextEvent(InputTextEventVm {
                kind,
                metadata,
                payload: InputTextEventPayloadVm { text },
            }))
        }
        InputEvent::InputTouchEvent(value) => {
            let kind = native_string_to_vm(context, value.kind)?;
            let metadata = event_metadata_to_vm(context, value.metadata)?;
            Ok(InputEventVm::InputTouchEvent(InputTouchEventVm {
                kind,
                metadata,
                payload: value.payload,
            }))
        }
    }
}

/// Convert one native input monitor event payload into its VM shape.
fn monitor_event_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: InputMonitorEvent,
) -> RuntimeResult<InputMonitorEventVm> {
    match value {
        InputMonitorEvent::InputMonitorChangeEvent(value) => {
            let kind = native_string_to_vm(context, value.kind)?;
            let metadata = monitor_event_metadata_to_vm(context, value.metadata)?;
            Ok(InputMonitorEventVm::InputMonitorChangeEvent(
                InputMonitorChangeEventVm { kind, metadata },
            ))
        }
        InputMonitorEvent::InputMonitorConnectEvent(value) => {
            let kind = native_string_to_vm(context, value.kind)?;
            let metadata = monitor_event_metadata_to_vm(context, value.metadata)?;
            Ok(InputMonitorEventVm::InputMonitorConnectEvent(
                InputMonitorConnectEventVm { kind, metadata },
            ))
        }
        InputMonitorEvent::InputMonitorDisconnectEvent(value) => {
            let kind = native_string_to_vm(context, value.kind)?;
            let metadata = monitor_event_metadata_to_vm(context, value.metadata)?;
            Ok(InputMonitorEventVm::InputMonitorDisconnectEvent(
                InputMonitorDisconnectEventVm { kind, metadata },
            ))
        }
    }
}

/// Convert one native runtime string into one VM string handle.
fn native_string_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeStringRef,
) -> RuntimeResult<vm::StringHandle> {
    let value = unsafe { value.as_str()? };
    Ok(vm::StringHandle::new(context.intern_string(value)))
}

/// Convert one native event metadata payload into its VM shape.
fn event_metadata_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: InputEventMetadata,
) -> RuntimeResult<InputEventMetadataVm> {
    Ok(InputEventMetadataVm {
        timestamp_ns: value.timestamp_ns,
        sequence: value.sequence,
        device_id: native_string_to_vm(context, value.device_id)?,
    })
}

/// Convert one native monitor event metadata payload into its VM shape.
fn monitor_event_metadata_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: InputMonitorEventMetadata,
) -> RuntimeResult<InputMonitorEventMetadataVm> {
    Ok(InputMonitorEventMetadataVm {
        timestamp_ns: value.timestamp_ns,
        sequence: value.sequence,
        device_id: native_string_to_vm(context, value.device_id)?,
        device_kind: value.device_kind,
        connected: value.connected,
    })
}

/// Convert one native capabilities payload into its VM shape.
fn capabilities_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: InputDeviceCapabilities,
) -> RuntimeResult<InputDeviceCapabilitiesVm> {
    let kinds = unsafe { value.kinds.as_slice()? };
    let axes = unsafe { value.axes.as_slice()? };
    let buttons = unsafe { value.buttons.as_slice()? };

    Ok(InputDeviceCapabilitiesVm {
        kinds: VmArray::from_values(context, kinds)?,
        axes: VmArray::from_values(context, axes)?,
        buttons: VmArray::from_values(context, buttons)?,
        metadata_origin: value.metadata_origin,
        axis_metadata_fidelity: value.axis_metadata_fidelity,
        button_metadata_fidelity: value.button_metadata_fidelity,
        supports_relative_pointer: value.supports_relative_pointer,
        supports_pointer_grab: value.supports_pointer_grab,
        supports_pointer_capture: value.supports_pointer_capture,
        supports_pointer_warp: value.supports_pointer_warp,
        supports_text_input: value.supports_text_input,
        supports_composition: value.supports_composition,
        supports_rumble: value.supports_rumble,
        supports_trigger_rumble: value.supports_trigger_rumble,
        supports_sensors: value.supports_sensors,
        supports_battery_state: value.supports_battery_state,
        supports_light_control: value.supports_light_control,
        supports_raw_hid: value.supports_raw_hid,
        supports_player_index: value.supports_player_index,
    })
}

/// Convert one native device-info slice into one VM slice.
fn list_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    values: NativeSlice<InputDeviceDescriptor>,
) -> RuntimeResult<VmSlice<InputDeviceDescriptorVm>> {
    // decode native slice and convert each item
    let values = unsafe { values.as_slice()? };
    let mut vm_values = Vec::with_capacity(values.len());
    for value in values {
        vm_values.push(device_info_to_vm(context, *value)?);
    }
    VmSlice::from_values(context, &vm_values)
}

/// Convert one native event array into one VM array.
fn event_array_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    values: NativeArray<InputEvent>,
) -> RuntimeResult<VmArray<InputEventVm>> {
    // decode native array and convert each item
    let values = unsafe { values.as_slice()? };
    let mut vm_values = Vec::with_capacity(values.len());
    for value in values {
        vm_values.push(event_to_vm(context, *value)?);
    }
    VmArray::from_values(context, &vm_values)
}

/// Convert one native array into one VM array.
fn array_to_vm<T: VmAggregateCodec>(
    context: &mut vm::ExternalCallContext<'_>,
    values: NativeArray<T>,
) -> RuntimeResult<VmArray<T>> {
    // decode native backing storage
    let values = unsafe { values.as_slice()? };

    // allocate one VM array from decoded values
    VmArray::from_values(context, values)
}

/// Convert one VM byte slice into one runtime-owned native slice.
fn bytes_slice_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    values: VmSlice<u8>,
) -> RuntimeResult<NativeSlice<u8>> {
    // copy VM bytes into runtime-owned backing storage
    let values = values.read_bytes(context)?;
    Ok(runtime.store_slice(values))
}

/// Convert one native byte slice into one VM byte slice.
fn bytes_slice_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    values: NativeSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    // expose native bytes through one VM byte slice
    let values = unsafe { values.as_slice()? };
    Ok(VmSlice::from_bytes(context, values))
}

/// Convert one native gamepad state snapshot into its VM shape.
fn gamepad_state_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: InputGamepadState,
) -> RuntimeResult<InputGamepadStateVm> {
    // convert variable-length gamepad arrays
    let axes = array_to_vm(context, value.axes)?;
    let buttons = array_to_vm(context, value.buttons)?;
    let touches = array_to_vm(context, value.touches)?;

    // build one VM snapshot
    Ok(InputGamepadStateVm {
        timestamp_ns: value.timestamp_ns,
        connected: value.connected,
        mapping: value.mapping,
        connection_type: value.connection_type,
        player_index: value.player_index,
        battery: value.battery,
        supports_rumble: value.supports_rumble,
        supports_trigger_rumble: value.supports_trigger_rumble,
        axes,
        buttons,
        touches,
    })
}

/// Convert one native keyboard state snapshot into its VM shape.
fn keyboard_state_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: InputKeyboardState,
) -> RuntimeResult<InputKeyboardStateVm> {
    // decode borrowed native device id
    let device_id = unsafe { value.device_id.as_str()? };

    // convert variable-length key arrays
    let pressed_codes = array_to_vm(context, value.pressed_codes)?;
    let pressed_scan_codes = array_to_vm(context, value.pressed_scan_codes)?;

    // build one VM snapshot
    Ok(InputKeyboardStateVm {
        timestamp_ns: value.timestamp_ns,
        sequence: value.sequence,
        device_id: vm::StringHandle::new(context.intern_string(device_id)),
        modifiers: value.modifiers,
        pressed_codes,
        pressed_scan_codes,
    })
}

/// Convert one native raw-hid report into its VM shape.
fn raw_hid_report_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: InputRawHidReport,
) -> RuntimeResult<InputRawHidReportVm> {
    // convert report payload bytes
    let data = bytes_slice_to_vm(context, value.data)?;

    // build one VM raw-hid report
    Ok(InputRawHidReportVm {
        timestamp_ns: value.timestamp_ns,
        sequence: value.sequence,
        report_id: value.report_id,
        data,
    })
}

/// Convert one native composition event into its VM shape.
fn composition_event_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: InputCompositionEvent,
) -> RuntimeResult<InputCompositionEventVm> {
    // build one VM composition event
    Ok(InputCompositionEventVm {
        kind: native_string_to_vm(context, value.kind)?,
        metadata: event_metadata_to_vm(context, value.metadata)?,
        payload: InputCompositionEventPayloadVm {
            action: value.payload.action,
            text: native_string_to_vm(context, value.payload.text)?,
            selection_start: value.payload.selection_start,
            selection_end: value.payload.selection_end,
        },
    })
}

/// Convert one native touch state snapshot into its VM shape.
fn touch_state_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: InputTouchState,
) -> RuntimeResult<InputTouchStateVm> {
    // decode borrowed native device id
    let device_id = unsafe { value.device_id.as_str()? };

    // convert variable-length contact array
    let contacts = array_to_vm(context, value.contacts)?;

    // build one VM touch snapshot
    Ok(InputTouchStateVm {
        timestamp_ns: value.timestamp_ns,
        sequence: value.sequence,
        device_id: vm::StringHandle::new(context.intern_string(device_id)),
        contacts,
    })
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
pub(crate) fn destack_input_close(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_close(runtime, handle) }
}

/// List available input devices.
///
/// Enumerate host input devices and return stable identifiers and typed device metadata.
/// Device ordering and hotplug visibility follow host input subsystem semantics.
///
/// # Platform
/// Unix and Windows.
/// Returns operation-level `notSupported` on hosts that do not expose one discoverable input backend.
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
pub(crate) fn destack_input_list(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<InputDeviceDescriptorVm>> {
    let values = call_out(|out| unsafe { host_input::destack_input_list(runtime, out) })?;
    list_to_vm(context, values)
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
pub(crate) fn destack_input_open(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<resource::InputDeviceHandle> {
    let id = string_from_vm(runtime, context, id)?;
    call_out(|out| unsafe { host_input::destack_input_open(runtime, out, id) })
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
/// Uses backend capability tables when available.
/// Falls back to deriving capabilities from available device summary metadata.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_capabilities(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputDeviceCapabilitiesVm> {
    let value =
        call_out(|out| unsafe { host_input::destack_input_capabilities(runtime, out, handle) })?;
    capabilities_to_vm(context, value)
}

/// Close one global input event monitor.
///
/// Close one opened monitor stream and release host subscription resources.
/// Pending unread monitor events are discarded.
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
pub(crate) fn destack_input_monitor_close(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_monitor_close(runtime, handle) }
}

/// Open one global input event monitor.
///
/// Open one monitor stream that reports host input topology events, including connect and disconnect.
/// Monitor streams are independent from per-device data streams.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one global monitor stream.
/// Uses inotify-backed `/dev/input` monitor events on Linux.
/// Falls back to snapshot scans on Linux when watcher setup is unavailable.
/// Uses session and terminal-device scans on macOS and other Unix hosts.
/// Uses raw-input device-change subscriptions on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_monitor_open(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<resource::InputMonitorHandle> {
    call_out(|out| unsafe { host_input::destack_input_monitor_open(runtime, out) })
}

/// Read one global input monitor event.
///
/// Read one pending monitor event from the global input monitor stream.
/// This stream is the canonical source for device connect, disconnect, and metadata-change events.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one global monitor stream.
/// Uses blocking reads from inotify-backed Linux monitor queues.
/// Falls back to snapshot scans on Linux when watcher setup is unavailable.
/// Uses terminal or session monitor streams on Unix hosts.
/// Uses raw-input monitor queues on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_monitor_read(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<InputMonitorEventVm> {
    let value =
        call_out(|out| unsafe { host_input::destack_input_monitor_read(runtime, out, handle) })?;
    monitor_event_to_vm(context, value)
}

/// Poll one global input monitor event without blocking.
///
/// Poll one pending monitor event and return immediately when no event is queued.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one global monitor stream.
/// Uses nonblocking reads from inotify-backed Linux monitor queues.
/// Falls back to snapshot scans on Linux when watcher setup is unavailable.
/// Uses terminal or session monitor streams on Unix hosts.
/// Uses raw-input monitor queues on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_monitor_try_read(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<InputMonitorEventVm> {
    let value = call_out(|out| unsafe {
        host_input::destack_input_monitor_try_read(runtime, out, handle)
    })?;
    monitor_event_to_vm(context, value)
}

/// Read one input event.
///
/// Read one pending input event from one opened device stream.
/// Per-device streams report control and motion events for that device and exclude global device topology events.
/// Backend framing packets are filtered from this semantic stream.
/// Queue pressure can report one device cancel packet through the typed payload.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one readable input backend.
/// Uses evdev event reads on Linux.
/// Uses event-tap queue reads on macOS.
/// Uses terminal-byte event reads on other Unix hosts.
/// Uses `ReadConsoleInputW` queue reads or raw-state polling on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_read(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputEventVm> {
    let value = call_out(|out| unsafe { host_input::destack_input_read(runtime, out, handle) })?;
    event_to_vm(context, value)
}

/// Read one batch of input events.
///
/// Read up to `maxEvents` events from one opened device stream in one call.
/// Batch ordering matches backend delivery order and excludes global device topology events.
/// Backend framing packets are filtered from this semantic stream.
/// Queue pressure can report one device cancel packet through the typed payload.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one readable input backend.
/// Uses batched reads when supported and runtime looped reads otherwise.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_read_batch(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    maxevents: u32,
) -> RuntimeResult<VmArray<InputEventVm>> {
    let values = call_out(|out| unsafe {
        host_input::destack_input_read_batch(runtime, out, handle, maxevents)
    })?;
    event_array_to_vm(context, values)
}

/// Enable or disable exclusive device grab.
///
/// Toggle exclusive-grab mode for one input device when the host backend supports it.
/// This is one device-wide exclusivity control and is distinct from pointer confinement or locking modes.
/// Grabs can prevent event delivery to other clients.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where exclusive grab is not defined by host policy.
/// Uses `EVIOCGRAB` on Linux.
/// Returns `notSupported` for global-session and terminal-backed Unix input.
/// Uses `SetConsoleMode` capture toggles on Windows console input.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.grab`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_set_exclusive_grab(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    enable: bool,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_set_exclusive_grab(runtime, handle, enable) }
}

/// Select event decoding mode for one input stream.
///
/// Select translated or raw decoding mode for one opened input endpoint.
/// Hosts can return notSupported when raw mode is unavailable for the selected endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses per-stream runtime mode selection on Linux evdev and macOS session backends.
/// Uses termios raw and cooked mode updates on Unix TTY paths.
/// Uses `SetConsoleMode` updates on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_set_read_mode(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    mode: InputReadMode,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_set_read_mode(runtime, handle, mode) }
}

/// Poll one input event without blocking.
///
/// Poll one pending input event from one opened device stream and return immediately when no event is queued.
/// Empty queue state is reported through ioWouldBlock.
/// Backend framing packets are filtered from this semantic stream.
/// Queue pressure can report one device cancel packet through the typed payload.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one readable input backend.
/// Uses nonblocking evdev reads on Linux.
/// Uses nonblocking event-tap queue reads on macOS.
/// Uses nonblocking terminal-byte reads on other Unix hosts.
/// Uses nonblocking console queue reads or raw-state polling on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_try_read(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputEventVm> {
    let value =
        call_out(|out| unsafe { host_input::destack_input_try_read(runtime, out, handle) })?;
    event_to_vm(context, value)
}
/// Set one gamepad light color.
///
/// Apply one rgb light color for one opened gamepad-capable device when supported.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where gamepad light control is unavailable.
/// Uses backend-specific gamepad light-control APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_gamepad_set_light(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    red: u8,
    green: u8,
    blue: u8,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_gamepad_set_light(runtime, handle, red, green, blue) }
}

/// Set one gamepad player index.
///
/// Apply one player index hint for one opened gamepad-capable device.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where player-index assignment is unavailable.
/// Uses backend-specific gamepad player-index assignment APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_gamepad_set_player_index(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    playerindex: u8,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_gamepad_set_player_index(runtime, handle, playerindex) }
}

/// Read one gamepad state snapshot.
///
/// Return one full gamepad state snapshot for one opened gamepad-capable device.
/// Snapshot fields mirror backend-standardized gamepad semantics for axes, buttons, touches, and battery metadata.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where gamepad snapshots are unavailable.
/// Uses backend-specific gamepad state APIs with normalized axes, buttons, touch contacts, and battery metadata.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_gamepad_state(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputGamepadStateVm> {
    let value =
        call_out(|out| unsafe { host_input::destack_input_gamepad_state(runtime, out, handle) })?;
    gamepad_state_to_vm(context, value)
}

/// List supported haptic effects.
///
/// Return supported haptic effect kinds for one opened haptics-capable device.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where haptics is unavailable.
/// Uses backend-specific haptic capability queries for controller and endpoint actuators.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `input.haptics`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_haptics_effects(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<VmArray<InputHapticEffectType>> {
    let values =
        call_out(|out| unsafe { host_input::destack_input_haptics_effects(runtime, out, handle) })?;
    array_to_vm(context, values)
}

/// Play one haptic effect.
///
/// Schedule one haptic effect on one opened haptics-capable device.
/// Effect playback timing and motor resolution follow backend capabilities.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where one effect type is unavailable.
/// Uses backend-specific rumble and haptics APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.haptics`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_haptics_play(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    effect: InputHapticEffectType,
    params: InputHapticEffectParametersVm,
) -> RuntimeResult<InputHapticsResult> {
    call_out(|out| unsafe {
        host_input::destack_input_haptics_play(runtime, out, handle, effect, params)
    })
}

/// Stop active haptic effects.
///
/// Stop active haptic playback on one opened haptics-capable device.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific haptic stop operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.haptics`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_haptics_stop(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_haptics_stop(runtime, handle) }
}

/// Read one keyboard state snapshot.
///
/// Return one current keyboard key and modifier snapshot for one opened keyboard-capable device.
/// Snapshot values represent one point-in-time backend state and can change immediately after read.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where keyboard snapshots are unavailable.
/// Uses backend-specific key-state tables from evdev or terminal backends on Unix.
/// Uses console or raw-input key-state paths on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_keyboard_state(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputKeyboardStateVm> {
    let value =
        call_out(|out| unsafe { host_input::destack_input_keyboard_state(runtime, out, handle) })?;
    keyboard_state_to_vm(context, value)
}

/// Enable or disable pointer capture.
///
/// Toggle pointer capture for one opened pointer-capable device and one optional window target.
/// Captured pointers can continue delivering events outside focused bounds when supported for that target scope.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where capture or one window scope is unavailable.
/// Uses backend-specific pointer capture primitives for global or window-scoped paths.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.grab`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_pointer_capture(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    target: InputWindowTargetVm,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_pointer_capture(runtime, handle, target, enabled) }
}

/// Read one relative pointer state snapshot.
///
/// Return one relative motion and button state snapshot for one opened pointer-capable device.
/// Delta units follow backend-native relative motion semantics.
/// Pen-capable devices can populate pressure and tilt metadata.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific relative motion streams from evdev or libinput-style backends on Unix.
/// Uses raw-input relative motion on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_pointer_relative_state(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputPointerStateVm> {
    call_out(|out| unsafe {
        host_input::destack_input_pointer_relative_state(runtime, out, handle)
    })
}

/// Set pointer grab mode.
///
/// Apply one grab mode for one opened pointer-capable device and one optional window target.
/// Grab modes can confine or lock pointer movement depending on backend support and target scope.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where one grab mode or one window scope is unavailable.
/// Uses backend-specific pointer grab or lock primitives for global or window-scoped paths.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.grab`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_pointer_set_grab_mode(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    target: InputWindowTargetVm,
    mode: InputPointerGrabMode,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_pointer_set_grab_mode(runtime, handle, target, mode) }
}

/// Enable or disable relative pointer mode.
///
/// Toggle relative pointer mode for one opened pointer-capable device.
/// Relative mode semantics follow backend pointer-lock behavior.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where relative mode is unavailable.
/// Uses backend-specific relative mode toggles for active input endpoints.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_pointer_set_relative_mode(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_pointer_set_relative_mode(runtime, handle, enabled) }
}

/// Read one absolute pointer state snapshot.
///
/// Return one current pointer position and button state snapshot for one opened pointer-capable device.
/// Position values follow backend coordinate space for that device.
/// Pen-capable devices can populate pressure and tilt metadata.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific pointer state queries from evdev or libinput-style streams on Unix.
/// Uses raw-input or console pointer state snapshots on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_pointer_state(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputPointerStateVm> {
    call_out(|out| unsafe { host_input::destack_input_pointer_state(runtime, out, handle) })
}

/// Warp pointer position.
///
/// Set one pointer position for one opened pointer-capable device and one optional window target.
/// Warped coordinates are interpreted in backend-native window or surface space for the selected target scope.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where pointer warping or one window scope is unavailable.
/// Uses backend-specific pointer warp operations for global or window-scoped paths.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_pointer_warp(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    target: InputWindowTargetVm,
    x: f64,
    y: f64,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_pointer_warp(runtime, handle, target, x, y) }
}

/// Read one raw-hid feature report.
///
/// Read one feature report from one opened raw-hid-capable input endpoint.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where raw-hid feature reports are unavailable.
/// Uses hid feature-report query APIs on Unix and Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_raw_hid_get_feature(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    reportid: u8,
    maxbytes: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let value = call_out(|out| unsafe {
        host_input::destack_input_raw_hid_get_feature(runtime, out, handle, reportid, maxbytes)
    })?;
    bytes_slice_to_vm(context, value)
}

/// Read one raw-hid report.
///
/// Read one pending raw-hid report from one opened raw-hid-capable input endpoint.
/// Timeout and blocking behavior follow backend raw-hid queue semantics.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where raw-hid reports are unavailable.
/// Uses hidraw or equivalent raw report APIs on Unix and raw-input hid report APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_raw_hid_read(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    maxbytes: u32,
    timeoutns: u64,
) -> RuntimeResult<InputRawHidReportVm> {
    let value = call_out(|out| unsafe {
        host_input::destack_input_raw_hid_read(runtime, out, handle, maxbytes, timeoutns)
    })?;
    raw_hid_report_to_vm(context, value)
}

/// Write one raw-hid feature report.
///
/// Write one feature report to one opened raw-hid-capable input endpoint.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where raw-hid feature reports are unavailable.
/// Uses hid feature-report set APIs on Unix and Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `input.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_raw_hid_set_feature(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    reportid: u8,
    data: VmSlice<u8>,
) -> RuntimeResult<()> {
    let data = bytes_slice_from_vm(runtime, context, data)?;
    unsafe { host_input::destack_input_raw_hid_set_feature(runtime, handle, reportid, data) }
}

/// Poll one raw-hid report without blocking.
///
/// Poll one pending raw-hid report and return immediately when none is available.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where raw-hid reports are unavailable.
/// Uses nonblocking hidraw or equivalent raw report APIs on Unix.
/// Uses nonblocking raw-input hid report APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_raw_hid_try_read(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    maxbytes: u32,
) -> RuntimeResult<InputRawHidReportVm> {
    let value = call_out(|out| unsafe {
        host_input::destack_input_raw_hid_try_read(runtime, out, handle, maxbytes)
    })?;
    raw_hid_report_to_vm(context, value)
}

/// Write one raw-hid output report.
///
/// Submit one raw-hid output report to one opened raw-hid-capable input endpoint.
/// Short writes can occur based on backend transport behavior.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where raw-hid output reports are unavailable.
/// Uses hidraw or equivalent raw report write APIs on Unix and raw-input hid report write APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `input.write`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_raw_hid_write(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    reportid: u8,
    data: VmSlice<u8>,
) -> RuntimeResult<u32> {
    let data = bytes_slice_from_vm(runtime, context, data)?;
    call_out(|out| unsafe {
        host_input::destack_input_raw_hid_write(runtime, out, handle, reportid, data)
    })
}

/// Configure one sensor stream.
///
/// Apply one enable and sample-rate configuration for one sensor stream on one opened input device.
/// Backends can negotiate one effective sample rate and one effective batching latency.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where sensor stream configuration is unavailable.
/// Uses backend-specific sensor configuration APIs on Unix and Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_sensor_configure(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
    config: InputSensorConfigVm,
) -> RuntimeResult<InputSensorEffectiveConfigVm> {
    call_out(|out| unsafe {
        host_input::destack_input_sensor_configure(runtime, out, handle, kind, config)
    })
}

/// List supported sensors for one opened input device.
///
/// Return sensor capability metadata for one opened sensor-capable device.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where sensor streams are unavailable.
/// Uses backend-specific sensor capability tables from evdev and hidraw class stacks on Unix.
/// Uses HID sensor or controller APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_sensor_list(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<VmArray<InputSensorDescriptorVm>> {
    let values =
        call_out(|out| unsafe { host_input::destack_input_sensor_list(runtime, out, handle) })?;
    array_to_vm(context, values)
}

/// Read one sensor sample.
///
/// Read one pending sample from one configured sensor stream.
/// Timeout and blocking behavior follow backend stream semantics.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where sensor streams are unavailable.
/// Uses backend-specific blocking sensor queue reads on Unix and Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_sensor_read(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
) -> RuntimeResult<InputSensorSampleVm> {
    call_out(|out| unsafe { host_input::destack_input_sensor_read(runtime, out, handle, kind) })
}

/// Poll one sensor sample without blocking.
///
/// Poll one pending sample from one configured sensor stream and return immediately when none is available.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where sensor streams are unavailable.
/// Uses backend-specific nonblocking sensor queue reads on Unix and Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_sensor_try_read(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
) -> RuntimeResult<InputSensorSampleVm> {
    call_out(|out| unsafe { host_input::destack_input_sensor_try_read(runtime, out, handle, kind) })
}

/// Get text input area.
///
/// Return the currently configured text input area and cursor position hint.
/// Resolve state for one opened input device and one optional window target.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where one window scope is unavailable.
/// Uses backend-specific text-area hint state tracking for global or window-scoped paths.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `input.text`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_text_get_area(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    target: InputWindowTargetVm,
) -> RuntimeResult<InputTextInputAreaVm> {
    call_out(|out| unsafe { host_input::destack_input_text_get_area(runtime, out, handle, target) })
}

/// Query text input active state.
///
/// Return whether text input is currently active for one opened input device.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific text session status checks.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `input.text`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_text_is_active(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { host_input::destack_input_text_is_active(runtime, out, handle) })
}

/// Read one composition event.
///
/// Read one pending composition lifecycle event for one opened input device.
/// Composition events represent begin, update, commit, end, and cancel transitions.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where composition events are unavailable.
/// Uses backend-specific IME composition queues.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `input.text`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_text_read_composition(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputCompositionEventVm> {
    let value = call_out(|out| unsafe {
        host_input::destack_input_text_read_composition(runtime, out, handle)
    })?;
    composition_event_to_vm(context, value)
}

/// Set text input area.
///
/// Set one text input area and cursor position hint for one opened input device and one optional window target.
/// Area hints are used by host IME placement when supported for the selected target scope.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where text-area hints or one window scope are unavailable.
/// Uses backend-specific IME candidate window placement hints for global or window-scoped paths.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `input.text`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_text_set_area(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    target: InputWindowTargetVm,
    area: InputTextInputAreaVm,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_text_set_area(runtime, handle, target, area) }
}

/// Start text input.
///
/// Enable text input and composition dispatch for one opened input device and one optional window target.
/// Text conversion behavior follows host IME and keyboard policy for the selected target scope.
///
/// # Platform
/// Unix and Windows.
/// Returns operation-level `notSupported` where text input sessions or one window scope are unavailable.
/// Uses backend-specific text input activation primitives.
/// Uses host IME activation for global or window-scoped paths.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.text`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_text_start(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    target: InputWindowTargetVm,
    inputtype: InputTextInputType,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_text_start(runtime, handle, target, inputtype) }
}

/// Stop text input.
///
/// Disable text input and composition dispatch for one opened input device and one optional window target.
/// Pending composition updates are finalized or canceled according to backend policy for the selected target scope.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where one window scope is unavailable.
/// Uses backend-specific text input deactivation primitives for global or window-scoped paths.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.text`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_text_stop(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    target: InputWindowTargetVm,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_text_stop(runtime, handle, target) }
}

/// Poll one composition event without blocking.
///
/// Poll one pending composition lifecycle event and return immediately when no event is queued.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where composition events are unavailable.
/// Uses backend-specific nonblocking IME composition queue reads.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.text`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_text_try_read_composition(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputCompositionEventVm> {
    let value = call_out(|out| unsafe {
        host_input::destack_input_text_try_read_composition(runtime, out, handle)
    })?;
    composition_event_to_vm(context, value)
}

/// Read one touch state snapshot.
///
/// Return one current touch-contact snapshot for one opened touch-capable device.
/// Contact ordering follows backend delivery order.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where touch snapshots are unavailable.
/// Uses backend-specific contact tables from evdev or libinput-style paths on Unix.
/// Uses pointer-contact APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_touch_state(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputTouchStateVm> {
    let value =
        call_out(|out| unsafe { host_input::destack_input_touch_state(runtime, out, handle) })?;
    touch_state_to_vm(context, value)
}
