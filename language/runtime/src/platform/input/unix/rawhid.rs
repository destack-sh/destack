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
pub(crate) unsafe fn destack_input_raw_hid_get_feature(
    context: &RuntimeCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::InputDeviceHandle,
    reportid: u8,
    maxbytes: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, reportid, maxbytes);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.rawhid.getFeature",
    ))
    .boxed())
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
pub(crate) unsafe fn destack_input_raw_hid_read(
    context: &RuntimeCallContext,
    out: *mut InputRawHidReport,
    handle: resource::InputDeviceHandle,
    maxbytes: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, maxbytes, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.rawhid.read")).boxed())
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
pub(crate) unsafe fn destack_input_raw_hid_set_feature(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    reportid: u8,
    data: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (context, handle, reportid, data);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.rawhid.setFeature",
    ))
    .boxed())
}

/// Poll one raw-hid report without blocking.
///
/// Poll one pending raw-hid report and return immediately when none is available.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where raw-hid reports are unavailable.
/// Uses nonblocking hidraw or equivalent raw report APIs on Unix and nonblocking raw-input hid report APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_raw_hid_try_read(
    context: &RuntimeCallContext,
    out: *mut InputRawHidReport,
    handle: resource::InputDeviceHandle,
    maxbytes: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, maxbytes);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.rawhid.tryRead")).boxed())
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
pub(crate) unsafe fn destack_input_raw_hid_write(
    context: &RuntimeCallContext,
    out: *mut u32,
    handle: resource::InputDeviceHandle,
    reportid: u8,
    data: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, reportid, data);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.rawhid.write")).boxed())
}
