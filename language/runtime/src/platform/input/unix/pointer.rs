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
pub(crate) unsafe fn destack_input_pointer_capture(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (context, handle, target, enabled);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.pointer.capture",
    ))
    .boxed())
}

/// Read one relative pointer state snapshot.
///
/// Return one relative motion and button state snapshot for one opened pointer-capable device.
/// Delta units follow backend-native relative motion semantics.
/// Pen-capable devices can populate pressure and tilt metadata.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific relative motion streams from evdev or libinput style backends on Unix and raw-input relative motion on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_pointer_relative_state(
    context: &RuntimeCallContext,
    out: *mut InputPointerState,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.pointer.relativeState",
    ))
    .boxed())
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
pub(crate) unsafe fn destack_input_pointer_set_grab_mode(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    mode: InputPointerGrabMode,
) -> RuntimeResult<()> {
    let _ = (context, handle, target, mode);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.pointer.setGrabMode",
    ))
    .boxed())
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
pub(crate) unsafe fn destack_input_pointer_set_relative_mode(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (context, handle, enabled);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.pointer.setRelativeMode",
    ))
    .boxed())
}

/// Read one absolute pointer state snapshot.
///
/// Return one current pointer position and button state snapshot for one opened pointer-capable device.
/// Position values follow backend coordinate space for that device.
/// Pen-capable devices can populate pressure and tilt metadata.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific pointer state queries from evdev or libinput style streams on Unix and raw-input or console pointer state snapshots on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_pointer_state(
    context: &RuntimeCallContext,
    out: *mut InputPointerState,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.pointer.state")).boxed())
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
pub(crate) unsafe fn destack_input_pointer_warp(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    x: f64,
    y: f64,
) -> RuntimeResult<()> {
    let _ = (context, handle, target, x, y);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.pointer.warp")).boxed())
}
