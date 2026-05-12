use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use windows_sys::Win32::Foundation::{
    CloseHandle, DUPLICATE_SAME_ACCESS, DuplicateHandle, ERROR_ACCESS_DENIED,
    ERROR_DEVICE_NOT_CONNECTED, ERROR_INVALID_HANDLE, ERROR_INVALID_PARAMETER, HANDLE, HWND,
    INVALID_HANDLE_VALUE, POINT, RECT,
};
use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
use windows_sys::Win32::System::Console::{
    ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT, GetConsoleMode, GetStdHandle,
    INPUT_RECORD, STD_INPUT_HANDLE, SetConsoleMode,
};
use windows_sys::Win32::System::Threading::GetCurrentProcess;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, VK_CAPITAL, VK_CONTROL, VK_LBUTTON, VK_MBUTTON, VK_MENU,
    VK_NUMLOCK, VK_RBUTTON, VK_SCROLL, VK_SHIFT, VK_XBUTTON1, VK_XBUTTON2,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{ClipCursor, GetClientRect, GetCursorPos};

use super::{raw as raw_input, xinput as xinput_input};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{
    InputCompositionEvent, InputCompositionEventPayload, InputDeviceEvent, InputDeviceEventPayload,
    InputEvent, InputEventAction, InputEventMetadata, InputGamepadEvent, InputGamepadEventPayload,
    InputKeyEvent, InputKeyEventPayload, InputPointerButtonEvent, InputPointerButtonEventPayload,
    InputPointerMotionEvent, InputPointerMotionEventPayload, InputReadMode, InputScrollEvent,
    InputScrollEventPayload, InputSensorEffectiveConfig, InputSensorEvent, InputSensorEventPayload,
    InputSensorKind, InputTextEvent, InputTextEventPayload, InputTextGeometry, InputTextInputType,
    InputTouchEvent, InputTouchEventPayload, InputWindowTarget,
};
use crate::platform::resource::{ResourceFinalizer, ResourceId, ResourceKind, WindowHandle};
use crate::platform::{PlatformError, core as core_platform, resource};
use crate::runtime::BindingCallContext;

/// Resource-table label for opened input-device entries.
pub(super) const INPUT_RESOURCE_LABEL: &str = "input.device";
/// Stable console input device identifier.
pub(super) const WINDOWS_INPUT_DEVICE_ID: &str = "console:stdin";
/// Console identifier alias accepted by open.
pub(super) const WINDOWS_INPUT_DEVICE_ID_ALIAS: &str = "console";
/// Stdin identifier alias accepted by open.
pub(super) const WINDOWS_INPUT_DEVICE_ID_STDIN: &str = "stdin";
/// Win32 console input pseudo-path accepted by open.
pub(super) const WINDOWS_INPUT_DEVICE_ID_PATH: &str = "\\\\.\\CONIN$";
/// Display name for console input device metadata.
pub(super) const WINDOWS_INPUT_DEVICE_NAME: &str = "windows console input";
/// Reported key count for console input.
pub(super) const WINDOWS_CONSOLE_KEY_COUNT: u16 = 255;
/// Reported button count for console input.
pub(super) const WINDOWS_CONSOLE_BUTTON_COUNT: u16 = 5;
/// Reported axis count for console input.
pub(super) const WINDOWS_CONSOLE_AXIS_COUNT: u16 = 2;
/// CONTROL_KEY_STATE bit for shift.
const SHIFT_PRESSED: u32 = 0x0010;
/// CONTROL_KEY_STATE bit for left control.
const LEFT_CTRL_PRESSED: u32 = 0x0008;
/// CONTROL_KEY_STATE bit for left alt.
const LEFT_ALT_PRESSED: u32 = 0x0002;
/// CONTROL_KEY_STATE bit for caps lock.
const CAPSLOCK_ON: u32 = 0x0080;
/// CONTROL_KEY_STATE bit for num lock.
const NUMLOCK_ON: u32 = 0x0020;
/// CONTROL_KEY_STATE bit for scroll lock.
const SCROLLLOCK_ON: u32 = 0x0040;
/// Stable bit for left pointer button in pointer snapshots.
const POINTER_BUTTON_LEFT: u32 = 1u32 << 0;
/// Stable bit for right pointer button in pointer snapshots.
const POINTER_BUTTON_RIGHT: u32 = 1u32 << 1;
/// Stable bit for middle pointer button in pointer snapshots.
const POINTER_BUTTON_MIDDLE: u32 = 1u32 << 2;
/// Stable bit for x1 pointer button in pointer snapshots.
const POINTER_BUTTON_X1: u32 = 1u32 << 3;
/// Stable bit for x2 pointer button in pointer snapshots.
const POINTER_BUTTON_X2: u32 = 1u32 << 4;
/// Sentinel window resource id for one omitted target window.
const WINDOW_TARGET_DEFAULT_RESOURCE_ID: u64 = 0;
/// Minimum supported XInput player index.
const XINPUT_PLAYER_INDEX_MIN: u8 = 1;
/// Maximum supported XInput player index.
const XINPUT_PLAYER_INDEX_MAX: u8 = 4;
/// Number of active console input streams.
pub(super) static WINDOWS_CONSOLE_STREAMS: AtomicUsize = AtomicUsize::new(0);

