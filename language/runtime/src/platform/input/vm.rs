use super::{host as host_input, native as input_native};
use destack_vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::{
    VmAbiCodec, bytes_to_vm, call_out, map_native_array_to_vm, map_native_slice_to_vm,
    store_bytes_from_vm as bytes_from_vm, store_string_from_vm as string_from_vm,
    values_array_to_vm,
};
use crate::platform::fs::OsPathVm;
use crate::platform::input::{
    ClipboardItem, ClipboardItemDescriptorVm, ClipboardItemVm, InputDeviceCapabilities,
    InputDeviceCapabilitiesVm, InputDeviceDescriptor, InputDeviceDescriptorVm, InputEvent,
    InputEventVm, InputGamepadState, InputGamepadStateVm, InputHapticEffectParametersVm,
    InputHapticEffectType, InputHapticsResult, InputKeyboardLayoutInfoVm, InputKeyboardState,
    InputKeyboardStateVm, InputMonitorEvent, InputMonitorEventVm, InputPointerGrabMode,
    InputPointerStateVm, InputRawHidReport, InputRawHidReportVm, InputReadMode,
    InputSensorConfigVm, InputSensorDescriptorVm, InputSensorEffectiveConfigVm, InputSensorKind,
    InputSensorSampleVm, InputTextGeometry, InputTextGeometryVm, InputTextSessionConfig,
    InputTextSessionConfigVm, InputTextSessionEventVm, InputTextSessionState,
    InputTextSessionStateVm, InputTouchState, InputTouchStateVm, InputWindowTargetVm,
};
use crate::platform::{NativeAbiCodec, NativeSlice, NativeStringRef, VmArray, VmSlice, resource};
use crate::runtime::BindingCallContext;

/// Convert one native device-info payload into its VM shape.
fn device_info_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    value: InputDeviceDescriptor,
) -> RuntimeResult<InputDeviceDescriptorVm> {
    vm_value_from_native(context, value)
}

/// Convert one native input event payload into its VM shape.
fn event_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    value: InputEvent,
) -> RuntimeResult<InputEventVm> {
    match value {
        InputEvent::InputCompositionEvent(value) => Ok(InputEventVm::InputCompositionEvent(
            vm_value_from_native(context, value)?,
        )),
        InputEvent::InputDeviceEvent(value) => Ok(InputEventVm::InputDeviceEvent(
            vm_value_from_native(context, value)?,
        )),
        InputEvent::InputGamepadEvent(value) => Ok(InputEventVm::InputGamepadEvent(
            vm_value_from_native(context, value)?,
        )),
        InputEvent::InputKeyEvent(value) => Ok(InputEventVm::InputKeyEvent(vm_value_from_native(
            context, value,
        )?)),
        InputEvent::InputPointerButtonEvent(value) => Ok(InputEventVm::InputPointerButtonEvent(
            vm_value_from_native(context, value)?,
        )),
        InputEvent::InputPointerMotionEvent(value) => Ok(InputEventVm::InputPointerMotionEvent(
            vm_value_from_native(context, value)?,
        )),
        InputEvent::InputScrollEvent(value) => Ok(InputEventVm::InputScrollEvent(
            vm_value_from_native(context, value)?,
        )),
        InputEvent::InputSensorEvent(value) => Ok(InputEventVm::InputSensorEvent(
            vm_value_from_native(context, value)?,
        )),
        InputEvent::InputTextEvent(value) => Ok(InputEventVm::InputTextEvent(
            vm_value_from_native(context, value)?,
        )),
        InputEvent::InputTouchEvent(value) => Ok(InputEventVm::InputTouchEvent(
            vm_value_from_native(context, value)?,
        )),
    }
}

