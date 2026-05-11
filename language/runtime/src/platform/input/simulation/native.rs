#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::{NativeArray, PlatformError};

use crate::platform::input::{
    ClipboardItem, ClipboardItemDescriptor, InputDeviceCapabilities, InputDeviceDescriptor,
    InputEvent, InputGamepadState, InputHapticEffectParameters, InputHapticEffectType,
    InputHapticsResult, InputKeyboardLayoutInfo, InputKeyboardState, InputMonitorEvent,
    InputPointerGrabMode, InputPointerState, InputRawHidReport, InputReadMode, InputSensorConfig,
    InputSensorDescriptor, InputSensorEffectiveConfig, InputSensorKind, InputSensorSample,
    InputTextGeometry, InputTextSessionConfig, InputTextSessionEvent, InputTextSessionState,
    InputTouchState, InputWindowTarget,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

/// Query capabilities for one opened input device.
pub(crate) unsafe fn destack_input_capabilities(
    _binding: &BindingCallContext,
    out: *mut InputDeviceCapabilities,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.device.capabilities",
    ))
    .boxed())
}

/// Close one input device.
pub(crate) unsafe fn destack_input_close(
    _binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.device.close")).boxed())
}

/// List available input devices.
pub(crate) unsafe fn destack_input_list(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<InputDeviceDescriptor>,
) -> RuntimeResult<()> {
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.device.list")).boxed())
}

/// Open one input device.
pub(crate) unsafe fn destack_input_open(
    _binding: &BindingCallContext,
    out: *mut resource::InputDeviceHandle,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (out, id);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.device.open")).boxed())
}

/// Close one global input event monitor.
pub(crate) unsafe fn destack_input_monitor_close(
    _binding: &BindingCallContext,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.monitorClose",
    ))
    .boxed())
}

/// Open one global input event monitor.
pub(crate) unsafe fn destack_input_monitor_open(
    _binding: &BindingCallContext,
    out: *mut resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.monitorOpen",
    ))
    .boxed())
}

/// Read one global input monitor event.
pub(crate) unsafe fn destack_input_monitor_read(
    _binding: &BindingCallContext,
    out: *mut InputMonitorEvent,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.monitorRead",
    ))
    .boxed())
}

/// Poll one global input monitor event without blocking.
pub(crate) unsafe fn destack_input_monitor_try_read(
    _binding: &BindingCallContext,
    out: *mut InputMonitorEvent,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.monitorTryRead",
    ))
    .boxed())
}

/// Read one input event.
pub(crate) unsafe fn destack_input_read(
    _binding: &BindingCallContext,
    out: *mut InputEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.event.read")).boxed())
}

/// Read one batch of input events.
pub(crate) unsafe fn destack_input_read_batch(
    _binding: &BindingCallContext,
    out: *mut NativeArray<InputEvent>,
    handle: resource::InputDeviceHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    let _ = (out, handle, maxevents);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.readBatch",
    ))
    .boxed())
}

/// Enable or disable exclusive device grab.
pub(crate) unsafe fn destack_input_set_exclusive_grab(
    _binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    enable: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enable);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.setExclusiveGrab",
    ))
    .boxed())
}

/// Select event decoding mode for one input stream.
pub(crate) unsafe fn destack_input_set_read_mode(
    _binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    mode: InputReadMode,
) -> RuntimeResult<()> {
    let _ = (handle, mode);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.setReadMode",
    ))
    .boxed())
}

/// Poll one input event without blocking.
pub(crate) unsafe fn destack_input_try_read(
    _binding: &BindingCallContext,
    out: *mut InputEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.event.tryRead")).boxed())
}

/// Set one gamepad light color.
pub(crate) unsafe fn destack_input_gamepad_set_light(
    _binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    red: u8,
    green: u8,
    blue: u8,
) -> RuntimeResult<()> {
    let _ = (handle, red, green, blue);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.gamepad.setLight",
    ))
    .boxed())
}

/// Set one gamepad motion sensor sample rate.
pub(crate) unsafe fn destack_input_gamepad_set_motion_sensor_sample_rate(
    _binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    sampleratehz: f64,
) -> RuntimeResult<()> {
    let _ = (handle, sampleratehz);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.gamepad.setMotionSensorSampleRate",
    ))
    .boxed())
}

/// Enable or disable one gamepad motion sensor stream.
pub(crate) unsafe fn destack_input_gamepad_set_motion_sensors_enabled(
    _binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.gamepad.setMotionSensorsEnabled",
    ))
    .boxed())
}

/// Set one gamepad player index.
pub(crate) unsafe fn destack_input_gamepad_set_player_index(
    _binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    playerindex: u8,
) -> RuntimeResult<()> {
    let _ = (handle, playerindex);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.gamepad.setPlayerIndex",
    ))
    .boxed())
}

/// Read one gamepad state snapshot.
pub(crate) unsafe fn destack_input_gamepad_state(
    _binding: &BindingCallContext,
    out: *mut InputGamepadState,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.gamepad.state")).boxed())
}

