#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::bindings_generated as bindings;
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, PlatformError};

use crate::runtime::BindingCallContext;
use bindings::*;

#[cfg(any(target_os = "android", target_os = "ios"))]
use crate::platform::input::mobile_text;
use crate::platform::input::{
    InputAxisMetadata, InputButtonMetadata, InputCompositionEventPayload, InputDeviceCapabilities,
    InputDeviceCapabilityKind, InputDeviceDescriptor, InputDeviceEventPayload, InputDeviceKind,
    InputEvent, InputEventAction, InputEventKind, InputEventPayload, InputGamepadBatteryState,
    InputGamepadBatteryStatus, InputGamepadButtonState, InputGamepadConnectionType,
    InputGamepadEventPayload, InputGamepadMappingType, InputGamepadState, InputGamepadTouchState,
    InputHapticEffectParameters, InputHapticEffectType, InputHapticsResult, InputKeyEventPayload,
    InputKeyboardLayoutInfo, InputKeyboardState, InputMonitorEvent, InputMonitorEventKind,
    InputPointerButtonEventPayload, InputPointerGrabMode, InputPointerMotionEventPayload,
    InputPointerState, InputRawHidReport, InputReadMode, InputScrollEventPayload,
    InputSensorConfig, InputSensorDescriptor, InputSensorEffectiveConfig, InputSensorEventPayload,
    InputSensorKind, InputSensorSample, InputTextEventPayload, InputTextGeometry,
    InputTextSessionConfig, InputTextSessionEvent, InputTextSessionState, InputTouchContactPhase,
    InputTouchContactState, InputTouchEventPayload, InputTouchState,
};
use crate::platform::resource;

/// Query capabilities for one opened input device.
pub(crate) unsafe fn destack_input_capabilities(
    binding: &BindingCallContext,
    out: *mut InputDeviceCapabilities,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.device.capabilities",
    ))
    .boxed())
}

/// Close one input device.
pub(crate) unsafe fn destack_input_close(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.device.close")).boxed())
}

/// List available input devices.
pub(crate) unsafe fn destack_input_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<InputDeviceDescriptor>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.device.list")).boxed())
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
    let _ = (binding, out, id);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.device.open")).boxed())
}

/// Close one global input event monitor.
pub(crate) unsafe fn destack_input_monitor_close(
    binding: &BindingCallContext,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.monitorClose",
    ))
    .boxed())
}

/// Open one global input event monitor.
pub(crate) unsafe fn destack_input_monitor_open(
    binding: &BindingCallContext,
    out: *mut resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.monitorOpen",
    ))
    .boxed())
}

/// Read one global input monitor event.
pub(crate) unsafe fn destack_input_monitor_read(
    binding: &BindingCallContext,
    out: *mut InputMonitorEvent,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.monitorRead",
    ))
    .boxed())
}

/// Poll one global input monitor event without blocking.
pub(crate) unsafe fn destack_input_monitor_try_read(
    binding: &BindingCallContext,
    out: *mut InputMonitorEvent,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.monitorTryRead",
    ))
    .boxed())
}

/// Read one input event.
pub(crate) unsafe fn destack_input_read(
    binding: &BindingCallContext,
    out: *mut InputEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.event.read")).boxed())
}

/// Read one batch of input events.
pub(crate) unsafe fn destack_input_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<InputEvent>,
    handle: resource::InputDeviceHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, maxevents);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.readBatch",
    ))
    .boxed())
}

/// Enable or disable exclusive device grab.
pub(crate) unsafe fn destack_input_set_exclusive_grab(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    enable: bool,
) -> RuntimeResult<()> {
    let _ = (binding, handle, enable);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.setExclusiveGrab",
    ))
    .boxed())
}

/// Select event decoding mode for one input stream.
pub(crate) unsafe fn destack_input_set_read_mode(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    mode: InputReadMode,
) -> RuntimeResult<()> {
    let _ = (binding, handle, mode);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.setReadMode",
    ))
    .boxed())
}

/// Poll one input event without blocking.
pub(crate) unsafe fn destack_input_try_read(
    binding: &BindingCallContext,
    out: *mut InputEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.event.tryRead")).boxed())
}

/// Set one gamepad light color.
pub(crate) unsafe fn destack_input_gamepad_set_light(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    red: u8,
    green: u8,
    blue: u8,
) -> RuntimeResult<()> {
    let _ = (binding, handle, red, green, blue);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.gamepad.setLight",
    ))
    .boxed())
}

