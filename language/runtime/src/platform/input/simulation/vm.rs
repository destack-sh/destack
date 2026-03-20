#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{
    ClipboardBinaryFormat, InputCompositionEventVm, InputDeviceCapabilitiesVm,
    InputDeviceDescriptorVm, InputEventVm, InputGamepadStateVm, InputHapticEffectParametersVm,
    InputHapticEffectType, InputHapticsResult, InputKeyboardStateVm, InputMonitorEventVm,
    InputPointerGrabMode, InputPointerStateVm, InputRawHidReportVm, InputReadMode,
    InputSensorConfigVm, InputSensorDescriptorVm, InputSensorEffectiveConfigVm, InputSensorKind,
    InputSensorSampleVm, InputTextInputAreaVm, InputTextInputType, InputTouchStateVm,
    InputWindowTargetVm,
};
use crate::platform::{PlatformError, VmArray, VmSlice, resource};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputDeviceCapabilitiesVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.device.capabilities",
    ))
    .boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.input.device.close")).boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<InputDeviceDescriptorVm>> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.input.device.list")).boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<resource::InputDeviceHandle> {
    let _ = id;
    Err(RuntimeError::from(PlatformError::not_supported("destack.input.device.open")).boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.monitorClose",
    ))
    .boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<resource::InputMonitorHandle> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.monitorOpen",
    ))
    .boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<InputMonitorEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.monitorRead",
    ))
    .boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<InputMonitorEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.monitorTryRead",
    ))
    .boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.input.event.read")).boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    maxevents: u32,
) -> RuntimeResult<VmArray<InputEventVm>> {
    let _ = (handle, maxevents);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.readBatch",
    ))
    .boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.input.event.tryRead")).boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputGamepadStateVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.input.gamepad.state")).boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<VmArray<InputHapticEffectType>> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.haptics.effects",
    ))
    .boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    effect: InputHapticEffectType,
    params: InputHapticEffectParametersVm,
) -> RuntimeResult<InputHapticsResult> {
    let _ = (handle, effect, params);
    Err(RuntimeError::from(PlatformError::not_supported("destack.input.haptics.play")).boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.input.haptics.stop")).boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputKeyboardStateVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.input.keyboard.state")).boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    target: InputWindowTargetVm,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, target, enabled);
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputPointerStateVm> {
    let _ = handle;
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
pub(crate) fn destack_input_pointer_set_grab_mode(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    target: InputWindowTargetVm,
    mode: InputPointerGrabMode,
) -> RuntimeResult<()> {
    let _ = (handle, target, mode);
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
pub(crate) fn destack_input_pointer_set_relative_mode(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputPointerStateVm> {
    let _ = handle;
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
pub(crate) fn destack_input_pointer_warp(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    target: InputWindowTargetVm,
    x: f64,
    y: f64,
) -> RuntimeResult<()> {
    let _ = (handle, target, x, y);
    Err(RuntimeError::from(PlatformError::not_supported("destack.input.pointer.warp")).boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    reportid: u8,
    maxbytes: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, reportid, maxbytes);
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
pub(crate) fn destack_input_raw_hid_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    maxbytes: u32,
    timeoutns: u64,
) -> RuntimeResult<InputRawHidReportVm> {
    let _ = (handle, maxbytes, timeoutns);
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
pub(crate) fn destack_input_raw_hid_set_feature(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    reportid: u8,
    data: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (handle, reportid, data);
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    maxbytes: u32,
) -> RuntimeResult<InputRawHidReportVm> {
    let _ = (handle, maxbytes);
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
pub(crate) fn destack_input_raw_hid_write(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    reportid: u8,
    data: VmSlice<u8>,
) -> RuntimeResult<u32> {
    let _ = (handle, reportid, data);
    Err(RuntimeError::from(PlatformError::not_supported("destack.input.rawhid.write")).boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
    config: InputSensorConfigVm,
) -> RuntimeResult<InputSensorEffectiveConfigVm> {
    let _ = (handle, kind, config);
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<VmArray<InputSensorDescriptorVm>> {
    let _ = handle;
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
pub(crate) fn destack_input_sensor_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
) -> RuntimeResult<InputSensorSampleVm> {
    let _ = (handle, kind);
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
pub(crate) fn destack_input_sensor_try_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
) -> RuntimeResult<InputSensorSampleVm> {
    let _ = (handle, kind);
    Err(RuntimeError::from(PlatformError::not_supported("destack.input.sensor.tryRead")).boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    target: InputWindowTargetVm,
) -> RuntimeResult<InputTextInputAreaVm> {
    let _ = (handle, target);
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
pub(crate) fn destack_input_text_is_active(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<bool> {
    let _ = handle;
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
pub(crate) fn destack_input_text_read_composition(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputCompositionEventVm> {
    let _ = handle;
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
pub(crate) fn destack_input_text_set_area(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    target: InputWindowTargetVm,
    area: InputTextInputAreaVm,
) -> RuntimeResult<()> {
    let _ = (handle, target, area);
    Err(RuntimeError::from(PlatformError::not_supported("destack.input.text.setArea")).boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    target: InputWindowTargetVm,
    inputtype: InputTextInputType,
) -> RuntimeResult<()> {
    let _ = (handle, target, inputtype);
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
pub(crate) fn destack_input_text_stop(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    target: InputWindowTargetVm,
) -> RuntimeResult<()> {
    let _ = (handle, target);
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
pub(crate) fn destack_input_text_try_read_composition(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputCompositionEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.text.tryReadComposition",
    ))
    .boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputTouchStateVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.input.touch.state")).boxed())
}

/// Clear clipboard payload.
pub(crate) fn destack_input_clipboard_clear(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.clipboard.clear",
    ))
    .boxed())
}

/// Query whether text clipboard payload exists.
pub(crate) fn destack_input_clipboard_has_text(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<bool> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.clipboard.hasText",
    ))
    .boxed())
}

/// Read binary clipboard payload.
pub(crate) fn destack_input_clipboard_read_bytes(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    format: ClipboardBinaryFormat,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = format;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.clipboard.readBytes",
    ))
    .boxed())
}

/// Read text clipboard payload.
pub(crate) fn destack_input_clipboard_read_text(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<vm::StringHandle> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.clipboard.readText",
    ))
    .boxed())
}

/// Read clipboard sequence number.
pub(crate) fn destack_input_clipboard_sequence(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<u64> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.clipboard.sequence",
    ))
    .boxed())
}

/// Write binary clipboard payload.
pub(crate) fn destack_input_clipboard_write_bytes(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    format: ClipboardBinaryFormat,
    argument_bytes: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (format, argument_bytes);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.clipboard.writeBytes",
    ))
    .boxed())
}

/// Write text clipboard payload.
pub(crate) fn destack_input_clipboard_write_text(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    text: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = text;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.clipboard.writeText",
    ))
    .boxed())
}
