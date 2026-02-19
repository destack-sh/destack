#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::bindings_generated as bindings;
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, PlatformError};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::input::{
    InputAxisInfo, InputButtonInfo, InputCompositionEvent, InputCompositionEventPayload,
    InputDeviceCapabilities, InputDeviceCapabilityKind, InputDeviceEventPayload, InputDeviceInfo,
    InputDeviceKind, InputEvent, InputEventAction, InputEventKind, InputEventPayload,
    InputGamepadBatteryInfo, InputGamepadBatteryState, InputGamepadButtonState,
    InputGamepadConnectionType, InputGamepadEventPayload, InputGamepadMappingType,
    InputGamepadState, InputGamepadTouchState, InputHapticEffectParameters, InputHapticEffectType,
    InputHapticsResult, InputKeyEventPayload, InputKeyboardState, InputMonitorEvent,
    InputMonitorEventKind, InputPointerButtonEventPayload, InputPointerGrabMode,
    InputPointerMotionEventPayload, InputPointerState, InputRawHidReport, InputReadMode,
    InputScrollEventPayload, InputSensorConfig, InputSensorEffectiveConfig,
    InputSensorEventPayload, InputSensorInfo, InputSensorKind, InputSensorSample,
    InputTextEventPayload, InputTextInputArea, InputTextInputType, InputTouchContactPhase,
    InputTouchContactState, InputTouchEventPayload, InputTouchState, InputWindowTarget,
};
use crate::platform::resource;

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
pub(crate) unsafe fn destack_input_sensor_configure(
    context: &RuntimeCallContext,
    out: *mut InputSensorEffectiveConfig,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
    config: InputSensorConfig,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, kind, config);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.sensor.configure",
    ))
    .boxed())
}

/// List supported sensors for one opened input device.
///
/// Return sensor capability metadata for one opened sensor-capable device.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where sensor streams are unavailable.
/// Uses backend-specific sensor capability tables from evdev and hidraw class stacks on Unix and HID sensor or controller APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_sensor_list(
    context: &RuntimeCallContext,
    out: *mut NativeArray<InputSensorInfo>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.sensor.list")).boxed())
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
pub(crate) unsafe fn destack_input_sensor_read(
    context: &RuntimeCallContext,
    out: *mut InputSensorSample,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, kind);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.sensor.read")).boxed())
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
pub(crate) unsafe fn destack_input_sensor_try_read(
    context: &RuntimeCallContext,
    out: *mut InputSensorSample,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, kind);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.sensor.tryRead")).boxed())
}
