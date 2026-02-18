use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use windows_sys::Win32::Foundation::{
    CloseHandle, DUPLICATE_SAME_ACCESS, DuplicateHandle, ERROR_ACCESS_DENIED, ERROR_INVALID_HANDLE,
    ERROR_INVALID_PARAMETER, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::System::Console::{
    ENABLE_ECHO_INPUT, ENABLE_EXTENDED_FLAGS, ENABLE_LINE_INPUT, ENABLE_MOUSE_INPUT,
    ENABLE_PROCESSED_INPUT, ENABLE_QUICK_EDIT_MODE, ENABLE_WINDOW_INPUT, FOCUS_EVENT,
    FROM_LEFT_1ST_BUTTON_PRESSED, FROM_LEFT_2ND_BUTTON_PRESSED, FROM_LEFT_3RD_BUTTON_PRESSED,
    FROM_LEFT_4TH_BUTTON_PRESSED, GetConsoleMode, GetNumberOfConsoleInputEvents, GetStdHandle,
    INPUT_RECORD, KEY_EVENT, MENU_EVENT, MOUSE_EVENT, MOUSE_HWHEELED, MOUSE_MOVED, MOUSE_WHEELED,
    RIGHTMOST_BUTTON_PRESSED, ReadConsoleInputW, STD_INPUT_HANDLE, SetConsoleMode,
    WINDOW_BUFFER_SIZE_EVENT,
};
use windows_sys::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

use super::raw as raw_input;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{
    InputDeviceInfo, InputDeviceKind, InputEvent, InputEventAction, InputEventKind, InputReadMode,
};
use crate::platform::resource::{ResourceEntry, ResourceFinalizer, ResourceId, ResourceKind};
use crate::platform::{PlatformError, core as core_platform, resource};
use crate::runtime::RuntimeCallContext;

/// Resource-table label for opened input-device entries.
pub(super) const INPUT_RESOURCE_LABEL: &str = "input.device";
/// Stable console input device identifier.
const WINDOWS_INPUT_DEVICE_ID: &str = "console:stdin";
/// Console identifier alias accepted by open.
const WINDOWS_INPUT_DEVICE_ID_ALIAS: &str = "console";
/// Stdin identifier alias accepted by open.
const WINDOWS_INPUT_DEVICE_ID_STDIN: &str = "stdin";
/// Win32 console input pseudo-path accepted by open.
const WINDOWS_INPUT_DEVICE_ID_PATH: &str = "\\\\.\\CONIN$";
/// Display name for console input device metadata.
const WINDOWS_INPUT_DEVICE_NAME: &str = "windows console input";
/// Reported key count for console input.
const WINDOWS_CONSOLE_KEY_COUNT: u16 = 255;
/// Reported button count for console input.
const WINDOWS_CONSOLE_BUTTON_COUNT: u16 = 5;
/// Reported axis count for console input.
const WINDOWS_CONSOLE_AXIS_COUNT: u16 = 2;
/// Stable raw keyboard pseudo-device identifier.
pub(super) const WINDOWS_INPUT_RAW_KEYBOARD_ID: &str = "raw:keyboard";
/// Stable raw mouse pseudo-device identifier.
pub(super) const WINDOWS_INPUT_RAW_MOUSE_ID: &str = "raw:mouse";
/// Display name for raw keyboard pseudo-device metadata.
const WINDOWS_INPUT_RAW_KEYBOARD_NAME: &str = "windows raw keyboard state";
/// Display name for raw mouse pseudo-device metadata.
const WINDOWS_INPUT_RAW_MOUSE_NAME: &str = "windows raw mouse state";
/// Empty text payload for non-text events.
const WINDOWS_INPUT_EMPTY_TEXT: &str = "";
/// Singleton counter for active raw keyboard streams.
static WINDOWS_RAW_KEYBOARD_STREAMS: AtomicUsize = AtomicUsize::new(0);
/// Singleton counter for active raw mouse streams.
static WINDOWS_RAW_MOUSE_STREAMS: AtomicUsize = AtomicUsize::new(0);

/// Resolved backend kind for one opened windows input handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowsInputBackend {
    /// Console input queue backend.
    Console,
    /// Raw keyboard backend routed through the worker service.
    RawKeyboard,
    /// Raw mouse backend routed through the worker service.
    RawMouse,
}