/// List supported haptic effects.
pub(crate) unsafe fn destack_input_haptics_effects(
    _binding: &BindingCallContext,
    out: *mut NativeArray<InputHapticEffectType>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.haptics.effects",
    ))
    .boxed())
}

/// Play one haptic effect.
pub(crate) unsafe fn destack_input_haptics_play(
    _binding: &BindingCallContext,
    out: *mut InputHapticsResult,
    handle: resource::InputDeviceHandle,
    effect: InputHapticEffectType,
    params: InputHapticEffectParameters,
) -> RuntimeResult<()> {
    let _ = (out, handle, effect, params);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.haptics.play")).boxed())
}

/// Stop active haptic effects.
pub(crate) unsafe fn destack_input_haptics_stop(
    _binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.haptics.stop")).boxed())
}

/// Read one keyboard state snapshot.
pub(crate) unsafe fn destack_input_keyboard_state(
    _binding: &BindingCallContext,
    out: *mut InputKeyboardState,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.keyboard.state")).boxed())
}

/// Enable or disable pointer capture.
pub(crate) unsafe fn destack_input_pointer_capture(
    _binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, target, enabled);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.pointer.capture",
    ))
    .boxed())
}

/// Read one relative pointer state snapshot.
pub(crate) unsafe fn destack_input_pointer_relative_state(
    _binding: &BindingCallContext,
    out: *mut InputPointerState,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.pointer.relativeState",
    ))
    .boxed())
}

/// Set pointer grab mode.
pub(crate) unsafe fn destack_input_pointer_set_grab_mode(
    _binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    mode: InputPointerGrabMode,
) -> RuntimeResult<()> {
    let _ = (handle, target, mode);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.pointer.setGrabMode",
    ))
    .boxed())
}

/// Enable or disable relative pointer mode.
pub(crate) unsafe fn destack_input_pointer_set_relative_mode(
    _binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.pointer.setRelativeMode",
    ))
    .boxed())
}

/// Read one absolute pointer state snapshot.
pub(crate) unsafe fn destack_input_pointer_state(
    _binding: &BindingCallContext,
    out: *mut InputPointerState,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.pointer.state")).boxed())
}

/// Warp pointer position.
pub(crate) unsafe fn destack_input_pointer_warp(
    _binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    x: f64,
    y: f64,
) -> RuntimeResult<()> {
    let _ = (handle, target, x, y);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.pointer.warp")).boxed())
}

/// Read one raw-hid feature report.
pub(crate) unsafe fn destack_input_raw_hid_get_feature(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::InputDeviceHandle,
    reportid: u8,
    maxbytes: u32,
) -> RuntimeResult<()> {
    let _ = (out, handle, reportid, maxbytes);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.rawhid.getFeature",
    ))
    .boxed())
}

/// Read one raw-hid report.
pub(crate) unsafe fn destack_input_raw_hid_read(
    _binding: &BindingCallContext,
    out: *mut InputRawHidReport,
    handle: resource::InputDeviceHandle,
    maxbytes: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (out, handle, maxbytes, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.rawhid.read")).boxed())
}

/// Write one raw-hid feature report.
pub(crate) unsafe fn destack_input_raw_hid_set_feature(
    _binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    reportid: u8,
    data: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (handle, reportid, data);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.rawhid.setFeature",
    ))
    .boxed())
}

/// Poll one raw-hid report without blocking.
pub(crate) unsafe fn destack_input_raw_hid_try_read(
    _binding: &BindingCallContext,
    out: *mut InputRawHidReport,
    handle: resource::InputDeviceHandle,
    maxbytes: u32,
) -> RuntimeResult<()> {
    let _ = (out, handle, maxbytes);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.rawhid.tryRead")).boxed())
}

/// Write one raw-hid output report.
pub(crate) unsafe fn destack_input_raw_hid_write(
    _binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::InputDeviceHandle,
    reportid: u8,
    data: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, handle, reportid, data);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.rawhid.write")).boxed())
}

/// Configure one sensor stream.
pub(crate) unsafe fn destack_input_sensor_configure(
    _binding: &BindingCallContext,
    out: *mut InputSensorEffectiveConfig,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
    config: InputSensorConfig,
) -> RuntimeResult<()> {
    let _ = (out, handle, kind, config);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.sensor.configure",
    ))
    .boxed())
}

/// List supported sensors for one opened input device.
pub(crate) unsafe fn destack_input_sensor_list(
    _binding: &BindingCallContext,
    out: *mut NativeArray<InputSensorDescriptor>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.sensor.list")).boxed())
}

/// Read one sensor sample.
pub(crate) unsafe fn destack_input_sensor_read(
    _binding: &BindingCallContext,
    out: *mut InputSensorSample,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
) -> RuntimeResult<()> {
    let _ = (out, handle, kind);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.sensor.read")).boxed())
}

/// Poll one sensor sample without blocking.
pub(crate) unsafe fn destack_input_sensor_try_read(
    _binding: &BindingCallContext,
    out: *mut InputSensorSample,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
) -> RuntimeResult<()> {
    let _ = (out, handle, kind);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.sensor.tryRead")).boxed())
}