/// Set one gamepad motion sensor sample rate.
pub(crate) unsafe fn destack_input_gamepad_set_motion_sensor_sample_rate(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    sampleratehz: f64,
) -> RuntimeResult<()> {
    let _ = (binding, handle, sampleratehz);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.gamepad.setMotionSensorSampleRate",
    ))
    .boxed())
}

/// Enable or disable one gamepad motion sensor stream.
pub(crate) unsafe fn destack_input_gamepad_set_motion_sensors_enabled(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (binding, handle, enabled);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.gamepad.setMotionSensorsEnabled",
    ))
    .boxed())
}

/// Set one gamepad player index.
pub(crate) unsafe fn destack_input_gamepad_set_player_index(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    playerindex: u8,
) -> RuntimeResult<()> {
    let _ = (binding, handle, playerindex);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.gamepad.setPlayerIndex",
    ))
    .boxed())
}

/// Read one gamepad state snapshot.
pub(crate) unsafe fn destack_input_gamepad_state(
    binding: &BindingCallContext,
    out: *mut InputGamepadState,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.gamepad.state")).boxed())
}

/// List supported haptic effects.
pub(crate) unsafe fn destack_input_haptics_effects(
    binding: &BindingCallContext,
    out: *mut NativeArray<InputHapticEffectType>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.haptics.effects",
    ))
    .boxed())
}

/// Play one haptic effect.
pub(crate) unsafe fn destack_input_haptics_play(
    binding: &BindingCallContext,
    out: *mut InputHapticsResult,
    handle: resource::InputDeviceHandle,
    effect: InputHapticEffectType,
    params: InputHapticEffectParameters,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, effect, params);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.haptics.play")).boxed())
}

/// Stop active haptic effects.
pub(crate) unsafe fn destack_input_haptics_stop(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.haptics.stop")).boxed())
}

/// Read one keyboard state snapshot.
pub(crate) unsafe fn destack_input_keyboard_state(
    binding: &BindingCallContext,
    out: *mut InputKeyboardState,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.keyboard.state")).boxed())
}

/// Enable or disable pointer capture.
pub(crate) unsafe fn destack_input_pointer_capture(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (binding, handle, target, enabled);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.pointer.capture",
    ))
    .boxed())
}

/// Read one relative pointer state snapshot.
pub(crate) unsafe fn destack_input_pointer_relative_state(
    binding: &BindingCallContext,
    out: *mut InputPointerState,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.pointer.relativeState",
    ))
    .boxed())
}

/// Set pointer grab mode.
pub(crate) unsafe fn destack_input_pointer_set_grab_mode(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    mode: InputPointerGrabMode,
) -> RuntimeResult<()> {
    let _ = (binding, handle, target, mode);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.pointer.setGrabMode",
    ))
    .boxed())
}

/// Enable or disable relative pointer mode.
pub(crate) unsafe fn destack_input_pointer_set_relative_mode(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (binding, handle, enabled);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.pointer.setRelativeMode",
    ))
    .boxed())
}

/// Read one absolute pointer state snapshot.
pub(crate) unsafe fn destack_input_pointer_state(
    binding: &BindingCallContext,
    out: *mut InputPointerState,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.pointer.state")).boxed())
}

/// Warp pointer position.
pub(crate) unsafe fn destack_input_pointer_warp(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    x: f64,
    y: f64,
) -> RuntimeResult<()> {
    let _ = (binding, handle, target, x, y);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.pointer.warp")).boxed())
}

/// Read one raw-hid feature report.
pub(crate) unsafe fn destack_input_raw_hid_get_feature(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::InputDeviceHandle,
    reportid: u8,
    maxbytes: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, reportid, maxbytes);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.rawhid.getFeature",
    ))
    .boxed())
}

/// Read one raw-hid report.
pub(crate) unsafe fn destack_input_raw_hid_read(
    binding: &BindingCallContext,
    out: *mut InputRawHidReport,
    handle: resource::InputDeviceHandle,
    maxbytes: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, maxbytes, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.rawhid.read")).boxed())
}

/// Write one raw-hid feature report.
pub(crate) unsafe fn destack_input_raw_hid_set_feature(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    reportid: u8,
    data: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (binding, handle, reportid, data);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.rawhid.setFeature",
    ))
    .boxed())
}

