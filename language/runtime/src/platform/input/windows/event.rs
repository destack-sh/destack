use super::{core as input_core, raw as raw_input, xinput as xinput_input};
use std::collections::VecDeque;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicUsize, Ordering};

use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::System::Console::{
    ENABLE_ECHO_INPUT, ENABLE_EXTENDED_FLAGS, ENABLE_LINE_INPUT, ENABLE_MOUSE_INPUT,
    ENABLE_PROCESSED_INPUT, ENABLE_QUICK_EDIT_MODE, ENABLE_WINDOW_INPUT, FOCUS_EVENT,
    FROM_LEFT_1ST_BUTTON_PRESSED, FROM_LEFT_2ND_BUTTON_PRESSED, FROM_LEFT_3RD_BUTTON_PRESSED,
    FROM_LEFT_4TH_BUTTON_PRESSED, GetConsoleMode, GetNumberOfConsoleInputEvents, INPUT_RECORD,
    KEY_EVENT, MENU_EVENT, MOUSE_EVENT, MOUSE_HWHEELED, MOUSE_MOVED, MOUSE_WHEELED,
    RIGHTMOST_BUTTON_PRESSED, ReadConsoleInputW, SetConsoleMode, WINDOW_BUFFER_SIZE_EVENT,
};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{
    InputDeviceEventPayload, InputEvent, InputEventAction, InputGamepadEventPayload,
    InputKeyEventPayload, InputMonitorEvent, InputPointerButtonEventPayload,
    InputPointerMotionEventPayload, InputReadMode, InputScrollEventPayload, InputTextEventPayload,
    validation as input_validation,
};
use crate::platform::resource::{ResourceFinalizer, ResourceId, ResourceKind};
use crate::platform::{NativeArray, PlatformError, core as core_platform, resource};
use crate::runtime::BindingCallContext;

/// Resource-table label for opened monitor stream entries.
const INPUT_MONITOR_RESOURCE_LABEL: &str = "input.monitor";
/// Number of active windows monitor streams.
static WINDOWS_MONITOR_STREAMS: AtomicUsize = AtomicUsize::new(0);

/// Resource payload for one windows monitor handle.
#[derive(Debug)]
struct WindowsInputMonitorBinding {
    /// Next per-monitor event sequence number.
    next_sequence: u64,
}

/// Finalizer payload for monitor stream ownership.
#[derive(Debug)]
struct WindowsMonitorFinalizer;

impl ResourceFinalizer for WindowsMonitorFinalizer {
    /// Release one monitor stream lane on resource finalization.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        WINDOWS_MONITOR_STREAMS.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Return whether one runtime error carries io-would-block.
fn is_io_would_block(error: &RuntimeError) -> bool {
    error.platform_error().map(|platform| platform.code) == Some(PlatformErrorCode::IoWouldBlock)
}

/// Build io-not-found for one missing monitor handle.
fn monitor_not_found(
    operation: &'static str,
    handle: resource::InputMonitorHandle,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(operation.to_string()),
        None,
        format!("input monitor handle {} not found", handle.0.0),
    ))
    .boxed()
}

/// Validate that one monitor handle points to an input-monitor resource.
fn validate_monitor_handle(
    binding: &BindingCallContext,
    handle: resource::InputMonitorHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    // validate monitor resource kind
    let valid = binding.worker().resources.with_entry(handle.0, |entry| {
        entry.kind == ResourceKind::InputMonitor
            && entry.label.as_deref() == Some(INPUT_MONITOR_RESOURCE_LABEL)
            && entry
                .payload
                .as_ref()
                .and_then(|payload| payload.downcast_ref::<WindowsInputMonitorBinding>())
                .is_some()
    });
    if !matches!(valid, Some(true)) {
        return Err(monitor_not_found(operation, handle));
    }

    Ok(())
}