/// Normalized input-open selector parsed from one identifier.
#[derive(Debug, Clone, Copy)]
enum WindowsInputOpenSpec {
    /// Open console input.
    Console,
    /// Open raw keyboard stream.
    RawKeyboard,
    /// Open raw mouse stream.
    RawMouse,
}

/// Payload stored in the resource table for one opened windows input handle.
#[derive(Debug)]
pub(super) struct WindowsInputBinding {
    /// Active backend kind.
    backend: WindowsInputBackend,
    /// Current read-mode selection.
    read_mode: InputReadMode,
    /// Next per-handle event sequence number.
    next_sequence: u64,
    /// Last observed console button-state bitmask.
    console_button_state: u32,
    /// Original console mode captured at open time.
    original_mode: Option<u32>,
}

/// Finalizer payload for console-backed windows input resources.
#[derive(Debug)]
pub(super) struct WindowsInputFinalizer {
    /// Duplicated console input handle.
    pub(super) handle: HANDLE,
    /// Console mode snapshot to restore on close.
    pub(super) restore_mode: Option<u32>,
}

/// Resolved input-handle state for one read or control operation.
#[derive(Debug, Clone, Copy)]
struct WindowsInputResolved {
    /// Backend kind.
    backend: WindowsInputBackend,
    /// Active read mode.
    read_mode: InputReadMode,
    /// Last observed console button-state bitmask.
    console_button_state: u32,
    /// Host handle when this backend is descriptor-backed.
    host_handle: Option<HANDLE>,
    /// Original mode snapshot for console backends.
    original_mode: Option<u32>,
}

/// Finalizer payload for singleton raw-stream ownership counters.
#[derive(Debug)]
struct RawInputStreamFinalizer {
    /// Counter for one raw stream lane.
    counter: &'static AtomicUsize,
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
    }
}

impl ResourceFinalizer for RawInputStreamFinalizer {
    /// Release one raw stream lane on resource finalization.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        self.counter.fetch_sub(1, Ordering::AcqRel);
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
        format!("input device handle {} not found", handle.0.0),
    ))
    .boxed()
}