/// Poll one raw-hid report without blocking.
pub(crate) unsafe fn destack_input_raw_hid_try_read(
    binding: &BindingCallContext,
    out: *mut InputRawHidReport,
    handle: resource::InputDeviceHandle,
    maxbytes: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, maxbytes);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.rawhid.tryRead")).boxed())
}

/// Write one raw-hid output report.
pub(crate) unsafe fn destack_input_raw_hid_write(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::InputDeviceHandle,
    reportid: u8,
    data: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, reportid, data);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.rawhid.write")).boxed())
}

/// Configure one sensor stream.
pub(crate) unsafe fn destack_input_sensor_configure(
    binding: &BindingCallContext,
    out: *mut InputSensorEffectiveConfig,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
    config: InputSensorConfig,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, kind, config);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.sensor.configure",
    ))
    .boxed())
}

/// List supported sensors for one opened input device.
pub(crate) unsafe fn destack_input_sensor_list(
    binding: &BindingCallContext,
    out: *mut NativeArray<InputSensorDescriptor>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.sensor.list")).boxed())
}

/// Read one sensor sample.
pub(crate) unsafe fn destack_input_sensor_read(
    binding: &BindingCallContext,
    out: *mut InputSensorSample,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, kind);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.sensor.read")).boxed())
}

/// Poll one sensor sample without blocking.
pub(crate) unsafe fn destack_input_sensor_try_read(
    binding: &BindingCallContext,
    out: *mut InputSensorSample,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, kind);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.sensor.tryRead")).boxed())
}

/// Get text input area.
pub(crate) unsafe fn destack_input_text_get_geometry(
    binding: &BindingCallContext,
    out: *mut InputTextGeometry,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        return unsafe { mobile_text::destack_input_text_get_geometry(binding, out, session) };
    }

    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, session);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.text.getGeometry",
    ))
    .boxed())
}

/// Open one text input session.
pub(crate) unsafe fn destack_input_text_open(
    binding: &BindingCallContext,
    out: *mut resource::InputTextSessionHandle,
    config: InputTextSessionConfig,
    state: InputTextSessionState,
) -> RuntimeResult<()> {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        return unsafe { mobile_text::destack_input_text_open(binding, out, config, state) };
    }

    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, config, state);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.text.open")).boxed())
}

/// Close one text input session.
pub(crate) unsafe fn destack_input_text_close(
    binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        return unsafe { mobile_text::destack_input_text_close(binding, session) };
    }

    let _ = (binding, session);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.text.close")).boxed())
}

/// Read one composition event.
pub(crate) unsafe fn destack_input_text_read_event(
    binding: &BindingCallContext,
    out: *mut InputTextSessionEvent,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        return unsafe { mobile_text::destack_input_text_read_event(binding, out, session) };
    }

    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, session);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.text.readEvent")).boxed())
}

/// Set text input area.
pub(crate) unsafe fn destack_input_text_set_geometry(
    binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
    area: InputTextGeometry,
) -> RuntimeResult<()> {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        return unsafe { mobile_text::destack_input_text_set_geometry(binding, session, area) };
    }

    let _ = (binding, session, area);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.text.setGeometry",
    ))
    .boxed())
}

/// Set text input state.
pub(crate) unsafe fn destack_input_text_set_state(
    binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
    state: InputTextSessionState,
) -> RuntimeResult<()> {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        return unsafe { mobile_text::destack_input_text_set_state(binding, session, state) };
    }

    let _ = (binding, session, state);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.text.setState")).boxed())
}

/// Poll one composition event without blocking.
pub(crate) unsafe fn destack_input_text_try_read_event(
    binding: &BindingCallContext,
    out: *mut InputTextSessionEvent,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        return unsafe { mobile_text::destack_input_text_try_read_event(binding, out, session) };
    }

    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, session);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.text.tryReadEvent",
    ))
    .boxed())
}

/// Read one touch state snapshot.
pub(crate) unsafe fn destack_input_touch_state(
    binding: &BindingCallContext,
    out: *mut InputTouchState,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.touch.state")).boxed())
}

/// Read one keyboard layout snapshot.
pub(crate) unsafe fn destack_input_keyboard_layout(
    binding: &BindingCallContext,
    out: *mut InputKeyboardLayoutInfo,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.keyboard.layout",
    ))
    .boxed())
}