/// Backend-local input event lane selector for Windows input sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WindowsInputEventKind {
    /// Key event lane.
    Key,
    /// Pointer-motion event lane.
    PointerMotion,
    /// Pointer-button event lane.
    PointerButton,
    /// Scroll event lane.
    Scroll,
    /// Touch event lane.
    Touch,
    /// Gamepad event lane.
    Gamepad,
    /// Text event lane.
    Text,
    /// Device event lane.
    Device,
    /// Sensor event lane.
    Sensor,
    /// Composition event lane.
    Composition,
}

/// Backend-local payload shell for Windows event projection.
#[derive(Debug, Clone, Copy)]
pub(super) struct WindowsInputEventPayload {
    /// Key payload.
    pub(super) key: InputKeyEventPayload,
    /// Pointer-motion payload.
    pub(super) pointer_motion: InputPointerMotionEventPayload,
    /// Pointer-button payload.
    pub(super) pointer_button: InputPointerButtonEventPayload,
    /// Scroll payload.
    pub(super) scroll: InputScrollEventPayload,
    /// Touch payload.
    pub(super) touch: InputTouchEventPayload,
    /// Gamepad payload.
    pub(super) gamepad: InputGamepadEventPayload,
    /// Text payload.
    pub(super) text: InputTextEventPayload,
    /// Device payload.
    pub(super) device: InputDeviceEventPayload,
    /// Sensor payload.
    pub(super) sensor: InputSensorEventPayload,
    /// Composition payload.
    pub(super) composition: InputCompositionEventPayload,
}

/// Build one zeroed payload shell for event-kind projection.
pub(super) fn empty_event_payload(binding: &BindingCallContext) -> WindowsInputEventPayload {
    let empty_text = binding.store_string("");
    WindowsInputEventPayload {
        key: InputKeyEventPayload {
            action: InputEventAction::Cancel,
            backend_code: 0,
            backend_scan_code: 0,
            backend_value: 0,
            modifiers: 0,
            repeat: false,
        },
        pointer_motion: InputPointerMotionEventPayload {
            x: 0.0,
            y: 0.0,
            buttons: 0,
            modifiers: 0,
        },
        pointer_button: InputPointerButtonEventPayload {
            action: InputEventAction::Cancel,
            backend_code: 0,
            backend_value: 0,
            x: 0.0,
            y: 0.0,
            modifiers: 0,
        },
        scroll: InputScrollEventPayload {
            wheel_x: 0.0,
            wheel_y: 0.0,
            x: 0.0,
            y: 0.0,
            modifiers: 0,
        },
        touch: InputTouchEventPayload {
            action: InputEventAction::Cancel,
            contact_id: 0,
            x: 0.0,
            y: 0.0,
            pressure: 0.0,
        },
        gamepad: InputGamepadEventPayload {
            action: InputEventAction::Cancel,
            backend_code: 0,
            backend_value: 0,
        },
        text: InputTextEventPayload {
            text: empty_text,
            is_composing: false,
        },
        device: InputDeviceEventPayload {
            action: InputEventAction::Cancel,
            backend_code: 0,
            backend_value: 0,
        },
        sensor: InputSensorEventPayload {
            action: InputEventAction::Cancel,
            backend_code: 0,
            backend_value: 0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        composition: InputCompositionEventPayload {
            action: InputEventAction::Cancel,
            text: empty_text,
            selection_start: 0,
            selection_end: 0,
        },
    }
}

/// Build one typed input event from one prepared payload.
pub(super) fn build_input_event(
    binding: &BindingCallContext,
    kind: WindowsInputEventKind,
    timestamp_ns: u64,
    sequence: u64,
    device_id: &str,
    payload: WindowsInputEventPayload,
) -> InputEvent {
    let metadata = InputEventMetadata {
        timestamp_ns,
        sequence,
        device_id: binding.store_string(device_id),
    };

    match kind {
        WindowsInputEventKind::Key => InputEvent::InputKeyEvent(InputKeyEvent {
            kind: binding.store_string("key"),
            metadata,
            payload: payload.key,
        }),
        WindowsInputEventKind::PointerMotion => {
            InputEvent::InputPointerMotionEvent(InputPointerMotionEvent {
                kind: binding.store_string("pointerMotion"),
                metadata,
                payload: payload.pointer_motion,
            })
        }
        WindowsInputEventKind::PointerButton => {
            InputEvent::InputPointerButtonEvent(InputPointerButtonEvent {
                kind: binding.store_string("pointerButton"),
                metadata,
                payload: payload.pointer_button,
            })
        }
        WindowsInputEventKind::Scroll => InputEvent::InputScrollEvent(InputScrollEvent {
            kind: binding.store_string("scroll"),
            metadata,
            payload: payload.scroll,
        }),
        WindowsInputEventKind::Touch => InputEvent::InputTouchEvent(InputTouchEvent {
            kind: binding.store_string("touch"),
            metadata,
            payload: payload.touch,
        }),
        WindowsInputEventKind::Gamepad => InputEvent::InputGamepadEvent(InputGamepadEvent {
            kind: binding.store_string("gamepad"),
            metadata,
            payload: payload.gamepad,
        }),
        WindowsInputEventKind::Text => InputEvent::InputTextEvent(InputTextEvent {
            kind: binding.store_string("text"),
            metadata,
            payload: payload.text,
        }),
        WindowsInputEventKind::Device => InputEvent::InputDeviceEvent(InputDeviceEvent {
            kind: binding.store_string("device"),
            metadata,
            payload: payload.device,
        }),
        WindowsInputEventKind::Sensor => InputEvent::InputSensorEvent(InputSensorEvent {
            kind: binding.store_string("sensor"),
            metadata,
            payload: payload.sensor,
        }),
        WindowsInputEventKind::Composition => {
            InputEvent::InputCompositionEvent(InputCompositionEvent {
                kind: binding.store_string("composition"),
                metadata,
                payload: payload.composition,
            })
        }
    }
}

/// Set one sequence number on one windows input event.
pub(super) fn set_input_event_sequence(event: &mut InputEvent, sequence: u64) {
    match event {
        InputEvent::InputCompositionEvent(value) => value.metadata.sequence = sequence,
        InputEvent::InputDeviceEvent(value) => value.metadata.sequence = sequence,
        InputEvent::InputGamepadEvent(value) => value.metadata.sequence = sequence,
        InputEvent::InputKeyEvent(value) => value.metadata.sequence = sequence,
        InputEvent::InputPointerButtonEvent(value) => value.metadata.sequence = sequence,
        InputEvent::InputPointerMotionEvent(value) => value.metadata.sequence = sequence,
        InputEvent::InputScrollEvent(value) => value.metadata.sequence = sequence,
        InputEvent::InputSensorEvent(value) => value.metadata.sequence = sequence,
        InputEvent::InputTextEvent(value) => value.metadata.sequence = sequence,
        InputEvent::InputTouchEvent(value) => value.metadata.sequence = sequence,
    }
}

/// Resolved backend kind for one opened windows input handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WindowsInputBackend {
    /// Console input queue backend.
    Console,
    /// Native Win32 window-hosted text backend.
    Window,
    /// Per-device raw-input backend routed through the worker service.
    RawDevice,
    /// XInput gamepad backend.
    XInput,
}