/// Build io-would-block for one empty input queue read.
fn io_would_block(operation: &'static str, message: &'static str) -> Box<RuntimeError> {
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

/// Acquire one singleton raw input stream lane.
fn acquire_raw_stream(
    counter: &'static AtomicUsize,
    operation: &'static str,
    stream_name: &'static str,
) -> RuntimeResult<()> {
    let mut current = counter.load(Ordering::Acquire);
    loop {
        if current > 0 {
            return Err(io_would_block(
                operation,
                match stream_name {
                    "keyboard" => "raw keyboard stream is already open",
                    "mouse" => "raw mouse stream is already open",
                    _ => "raw stream is already open",
                },
            ));
        }

        match counter.compare_exchange_weak(
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

/// Build one windows io error with mapped platform error code.
fn io_error_with_code(
    operation: &'static str,
    syscall: &'static str,
    code: u32,
    message: &str,
) -> Box<RuntimeError> {
    // map known Win32 codes to stable platform error categories
    let platform_code = if code == ERROR_INVALID_HANDLE || code == ERROR_INVALID_PARAMETER {
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

/// Return the stdin console mode when console input is available.
fn get_stdin_console_mode() -> RuntimeResult<Option<(HANDLE, u32)>> {
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
fn duplicate_console_handle(handle: HANDLE) -> RuntimeResult<HANDLE> {
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

/// Normalize one input identifier into a known windows open spec.
fn normalize_windows_input_id(id: &str) -> RuntimeResult<WindowsInputOpenSpec> {
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

    // classify known raw pseudo-devices
    if id_lower == WINDOWS_INPUT_RAW_KEYBOARD_ID {
        return Ok(WindowsInputOpenSpec::RawKeyboard);
    }

    if id_lower == WINDOWS_INPUT_RAW_MOUSE_ID {
        return Ok(WindowsInputOpenSpec::RawMouse);
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "id",
        "id must be one supported windows input identifier",
    ))
    .boxed())
}

/// Derive runtime read mode from one console mode bitset.
fn read_mode_from_console_mode(mode: u32) -> InputReadMode {
    let is_cooked = (mode & ENABLE_PROCESSED_INPUT) != 0
        && (mode & ENABLE_LINE_INPUT) != 0
        && (mode & ENABLE_ECHO_INPUT) != 0;
    if is_cooked {
        InputReadMode::Cooked
    } else {
        InputReadMode::Raw
    }
}

/// Enumerate windows input devices and pseudo-devices.
pub(super) fn list_windows_devices(
    context: &RuntimeCallContext,
) -> RuntimeResult<Vec<InputDeviceInfo>> {
    // append console input when available for this process
    let mut devices = Vec::new();

    if get_stdin_console_mode()?.is_some() {
        devices.push(InputDeviceInfo {
            id: context.store_string(WINDOWS_INPUT_DEVICE_ID),
            name: context.store_string(WINDOWS_INPUT_DEVICE_NAME),
            kind: InputDeviceKind::Keyboard,
            vendor_id: 0,
            product_id: 0,
            key_count: WINDOWS_CONSOLE_KEY_COUNT,
            button_count: WINDOWS_CONSOLE_BUTTON_COUNT,
            axis_count: WINDOWS_CONSOLE_AXIS_COUNT,
            connected: true,
            supports_grab: true,
            supports_raw: true,
            supports_text: true,
            supports_rumble: false,
        });
    }

    // append raw keyboard pseudo-device
    devices.push(InputDeviceInfo {
        id: context.store_string(WINDOWS_INPUT_RAW_KEYBOARD_ID),
        name: context.store_string(WINDOWS_INPUT_RAW_KEYBOARD_NAME),
        kind: InputDeviceKind::Keyboard,
        vendor_id: 0,
        product_id: 0,
        key_count: 255,
        button_count: 0,
        axis_count: 0,
        connected: true,
        supports_grab: false,
        supports_raw: true,
        supports_text: false,
        supports_rumble: false,
    });

    // append raw mouse pseudo-device
    devices.push(InputDeviceInfo {
        id: context.store_string(WINDOWS_INPUT_RAW_MOUSE_ID),
        name: context.store_string(WINDOWS_INPUT_RAW_MOUSE_NAME),
        kind: InputDeviceKind::Mouse,
        vendor_id: 0,
        product_id: 0,
        key_count: 0,
        button_count: 5,
        axis_count: 2,
        connected: true,
        supports_grab: false,
        supports_raw: true,
        supports_text: false,
        supports_rumble: false,
    });

    Ok(devices)
}

/// Open one windows input endpoint by identifier.
pub(super) fn open_windows_device(
    context: &RuntimeCallContext,
    id: &str,
) -> RuntimeResult<resource::InputDeviceHandle> {
    // normalize the input identifier into one backend selector
    let spec = normalize_windows_input_id(id)?;

    match spec {
        WindowsInputOpenSpec::Console => {
            // resolve and duplicate console input for resource ownership
            let Some((stdin, mode)) = get_stdin_console_mode()? else {
                return Err(RuntimeError::from(PlatformError::io_with(
                    Some(PlatformErrorCode::IoNotFound),
                    None,
                    None,
                    Some("destack.input.device.open".to_string()),
                    None,
                    "console input is not available for this process",
                ))
                .boxed());
            };
            let duplicated = duplicate_console_handle(stdin)?;

            // insert console binding with restore finalizer
            let entry = ResourceEntry::new(ResourceKind::Input)
                .with_label(INPUT_RESOURCE_LABEL)
                .with_handle(duplicated as usize as *mut c_void)
                .with_payload(WindowsInputBinding {
                    backend: WindowsInputBackend::Console,
                    read_mode: read_mode_from_console_mode(mode),
                    next_sequence: 1,
                    console_button_state: 0,
                    original_mode: Some(mode),
                })
                .with_finalizer(WindowsInputFinalizer {
                    handle: duplicated,
                    restore_mode: Some(mode),
                });
            let resource_id = context.runtime().resources.insert(entry);
            Ok(resource::InputDeviceHandle(resource_id))
        }
        WindowsInputOpenSpec::RawKeyboard => {
            // ensure raw worker availability before creating handle
            raw_input::ensure_service("destack.input.device.open")?;
            acquire_raw_stream(
                &WINDOWS_RAW_KEYBOARD_STREAMS,
                "destack.input.device.open",
                "keyboard",
            )?;
            let entry = ResourceEntry::new(ResourceKind::Input)
                .with_label(INPUT_RESOURCE_LABEL)
                .with_payload(WindowsInputBinding {
                    backend: WindowsInputBackend::RawKeyboard,
                    read_mode: InputReadMode::Raw,
                    next_sequence: 1,
                    console_button_state: 0,
                    original_mode: None,
                })
                .with_finalizer(RawInputStreamFinalizer {
                    counter: &WINDOWS_RAW_KEYBOARD_STREAMS,
                });
            let resource_id = context.runtime().resources.insert(entry);
            Ok(resource::InputDeviceHandle(resource_id))
        }
        WindowsInputOpenSpec::RawMouse => {
            // ensure raw worker availability before creating handle
            raw_input::ensure_service("destack.input.device.open")?;
            acquire_raw_stream(
                &WINDOWS_RAW_MOUSE_STREAMS,
                "destack.input.device.open",
                "mouse",
            )?;
            let entry = ResourceEntry::new(ResourceKind::Input)
                .with_label(INPUT_RESOURCE_LABEL)
                .with_payload(WindowsInputBinding {
                    backend: WindowsInputBackend::RawMouse,
                    read_mode: InputReadMode::Raw,
                    next_sequence: 1,
                    console_button_state: 0,
                    original_mode: None,
                })
                .with_finalizer(RawInputStreamFinalizer {
                    counter: &WINDOWS_RAW_MOUSE_STREAMS,
                });
            let resource_id = context.runtime().resources.insert(entry);
            Ok(resource::InputDeviceHandle(resource_id))
        }
    }
}

/// Resolve one windows input handle from the resource table.
fn resolve_windows_input(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<WindowsInputResolved> {
    // resolve and validate resource entry shape
    let resolved = context.runtime().resources.with_entry(handle.0, |entry| {
        if entry.kind != ResourceKind::Input {
            return None;
        }

        if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
            return None;
        }

        let binding = entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<WindowsInputBinding>())?;

        Some(WindowsInputResolved {
            backend: binding.backend,
            read_mode: binding.read_mode,
            console_button_state: binding.console_button_state,
            host_handle: entry.handle().map(|handle| handle as HANDLE),
            original_mode: binding.original_mode,
        })
    });

    match resolved.flatten() {
        Some(resolved) => Ok(resolved),
        None => Err(input_not_found(operation, handle)),
    }
}

/// Close one windows input handle and run any finalizer.
pub(super) fn close_windows_device(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    // validate handle before attempting removal
    resolve_windows_input(context, handle, operation)?;

    // remove from resource table and run finalizer
    let removed = context.runtime().resources.remove_and_finalize(handle.0);
    if !removed {
        return Err(input_not_found(operation, handle));
    }

    Ok(())
}

/// Allocate the next sequence number for one windows input stream.
fn next_windows_sequence(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<u64> {
    let sequence = context
        .runtime()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::Input {
                return None;
            }

            if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
                return None;
            }

            let binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<WindowsInputBinding>())?;
            let next = binding.next_sequence;
            binding.next_sequence = binding.next_sequence.saturating_add(1);
            Some(next)
        });

    match sequence.flatten() {
        Some(sequence) => Ok(sequence),
        None => Err(input_not_found(operation, handle)),
    }
}

/// Read console queue depth for nonblocking checks.
fn read_queue_depth(handle: HANDLE, operation: &'static str) -> RuntimeResult<u32> {
    let mut queued = 0u32;
    let status = unsafe { GetNumberOfConsoleInputEvents(handle, &mut queued) };
    if status == 0 {
        let code = core_platform::last_error_code() as u32;
        return Err(io_error_with_code(
            operation,
            "GetNumberOfConsoleInputEvents",
            code,
            "failed to query console event queue depth",
        ));
    }

    Ok(queued)
}

/// Return the host performance-counter frequency.
fn performance_counter_frequency() -> u64 {
    static PERFORMANCE_COUNTER_FREQUENCY: OnceLock<u64> = OnceLock::new();
    *PERFORMANCE_COUNTER_FREQUENCY.get_or_init(|| {
        let mut frequency = 0i64;
        let status = unsafe { QueryPerformanceFrequency(&mut frequency) };
        if status == 0 || frequency <= 0 {
            return 0;
        }

        frequency as u64
    })
}

/// Read one monotonic timestamp from QueryPerformanceCounter.
fn now_timestamp_ns() -> u64 {
    let frequency = performance_counter_frequency();
    if frequency == 0 {
        return 0;
    }

    let mut counter = 0i64;
    let status = unsafe { QueryPerformanceCounter(&mut counter) };
    if status == 0 || counter < 0 {
        return 0;
    }

    ((counter as u128).saturating_mul(1_000_000_000u128) / u128::from(frequency)) as u64
}

/// Decode wheel delta from high-word packed button state.
fn mouse_wheel_delta(button_state: u32) -> i16 {
    let high_word = ((button_state >> 16) & 0xffff) as u16;
    high_word as i16
}

/// Decode one console button-state transition into one stable button event.
fn decode_console_button_transition(
    previous_state: u32,
    current_state: u32,
) -> Option<(u32, InputEventAction, i64)> {
    // map console button-bit masks into stable pointer button codes
    const BUTTON_CASES: &[(u32, u32)] = &[
        (FROM_LEFT_1ST_BUTTON_PRESSED, 0),
        (RIGHTMOST_BUTTON_PRESSED, 1),
        (FROM_LEFT_2ND_BUTTON_PRESSED, 2),
        (FROM_LEFT_3RD_BUTTON_PRESSED, 3),
        (FROM_LEFT_4TH_BUTTON_PRESSED, 4),
    ];

    // find the first changed button bit in stable code order
    let changed_bits = previous_state ^ current_state;
    for (button_bit, code) in BUTTON_CASES {
        if (changed_bits & *button_bit) == 0 {
            continue;
        }

        let is_pressed = (current_state & *button_bit) != 0;
        if is_pressed {
            return Some((*code, InputEventAction::Press, 1));
        }

        return Some((*code, InputEventAction::Release, 0));
    }

    None
}

/// Map one INPUT_RECORD into one runtime input event.
fn map_console_record(
    context: &RuntimeCallContext,
    record: INPUT_RECORD,
    device_id: &str,
    read_mode: InputReadMode,
    previous_button_state: u32,
) -> Option<(InputEvent, Option<u32>)> {
    // stamp event with one monotonic timestamp
    let timestamp_ns = now_timestamp_ns();

    match record.EventType as u32 {
        KEY_EVENT => {
            // decode keyboard or cooked text semantics
            let key = unsafe { record.Event.KeyEvent };
            let unicode = unsafe { key.uChar.UnicodeChar };
            let key_down = key.bKeyDown != 0;
            let is_text = key_down && unicode != 0 && read_mode == InputReadMode::Cooked;
            let kind = if is_text {
                InputEventKind::Text
            } else {
                InputEventKind::Key
            };
            let value = if is_text {
                unicode as i64
            } else if key_down {
                1
            } else {
                0
            };

            let action = if is_text {
                InputEventAction::Text
            } else if key_down && key.wRepeatCount > 1 {
                InputEventAction::Repeat
            } else if key_down {
                InputEventAction::Press
            } else {
                InputEventAction::Release
            };
            let text = if is_text {
                let code_unit = [unicode];
                let value = String::from_utf16_lossy(&code_unit);
                context.store_string(&value)
            } else {
                context.store_string(WINDOWS_INPUT_EMPTY_TEXT)
            };

            // build normalized key or text event
            let event = InputEvent {
                kind,
                timestamp_ns,
                sequence: 0,
                device_id: context.store_string(device_id),
                action,
                code: key.wVirtualKeyCode as u32,
                scan_code: key.wVirtualScanCode as u32,
                value,
                x: 0.0,
                y: 0.0,
                wheel_x: 0.0,
                wheel_y: 0.0,
                modifiers: key.dwControlKeyState,
                repeat: key_down && key.wRepeatCount > 1,
                text,
            };

            Some((event, None))
        }
        MOUSE_EVENT => {
            // decode pointer, wheel, and button semantics
            let mouse = unsafe { record.Event.MouseEvent };
            let current_button_state = mouse.dwButtonState;

            let kind =
                if mouse.dwEventFlags == MOUSE_WHEELED || mouse.dwEventFlags == MOUSE_HWHEELED {
                    InputEventKind::Scroll
                } else if mouse.dwEventFlags == MOUSE_MOVED {
                    InputEventKind::PointerMotion
                } else {
                    InputEventKind::PointerButton
                };
            let wheel_x = if mouse.dwEventFlags == MOUSE_HWHEELED {
                f64::from(mouse_wheel_delta(mouse.dwButtonState))
            } else {
                0.0
            };
            let wheel_y = if mouse.dwEventFlags == MOUSE_WHEELED {
                f64::from(mouse_wheel_delta(mouse.dwButtonState))
            } else {
                0.0
            };

            // derive pointer-button transition from the previous and current button bitmasks
            let (code, action, value) = if kind == InputEventKind::PointerButton {
                match decode_console_button_transition(previous_button_state, current_button_state)
                {
                    Some((code, action, value)) => (code, action, value),
                    None => return None,
                }
            } else if kind == InputEventKind::Scroll {
                (
                    mouse.dwEventFlags,
                    InputEventAction::Scroll,
                    i64::from(mouse_wheel_delta(mouse.dwButtonState)),
                )
            } else {
                (mouse.dwEventFlags, InputEventAction::Move, 0)
            };

            // build normalized pointer event
            let event = InputEvent {
                kind,
                timestamp_ns,
                sequence: 0,
                device_id: context.store_string(device_id),
                action,
                code,
                scan_code: code,
                value,
                x: mouse.dwMousePosition.X as f64,
                y: mouse.dwMousePosition.Y as f64,
                wheel_x,
                wheel_y,
                modifiers: mouse.dwControlKeyState,
                repeat: false,
                text: context.store_string(WINDOWS_INPUT_EMPTY_TEXT),
            };

            Some((event, Some(current_button_state)))
        }
        WINDOW_BUFFER_SIZE_EVENT => {
            // emit monitor-style resize notification
            let resize = unsafe { record.Event.WindowBufferSizeEvent };
            let event = InputEvent {
                kind: InputEventKind::Device,
                timestamp_ns,
                sequence: 0,
                device_id: context.store_string(device_id),
                action: InputEventAction::Move,
                code: WINDOW_BUFFER_SIZE_EVENT,
                scan_code: WINDOW_BUFFER_SIZE_EVENT,
                value: 0,
                x: resize.dwSize.X as f64,
                y: resize.dwSize.Y as f64,
                wheel_x: 0.0,
                wheel_y: 0.0,
                modifiers: 0,
                repeat: false,
                text: context.store_string(WINDOWS_INPUT_EMPTY_TEXT),
            };

            Some((event, None))
        }
        // map opaque system events as device notifications
        MENU_EVENT => Some((
            InputEvent {
                kind: InputEventKind::Device,
                timestamp_ns,
                sequence: 0,
                device_id: context.store_string(device_id),
                action: InputEventAction::Move,
                code: MENU_EVENT,
                scan_code: MENU_EVENT,
                value: 0,
                x: 0.0,
                y: 0.0,
                wheel_x: 0.0,
                wheel_y: 0.0,
                modifiers: 0,
                repeat: false,
                text: context.store_string(WINDOWS_INPUT_EMPTY_TEXT),
            },
            None,
        )),
        FOCUS_EVENT => Some((
            InputEvent {
                kind: InputEventKind::Device,
                timestamp_ns,
                sequence: 0,
                device_id: context.store_string(device_id),
                action: InputEventAction::Move,
                code: FOCUS_EVENT,
                scan_code: FOCUS_EVENT,
                value: 0,
                x: 0.0,
                y: 0.0,
                wheel_x: 0.0,
                wheel_y: 0.0,
                modifiers: 0,
                repeat: false,
                text: context.store_string(WINDOWS_INPUT_EMPTY_TEXT),
            },
            None,
        )),
        event_type => Some((
            InputEvent {
                kind: InputEventKind::Device,
                timestamp_ns,
                sequence: 0,
                device_id: context.store_string(device_id),
                action: InputEventAction::Move,
                code: event_type,
                scan_code: event_type,
                value: 0,
                x: 0.0,
                y: 0.0,
                wheel_x: 0.0,
                wheel_y: 0.0,
                modifiers: 0,
                repeat: false,
                text: context.store_string(WINDOWS_INPUT_EMPTY_TEXT),
            },
            None,
        )),
    }
}

/// Persist one console button-state snapshot for one input handle.
fn set_console_button_state(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    state: u32,
    operation: &'static str,
) -> RuntimeResult<()> {
    let updated = context
        .runtime()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::Input {
                return None;
            }

            if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
                return None;
            }

            let binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<WindowsInputBinding>())?;
            if binding.backend != WindowsInputBackend::Console {
                return Some(());
            }

            binding.console_button_state = state;
            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(input_not_found(operation, handle)),
    }
}