/// Acquire one singleton monitor stream lane.
fn acquire_monitor_stream(operation: &'static str) -> RuntimeResult<()> {
    let mut current = WINDOWS_MONITOR_STREAMS.load(Ordering::Acquire);
    loop {
        if current > 0 {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::IoWouldBlock),
                None,
                None,
                Some(operation.to_string()),
                None,
                "input monitor stream is already open".to_string(),
            ))
            .boxed());
        }

        match WINDOWS_MONITOR_STREAMS.compare_exchange_weak(
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

/// Allocate one sequence number for one monitor handle.
fn next_monitor_sequence(
    binding: &BindingCallContext,
    handle: resource::InputMonitorHandle,
    operation: &'static str,
) -> RuntimeResult<u64> {
    let sequence = binding.worker().resources.with_entry_mut(handle.0, |entry| {
        if entry.kind != ResourceKind::InputMonitor {
            return None;
        }

        if entry.label.as_deref() != Some(INPUT_MONITOR_RESOURCE_LABEL) {
            return None;
        }

        let resolved_binding = entry
            .payload
            .as_mut()
            .and_then(|payload| payload.downcast_mut::<WindowsInputMonitorBinding>())?;
        let next = resolved_binding.next_sequence;
        resolved_binding.next_sequence = resolved_binding.next_sequence.saturating_add(1);
        Some(next)
    });

    match sequence.flatten() {
        Some(sequence) => Ok(sequence),
        None => Err(monitor_not_found(operation, handle)),
    }
}

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
/// Maximum queued console records per opened console handle.
const WINDOWS_PENDING_CONSOLE_RECORD_LIMIT: usize = 4096;

/// One mapped console record result with optional deferred transitions.
struct ConsoleRecordMapping {
    /// Normalized runtime event for immediate return.
    event: InputEvent,
    /// Next console button-state snapshot for this handle.
    next_button_state: Option<u32>,
    /// Deferred pointer-button transitions from the same console record.
    pending_button_transitions: Vec<input_core::PendingConsoleButtonTransition>,
}

/// Read console queue depth for nonblocking checks.
fn read_queue_depth(handle: HANDLE, operation: &'static str) -> RuntimeResult<u32> {
    let mut queued = 0u32;
    let status = unsafe { GetNumberOfConsoleInputEvents(handle, &mut queued) };
    if status == 0 {
        let code = core_platform::last_error_code() as u32;
        return Err(input_core::io_error_with_code(
            operation,
            "GetNumberOfConsoleInputEvents",
            code,
            "failed to query console event queue depth",
        ));
    }

    Ok(queued)
}

/// Decode wheel delta from high-word packed button state.
fn mouse_wheel_delta(button_state: u32) -> i16 {
    let high_word = ((button_state >> 16) & 0xffff) as u16;
    high_word as i16
}

/// Pack one console buffer width and height pair into one backend scalar value.
fn pack_console_buffer_size(width: i16, height: i16) -> i64 {
    let width = width.max(0) as u16;
    let height = height.max(0) as u16;
    (u64::from(width) << 16 | u64::from(height)) as i64
}

/// Decode one console button-state transition into one stable button event.
fn decode_console_button_transition(
    previous_state: u32,
    current_state: u32,
) -> Vec<(u32, InputEventAction, i64)> {
    // map console mouse bits to stable runtime button codes
    const BUTTON_CASES: &[(u32, u32)] = &[
        (FROM_LEFT_1ST_BUTTON_PRESSED, 0),
        (RIGHTMOST_BUTTON_PRESSED, 1),
        (FROM_LEFT_2ND_BUTTON_PRESSED, 2),
        (FROM_LEFT_3RD_BUTTON_PRESSED, 3),
        (FROM_LEFT_4TH_BUTTON_PRESSED, 4),
    ];

    let mut transitions = Vec::new();
    let changed_bits = previous_state ^ current_state;
    for (button_bit, code) in BUTTON_CASES {
        if (changed_bits & *button_bit) == 0 {
            continue;
        }

        let is_pressed = (current_state & *button_bit) != 0;
        if is_pressed {
            transitions.push((*code, InputEventAction::Press, 1));
            continue;
        }

        transitions.push((*code, InputEventAction::Release, 0));
    }

    transitions
}

/// Map one console button-state bitset into stable runtime pointer bits.
fn stable_pointer_buttons_from_console_state(state: u32) -> u32 {
    let mut buttons = 0u32;

    if (state & FROM_LEFT_1ST_BUTTON_PRESSED) != 0 {
        buttons |= POINTER_BUTTON_LEFT;
    }
    if (state & RIGHTMOST_BUTTON_PRESSED) != 0 {
        buttons |= POINTER_BUTTON_RIGHT;
    }
    if (state & FROM_LEFT_2ND_BUTTON_PRESSED) != 0 {
        buttons |= POINTER_BUTTON_MIDDLE;
    }
    if (state & FROM_LEFT_3RD_BUTTON_PRESSED) != 0 {
        buttons |= POINTER_BUTTON_X1;
    }
    if (state & FROM_LEFT_4TH_BUTTON_PRESSED) != 0 {
        buttons |= POINTER_BUTTON_X2;
    }

    buttons
}

/// Map one INPUT_RECORD into one runtime input event.
fn map_console_record(
    binding: &BindingCallContext,
    record: INPUT_RECORD,
    device_id: &str,
    read_mode: InputReadMode,
    previous_button_state: u32,
    timestamp_ns: u64,
) -> Option<ConsoleRecordMapping> {
    match record.EventType as u32 {
        KEY_EVENT => {
            let key = unsafe { record.Event.KeyEvent };
            let unicode = unsafe { key.uChar.UnicodeChar };
            let key_down = key.bKeyDown != 0;
            let is_text = key_down && unicode != 0 && read_mode == InputReadMode::Cooked;
            let kind = if is_text {
                input_core::WindowsInputEventKind::Text
            } else {
                input_core::WindowsInputEventKind::Key
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
                Some(value)
            } else {
                None
            };

            let mut payload = input_core::empty_event_payload(binding);
            if is_text {
                payload.text = InputTextEventPayload {
                    text: binding.store_string(text.as_deref().unwrap_or("")),
                    is_composing: false,
                };
            } else {
                payload.key = InputKeyEventPayload {
                    action,
                    backend_code: key.wVirtualKeyCode as u32,
                    backend_scan_code: key.wVirtualScanCode as u32,
                    backend_value: value,
                    modifiers: key.dwControlKeyState,
                    repeat: key_down && key.wRepeatCount > 1,
                };
            }
            let event =
                input_core::build_input_event(binding, kind, timestamp_ns, 0, device_id, payload);

            Some(ConsoleRecordMapping {
                event,
                next_button_state: None,
                pending_button_transitions: Vec::new(),
            })
        }
        MOUSE_EVENT => {
            let mouse = unsafe { record.Event.MouseEvent };
            let current_button_state = mouse.dwButtonState;

            let kind =
                if mouse.dwEventFlags == MOUSE_WHEELED || mouse.dwEventFlags == MOUSE_HWHEELED {
                    input_core::WindowsInputEventKind::Scroll
                } else if mouse.dwEventFlags == MOUSE_MOVED {
                    input_core::WindowsInputEventKind::PointerMotion
                } else {
                    input_core::WindowsInputEventKind::PointerButton
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

            let transitions = if kind == input_core::WindowsInputEventKind::PointerButton {
                decode_console_button_transition(previous_button_state, current_button_state)
            } else {
                Vec::new()
            };
            if kind == input_core::WindowsInputEventKind::PointerButton && transitions.is_empty() {
                return None;
            }

            let (code, action, value) = if kind == input_core::WindowsInputEventKind::PointerButton
            {
                let (code, action, value) = transitions[0];
                (code, action, value)
            } else if kind == input_core::WindowsInputEventKind::Scroll {
                (
                    mouse.dwEventFlags,
                    InputEventAction::Scroll,
                    i64::from(mouse_wheel_delta(mouse.dwButtonState)),
                )
            } else {
                (mouse.dwEventFlags, InputEventAction::Move, 0)
            };

            let mut payload = input_core::empty_event_payload(binding);
            if kind == input_core::WindowsInputEventKind::PointerButton {
                payload.pointer_button = InputPointerButtonEventPayload {
                    action,
                    backend_code: code,
                    backend_value: value,
                    x: mouse.dwMousePosition.X as f64,
                    y: mouse.dwMousePosition.Y as f64,
                    modifiers: mouse.dwControlKeyState,
                };
            } else if kind == input_core::WindowsInputEventKind::Scroll {
                payload.scroll = InputScrollEventPayload {
                    wheel_x,
                    wheel_y,
                    x: mouse.dwMousePosition.X as f64,
                    y: mouse.dwMousePosition.Y as f64,
                    modifiers: mouse.dwControlKeyState,
                };
            } else {
                payload.pointer_motion = InputPointerMotionEventPayload {
                    x: mouse.dwMousePosition.X as f64,
                    y: mouse.dwMousePosition.Y as f64,
                    buttons: stable_pointer_buttons_from_console_state(current_button_state),
                    modifiers: mouse.dwControlKeyState,
                };
            }
            let event =
                input_core::build_input_event(binding, kind, timestamp_ns, 0, device_id, payload);

            let mut pending_button_transitions = Vec::new();
            if kind == input_core::WindowsInputEventKind::PointerButton && transitions.len() > 1 {
                for (code, action, value) in transitions.into_iter().skip(1) {
                    pending_button_transitions.push(input_core::PendingConsoleButtonTransition {
                        timestamp_ns,
                        code,
                        action,
                        value,
                        x: mouse.dwMousePosition.X as f64,
                        y: mouse.dwMousePosition.Y as f64,
                        modifiers: mouse.dwControlKeyState,
                    });
                }
            }

            Some(ConsoleRecordMapping {
                event,
                next_button_state: Some(current_button_state),
                pending_button_transitions,
            })
        }
        WINDOW_BUFFER_SIZE_EVENT => {
            let resize = unsafe { record.Event.WindowBufferSizeEvent };
            let mut payload = input_core::empty_event_payload(binding);
            payload.device = InputDeviceEventPayload {
                action: InputEventAction::Move,
                backend_code: WINDOW_BUFFER_SIZE_EVENT,
                backend_value: pack_console_buffer_size(resize.dwSize.X, resize.dwSize.Y),
            };
            let event = input_core::build_input_event(
                binding,
                input_core::WindowsInputEventKind::Device,
                timestamp_ns,
                0,
                device_id,
                payload,
            );

            Some(ConsoleRecordMapping {
                event,
                next_button_state: None,
                pending_button_transitions: Vec::new(),
            })
        }
        MENU_EVENT => Some(ConsoleRecordMapping {
            event: {
                let menu = unsafe { record.Event.MenuEvent };
                let mut payload = input_core::empty_event_payload(binding);
                payload.device = InputDeviceEventPayload {
                    action: InputEventAction::Move,
                    backend_code: MENU_EVENT,
                    backend_value: menu.dwCommandId as i64,
                };
                input_core::build_input_event(
                    binding,
                    input_core::WindowsInputEventKind::Device,
                    timestamp_ns,
                    0,
                    device_id,
                    payload,
                )
            },
            next_button_state: None,
            pending_button_transitions: Vec::new(),
        }),
        FOCUS_EVENT => Some(ConsoleRecordMapping {
            event: {
                let focus = unsafe { record.Event.FocusEvent };
                let mut payload = input_core::empty_event_payload(binding);
                payload.device = InputDeviceEventPayload {
                    action: InputEventAction::Move,
                    backend_code: FOCUS_EVENT,
                    backend_value: if focus.bSetFocus != 0 { 1 } else { 0 },
                };
                input_core::build_input_event(
                    binding,
                    input_core::WindowsInputEventKind::Device,
                    timestamp_ns,
                    0,
                    device_id,
                    payload,
                )
            },
            next_button_state: None,
            pending_button_transitions: Vec::new(),
        }),
        event_type => Some(ConsoleRecordMapping {
            event: {
                let mut payload = input_core::empty_event_payload(binding);
                payload.device = InputDeviceEventPayload {
                    action: InputEventAction::Move,
                    backend_code: event_type,
                    backend_value: 0,
                };
                input_core::build_input_event(
                    binding,
                    input_core::WindowsInputEventKind::Device,
                    timestamp_ns,
                    0,
                    device_id,
                    payload,
                )
            },
            next_button_state: None,
            pending_button_transitions: Vec::new(),
        }),
    }
}

/// Set one sequence number on one monitor event.
fn set_monitor_event_sequence(event: &mut InputMonitorEvent, sequence: u64) {
    match event {
        InputMonitorEvent::InputMonitorChangeEvent(value) => value.metadata.sequence = sequence,
        InputMonitorEvent::InputMonitorConnectEvent(value) => value.metadata.sequence = sequence,
        InputMonitorEvent::InputMonitorDisconnectEvent(value) => {
            value.metadata.sequence = sequence;
        }
    }
}

/// Pop one deferred console pointer-button transition for this handle.
fn pop_pending_console_button_transition(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<Option<input_core::PendingConsoleButtonTransition>> {
    let transition = binding.worker().resources.with_entry_mut(handle.0, |entry| {
        if entry.kind != ResourceKind::InputDevice {
            return None;
        }

        if entry.label.as_deref() != Some(input_core::INPUT_RESOURCE_LABEL) {
            return None;
        }

        let resolved_binding = entry
            .payload
            .as_mut()
            .and_then(|payload| payload.downcast_mut::<input_core::WindowsInputBinding>())?;
        if resolved_binding.backend != input_core::WindowsInputBackend::Console {
            return Some(None);
        }

        Some(
            resolved_binding
                .pending_console_button_transitions
                .pop_front(),
        )
    });

    match transition {
        Some(Some(Some(transition))) => Ok(Some(transition)),
        Some(Some(None)) | Some(None) => Ok(None),
        None => Err(input_core::input_not_found(operation, handle)),
    }
}

/// Queue deferred console pointer-button transitions for this handle.
fn push_pending_console_button_transitions(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    transitions: Vec<input_core::PendingConsoleButtonTransition>,
    operation: &'static str,
) -> RuntimeResult<()> {
    if transitions.is_empty() {
        return Ok(());
    }

    let updated = binding.worker().resources.with_entry_mut(handle.0, |entry| {
        if entry.kind != ResourceKind::InputDevice {
            return None;
        }

        if entry.label.as_deref() != Some(input_core::INPUT_RESOURCE_LABEL) {
            return None;
        }

        let resolved_binding = entry
            .payload
            .as_mut()
            .and_then(|payload| payload.downcast_mut::<input_core::WindowsInputBinding>())?;
        if resolved_binding.backend != input_core::WindowsInputBackend::Console {
            return Some(());
        }

        resolved_binding
            .pending_console_button_transitions
            .extend(transitions.iter().copied());
        Some(())
    });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(input_core::input_not_found(operation, handle)),
    }
}

/// Persist one console button-state snapshot for one input handle.
fn set_console_button_state(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    state: u32,
    operation: &'static str,
) -> RuntimeResult<()> {
    let updated = binding.worker().resources.with_entry_mut(handle.0, |entry| {
        if entry.kind != ResourceKind::InputDevice {
            return None;
        }

        if entry.label.as_deref() != Some(input_core::INPUT_RESOURCE_LABEL) {
            return None;
        }

        let resolved_binding = entry
            .payload
            .as_mut()
            .and_then(|payload| payload.downcast_mut::<input_core::WindowsInputBinding>())?;
        if resolved_binding.backend != input_core::WindowsInputBackend::Console {
            return Some(());
        }

        resolved_binding.console_button_state = state;
        Some(())
    });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(input_core::input_not_found(operation, handle)),
    }
}

/// Persist one xinput packet number snapshot for one input handle.
fn set_xinput_packet_number(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    packet_number: u32,
    operation: &'static str,
) -> RuntimeResult<()> {
    let updated = binding.worker().resources.with_entry_mut(handle.0, |entry| {
        if entry.kind != ResourceKind::InputDevice {
            return None;
        }

        if entry.label.as_deref() != Some(input_core::INPUT_RESOURCE_LABEL) {
            return None;
        }

        let resolved_binding = entry
            .payload
            .as_mut()
            .and_then(|payload| payload.downcast_mut::<input_core::WindowsInputBinding>())?;
        if resolved_binding.backend != input_core::WindowsInputBackend::XInput {
            return Some(());
        }

        resolved_binding.xinput_packet_number = packet_number;
        Some(())
    });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(input_core::input_not_found(operation, handle)),
    }
}

