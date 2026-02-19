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

/// Get text input area.
///
/// Return the currently configured text input area and cursor position hint for one opened input device and one optional window target.
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
pub(crate) unsafe fn destack_input_text_get_area(
    context: &RuntimeCallContext,
    out: *mut InputTextInputArea,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, target);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.text.getArea")).boxed())
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
pub(crate) unsafe fn destack_input_text_is_active(
    context: &RuntimeCallContext,
    out: *mut bool,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.text.isActive")).boxed())
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
pub(crate) unsafe fn destack_input_text_read_composition(
    context: &RuntimeCallContext,
    out: *mut InputCompositionEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.text.readComposition",
    ))
    .boxed())
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
pub(crate) unsafe fn destack_input_text_set_area(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    area: InputTextInputArea,
) -> RuntimeResult<()> {
    let _ = (context, handle, target, area);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.text.setArea")).boxed())
}

/// Start text input.
///
/// Enable text input and composition dispatch for one opened input device and one optional window target.
/// Text conversion behavior follows host IME and keyboard policy for the selected target scope.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where text input sessions or one window scope are unavailable.
/// Uses backend-specific text input activation primitives and host IME activation for global or window-scoped paths.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.text`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_text_start(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    inputtype: InputTextInputType,
) -> RuntimeResult<()> {
    let _ = (context, handle, target, inputtype);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.text.start")).boxed())
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
pub(crate) unsafe fn destack_input_text_stop(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
) -> RuntimeResult<()> {
    let _ = (context, handle, target);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.text.stop")).boxed())
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
pub(crate) unsafe fn destack_input_text_try_read_composition(
    context: &RuntimeCallContext,
    out: *mut InputCompositionEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.text.tryReadComposition",
    ))
    .boxed())
}