/// Normalized input-open selector parsed from one identifier.
#[derive(Debug, Clone)]
pub(super) enum WindowsInputOpenSpec {
    /// Open console input.
    Console,
    /// Open one enumerated raw-input device.
    RawDevice(raw_input::RawInputDeviceDescriptor),
    /// Open one xinput gamepad endpoint.
    XInput(u8),
}

/// Payload stored in the resource table for one opened windows input handle.
#[derive(Debug)]
pub(super) struct WindowsInputBinding {
    /// Active backend kind.
    pub(super) backend: WindowsInputBackend,
    /// Current read-mode selection.
    pub(super) read_mode: InputReadMode,
    /// Next per-handle event sequence number.
    pub(super) next_sequence: u64,
    /// Last observed console button-state bitmask.
    pub(super) console_button_state: u32,
    /// Pending pointer-button transitions split from one console record.
    pub(super) pending_console_button_transitions: VecDeque<PendingConsoleButtonTransition>,
    /// Pending console records queued for input.read calls.
    pub(super) pending_console_records: VecDeque<PendingConsoleRecord>,
    /// Original console mode captured at open time.
    pub(super) original_mode: Option<u32>,
    /// Raw-input descriptor for per-device backends.
    pub(super) raw_device: Option<raw_input::RawInputDeviceDescriptor>,
    /// XInput user index for gamepad backends.
    pub(super) xinput_user_index: Option<u8>,
    /// Last observed XInput packet number.
    pub(super) xinput_packet_number: u32,
    /// Optional player-index override set through control bindings.
    pub(super) xinput_player_index_override: Option<u8>,
    /// Last sampled pointer x position.
    pub(super) last_pointer_x: f64,
    /// Last sampled pointer y position.
    pub(super) last_pointer_y: f64,
    /// Whether relative pointer mode is enabled.
    pub(super) relative_mode_enabled: bool,
    /// Whether text input is currently active.
    pub(super) text_active: bool,
    /// Current text input type selection.
    pub(super) text_input_type: InputTextInputType,
    /// Current text-area hint for IME placement when one host hint exists.
    pub(super) text_area: Option<InputTextGeometry>,
    /// Enabled sensor stream kinds for this opened handle.
    pub(super) sensor_enabled_kinds: HashSet<InputSensorKind>,
    /// Effective sensor stream configurations for this opened handle.
    pub(super) sensor_effective_configs: HashMap<InputSensorKind, InputSensorEffectiveConfig>,
}