/// Read one input event from console or raw queues.
pub(super) fn read_windows_event(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputEvent> {
    // resolve resource binding and selected backend
    let resolved = resolve_windows_input(context, handle, operation)?;

    let (mut event, next_button_state) = match resolved.backend {
        WindowsInputBackend::Console => {
            // require a host handle for console backends
            let Some(host_handle) = resolved.host_handle else {
                return Err(input_not_found(operation, handle));
            };

            // enforce nonblocking queue semantics when requested
            if nonblocking {
                let queued = read_queue_depth(host_handle, operation)?;
                if queued == 0 {
                    return Err(io_would_block(operation, "input queue is empty"));
                }
            }

            // read records until one event payload is produced
            let event = loop {
                let mut record = MaybeUninit::<INPUT_RECORD>::uninit();
                let mut read_count = 0u32;
                let status = unsafe {
                    ReadConsoleInputW(host_handle, record.as_mut_ptr(), 1, &mut read_count)
                };
                if status == 0 {
                    let code = core_platform::last_error_code() as u32;
                    return Err(io_error_with_code(
                        operation,
                        "ReadConsoleInputW",
                        code,
                        "failed to read from console input",
                    ));
                }

                if read_count == 0 {
                    if nonblocking {
                        return Err(io_would_block(operation, "input queue is empty"));
                    }

                    continue;
                }

                // convert one console record into one runtime event
                let record = unsafe { record.assume_init() };
                let mapped = map_console_record(
                    context,
                    record,
                    WINDOWS_INPUT_DEVICE_ID,
                    resolved.read_mode,
                    resolved.console_button_state,
                );
                if let Some(mapped) = mapped {
                    break mapped;
                }
            };

            event
        }
        // delegate raw streams to the dedicated worker queues
        WindowsInputBackend::RawKeyboard => (
            raw_input::read_keyboard_event(context, nonblocking, operation)?,
            None,
        ),
        WindowsInputBackend::RawMouse => (
            raw_input::read_mouse_event(context, nonblocking, operation)?,
            None,
        ),
    };

    // persist updated console button state when one mouse snapshot was observed
    if let Some(next_button_state) = next_button_state {
        set_console_button_state(context, handle, next_button_state, operation)?;
    }

    // stamp one per-handle sequence number
    event.sequence = next_windows_sequence(context, handle, operation)?;

    Ok(event)
}

/// Enable or disable exclusive grab mode for console input.
pub(super) fn set_windows_grab(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    enable: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve handle and enforce console-only grab semantics
    let resolved = resolve_windows_input(context, handle, operation)?;

    if resolved.backend != WindowsInputBackend::Console {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.input.event.setGrab",
        ))
        .boxed());
    }

    let Some(host_handle) = resolved.host_handle else {
        return Err(input_not_found(operation, handle));
    };
    let original_mode = resolved.original_mode.unwrap_or(0);

    // compute next console mode for grab enable or disable
    let mut current_mode = 0u32;
    let status = unsafe { GetConsoleMode(host_handle, &mut current_mode) };
    if status == 0 {
        let code = core_platform::last_error_code() as u32;
        return Err(io_error_with_code(
            operation,
            "GetConsoleMode",
            code,
            "failed to read console input mode",
        ));
    }

    let mode = if enable {
        let mut grabbed_mode = current_mode | ENABLE_EXTENDED_FLAGS | ENABLE_MOUSE_INPUT;
        grabbed_mode |= ENABLE_WINDOW_INPUT;
        grabbed_mode &= !ENABLE_QUICK_EDIT_MODE;
        grabbed_mode
    } else {
        let mut released_mode = current_mode | ENABLE_EXTENDED_FLAGS;
        released_mode &= !ENABLE_MOUSE_INPUT;
        released_mode &= !ENABLE_WINDOW_INPUT;
        released_mode &= !ENABLE_QUICK_EDIT_MODE;

        if (original_mode & ENABLE_MOUSE_INPUT) != 0 {
            released_mode |= ENABLE_MOUSE_INPUT;
        }

        if (original_mode & ENABLE_WINDOW_INPUT) != 0 {
            released_mode |= ENABLE_WINDOW_INPUT;
        }

        if (original_mode & ENABLE_QUICK_EDIT_MODE) != 0 {
            released_mode |= ENABLE_QUICK_EDIT_MODE;
        }

        released_mode
    };

    // apply the selected console mode
    let status = unsafe { SetConsoleMode(host_handle, mode) };
    if status == 0 {
        let code = core_platform::last_error_code() as u32;
        return Err(io_error_with_code(
            operation,
            "SetConsoleMode",
            code,
            "failed to update console input mode",
        ));
    }

    Ok(())
}