/// Get text input area.
pub(crate) unsafe fn destack_input_text_get_geometry(
    _binding: &BindingCallContext,
    out: *mut InputTextGeometry,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    let _ = (out, session);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.text.getGeometry",
    ))
    .boxed())
}

/// Open one text input session.
pub(crate) unsafe fn destack_input_text_open(
    _binding: &BindingCallContext,
    out: *mut resource::InputTextSessionHandle,
    config: InputTextSessionConfig,
    state: InputTextSessionState,
) -> RuntimeResult<()> {
    let _ = (out, config, state);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.text.open")).boxed())
}

/// Close one text input session.
pub(crate) unsafe fn destack_input_text_close(
    _binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    let _ = session;

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.text.close")).boxed())
}

/// Read one composition event.
pub(crate) unsafe fn destack_input_text_read_event(
    _binding: &BindingCallContext,
    out: *mut InputTextSessionEvent,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    let _ = (out, session);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.text.readEvent")).boxed())
}

/// Set text input area.
pub(crate) unsafe fn destack_input_text_set_geometry(
    _binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
    area: InputTextGeometry,
) -> RuntimeResult<()> {
    let _ = (session, area);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.text.setGeometry",
    ))
    .boxed())
}

/// Set text input state.
pub(crate) unsafe fn destack_input_text_set_state(
    _binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
    state: InputTextSessionState,
) -> RuntimeResult<()> {
    let _ = (session, state);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.text.setState")).boxed())
}

/// Poll one composition event without blocking.
pub(crate) unsafe fn destack_input_text_try_read_event(
    _binding: &BindingCallContext,
    out: *mut InputTextSessionEvent,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    let _ = (out, session);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.text.tryReadEvent",
    ))
    .boxed())
}

/// Read one touch state snapshot.
pub(crate) unsafe fn destack_input_touch_state(
    _binding: &BindingCallContext,
    out: *mut InputTouchState,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.touch.state")).boxed())
}

/// Read one keyboard layout snapshot.
pub(crate) unsafe fn destack_input_keyboard_layout(
    _binding: &BindingCallContext,
    out: *mut InputKeyboardLayoutInfo,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.keyboard.layout",
    ))
    .boxed())
}

/// Clear clipboard payload.
pub(crate) unsafe fn destack_input_clipboard_clear(
    _binding: &BindingCallContext,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.clipboard.clear",
    ))
    .boxed())
}

/// Query whether text clipboard payload exists.
pub(crate) unsafe fn destack_input_clipboard_has_text(
    _binding: &BindingCallContext,
    out: *mut bool,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.clipboard.hasText",
    ))
    .boxed())
}

/// Read text clipboard payload.
pub(crate) unsafe fn destack_input_clipboard_read_text(
    _binding: &BindingCallContext,
    out: *mut NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.clipboard.readText",
    ))
    .boxed())
}

/// Read clipboard sequence number.
pub(crate) unsafe fn destack_input_clipboard_sequence(
    _binding: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.clipboard.sequence",
    ))
    .boxed())
}

/// List clipboard items.
pub(crate) unsafe fn destack_input_clipboard_list_items(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<ClipboardItemDescriptor>,
) -> RuntimeResult<()> {
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.clipboard.listItems",
    ))
    .boxed())
}

/// Read one clipboard item as bytes.
pub(crate) unsafe fn destack_input_clipboard_read_item_bytes(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    itemindex: u32,
    representationindex: u32,
) -> RuntimeResult<()> {
    let _ = (out, itemindex, representationindex);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.clipboard.readItemBytes",
    ))
    .boxed())
}

/// Read one clipboard item as one path.
pub(crate) unsafe fn destack_input_clipboard_read_item_path(
    _binding: &BindingCallContext,
    out: *mut crate::platform::fs::OsPath,
    itemindex: u32,
    representationindex: u32,
) -> RuntimeResult<()> {
    let _ = (out, itemindex, representationindex);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.clipboard.readItemPath",
    ))
    .boxed())
}

/// Read one clipboard item as text.
pub(crate) unsafe fn destack_input_clipboard_read_item_text(
    _binding: &BindingCallContext,
    out: *mut NativeStringRef,
    itemindex: u32,
    representationindex: u32,
) -> RuntimeResult<()> {
    let _ = (out, itemindex, representationindex);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.clipboard.readItemText",
    ))
    .boxed())
}

/// Write text clipboard payload.
pub(crate) unsafe fn destack_input_clipboard_write_text(
    _binding: &BindingCallContext,
    text: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = text;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.clipboard.writeText",
    ))
    .boxed())
}

/// Write clipboard items.
pub(crate) unsafe fn destack_input_clipboard_write_items(
    _binding: &BindingCallContext,
    items: NativeSlice<ClipboardItem>,
) -> RuntimeResult<()> {
    let _ = items;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.clipboard.writeItems",
    ))
    .boxed())
}