/// Deferred console pointer-button transition queued for later reads.
#[derive(Debug, Clone, Copy)]
pub(super) struct PendingConsoleButtonTransition {
    /// Event timestamp in monotonic nanoseconds.
    pub(super) timestamp_ns: u64,
    /// Stable pointer button code.
    pub(super) code: u32,
    /// Pointer action.
    pub(super) action: InputEventAction,
    /// Scalar value for press, release, or repeat transitions.
    pub(super) value: i64,
    /// Pointer x position snapshot.
    pub(super) x: f64,
    /// Pointer y position snapshot.
    pub(super) y: f64,
    /// Modifier bitset snapshot.
    pub(super) modifiers: u32,
}

/// Deferred console record queued for input-event decoding.
pub(super) struct PendingConsoleRecord {
    /// Event timestamp in monotonic nanoseconds.
    pub(super) timestamp_ns: u64,
    /// Host INPUT_RECORD payload.
    pub(super) record: INPUT_RECORD,
}

impl std::fmt::Debug for PendingConsoleRecord {
    /// Format queued console metadata without dumping non-debug host unions.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PendingConsoleRecord")
            .field("timestamp_ns", &self.timestamp_ns)
            .field("event_type", &(self.record.EventType as u32))
            .finish()
    }
}

/// Finalizer payload for console-backed windows input resources.
#[derive(Debug)]
pub(super) struct WindowsInputFinalizer {
    /// Duplicated console input handle.
    pub(super) handle: HANDLE,
    /// Console mode snapshot to restore on close.
    pub(super) restore_mode: Option<u32>,
    /// Whether this finalizer releases the singleton console stream lane.
    pub(super) release_console_lane: bool,
}

/// Resolved input-handle state for one read or control operation.
#[derive(Debug, Clone)]
pub(super) struct WindowsInputResolved {
    /// Backend kind.
    pub(super) backend: WindowsInputBackend,
    /// Active read mode.
    pub(super) read_mode: InputReadMode,
    /// Last observed console button-state bitmask.
    pub(super) console_button_state: u32,
    /// Host handle when this backend is descriptor-backed.
    pub(super) host_handle: Option<HANDLE>,
    /// Original mode snapshot for console backends.
    pub(super) original_mode: Option<u32>,
    /// Raw-input descriptor for per-device backends.
    pub(super) raw_device: Option<raw_input::RawInputDeviceDescriptor>,
    /// XInput user index for gamepad backends.
    pub(super) xinput_user_index: Option<u8>,
    /// Last observed XInput packet number.
    pub(super) xinput_packet_number: u32,
    /// Optional player-index override set through control bindings.
    pub(super) xinput_player_index_override: Option<u8>,
    /// Last sampled pointer x position.
    pub(super) last_pointer_x: f64,
    /// Last sampled pointer y position.
    pub(super) last_pointer_y: f64,
    /// Whether relative pointer mode is enabled.
    pub(super) relative_mode_enabled: bool,
    /// Whether text input is currently active.
    pub(super) text_active: bool,
    /// Current text input type selection.
    pub(super) text_input_type: InputTextInputType,
    /// Current text-area hint for IME placement when one host hint exists.
    pub(super) text_area: Option<InputTextGeometry>,
}

impl ResourceFinalizer for WindowsInputFinalizer {
    /// Restore console mode and close the duplicated input handle.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        if let Some(mode) = self.restore_mode {
            unsafe {
                SetConsoleMode(self.handle, mode);
            }
        }

        unsafe {
            CloseHandle(self.handle);
        }

        if self.release_console_lane {
            WINDOWS_CONSOLE_STREAMS.fetch_sub(1, Ordering::AcqRel);
        }
    }
}

/// Finalizer payload for one raw-input stream registration.
#[derive(Debug)]
pub(super) struct RawInputDeviceFinalizer {
    /// Registered raw-input device identifier.
    pub(super) device_id: String,
    /// Runtime-owned raw input state for queue cleanup.
    pub(super) runtime_state: Arc<raw_input::WindowsRawInputRuntimeState>,
}

impl ResourceFinalizer for RawInputDeviceFinalizer {
    /// Release one registered raw-input stream on resource finalization.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        raw_input::release_input_stream(&self.runtime_state, &self.device_id);
    }
}

/// Build io-not-found for one missing input device handle.
pub(super) fn input_not_found(
    operation: &'static str,
    handle: resource::InputDeviceHandle,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(operation.to_string()),
        None,
        format!("input device handle {} not found", handle.0.local_id),
    ))
    .boxed()
}

/// Build io-would-block for one empty input queue read.
pub(super) fn io_would_block(operation: &'static str, message: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoWouldBlock),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.to_string(),
    ))
    .boxed()
}

/// Build one windows io error with mapped platform error code.
pub(super) fn io_error_with_code(
    operation: &'static str,
    syscall: &'static str,
    code: u32,
    message: &str,
) -> Box<RuntimeError> {
    // map known Win32 codes to stable platform error categories
    let platform_code = if code == ERROR_INVALID_HANDLE
        || code == ERROR_INVALID_PARAMETER
        || code == ERROR_DEVICE_NOT_CONNECTED
    {
        Some(PlatformErrorCode::IoNotFound)
    } else if code == ERROR_ACCESS_DENIED {
        Some(PlatformErrorCode::IoPermissionDenied)
    } else {
        None
    };

    RuntimeError::from(PlatformError::io_with(
        platform_code,
        None,
        Some(code as i32),
        Some(operation.to_string()),
        None,
        format!("{syscall} failed: {message} ({code})"),
    ))
    .boxed()
}