/// Set read mode for one windows input handle.
pub(super) fn set_windows_read_mode(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    mode: InputReadMode,
    operation: &'static str,
) -> RuntimeResult<()> {
    // mutate binding state and host mode in one resource-table transaction
    let result = context
        .runtime()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::Input {
                return None;
            }

            if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
                return None;
            }

            let host_handle = entry.handle().map(|handle| handle as HANDLE);
            let binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<WindowsInputBinding>())?;

            let update = match binding.backend {
                WindowsInputBackend::Console => {
                    // update console cooked or raw processing flags
                    let Some(host_handle) = host_handle else {
                        return Some(Err(input_not_found(operation, handle)));
                    };

                    let mut current_mode = 0u32;
                    let status = unsafe { GetConsoleMode(host_handle, &mut current_mode) };
                    if status == 0 {
                        let code = core_platform::last_error_code() as u32;
                        return Some(Err(io_error_with_code(
                            operation,
                            "GetConsoleMode",
                            code,
                            "failed to read console input mode",
                        )));
                    }

                    let mut next_mode = current_mode;
                    if mode == InputReadMode::Cooked {
                        next_mode |= ENABLE_PROCESSED_INPUT;
                        next_mode |= ENABLE_LINE_INPUT;
                        next_mode |= ENABLE_ECHO_INPUT;
                    } else {
                        next_mode &= !ENABLE_PROCESSED_INPUT;
                        next_mode &= !ENABLE_LINE_INPUT;
                        next_mode &= !ENABLE_ECHO_INPUT;
                    }

                    let status = unsafe { SetConsoleMode(host_handle, next_mode) };
                    if status == 0 {
                        let code = core_platform::last_error_code() as u32;
                        return Some(Err(io_error_with_code(
                            operation,
                            "SetConsoleMode",
                            code,
                            "failed to update console input mode",
                        )));
                    }

                    Ok(())
                }
                WindowsInputBackend::RawKeyboard | WindowsInputBackend::RawMouse => {
                    // raw pseudo-devices only support raw mode
                    if mode == InputReadMode::Cooked {
                        Err(RuntimeError::from(PlatformError::not_supported(
                            "destack.input.event.setReadMode",
                        ))
                        .boxed())
                    } else {
                        Ok(())
                    }
                }
            };

            // persist read mode only after host updates succeed
            if update.is_ok() {
                binding.read_mode = mode;
            }

            Some(update)
        });

    // map missing entries to io-not-found
    match result.flatten() {
        Some(result) => result,
        None => Err(input_not_found(operation, handle)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Decode release transitions even when another button remains pressed.
    #[test]
    fn test_decode_console_button_transition_reports_release_with_other_pressed() {
        let previous = FROM_LEFT_1ST_BUTTON_PRESSED | RIGHTMOST_BUTTON_PRESSED;
        let current = RIGHTMOST_BUTTON_PRESSED;
        let transition = decode_console_button_transition(previous, current);
        let transition = transition.expect("transition should be detected");
        assert_eq!(transition.0, 0);
        assert_eq!(transition.1, InputEventAction::Release);
        assert_eq!(transition.2, 0);
    }

    /// Decode press transitions into stable button codes.
    #[test]
    fn test_decode_console_button_transition_reports_press_code() {
        let previous = RIGHTMOST_BUTTON_PRESSED;
        let current = RIGHTMOST_BUTTON_PRESSED | FROM_LEFT_1ST_BUTTON_PRESSED;
        let transition = decode_console_button_transition(previous, current);
        let transition = transition.expect("transition should be detected");
        assert_eq!(transition.0, 0);
        assert_eq!(transition.1, InputEventAction::Press);
        assert_eq!(transition.2, 1);
    }

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
