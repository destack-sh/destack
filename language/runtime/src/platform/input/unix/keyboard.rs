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

/// Read one keyboard state snapshot.
///
/// Return one current keyboard key and modifier snapshot for one opened keyboard-capable device.
/// Snapshot values represent one point-in-time backend state and can change immediately after read.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where keyboard snapshots are unavailable.
/// Uses backend-specific key-state tables from evdev or terminal backends on Unix and console or raw-input key-state paths on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_keyboard_state(
    context: &RuntimeCallContext,
    out: *mut InputKeyboardState,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.keyboard.state")).boxed())
}