/// Return one current pointer position in desktop coordinates.
pub(super) fn current_pointer_position() -> (f64, f64) {
    let mut point = POINT { x: 0, y: 0 };
    let status = unsafe { GetCursorPos(&mut point) };
    if status == 0 {
        return (0.0, 0.0);
    }

    (point.x as f64, point.y as f64)
}

/// Return whether one virtual key is currently pressed.
pub(super) fn is_key_pressed(virtual_key: i32) -> bool {
    let key_state = unsafe { GetAsyncKeyState(virtual_key) as u16 };
    (key_state & 0x8000) != 0
}

/// Return one stable pointer-button bitset from host key state.
pub(super) fn pointer_buttons_from_host() -> u32 {
    let mut buttons = 0u32;
    if is_key_pressed(i32::from(VK_LBUTTON)) {
        buttons |= POINTER_BUTTON_LEFT;
    }
    if is_key_pressed(i32::from(VK_RBUTTON)) {
        buttons |= POINTER_BUTTON_RIGHT;
    }
    if is_key_pressed(i32::from(VK_MBUTTON)) {
        buttons |= POINTER_BUTTON_MIDDLE;
    }
    if is_key_pressed(i32::from(VK_XBUTTON1)) {
        buttons |= POINTER_BUTTON_X1;
    }
    if is_key_pressed(i32::from(VK_XBUTTON2)) {
        buttons |= POINTER_BUTTON_X2;
    }

    buttons
}

/// Return one CONTROL_KEY_STATE-compatible modifier bitset from host key state.
pub(super) fn modifier_bits_from_host() -> u32 {
    let mut modifiers = 0u32;
    if is_key_pressed(i32::from(VK_SHIFT)) {
        modifiers |= SHIFT_PRESSED;
    }
    if is_key_pressed(i32::from(VK_CONTROL)) {
        modifiers |= LEFT_CTRL_PRESSED;
    }
    if is_key_pressed(i32::from(VK_MENU)) {
        modifiers |= LEFT_ALT_PRESSED;
    }

    let caps_lock = unsafe { GetKeyState(i32::from(VK_CAPITAL)) as u16 };
    if (caps_lock & 0x0001) != 0 {
        modifiers |= CAPSLOCK_ON;
    }

    let num_lock = unsafe { GetKeyState(i32::from(VK_NUMLOCK)) as u16 };
    if (num_lock & 0x0001) != 0 {
        modifiers |= NUMLOCK_ON;
    }

    let scroll_lock = unsafe { GetKeyState(i32::from(VK_SCROLL)) as u16 };
    if (scroll_lock & 0x0001) != 0 {
        modifiers |= SCROLLLOCK_ON;
    }

    modifiers
}

/// Return the stdin console mode when console input is available.
pub(super) fn get_stdin_console_mode() -> RuntimeResult<Option<(HANDLE, u32)>> {
    // resolve standard input handle
    let std_input = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    if std_input == 0 || std_input == INVALID_HANDLE_VALUE {
        return Ok(None);
    }

    // query console input mode or classify non-console handles
    let mut mode = 0u32;
    let mode_status = unsafe { GetConsoleMode(std_input, &mut mode) };
    if mode_status == 0 {
        let code = core_platform::last_error_code() as u32;
        if code == ERROR_INVALID_HANDLE || code == ERROR_INVALID_PARAMETER {
            return Ok(None);
        }

        return Err(core_platform::io_error_with_code(
            "GetConsoleMode",
            code as i32,
        ));
    }

    Ok(Some((std_input, mode)))
}

/// Duplicate one console handle for independent resource ownership.
pub(super) fn duplicate_console_handle(handle: HANDLE) -> RuntimeResult<HANDLE> {
    // duplicate into the current process with matching access rights
    let process = unsafe { GetCurrentProcess() };
    let mut duplicated = 0;
    let status = unsafe {
        DuplicateHandle(
            process,
            handle,
            process,
            &mut duplicated,
            0,
            0,
            DUPLICATE_SAME_ACCESS,
        )
    };
    if status == 0 || duplicated == 0 {
        let code = core_platform::last_error_code() as u32;
        return Err(io_error_with_code(
            "destack.input.device.open",
            "DuplicateHandle",
            code,
            "failed to duplicate console input handle",
        ));
    }

    Ok(duplicated)
}

/// Acquire the singleton console stream lane for this process.
pub(super) fn acquire_console_stream(operation: &'static str) -> RuntimeResult<()> {
    let mut current = WINDOWS_CONSOLE_STREAMS.load(Ordering::Acquire);
    loop {
        if current > 0 {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::IoWouldBlock),
                None,
                None,
                Some(operation.to_string()),
                None,
                "console input stream is already open".to_string(),
            ))
            .boxed());
        }

        match WINDOWS_CONSOLE_STREAMS.compare_exchange_weak(
            current,
            current + 1,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => return Ok(()),
            Err(next) => current = next,
        }
    }
}