/// Push one pending console record with one bounded queue limit.
fn push_bounded_console_record(
    queue: &mut VecDeque<input_core::PendingConsoleRecord>,
    pending_record: input_core::PendingConsoleRecord,
) {
    if queue.len() >= WINDOWS_PENDING_CONSOLE_RECORD_LIMIT {
        queue.pop_front();
    }

    queue.push_back(pending_record);
}

/// Read one host console record with blocking or nonblocking behavior.
pub(super) fn read_console_record_from_host(
    host_handle: HANDLE,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<input_core::PendingConsoleRecord> {
    loop {
        if nonblocking {
            let queued = read_queue_depth(host_handle, operation)?;
            if queued == 0 {
                return Err(input_core::io_would_block(
                    operation,
                    "input queue is empty",
                ));
            }
        }

        let mut record = MaybeUninit::<INPUT_RECORD>::uninit();
        let mut read_count = 0u32;
        let status =
            unsafe { ReadConsoleInputW(host_handle, record.as_mut_ptr(), 1, &mut read_count) };
        if status == 0 {
            let code = core_platform::last_error_code() as u32;
            return Err(input_core::io_error_with_code(
                operation,
                "ReadConsoleInputW",
                code,
                "failed to read from console input",
            ));
        }

        if read_count == 0 {
            if nonblocking {
                return Err(input_core::io_would_block(
                    operation,
                    "input queue is empty",
                ));
            }

            continue;
        }

        let record = unsafe { record.assume_init() };
        return Ok(input_core::PendingConsoleRecord {
            timestamp_ns: input_core::now_timestamp_ns(),
            record,
        });
    }
}

/// Queue one console record into input and composition demux lanes.
pub(super) fn queue_console_record_for_demux(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    pending_record: input_core::PendingConsoleRecord,
    operation: &'static str,
) -> RuntimeResult<()> {
    let updated = binding.worker().resources.with_entry_mut(handle.0, |entry| {
        if entry.kind != ResourceKind::InputDevice {
            return None;
        }

        if entry.label.as_deref() != Some(input_core::INPUT_RESOURCE_LABEL) {
            return None;
        }

        let resolved_binding = entry
            .payload
            .as_mut()
            .and_then(|payload| payload.downcast_mut::<input_core::WindowsInputBinding>())?;
        if resolved_binding.backend != input_core::WindowsInputBackend::Console {
            return Some(());
        }

        push_bounded_console_record(
            &mut resolved_binding.pending_console_records,
            pending_record,
        );

        Some(())
    });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(input_core::input_not_found(operation, handle)),
    }
}