/// Convert one native input monitor event payload into its VM shape.
fn monitor_event_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    value: InputMonitorEvent,
) -> RuntimeResult<InputMonitorEventVm> {
    match value {
        InputMonitorEvent::InputMonitorChangeEvent(value) => Ok(
            InputMonitorEventVm::InputMonitorChangeEvent(vm_value_from_native(context, value)?),
        ),
        InputMonitorEvent::InputMonitorConnectEvent(value) => Ok(
            InputMonitorEventVm::InputMonitorConnectEvent(vm_value_from_native(context, value)?),
        ),
        InputMonitorEvent::InputMonitorDisconnectEvent(value) => Ok(
            InputMonitorEventVm::InputMonitorDisconnectEvent(vm_value_from_native(context, value)?),
        ),
    }
}

/// Convert one native runtime string into one VM string handle.
fn native_string_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    value: NativeStringRef,
) -> RuntimeResult<destack_vm::StringHandle> {
    let value = unsafe { value.as_str()? };
    context
        .string_handle(value)
        .map_err(Box::<RuntimeError>::from)
}

/// Decode one VM binding payload into one native binding payload.
fn native_value_from_vm<Native, Vm>(
    binding: &BindingCallContext,
    context: &destack_vm::BindingContext<'_>,
    value: Vm,
) -> RuntimeResult<Native>
where
    Native: NativeAbiCodec<Value = Vm::Value>,
    Vm: VmAbiCodec,
{
    let context = context.read();
    let value = value.into_value(&context)?;

    Ok(Native::from_value(binding, value))
}

/// Encode one native binding payload into one VM binding payload.
fn vm_value_from_native<Native, Vm>(
    context: &mut destack_vm::BindingContext<'_>,
    value: Native,
) -> RuntimeResult<Vm>
where
    Native: NativeAbiCodec,
    Vm: VmAbiCodec<Value = Native::Value>,
{
    let value = unsafe { value.into_value()? };
    let mut context = context.write();

    Vm::from_value(&mut context, value)
}

/// Convert one native capabilities payload into its VM shape.
fn capabilities_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    value: InputDeviceCapabilities,
) -> RuntimeResult<InputDeviceCapabilitiesVm> {
    vm_value_from_native(context, value)
}

/// Convert one native gamepad state snapshot into its VM shape.
fn gamepad_state_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    value: InputGamepadState,
) -> RuntimeResult<InputGamepadStateVm> {
    vm_value_from_native(context, value)
}

/// Convert one native keyboard state snapshot into its VM shape.
fn keyboard_state_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    value: InputKeyboardState,
) -> RuntimeResult<InputKeyboardStateVm> {
    vm_value_from_native(context, value)
}

/// Convert one native raw-hid report into its VM shape.
fn raw_hid_report_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    value: InputRawHidReport,
) -> RuntimeResult<InputRawHidReportVm> {
    // convert report payload bytes
    let data = bytes_to_vm(context, value.data)?;

    // build one VM raw-hid report
    Ok(InputRawHidReportVm {
        timestamp_ns: value.timestamp_ns,
        sequence: value.sequence,
        report_id: value.report_id,
        data,
    })
}