/// Normalize one input identifier into a known windows open spec.
pub(super) fn normalize_input_id(id: &str) -> RuntimeResult<WindowsInputOpenSpec> {
    // validate basic identifier invariants
    if id.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "id cannot be empty",
        ))
        .boxed());
    }

    if id.contains('\0') {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "id contains nul byte",
        ))
        .boxed());
    }

    // classify known console aliases
    let id_lower = id.to_ascii_lowercase();
    if id_lower == WINDOWS_INPUT_DEVICE_ID
        || id_lower == WINDOWS_INPUT_DEVICE_ID_ALIAS
        || id_lower == WINDOWS_INPUT_DEVICE_ID_STDIN
        || id_lower == WINDOWS_INPUT_DEVICE_ID_PATH.to_ascii_lowercase()
    {
        return Ok(WindowsInputOpenSpec::Console);
    }

    // classify xinput gamepad identifiers
    if let Some(user_index) = xinput_input::parse_xinput_device_id(&id_lower) {
        return Ok(WindowsInputOpenSpec::XInput(user_index));
    }

    // classify one enumerated raw-input device id
    let raw_device = raw_input::resolve_raw_input_device(&id_lower, "destack.input.device.open")?;
    if let Some(raw_device) = raw_device {
        return Ok(WindowsInputOpenSpec::RawDevice(raw_device));
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "id",
        "id must be one supported windows input identifier from input.list",
    ))
    .boxed())
}

/// Derive runtime read mode from one console mode bitset.
pub(super) fn read_mode_from_console_mode(mode: u32) -> InputReadMode {
    let is_cooked = (mode & ENABLE_PROCESSED_INPUT) != 0
        && (mode & ENABLE_LINE_INPUT) != 0
        && (mode & ENABLE_ECHO_INPUT) != 0;
    if is_cooked {
        InputReadMode::Cooked
    } else {
        InputReadMode::Raw
    }
}

/// Resolve one windows input handle from the resource table.
pub(super) fn resolve_input(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<WindowsInputResolved> {
    // resolve and validate resource entry shape
    let resolved = binding.worker().resources.with_entry(handle.0, |entry| {
        if entry.kind != ResourceKind::InputDevice {
            return None;
        }

        if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
            return None;
        }

        let resolved_binding = entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<WindowsInputBinding>())?;

        Some(WindowsInputResolved {
            backend: resolved_binding.backend,
            read_mode: resolved_binding.read_mode,
            console_button_state: resolved_binding.console_button_state,
            host_handle: entry.handle().map(|handle| handle as HANDLE),
            original_mode: resolved_binding.original_mode,
            raw_device: resolved_binding.raw_device.clone(),
            xinput_user_index: resolved_binding.xinput_user_index,
            xinput_packet_number: resolved_binding.xinput_packet_number,
            xinput_player_index_override: resolved_binding.xinput_player_index_override,
            last_pointer_x: resolved_binding.last_pointer_x,
            last_pointer_y: resolved_binding.last_pointer_y,
            relative_mode_enabled: resolved_binding.relative_mode_enabled,
            text_active: resolved_binding.text_active,
            text_input_type: resolved_binding.text_input_type,
            text_area: resolved_binding.text_area,
        })
    });

    match resolved.flatten() {
        Some(resolved) => Ok(resolved),
        None => Err(input_not_found(operation, handle)),
    }
}

/// Resolve one raw-input descriptor for one opened windows input handle.
pub(super) fn raw_device(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<raw_input::RawInputDeviceDescriptor> {
    let resolved = resolve_input(binding, handle, operation)?;
    if resolved.backend != WindowsInputBackend::RawDevice {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    let Some(raw_device) = resolved.raw_device else {
        return Err(input_not_found(operation, handle));
    };

    Ok(raw_device)
}

/// Allocate the next sequence number for one windows input stream.
pub(super) fn next_sequence(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<u64> {
    let sequence = binding
        .worker()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::InputDevice {
                return None;
            }

            if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
                return None;
            }

            let resolved_binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<WindowsInputBinding>())?;
            let next = resolved_binding.next_sequence;
            resolved_binding.next_sequence = resolved_binding.next_sequence.saturating_add(1);
            Some(next)
        });

    match sequence.flatten() {
        Some(sequence) => Ok(sequence),
        None => Err(input_not_found(operation, handle)),
    }
}

/// Read one monotonic timestamp from the shared runtime clock domain.
pub(super) fn now_timestamp_ns() -> u64 {
    core_platform::monotonic_now_ns()
}

/// Return the configured raw-input queue capacity.
pub(super) fn windows_raw_input_queue_capacity(binding: &BindingCallContext) -> usize {
    binding
        .worker()
        .options
        .input
        .event_queue_capacity
        .and_then(|capacity| usize::try_from(capacity).ok())
        .unwrap_or(raw_input::RAW_INPUT_QUEUE_LIMIT)
        .max(1)
}