/// Pop one queued console record for input-event decoding.
fn pop_pending_console_record(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<Option<input_core::PendingConsoleRecord>> {
    let record = binding.worker().resources.with_entry_mut(handle.0, |entry| {
        if entry.kind != ResourceKind::InputDevice {
            return None;
        }

        if entry.label.as_deref() != Some(input_core::INPUT_RESOURCE_LABEL) {
            return None;
        }

        let resolved_binding = entry
            .payload
            .as_mut()
            .and_then(|payload| payload.downcast_mut::<input_core::WindowsInputBinding>())?;
        if resolved_binding.backend != input_core::WindowsInputBackend::Console {
            return Some(None);
        }

        Some(resolved_binding.pending_console_records.pop_front())
    });

    match record {
        Some(Some(Some(record))) => Ok(Some(record)),
        Some(Some(None)) | Some(None) => Ok(None),
        None => Err(input_core::input_not_found(operation, handle)),
    }
}

/// Read one committed console text event for one text session.
pub(super) fn read_console_text_event(
    binding: &BindingCallContext,
    input_binding: &mut input_core::WindowsInputBinding,
    host_handle: HANDLE,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputEvent> {
    // require cooked console mode so host text records stay translated
    if input_binding.read_mode != InputReadMode::Cooked {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // keep consuming console records until one committed text payload arrives
    loop {
        let Some(pending_record) = input_binding.pending_console_records.pop_front() else {
            let pending_record =
                read_console_record_from_host(host_handle, nonblocking, operation)?;
            push_bounded_console_record(&mut input_binding.pending_console_records, pending_record);
            continue;
        };

        let mapped = map_console_record(
            binding,
            pending_record.record,
            input_core::WINDOWS_INPUT_DEVICE_ID,
            input_binding.read_mode,
            input_binding.console_button_state,
            pending_record.timestamp_ns,
        );
        let Some(mapped) = mapped else {
            continue;
        };

        if let Some(next_button_state) = mapped.next_button_state {
            input_binding.console_button_state = next_button_state;
        }

        if let InputEvent::InputTextEvent(event) = mapped.event {
            return Ok(InputEvent::InputTextEvent(event));
        }
    }
}

/// Read one input event from console or raw queues.
pub(super) fn read_event(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputEvent> {
    // resolve resource binding and selected backend
    let resolved = input_core::resolve_input(binding, handle, operation)?;

    // drain deferred console button transitions before reading host records
    if resolved.backend == input_core::WindowsInputBackend::Console
        && let Some(transition) = pop_pending_console_button_transition(binding, handle, operation)?
    {
        let mut payload = input_core::empty_event_payload(binding);
        payload.pointer_button = InputPointerButtonEventPayload {
            action: transition.action,
            backend_code: transition.code,
            backend_value: transition.value,
            x: transition.x,
            y: transition.y,
            modifiers: transition.modifiers,
        };
        let mut event = input_core::build_input_event(
            binding,
            input_core::WindowsInputEventKind::PointerButton,
            transition.timestamp_ns,
            0,
            input_core::WINDOWS_INPUT_DEVICE_ID,
            payload,
        );
        let sequence = input_core::next_sequence(binding, handle, operation)?;
        input_core::set_input_event_sequence(&mut event, sequence);
        return Ok(event);
    }

    let (mut event, next_button_state, pending_button_transitions, next_xinput_packet) =
        match resolved.backend {
            input_core::WindowsInputBackend::Console => {
                // require a host handle for console backends
                let Some(host_handle) = resolved.host_handle else {
                    return Err(input_core::input_not_found(operation, handle));
                };

                // read and demux records until one input payload is produced
                let event = loop {
                    // decode one already-queued record before reading from host
                    let Some(pending_record) =
                        pop_pending_console_record(binding, handle, operation)?
                    else {
                        let pending_record =
                            read_console_record_from_host(host_handle, nonblocking, operation)?;
                        queue_console_record_for_demux(binding, handle, pending_record, operation)?;
                        continue;
                    };

                    // convert one queued console record into one runtime event
                    let mapped = map_console_record(
                        binding,
                        pending_record.record,
                        input_core::WINDOWS_INPUT_DEVICE_ID,
                        resolved.read_mode,
                        resolved.console_button_state,
                        pending_record.timestamp_ns,
                    );
                    if let Some(mapped) = mapped {
                        break mapped;
                    }
                };

                (
                    event.event,
                    event.next_button_state,
                    event.pending_button_transitions,
                    None,
                )
            }
            input_core::WindowsInputBackend::Window => {
                return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
            }
            // delegate per-device raw streams to the dedicated worker queues
            input_core::WindowsInputBackend::RawDevice => {
                let Some(raw_device) = resolved.raw_device.as_ref() else {
                    return Err(input_core::input_not_found(operation, handle));
                };
                (
                    raw_input::read_device_event(binding, raw_device, nonblocking, operation)?,
                    None,
                    Vec::new(),
                    None,
                )
            }
            input_core::WindowsInputBackend::XInput => {
                let Some(user_index) = resolved.xinput_user_index else {
                    return Err(input_core::input_not_found(operation, handle));
                };
                let service = binding
                    .worker()
                    .platform_state
                    .input
                    .windows_xinput_service(operation)?;

                // wait for the next packet edge through the shared XInput service
                let next_packet = service.wait_for_packet_change(
                    user_index,
                    resolved.xinput_packet_number,
                    nonblocking,
                    operation,
                )?;

                // publish one gamepad-change event keyed by packet-number deltas
                let mut payload = input_core::empty_event_payload(binding);
                payload.gamepad = InputGamepadEventPayload {
                    action: InputEventAction::Axis,
                    backend_code: user_index as u32,
                    backend_value: next_packet as i64,
                };
                let event = input_core::build_input_event(
                    binding,
                    input_core::WindowsInputEventKind::Gamepad,
                    input_core::now_timestamp_ns(),
                    0,
                    &xinput_input::xinput_device_id(user_index),
                    payload,
                );

                (event, None, Vec::new(), Some(next_packet))
            }
        };

    // persist deferred console transitions emitted from one host record
    if resolved.backend == input_core::WindowsInputBackend::Console {
        push_pending_console_button_transitions(
            binding,
            handle,
            pending_button_transitions,
            operation,
        )?;
    }

    // persist updated console button state when one mouse snapshot was observed
    if let Some(next_button_state) = next_button_state {
        set_console_button_state(binding, handle, next_button_state, operation)?;
    }

    // persist updated xinput packet numbers
    if let Some(next_packet) = next_xinput_packet {
        set_xinput_packet_number(binding, handle, next_packet, operation)?;
    }

    // stamp one per-handle sequence number
    let sequence = input_core::next_sequence(binding, handle, operation)?;
    input_core::set_input_event_sequence(&mut event, sequence);

    Ok(event)
}

/// Enable or disable exclusive grab mode for console input.
pub(super) fn set_grab(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    enable: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve handle and enforce console-only grab semantics
    let resolved = input_core::resolve_input(binding, handle, operation)?;

    if resolved.backend != input_core::WindowsInputBackend::Console {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.input.event.setExclusiveGrab",
        ))
        .boxed());
    }

    let Some(host_handle) = resolved.host_handle else {
        return Err(input_core::input_not_found(operation, handle));
    };
    let original_mode = resolved.original_mode.unwrap_or(0);

    // compute next console mode for grab enable or disable
    let mut current_mode = 0u32;
    let status = unsafe { GetConsoleMode(host_handle, &mut current_mode) };
    if status == 0 {
        let code = core_platform::last_error_code() as u32;
        return Err(input_core::io_error_with_code(
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
        return Err(input_core::io_error_with_code(
            operation,
            "SetConsoleMode",
            code,
            "failed to update console input mode",
        ));
    }

    Ok(())
}

/// Set read mode for one windows input handle.
pub(super) fn set_read_mode(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    mode: InputReadMode,
    operation: &'static str,
) -> RuntimeResult<()> {
    // mutate resolved_binding state and host mode in one resource-table transaction
    let result = binding.worker().resources.with_entry_mut(handle.0, |entry| {
        if entry.kind != ResourceKind::InputDevice {
            return None;
        }

        if entry.label.as_deref() != Some(input_core::INPUT_RESOURCE_LABEL) {
            return None;
        }

        let host_handle = entry.handle().map(|handle| handle as HANDLE);
        let resolved_binding = entry
            .payload
            .as_mut()
            .and_then(|payload| payload.downcast_mut::<input_core::WindowsInputBinding>())?;

        let update = match resolved_binding.backend {
            input_core::WindowsInputBackend::Console => {
                // update console cooked or raw processing flags
                let Some(host_handle) = host_handle else {
                    return Some(Err(input_core::input_not_found(operation, handle)));
                };

                let mut current_mode = 0u32;
                let status = unsafe { GetConsoleMode(host_handle, &mut current_mode) };
                if status == 0 {
                    let code = core_platform::last_error_code() as u32;
                    return Some(Err(input_core::io_error_with_code(
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
                    return Some(Err(input_core::io_error_with_code(
                        operation,
                        "SetConsoleMode",
                        code,
                        "failed to update console input mode",
                    )));
                }

                Ok(())
            }
            input_core::WindowsInputBackend::Window => Err(RuntimeError::from(
                PlatformError::not_supported("destack.input.event.setReadMode"),
            )
            .boxed()),
            input_core::WindowsInputBackend::RawDevice => {
                // raw-input devices only support raw mode
                if mode == InputReadMode::Cooked {
                    Err(RuntimeError::from(PlatformError::not_supported(
                        "destack.input.event.setReadMode",
                    ))
                    .boxed())
                } else {
                    Ok(())
                }
            }
            input_core::WindowsInputBackend::XInput => {
                // xinput gamepads expose one raw polling stream only
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
            resolved_binding.read_mode = mode;
        }

        Some(update)
    });

    // map missing entries to io-not-found
    match result.flatten() {
        Some(result) => result,
        None => Err(input_core::input_not_found(operation, handle)),
    }
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
pub(crate) unsafe fn destack_input_read(
    binding: &BindingCallContext,
    out: *mut InputEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read one event from the selected input backend
    let event = read_event(binding, handle, false, "destack.input.event.read")?;

    // write event payload to output
    unsafe {
        *out = event;
    }

    Ok(())
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
pub(crate) unsafe fn destack_input_monitor_close(
    binding: &BindingCallContext,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    // validate monitor handle shape
    validate_monitor_handle(binding, handle, "destack.input.event.monitorClose")?;

    // remove and finalize monitor resource
    let removed = binding.worker().resources.remove_and_finalize(
        &binding.world(),
        handle.0,
        Some(binding.engine()),
    );
    if !removed {
        return Err(monitor_not_found(
            "destack.input.event.monitorClose",
            handle,
        ));
    }

    Ok(())
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
pub(crate) unsafe fn destack_input_monitor_open(
    binding: &BindingCallContext,
    out: *mut resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // ensure raw monitor backend is active
    raw_input::ensure_service(binding, "destack.input.event.monitorOpen")?;
    acquire_monitor_stream("destack.input.event.monitorOpen")?;

    // allocate monitor handle in the resource table
    let entry = resource::ResourceEntry::new(ResourceKind::InputMonitor)
        .with_label(INPUT_MONITOR_RESOURCE_LABEL)
        .with_payload(WindowsInputMonitorBinding { next_sequence: 1 })
        .with_finalizer(WindowsMonitorFinalizer);
    let handle = resource::InputMonitorHandle(binding.worker().resources.insert(
        &binding.world(),
        entry,
        Some(binding.engine()),
    ));

    // write handle to output
    unsafe {
        *out = handle;
    }

    Ok(())
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
pub(crate) unsafe fn destack_input_monitor_read(
    binding: &BindingCallContext,
    out: *mut InputMonitorEvent,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    validate_monitor_handle(binding, handle, "destack.input.event.monitorRead")?;

    // read one monitor event from the raw-input service
    let mut event =
        raw_input::read_monitor_event(binding, false, "destack.input.event.monitorRead")?;
    let sequence = next_monitor_sequence(binding, handle, "destack.input.event.monitorRead")?;
    set_monitor_event_sequence(&mut event, sequence);

    // write event to output
    unsafe {
        *out = event;
    }

    Ok(())
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
pub(crate) unsafe fn destack_input_monitor_try_read(
    binding: &BindingCallContext,
    out: *mut InputMonitorEvent,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    validate_monitor_handle(binding, handle, "destack.input.event.monitorTryRead")?;

    // poll one monitor event from the raw-input service
    let mut event =
        raw_input::read_monitor_event(binding, true, "destack.input.event.monitorTryRead")?;
    let sequence = next_monitor_sequence(binding, handle, "destack.input.event.monitorTryRead")?;
    set_monitor_event_sequence(&mut event, sequence);

    // write event to output
    unsafe {
        *out = event;
    }

    Ok(())
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
pub(crate) unsafe fn destack_input_set_exclusive_grab(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    enable: bool,
) -> RuntimeResult<()> {
    // delegate grab control to shared windows core helpers
    set_grab(
        binding,
        handle,
        enable,
        "destack.input.event.setExclusiveGrab",
    )
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
pub(crate) unsafe fn destack_input_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<InputEvent>,
    handle: resource::InputDeviceHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate max-events contract
    let maxevents = input_validation::validate_read_batch_maxevents(maxevents)?;

    // read at least one event to preserve blocking readBatch semantics
    let mut events = Vec::with_capacity(maxevents);
    let first = read_event(binding, handle, false, "destack.input.event.readBatch")?;
    events.push(first);

    // keep polling until queue is drained or the batch is full
    while events.len() < maxevents {
        match read_event(binding, handle, true, "destack.input.event.readBatch") {
            Ok(event) => events.push(event),
            Err(error) => {
                if is_io_would_block(error.as_ref()) {
                    break;
                }

                return Err(error);
            }
        }
    }

    // write collected events to output array
    unsafe {
        *out = binding.store_array(events);
    }

    Ok(())
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
pub(crate) unsafe fn destack_input_set_read_mode(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    mode: InputReadMode,
) -> RuntimeResult<()> {
    // delegate read-mode selection to shared windows core helpers
    set_read_mode(binding, handle, mode, "destack.input.event.setReadMode")
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
pub(crate) unsafe fn destack_input_try_read(
    binding: &BindingCallContext,
    out: *mut InputEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read one event in nonblocking mode
    let event = read_event(binding, handle, true, "destack.input.event.tryRead")?;

    // write event payload to output
    unsafe {
        *out = event;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use crate::platform::input::InputEventAction;
    use crate::platform::input::host::windows::event::{
        FROM_LEFT_1ST_BUTTON_PRESSED, FROM_LEFT_2ND_BUTTON_PRESSED, FROM_LEFT_3RD_BUTTON_PRESSED,
        FROM_LEFT_4TH_BUTTON_PRESSED, INPUT_RECORD, KEY_EVENT, RIGHTMOST_BUTTON_PRESSED,
        WINDOWS_PENDING_CONSOLE_RECORD_LIMIT, decode_console_button_transition, input_core,
        pack_console_buffer_size, push_bounded_console_record,
        stable_pointer_buttons_from_console_state,
    };

    /// Decode release transitions even when another button remains pressed.
    #[test]
    fn test_decode_console_button_transition_reports_release_with_other_pressed() {
        let previous = FROM_LEFT_1ST_BUTTON_PRESSED | RIGHTMOST_BUTTON_PRESSED;
        let current = RIGHTMOST_BUTTON_PRESSED;
        let transitions = decode_console_button_transition(previous, current);
        assert_eq!(transitions.len(), 1, "one transition should be detected");
        let transition = transitions[0];
        assert_eq!(transition.0, 0);
        assert_eq!(transition.1, InputEventAction::Release);
        assert_eq!(transition.2, 0);
    }

    /// Decode press transitions into stable button codes.
    #[test]
    fn test_decode_console_button_transition_reports_press_code() {
        let previous = RIGHTMOST_BUTTON_PRESSED;
        let current = RIGHTMOST_BUTTON_PRESSED | FROM_LEFT_1ST_BUTTON_PRESSED;
        let transitions = decode_console_button_transition(previous, current);
        assert_eq!(transitions.len(), 1, "one transition should be detected");
        let transition = transitions[0];
        assert_eq!(transition.0, 0);
        assert_eq!(transition.1, InputEventAction::Press);
        assert_eq!(transition.2, 1);
    }

    /// Map console button-state bits into stable pointer bit fields.
    #[test]
    fn test_stable_pointer_buttons_from_console_state_maps_all_buttons() {
        let state = FROM_LEFT_1ST_BUTTON_PRESSED
            | RIGHTMOST_BUTTON_PRESSED
            | FROM_LEFT_2ND_BUTTON_PRESSED
            | FROM_LEFT_3RD_BUTTON_PRESSED
            | FROM_LEFT_4TH_BUTTON_PRESSED;
        let buttons = stable_pointer_buttons_from_console_state(state);
        assert_eq!(buttons, 0b1_1111);
    }

    /// Pack console resize dimensions into one stable backend scalar value.
    #[test]
    fn test_pack_console_buffer_size_packs_nonnegative_dimensions() {
        let packed = pack_console_buffer_size(120, 40);
        let expected = ((120u64 << 16) | 40u64) as i64;
        assert_eq!(packed, expected);

        let packed_clamped = pack_console_buffer_size(-1, -2);
        assert_eq!(packed_clamped, 0);
    }

    /// Keep pending console-record queues bounded.
    #[test]
    fn test_push_bounded_console_record_enforces_limit() {
        let mut queue = VecDeque::new();
        for index in 0..(WINDOWS_PENDING_CONSOLE_RECORD_LIMIT + 1) {
            let mut record: INPUT_RECORD = unsafe { std::mem::zeroed() };
            record.EventType = KEY_EVENT as u16;
            record.Event.KeyEvent.uChar.UnicodeChar = (index % 64) as u16;

            push_bounded_console_record(
                &mut queue,
                input_core::PendingConsoleRecord {
                    timestamp_ns: index as u64,
                    record,
                },
            );
        }

        assert_eq!(
            queue.len(),
            WINDOWS_PENDING_CONSOLE_RECORD_LIMIT,
            "record queue should remain bounded"
        );
        assert_eq!(
            queue.front().map(|record| record.timestamp_ns),
            Some(1),
            "oldest entries should be evicted first"
        );
    }
}