/// Convert one native touch state snapshot into its VM shape.
fn touch_state_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    value: InputTouchState,
) -> RuntimeResult<InputTouchStateVm> {
    vm_value_from_native(context, value)
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
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_close(binding, handle) }
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
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<InputDeviceDescriptorVm>> {
    let values = call_out(|out| unsafe { host_input::destack_input_list(binding, out) })?;

    map_native_slice_to_vm(context, values, |context, value| {
        device_info_to_vm(context, *value)
    })
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
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    id: destack_vm::StringHandle,
) -> RuntimeResult<resource::InputDeviceHandle> {
    let id = string_from_vm(binding, context, id)?;
    call_out(|out| unsafe { host_input::destack_input_open(binding, out, id) })
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
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputDeviceCapabilitiesVm> {
    let value =
        call_out(|out| unsafe { host_input::destack_input_capabilities(binding, out, handle) })?;
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
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_monitor_close(binding, handle) }
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
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<resource::InputMonitorHandle> {
    call_out(|out| unsafe { host_input::destack_input_monitor_open(binding, out) })
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
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<InputMonitorEventVm> {
    let value =
        call_out(|out| unsafe { host_input::destack_input_monitor_read(binding, out, handle) })?;
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
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<InputMonitorEventVm> {
    let value = call_out(|out| unsafe {
        host_input::destack_input_monitor_try_read(binding, out, handle)
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
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputEventVm> {
    let value = call_out(|out| unsafe { host_input::destack_input_read(binding, out, handle) })?;
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
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    maxevents: u32,
) -> RuntimeResult<VmArray<InputEventVm>> {
    let values = call_out(|out| unsafe {
        host_input::destack_input_read_batch(binding, out, handle, maxevents)
    })?;

    map_native_array_to_vm(context, values, |context, value| {
        event_to_vm(context, *value)
    })
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
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    enable: bool,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_set_exclusive_grab(binding, handle, enable) }
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
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    mode: InputReadMode,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_set_read_mode(binding, handle, mode) }
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
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputEventVm> {
    let value =
        call_out(|out| unsafe { host_input::destack_input_try_read(binding, out, handle) })?;
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
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    red: u8,
    green: u8,
    blue: u8,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_gamepad_set_light(binding, handle, red, green, blue) }
}

/// Set one gamepad motion sensor sample rate.
pub(crate) fn destack_input_gamepad_set_motion_sensor_sample_rate(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    sampleratehz: f64,
) -> RuntimeResult<()> {
    unsafe {
        host_input::destack_input_gamepad_set_motion_sensor_sample_rate(
            binding,
            handle,
            sampleratehz,
        )
    }
}

/// Enable or disable one gamepad motion sensor stream.
pub(crate) fn destack_input_gamepad_set_motion_sensors_enabled(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe {
        host_input::destack_input_gamepad_set_motion_sensors_enabled(binding, handle, enabled)
    }
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
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    playerindex: u8,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_gamepad_set_player_index(binding, handle, playerindex) }
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
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputGamepadStateVm> {
    let value =
        call_out(|out| unsafe { host_input::destack_input_gamepad_state(binding, out, handle) })?;
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
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<VmArray<InputHapticEffectType>> {
    let values =
        call_out(|out| unsafe { host_input::destack_input_haptics_effects(binding, out, handle) })?;
    values_array_to_vm(context, values)
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
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    effect: InputHapticEffectType,
    params: InputHapticEffectParametersVm,
) -> RuntimeResult<InputHapticsResult> {
    call_out(|out| unsafe {
        host_input::destack_input_haptics_play(binding, out, handle, effect, params)
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
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_haptics_stop(binding, handle) }
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
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputKeyboardStateVm> {
    let value =
        call_out(|out| unsafe { host_input::destack_input_keyboard_state(binding, out, handle) })?;
    keyboard_state_to_vm(context, value)
}

/// Read one keyboard layout snapshot.
pub(crate) fn destack_input_keyboard_layout(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputKeyboardLayoutInfoVm> {
    let value =
        call_out(|out| unsafe { host_input::destack_input_keyboard_layout(binding, out, handle) })?;

    vm_value_from_native(context, value)
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
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    target: InputWindowTargetVm,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_pointer_capture(binding, handle, target, enabled) }
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
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputPointerStateVm> {
    let value = call_out(|out| unsafe {
        host_input::destack_input_pointer_relative_state(binding, out, handle)
    })?;

    vm_value_from_native(context, value)
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
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    target: InputWindowTargetVm,
    mode: InputPointerGrabMode,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_pointer_set_grab_mode(binding, handle, target, mode) }
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
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_pointer_set_relative_mode(binding, handle, enabled) }
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
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputPointerStateVm> {
    let value =
        call_out(|out| unsafe { host_input::destack_input_pointer_state(binding, out, handle) })?;

    vm_value_from_native(context, value)
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
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    target: InputWindowTargetVm,
    x: f64,
    y: f64,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_pointer_warp(binding, handle, target, x, y) }
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
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    reportid: u8,
    maxbytes: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let value = call_out(|out| unsafe {
        host_input::destack_input_raw_hid_get_feature(binding, out, handle, reportid, maxbytes)
    })?;
    bytes_to_vm(context, value)
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
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    maxbytes: u32,
    timeoutns: u64,
) -> RuntimeResult<InputRawHidReportVm> {
    let value = call_out(|out| unsafe {
        host_input::destack_input_raw_hid_read(binding, out, handle, maxbytes, timeoutns)
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
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    reportid: u8,
    data: VmSlice<u8>,
) -> RuntimeResult<()> {
    let data = bytes_from_vm(binding, context, data)?;
    unsafe { host_input::destack_input_raw_hid_set_feature(binding, handle, reportid, data) }
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
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    maxbytes: u32,
) -> RuntimeResult<InputRawHidReportVm> {
    let value = call_out(|out| unsafe {
        host_input::destack_input_raw_hid_try_read(binding, out, handle, maxbytes)
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
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    reportid: u8,
    data: VmSlice<u8>,
) -> RuntimeResult<u32> {
    let data = bytes_from_vm(binding, context, data)?;
    call_out(|out| unsafe {
        host_input::destack_input_raw_hid_write(binding, out, handle, reportid, data)
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
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
    config: InputSensorConfigVm,
) -> RuntimeResult<InputSensorEffectiveConfigVm> {
    call_out(|out| unsafe {
        host_input::destack_input_sensor_configure(binding, out, handle, kind, config)
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
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<VmArray<InputSensorDescriptorVm>> {
    let values =
        call_out(|out| unsafe { host_input::destack_input_sensor_list(binding, out, handle) })?;
    values_array_to_vm(context, values)
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
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
) -> RuntimeResult<InputSensorSampleVm> {
    call_out(|out| unsafe { host_input::destack_input_sensor_read(binding, out, handle, kind) })
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
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
) -> RuntimeResult<InputSensorSampleVm> {
    call_out(|out| unsafe { host_input::destack_input_sensor_try_read(binding, out, handle, kind) })
}

/// Close one text input session.
pub(crate) fn destack_input_text_close(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_text_close(binding, session) }
}

/// Get text input area.
pub(crate) fn destack_input_text_get_geometry(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<InputTextGeometryVm> {
    let value: InputTextGeometry = call_out(|out| unsafe {
        host_input::destack_input_text_get_geometry(binding, out, session)
    })?;

    vm_value_from_native(context, value)
}

/// Open one text input session.
pub(crate) fn destack_input_text_open(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    config: InputTextSessionConfigVm,
    state: InputTextSessionStateVm,
) -> RuntimeResult<resource::InputTextSessionHandle> {
    let config: InputTextSessionConfig = native_value_from_vm(binding, context, config)?;
    let state: InputTextSessionState = native_value_from_vm(binding, context, state)?;

    call_out(|out| unsafe { host_input::destack_input_text_open(binding, out, config, state) })
}

/// Read one text session event.
///
/// Read one pending text session event for one active text session.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where text session events are unavailable.
/// Uses backend-specific text or IME event delivery.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `input.text`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_text_read_event(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<InputTextSessionEventVm> {
    let value = call_out(|out| unsafe {
        host_input::destack_input_text_read_event(binding, out, session)
    })?;

    vm_value_from_native(context, value)
}

/// Set text input area.
pub(crate) fn destack_input_text_set_geometry(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    session: resource::InputTextSessionHandle,
    area: InputTextGeometryVm,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_text_set_geometry(binding, session, area) }
}

/// Set text input state.
pub(crate) fn destack_input_text_set_state(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    session: resource::InputTextSessionHandle,
    state: InputTextSessionStateVm,
) -> RuntimeResult<()> {
    let state: InputTextSessionState = native_value_from_vm(binding, context, state)?;

    unsafe { host_input::destack_input_text_set_state(binding, session, state) }
}

/// Poll one text session event without blocking.
///
/// Poll one pending text session event and return immediately when no event is queued.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where text session events are unavailable.
/// Uses backend-specific nonblocking text or IME event delivery.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.text`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_text_try_read_event(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<InputTextSessionEventVm> {
    let value = call_out(|out| unsafe {
        host_input::destack_input_text_try_read_event(binding, out, session)
    })?;

    vm_value_from_native(context, value)
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
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputTouchStateVm> {
    let value =
        call_out(|out| unsafe { host_input::destack_input_touch_state(binding, out, handle) })?;
    touch_state_to_vm(context, value)
}

/// Clear clipboard payload.
pub(crate) fn destack_input_clipboard_clear(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<()> {
    input_native::clear()
}

/// Query whether text clipboard payload exists.
pub(crate) fn destack_input_clipboard_has_text(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<bool> {
    input_native::has_text()
}

/// List clipboard items.
pub(crate) fn destack_input_clipboard_list_items(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<ClipboardItemDescriptorVm>> {
    let value =
        call_out(|out| unsafe { input_native::destack_input_clipboard_list_items(binding, out) })?;

    vm_value_from_native(context, value)
}

/// Read one clipboard item as bytes.
pub(crate) fn destack_input_clipboard_read_item_bytes(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    itemindex: u32,
    representationindex: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let value = call_out(|out| unsafe {
        input_native::destack_input_clipboard_read_item_bytes(
            binding,
            out,
            itemindex,
            representationindex,
        )
    })?;

    let mut context = context.write();

    VmSlice::from_bytes(&mut context, unsafe { value.as_slice()? })
}

/// Read one clipboard item as one path.
pub(crate) fn destack_input_clipboard_read_item_path(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    itemindex: u32,
    representationindex: u32,
) -> RuntimeResult<OsPathVm> {
    let value = call_out(|out| unsafe {
        input_native::destack_input_clipboard_read_item_path(
            binding,
            out,
            itemindex,
            representationindex,
        )
    })?;

    vm_value_from_native(context, value)
}

/// Read one clipboard item as text.
pub(crate) fn destack_input_clipboard_read_item_text(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    itemindex: u32,
    representationindex: u32,
) -> RuntimeResult<destack_vm::StringHandle> {
    let value = call_out(|out| unsafe {
        input_native::destack_input_clipboard_read_item_text(
            binding,
            out,
            itemindex,
            representationindex,
        )
    })?;

    native_string_to_vm(context, value)
}

/// Read text clipboard payload.
pub(crate) fn destack_input_clipboard_read_text(
    _binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<destack_vm::StringHandle> {
    let value = input_native::read_text()?;

    Ok(destack_vm::StringHandle::new(
        context.intern_string(&value)?,
    ))
}

/// Read clipboard sequence number.
pub(crate) fn destack_input_clipboard_sequence(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<u64> {
    input_native::sequence()
}

/// Write text clipboard payload.
pub(crate) fn destack_input_clipboard_write_text(
    _binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    text: destack_vm::StringHandle,
) -> RuntimeResult<()> {
    let text = context
        .string_ref(text)
        .map_err(|error| RuntimeError::from(error).boxed())?;

    input_native::write_text(text.as_str())
}

/// Write clipboard items.
pub(crate) fn destack_input_clipboard_write_items(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    items: VmSlice<ClipboardItemVm>,
) -> RuntimeResult<()> {
    let items: NativeSlice<ClipboardItem> = native_value_from_vm(binding, context, items)?;

    unsafe { input_native::destack_input_clipboard_write_items(binding, items) }
}
