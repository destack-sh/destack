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
pub(crate) fn destack_input_close(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_close(binding, handle) }
}

/// List available input devices.
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
pub(crate) fn destack_input_open(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    id: destack_vm::StringHandle,
) -> RuntimeResult<resource::InputDeviceHandle> {
    let id = string_from_vm(binding, context, id)?;
    call_out(|out| unsafe { host_input::destack_input_open(binding, out, id) })
}

/// Query capabilities for one opened input device.
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
pub(crate) fn destack_input_monitor_close(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_monitor_close(binding, handle) }
}

/// Open one global input event monitor.
pub(crate) fn destack_input_monitor_open(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<resource::InputMonitorHandle> {
    call_out(|out| unsafe { host_input::destack_input_monitor_open(binding, out) })
}

/// Read one global input monitor event.
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
pub(crate) fn destack_input_read(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputEventVm> {
    let value = call_out(|out| unsafe { host_input::destack_input_read(binding, out, handle) })?;
    event_to_vm(context, value)
}

/// Read one batch of input events.
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
pub(crate) fn destack_input_set_exclusive_grab(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    enable: bool,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_set_exclusive_grab(binding, handle, enable) }
}

/// Select event decoding mode for one input stream.
pub(crate) fn destack_input_set_read_mode(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    mode: InputReadMode,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_set_read_mode(binding, handle, mode) }
}

/// Poll one input event without blocking.
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
pub(crate) fn destack_input_gamepad_set_player_index(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    playerindex: u8,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_gamepad_set_player_index(binding, handle, playerindex) }
}

/// Read one gamepad state snapshot.
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
pub(crate) fn destack_input_haptics_stop(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_haptics_stop(binding, handle) }
}

/// Read one keyboard state snapshot.
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
pub(crate) fn destack_input_pointer_set_relative_mode(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_pointer_set_relative_mode(binding, handle, enabled) }
}

/// Read one absolute pointer state snapshot.
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
pub(crate) fn destack_input_sensor_read(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
) -> RuntimeResult<InputSensorSampleVm> {
    call_out(|out| unsafe { host_input::destack_input_sensor_read(binding, out, handle, kind) })
}

/// Poll one sensor sample without blocking.
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