/// Return the configured raw monitor queue capacity.
pub(super) fn windows_raw_monitor_queue_capacity(binding: &BindingCallContext) -> usize {
    binding
        .worker()
        .options
        .input
        .event_queue_capacity
        .and_then(|capacity| usize::try_from(capacity).ok())
        .unwrap_or(raw_input::RAW_MONITOR_QUEUE_LIMIT)
        .max(1)
}

/// Return the configured raw HID queue capacity.
pub(super) fn windows_raw_hid_queue_capacity(binding: &BindingCallContext) -> usize {
    binding
        .worker()
        .options
        .input
        .event_queue_capacity
        .and_then(|capacity| usize::try_from(capacity).ok())
        .unwrap_or(raw_input::RAW_HID_QUEUE_LIMIT)
        .max(1)
}

/// Return the configured raw touch queue capacity.
pub(super) fn windows_raw_touch_queue_capacity(binding: &BindingCallContext) -> usize {
    binding
        .worker()
        .options
        .input
        .event_queue_capacity
        .and_then(|capacity| usize::try_from(capacity).ok())
        .unwrap_or(raw_input::RAW_TOUCH_QUEUE_LIMIT)
        .max(1)
}

/// Persist one xinput player-index override for one input handle.
pub(super) fn set_xinput_player_index_override(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    player_index: u8,
    operation: &'static str,
) -> RuntimeResult<()> {
    // reject out-of-range player indices before mutating handle state
    if !(XINPUT_PLAYER_INDEX_MIN..=XINPUT_PLAYER_INDEX_MAX).contains(&player_index) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "playerindex",
            "playerindex must be in range 1..=4",
        ))
        .boxed());
    }

    let updated = binding
        .worker()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::InputDevice {
                return None;
            }

            if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
                return None;
            }

            let resolved_binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<WindowsInputBinding>())?;
            if resolved_binding.backend != WindowsInputBackend::XInput {
                return Some(Err(RuntimeError::from(PlatformError::not_supported(
                    operation,
                ))
                .boxed()));
            }

            resolved_binding.xinput_player_index_override = Some(player_index);
            Some(Ok(()))
        });

    match updated.flatten() {
        Some(result) => result,
        None => Err(input_not_found(operation, handle)),
    }
}

/// Return whether one target selects one explicit window resource.
pub(super) fn has_explicit_window_target(target: InputWindowTarget) -> bool {
    target
        .window
        .is_some_and(|window| window.0.local_id != WINDOW_TARGET_DEFAULT_RESOURCE_ID)
}

/// Build io-not-found for one missing explicit window target handle.
fn window_target_not_found(
    operation: &'static str,
    target: InputWindowTarget,
) -> Box<RuntimeError> {
    let window = target.window.unwrap_or(WindowHandle(ResourceId::local(0)));

    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(operation.to_string()),
        None,
        format!("window handle {} not found", window.0.local_id),
    ))
    .boxed()
}

/// Resolve one optional explicit window target into one host hwnd.
pub(super) fn resolve_window_target_handle(
    binding: &BindingCallContext,
    target: InputWindowTarget,
    operation: &'static str,
) -> RuntimeResult<Option<HWND>> {
    // keep default process-scoped routing for zero-valued targets
    if !has_explicit_window_target(target) {
        return Ok(None);
    }

    // resolve explicit window resources from the shared resource table
    let window_resource_id = target
        .window
        .expect("explicit window targets should carry one handle")
        .0;
    let hwnd = binding
        .worker()
        .resources
        .with_entry(window_resource_id, |entry| {
            if entry.kind != ResourceKind::Window {
                return None;
            }

            entry.handle().map(|handle| handle as HWND)
        });

    match hwnd.flatten() {
        Some(hwnd) if hwnd != 0 => Ok(Some(hwnd)),
        _ => Err(window_target_not_found(operation, target)),
    }
}

/// Convert one client-area point into one screen-space point for cursor APIs.
pub(super) fn client_to_screen_point(
    hwnd: HWND,
    x: f64,
    y: f64,
    operation: &'static str,
) -> RuntimeResult<POINT> {
    // validate finite client-space coordinates before conversion
    if !x.is_finite() || !y.is_finite() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "x",
            "x and y must be finite",
        ))
        .boxed());
    }

    // convert window-relative coordinates to desktop coordinates
    let mut point = POINT {
        x: x.round() as i32,
        y: y.round() as i32,
    };
    let status = unsafe { ClientToScreen(hwnd, &mut point) };
    if status == 0 {
        let code = core_platform::last_error_code() as u32;
        return Err(io_error_with_code(
            operation,
            "ClientToScreen",
            code,
            "failed to convert client coordinates into screen coordinates",
        ));
    }

    Ok(point)
}

/// Confine cursor movement to one explicit window target.
pub(super) fn confine_cursor_to_window(hwnd: HWND, operation: &'static str) -> RuntimeResult<()> {
    // resolve the client rectangle for this window target
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let status = unsafe { GetClientRect(hwnd, &mut rect) };
    if status == 0 {
        let code = core_platform::last_error_code() as u32;
        return Err(io_error_with_code(
            operation,
            "GetClientRect",
            code,
            "failed to resolve window client bounds",
        ));
    }

    // convert client bounds into desktop clip bounds
    let top_left =
        client_to_screen_point(hwnd, f64::from(rect.left), f64::from(rect.top), operation)?;
    let bottom_right = client_to_screen_point(
        hwnd,
        f64::from(rect.right),
        f64::from(rect.bottom),
        operation,
    )?;
    rect.left = top_left.x;
    rect.top = top_left.y;
    rect.right = bottom_right.x;
    rect.bottom = bottom_right.y;

    // apply system-level cursor clipping
    let status = unsafe { ClipCursor(&rect) };
    if status == 0 {
        let code = core_platform::last_error_code() as u32;
        return Err(io_error_with_code(
            operation,
            "ClipCursor",
            code,
            "failed to confine cursor to the target window",
        ));
    }

    Ok(())
}

/// Release any active cursor confinement clip region.
pub(super) fn release_cursor_confine(operation: &'static str) -> RuntimeResult<()> {
    let status = unsafe { ClipCursor(std::ptr::null()) };
    if status == 0 {
        let code = core_platform::last_error_code() as u32;
        return Err(io_error_with_code(
            operation,
            "ClipCursor",
            code,
            "failed to release cursor confinement",
        ));
    }

    Ok(())
}

/// Persist one pointer-position snapshot for one input handle.
pub(super) fn set_pointer_snapshot(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    x: f64,
    y: f64,
    operation: &'static str,
) -> RuntimeResult<()> {
    let updated = binding
        .worker()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::InputDevice {
                return None;
            }

            if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
                return None;
            }

            let resolved_binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<WindowsInputBinding>())?;
            resolved_binding.last_pointer_x = x;
            resolved_binding.last_pointer_y = y;
            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(input_not_found(operation, handle)),
    }
}

/// Persist one relative-mode flag for one input handle.
pub(super) fn set_relative_mode_flag(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    enabled: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    let updated = binding
        .worker()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::InputDevice {
                return None;
            }

            if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
                return None;
            }

            let resolved_binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<WindowsInputBinding>())?;
            resolved_binding.relative_mode_enabled = enabled;
            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(input_not_found(operation, handle)),
    }
}

/// Persist one sensor-stream enabled flag for one input handle and sensor lane.
pub(super) fn set_sensor_stream_enabled(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    sensor_kind: InputSensorKind,
    enabled: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    let updated = binding
        .worker()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::InputDevice {
                return None;
            }

            if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
                return None;
            }

            let resolved_binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<WindowsInputBinding>())?;
            if enabled {
                resolved_binding.sensor_enabled_kinds.insert(sensor_kind);
            } else {
                resolved_binding.sensor_enabled_kinds.remove(&sensor_kind);
            }

            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(input_not_found(operation, handle)),
    }
}

/// Persist one effective sensor-stream configuration for one input handle and sensor lane.
pub(super) fn set_sensor_stream_config(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    sensor_kind: InputSensorKind,
    config: InputSensorEffectiveConfig,
    operation: &'static str,
) -> RuntimeResult<()> {
    // update stream-enabled state before storing effective configuration
    set_sensor_stream_enabled(binding, handle, sensor_kind, config.enabled, operation)?;

    let updated = binding
        .worker()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::InputDevice {
                return None;
            }

            if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
                return None;
            }

            let resolved_binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<WindowsInputBinding>())?;
            if config.enabled {
                resolved_binding
                    .sensor_effective_configs
                    .insert(sensor_kind, config);
            } else {
                resolved_binding
                    .sensor_effective_configs
                    .remove(&sensor_kind);
            }

            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(input_not_found(operation, handle)),
    }
}

/// Return whether one sensor stream is currently enabled for one input handle.
pub(super) fn is_sensor_stream_enabled(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    sensor_kind: InputSensorKind,
    operation: &'static str,
) -> RuntimeResult<bool> {
    let enabled = binding.worker().resources.with_entry(handle.0, |entry| {
        if entry.kind != ResourceKind::InputDevice {
            return None;
        }

        if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
            return None;
        }

        let resolved_binding = entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<WindowsInputBinding>())?;
        Some(resolved_binding.sensor_enabled_kinds.contains(&sensor_kind))
    });

    match enabled.flatten() {
        Some(enabled) => Ok(enabled),
        None => Err(input_not_found(operation, handle)),
    }
}

#[cfg(test)]
mod tests {
    use crate::platform::input::InputReadMode;
    use crate::platform::input::host::windows::core::{
        ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT, read_mode_from_console_mode,
    };

    /// Detect cooked mode from the corresponding console mode bits.
    #[test]
    fn test_read_mode_from_console_mode_detects_cooked_bits() {
        let mode = ENABLE_PROCESSED_INPUT | ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT;
        let read_mode = read_mode_from_console_mode(mode);
        assert_eq!(read_mode, InputReadMode::Cooked);
    }

    /// Detect raw mode when cooked-mode bits are incomplete.
    #[test]
    fn test_read_mode_from_console_mode_detects_raw_bits() {
        let mode = ENABLE_PROCESSED_INPUT;
        let read_mode = read_mode_from_console_mode(mode);
        assert_eq!(read_mode, InputReadMode::Raw);
    }
}
