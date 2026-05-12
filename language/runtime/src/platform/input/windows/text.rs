use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::c_void;
use std::sync::Arc;

use windows_sys::Win32::Foundation::{HANDLE, HWND, WPARAM};

use super::{core as input_core, event as input_event};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::display::windows::win32 as win32_display;
use crate::platform::input::{
    InputClipboardCommandEventValue, InputClipboardCommandTypeValue, InputEditIntentEventValue,
    InputEditIntentTypeValue, InputEvent, InputEventMetadataValue, InputReadMode,
    InputTextGeometry, InputTextRange, InputTextSessionConfig, InputTextSessionEvent,
    InputTextSessionEventValue, InputTextSessionState, InputTextSessionStateEventValue,
    InputTextSessionStateValue,
};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{NativeAbiCodec, PlatformError, core as core_platform, resource};
use crate::runtime::BindingCallContext;
use parking_lot::{Condvar, Mutex};

/// Resource-table label for active text-session entries.
const TEXT_SESSION_RESOURCE_LABEL: &str = "input.text.session";
/// Stable device identifier used for Win32 window-backed text-session events.
const WIN32_WINDOW_TEXT_DEVICE_ID: &str = "win32.window.text";

/// Waitable queued-event signal for one text session.
#[derive(Debug, Default)]
struct TextRepositoryEventSignal {
    /// Monotonic wake generation for this session queue.
    generation: Mutex<u64>,
    /// Wake signal for queued session events.
    wake: Condvar,
}

/// Native event source for one Windows text session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowsTextRepositorySource {
    /// Console-backed cooked text input.
    Console,
    /// Native Win32 window-backed text input.
    Window,
}

/// Stored text-session payload for one Windows host session.
#[derive(Debug, Clone)]
struct WindowsTextRepository {
    /// The duplicated console input handle.
    console_handle: HANDLE,
    /// The opened Windows text binding.
    binding: input_core::WindowsInputBinding,
    /// The session configuration.
    config: InputTextSessionConfig,
    /// The current renderer-owned geometry hint.
    geometry: Option<InputTextGeometry>,
    /// The renderer-owned text state.
    state: InputTextSessionStateValue,
    /// The next session event sequence number.
    next_sequence: u64,
    /// The selected target window when one explicit target is active.
    target_window: Option<resource::WindowHandle>,
    /// The native hwnd used for one explicit target when available.
    target_hwnd: Option<HWND>,
    /// The native event source used by this session.
    source: WindowsTextRepositorySource,
    /// Pending queued session events from native host callbacks.
    events: VecDeque<InputTextSessionEventValue>,
    /// Waitable signal for queued session events.
    event_signal: Arc<TextRepositoryEventSignal>,
    /// The renderer state snapshot captured before one active IME composition.
    composition_base_state: Option<InputTextSessionStateValue>,
    /// Pending high surrogate from one split UTF-16 char-message pair.
    pending_high_surrogate: Option<u16>,
    /// Pending utf16 units to suppress after one authoritative IME result commit.
    pending_commit_units_to_ignore: u32,
}

/// Return the UTF-16 code-unit length for one string.
fn utf16_length(text: &str) -> u32 {
    text.encode_utf16().count() as u32
}

/// Validate one text range against one UTF-16 text length.
fn validate_text_range(
    field: &'static str,
    range: InputTextRange,
    text_length: u32,
) -> RuntimeResult<()> {
    // reject inverted ranges
    if range.start_offset > range.end_offset {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "text range start must not be greater than the end",
        ))
        .boxed());
    }

    // reject out-of-bounds ranges
    if range.end_offset > text_length {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "text range exceeds the current UTF-16 text length",
        ))
        .boxed());
    }

    Ok(())
}

/// Validate one full text-session state payload.
fn validate_text_session_state(state: &InputTextSessionStateValue) -> RuntimeResult<()> {
    let text_length = utf16_length(&state.text);

    // selection
    validate_text_range("state.selection", state.selection, text_length)?;

    // composing
    if let Some(composing) = state.composing {
        validate_text_range("state.composing", composing, text_length)?;
    }

    Ok(())
}

/// Resolve one optional explicit text target window.
fn resolve_text_target_window(
    binding: &BindingCallContext,
    config: InputTextSessionConfig,
    operation: &'static str,
) -> RuntimeResult<Option<resource::WindowHandle>> {
    let target_window = if input_core::has_explicit_window_target(config.target) {
        config.target.window
    } else {
        None
    };

    let _ = input_core::resolve_window_target_handle(binding, config.target, operation)?;

    Ok(target_window)
}

/// Return one byte index for one UTF-16 boundary.
fn utf16_byte_index(text: &str, offset: u32) -> RuntimeResult<usize> {
    let mut current_offset = 0;

    for (byte_index, character) in text.char_indices() {
        if current_offset == offset {
            return Ok(byte_index);
        }

        current_offset += character.len_utf16() as u32;
    }

    if current_offset == offset {
        return Ok(text.len());
    }

    Err(RuntimeError::from(PlatformError::invalid_data(
        "text state contains one invalid UTF-16 boundary",
    ))
    .boxed())
}

/// Replace one UTF-16 range and return the inserted range.
fn replace_utf16_range(
    text: &mut String,
    range: InputTextRange,
    replacement: &str,
) -> RuntimeResult<InputTextRange> {
    let start_index = utf16_byte_index(text, range.start_offset)?;
    let end_index = utf16_byte_index(text, range.end_offset)?;
    text.replace_range(start_index..end_index, replacement);

    Ok(InputTextRange {
        start_offset: range.start_offset,
        end_offset: range.start_offset + utf16_length(replacement),
    })
}

/// Allocate one session event metadata payload.
fn next_text_event_metadata(
    session: &mut WindowsTextRepository,
    timestamp_ns: u64,
) -> InputEventMetadataValue {
    let metadata = InputEventMetadataValue {
        timestamp_ns,
        sequence: session.next_sequence,
        device_id: input_core::WINDOWS_INPUT_DEVICE_ID.to_string(),
        target_window: session.target_window,
    };

    session.next_sequence = session.next_sequence.wrapping_add(1);

    metadata
}

/// Build one committed text edit-intent event.
fn committed_text_edit_intent(
    session: &mut WindowsTextRepository,
    timestamp_ns: u64,
    text: String,
    is_composing: bool,
) -> InputTextSessionEventValue {
    // map line breaks to one explicit line-break edit intent
    let (input_type, data) = if matches!(text.as_str(), "\n" | "\r") {
        (InputEditIntentTypeValue::InsertLineBreak, None)
    } else {
        (InputEditIntentTypeValue::InsertText, Some(text))
    };

    InputTextSessionEventValue::InputEditIntentEvent(InputEditIntentEventValue {
        kind: "editIntent".to_string(),
        metadata: next_text_event_metadata(session, timestamp_ns),
        input_type,
        data,
        target_ranges: vec![session.state.selection],
        clipboard_items: None,
        drag: None,
        is_composing,
    })
}

/// Allocate one window-text session event metadata payload.
fn next_window_text_event_metadata(session: &mut WindowsTextRepository) -> InputEventMetadataValue {
    let metadata = InputEventMetadataValue {
        timestamp_ns: core_platform::monotonic_now_ns(),
        sequence: session.next_sequence,
        device_id: WIN32_WINDOW_TEXT_DEVICE_ID.to_string(),
        target_window: session.target_window,
    };

    session.next_sequence = session.next_sequence.wrapping_add(1);

    metadata
}

/// Queue one host-authoritative state event.
fn queue_window_text_state_event(session: &mut WindowsTextRepository) {
    let metadata = next_window_text_event_metadata(session);
    let state = session.state.clone();

    session
        .events
        .push_back(InputTextSessionEventValue::InputTextSessionStateEvent(
            InputTextSessionStateEventValue {
                kind: "stateChanged".to_string(),
                metadata,
                state,
            },
        ));
    notify_text_session_event(&session.event_signal);
}

/// Queue one window-backed edit-intent event.
fn queue_window_text_edit_intent_event(
    session: &mut WindowsTextRepository,
    input_type: InputEditIntentTypeValue,
    data: Option<String>,
    is_composing: bool,
) {
    let metadata = next_window_text_event_metadata(session);

    session
        .events
        .push_back(InputTextSessionEventValue::InputEditIntentEvent(
            InputEditIntentEventValue {
                kind: "editIntent".to_string(),
                metadata,
                input_type,
                data,
                target_ranges: vec![session.state.selection],
                clipboard_items: None,
                drag: None,
                is_composing,
            },
        ));
    notify_text_session_event(&session.event_signal);
}

/// Queue one window-backed clipboard-command event.
fn queue_window_text_clipboard_command_event(
    session: &mut WindowsTextRepository,
    command: InputClipboardCommandTypeValue,
    is_composing: bool,
) {
    let metadata = next_window_text_event_metadata(session);

    session
        .events
        .push_back(InputTextSessionEventValue::InputClipboardCommandEvent(
            InputClipboardCommandEventValue {
                kind: "clipboardCommand".to_string(),
                metadata,
                command,
                clipboard_items: None,
                is_composing,
            },
        ));
    notify_text_session_event(&session.event_signal);
}

/// Return whether one windows session allows one clipboard command.
fn allows_window_clipboard_command(
    session: &WindowsTextRepository,
    command: InputClipboardCommandTypeValue,
) -> bool {
    if session.config.is_secure
        && matches!(
            command,
            InputClipboardCommandTypeValue::Copy | InputClipboardCommandTypeValue::Cut
        )
    {
        return false;
    }

    true
}

/// Return whether one windows session allows one committed text payload.
fn allows_window_committed_text(session: &WindowsTextRepository, text: &str) -> bool {
    if !session.config.is_multiline && matches!(text, "\n" | "\r") {
        return false;
    }

    true
}

/// Apply one committed text replacement to one session state.
fn apply_committed_window_text(
    session: &mut WindowsTextRepository,
    text: &str,
) -> RuntimeResult<()> {
    let base_state = session
        .composition_base_state
        .take()
        .unwrap_or_else(|| session.state.clone());
    let mut next_text = base_state.text.clone();
    let inserted = replace_utf16_range(&mut next_text, base_state.selection, text)?;
    let caret_offset = inserted.end_offset;

    session.state.text = next_text;
    session.state.selection = InputTextRange {
        start_offset: caret_offset,
        end_offset: caret_offset,
    };
    session.state.composing = None;
    validate_text_session_state(&session.state)?;
    queue_window_text_state_event(session);

    Ok(())
}

/// Apply one active IME composing string to one session state.
fn apply_window_composing_text(
    session: &mut WindowsTextRepository,
    text: &str,
    cursor_offset: Option<u32>,
) -> RuntimeResult<()> {
    if session.composition_base_state.is_none() {
        session.composition_base_state = Some(session.state.clone());
    }

    let Some(base_state) = session.composition_base_state.clone() else {
        return Ok(());
    };
    let mut next_text = base_state.text.clone();
    let inserted = replace_utf16_range(&mut next_text, base_state.selection, text)?;
    let composing_length = inserted.end_offset - inserted.start_offset;
    let cursor_offset = cursor_offset
        .unwrap_or(composing_length)
        .min(composing_length);
    let selection_offset = inserted.start_offset + cursor_offset;

    session.state.text = next_text;
    session.state.selection = InputTextRange {
        start_offset: selection_offset,
        end_offset: selection_offset,
    };
    session.state.composing = Some(inserted);
    validate_text_session_state(&session.state)?;
    queue_window_text_state_event(session);

    Ok(())
}

/// Cancel one active IME composition and restore the base state.
fn cancel_window_composition(session: &mut WindowsTextRepository) -> RuntimeResult<()> {
    let Some(base_state) = session.composition_base_state.take() else {
        return Ok(());
    };

    session.state = base_state;
    validate_text_session_state(&session.state)?;
    queue_window_text_state_event(session);

    Ok(())
}

/// Decode one incoming text message into one committed string.
fn decode_window_text_message(
    session: &mut WindowsTextRepository,
    message: u32,
    wparam: WPARAM,
) -> Option<String> {
    if message == WM_UNICHAR {
        if wparam as u32 == WM_UNICHAR_NOCHAR {
            return Some(String::new());
        }

        return char::from_u32(wparam as u32).map(|character| character.to_string());
    }

    let unit = wparam as u16;

    if char::decode_utf16([unit])
        .next()
        .is_some_and(|result| result.is_ok())
    {
        session.pending_high_surrogate = None;
    }

    if (0xD800..=0xDBFF).contains(&unit) {
        session.pending_high_surrogate = Some(unit);
        return None;
    }

    if (0xDC00..=0xDFFF).contains(&unit)
        && let Some(high_surrogate) = session.pending_high_surrogate.take()
    {
        let mut units = [high_surrogate, unit].into_iter();
        return char::decode_utf16(&mut units)
            .next()
            .and_then(Result::ok)
            .map(|character| character.to_string());
    }

    char::from_u32(unit as u32).map(|character| character.to_string())
}

/// Return whether one native char message should be ignored after one IME result commit.
fn suppress_window_text_message_commit(
    session: &mut WindowsTextRepository,
    message: u32,
    wparam: WPARAM,
) -> bool {
    if session.pending_commit_units_to_ignore == 0 {
        return false;
    }

    let consumed_units = if message == WM_UNICHAR {
        if wparam as u32 == WM_UNICHAR_NOCHAR {
            return true;
        }

        char::from_u32(wparam as u32)
            .map(|character| character.len_utf16() as u32)
            .unwrap_or(1)
    } else {
        1
    };

    session.pending_high_surrogate = None;
    session.pending_commit_units_to_ignore = session
        .pending_commit_units_to_ignore
        .saturating_sub(consumed_units);

    true
}

/// Wake queued-event readers after one session event is appended.
fn notify_text_session_event(signal: &TextRepositoryEventSignal) {
    let mut generation = signal.generation.lock();
    *generation = generation.wrapping_add(1);
    signal.wake.notify_all();
}

/// Pop one queued native session event when one is available.
fn pop_queued_text_session_event(
    binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
    operation: &'static str,
) -> RuntimeResult<Option<InputTextSessionEvent>> {
    let next = binding
        .worker()
        .resources
        .with_entry_mut(session.0, |entry| {
            if entry.kind != ResourceKind::InputTextSession {
                return None;
            }

            if entry.label.as_deref() != Some(TEXT_SESSION_RESOURCE_LABEL) {
                return None;
            }

            let session = entry.payload_mut::<WindowsTextRepository>()?;
            let event = session.events.pop_front()?;
            Some(InputTextSessionEvent::from_value(binding, event))
        });

    match next {
        Some(Some(event)) => Ok(Some(event)),
        Some(None) => Ok(None),
        None => Err(text_session_not_found(operation, session)),
    }
}

/// Wait for one queued native session event to become available.
fn wait_for_text_session_event(
    binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
    operation: &'static str,
) -> RuntimeResult<InputTextSessionEvent> {
    loop {
        // fast path
        if let Some(event) = pop_queued_text_session_event(binding, session, operation)? {
            return Ok(event);
        }

        let signal = resolve_text_session(binding, session, operation)?.event_signal;
        let mut generation = signal.generation.lock();
        let observed_generation = *generation;

        // avoid sleeping when one event raced in after the fast path
        if let Some(event) = pop_queued_text_session_event(binding, session, operation)? {
            return Ok(event);
        }

        while *generation == observed_generation {
            signal.wake.wait(&mut generation);
        }
    }
}

/// Build io-not-found for one missing Windows text session.
fn text_session_not_found(
    operation: &'static str,
    session: resource::InputTextSessionHandle,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(crate::platform::diagnostic::PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(operation.to_string()),
        None,
        format!("text session {} not found", session.0.local_id),
    ))
    .boxed()
}

/// Resolve one active Windows text session from the resource table.
fn resolve_text_session(
    binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
    operation: &'static str,
) -> RuntimeResult<WindowsTextRepository> {
    let session = binding.worker().resources.with_entry(session.0, |entry| {
        if entry.kind != ResourceKind::InputTextSession {
            return None;
        }

        if entry.label.as_deref() != Some(TEXT_SESSION_RESOURCE_LABEL) {
            return None;
        }

        entry.payload_cloned::<WindowsTextRepository>()
    });

    match session.flatten() {
        Some(session) => Ok(session),
        None => Err(text_session_not_found(operation, session)),
    }
}

/// Resolve one active Win32 window text session and its current state.
pub(crate) fn resolve_win32_window_text_session(
    runtime_state: &win32_display::Win32RuntimeState,
    window: resource::WindowHandle,
) -> RuntimeResult<
    Option<(
        resource::InputTextSessionHandle,
        InputTextSessionConfig,
        InputTextSessionStateValue,
        Option<InputTextGeometry>,
    )>,
> {
    let Some(session_handle) = runtime_state
        .active_text_sessions
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .get(&window)
        .copied()
    else {
        return Ok(None);
    };

    let session = runtime_state
        .resource_table()
        .with_entry(session_handle.0, |entry| {
            if entry.kind != ResourceKind::InputTextSession {
                return None;
            }

            if entry.label.as_deref() != Some(TEXT_SESSION_RESOURCE_LABEL) {
                return None;
            }

            entry.payload_cloned::<WindowsTextRepository>()
        })
        .flatten()
        .ok_or_else(|| {
            text_session_not_found("destack.input.text.resolveWindowRepository", session_handle)
        })?;

    Ok(Some((
        session_handle,
        session.config,
        session.state,
        session.geometry,
    )))
}

/// Queue one native window text-state update.
fn update_win32_window_text_state(
    runtime_state: &win32_display::Win32RuntimeState,
    window: resource::WindowHandle,
    update: impl FnOnce(&mut WindowsTextRepository) -> RuntimeResult<()>,
    operation: &'static str,
) -> RuntimeResult<()> {
    let Some(session_handle) = runtime_state
        .active_text_sessions
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .get(&window)
        .copied()
    else {
        return Ok(());
    };

    let updated = runtime_state
        .resource_table()
        .with_entry_mut(session_handle.0, |entry| {
            if entry.kind != ResourceKind::InputTextSession {
                return None;
            }

            if entry.label.as_deref() != Some(TEXT_SESSION_RESOURCE_LABEL) {
                return None;
            }

            let session = entry.payload_mut::<WindowsTextRepository>()?;
            Some(update(session))
        });

    match updated.flatten() {
        Some(Ok(())) => Ok(()),
        Some(Err(error)) => Err(error),
        None => Err(text_session_not_found(operation, session_handle)),
    }
}

/// Queue one committed text update from one native Win32 text message.
pub(crate) fn notify_win32_window_text_message(
    runtime_state: &win32_display::Win32RuntimeState,
    window: resource::WindowHandle,
    message: u32,
    wparam: WPARAM,
) -> RuntimeResult<()> {
    update_win32_window_text_state(
        runtime_state,
        window,
        |session| {
            // prefer the ime result-string lane once one composition has committed text
            if suppress_window_text_message_commit(session, message, wparam) {
                return Ok(());
            }

            let Some(text) = decode_window_text_message(session, message, wparam) else {
                return Ok(());
            };

            // single-line sessions should not commit native line breaks
            if !allows_window_committed_text(session, &text) {
                return Ok(());
            }

            apply_committed_window_text(session, &text)
        },
        "destack.input.text.handleWindowMessage",
    )
}

/// Queue one native Win32 edit-intent event.
pub(crate) fn notify_win32_window_edit_intent(
    runtime_state: &win32_display::Win32RuntimeState,
    window: resource::WindowHandle,
    input_type: InputEditIntentTypeValue,
    data: Option<String>,
) -> RuntimeResult<()> {
    update_win32_window_text_state(
        runtime_state,
        window,
        |session| {
            queue_window_text_edit_intent_event(
                session,
                input_type,
                data,
                session.state.composing.is_some(),
            );

            Ok(())
        },
        "destack.input.text.handleWindowMessage",
    )
}

/// Queue one native Win32 clipboard-command event.
pub(crate) fn notify_win32_window_clipboard_command(
    runtime_state: &win32_display::Win32RuntimeState,
    window: resource::WindowHandle,
    command: InputClipboardCommandTypeValue,
) -> RuntimeResult<()> {
    update_win32_window_text_state(
        runtime_state,
        window,
        |session| {
            if !allows_window_clipboard_command(session, command) {
                return Ok(());
            }

            queue_window_text_clipboard_command_event(
                session,
                command,
                session.state.composing.is_some(),
            );

            Ok(())
        },
        "destack.input.text.handleWindowMessage",
    )
}

#[cfg(test)]
mod tests {
    use super::{
        TextRepositoryEventSignal, WindowsTextRepository, WindowsTextRepositorySource,
        allows_window_clipboard_command, allows_window_committed_text,
    };
    use crate::platform::input::windows::core::WindowsInputBinding;
    use crate::platform::input::{
        InputClipboardCommandTypeValue, InputReadMode, InputTextInputType, InputTextRange,
        InputTextSessionConfig, InputTextSessionStateValue, InputWindowTarget,
    };
    use std::collections::{HashMap, HashSet, VecDeque};
    use std::sync::Arc;

    /// Reject export clipboard commands for secure windows sessions.
    #[test]
    fn test_secure_windows_sessions_reject_copy_and_cut() {
        let session = test_windows_text_session(InputTextSessionConfig {
            target: InputWindowTarget { window: None },
            input_type: InputTextInputType::Password,
            is_multiline: false,
            is_secure: true,
        });

        assert!(!allows_window_clipboard_command(
            &session,
            InputClipboardCommandTypeValue::Copy,
        ));
        assert!(!allows_window_clipboard_command(
            &session,
            InputClipboardCommandTypeValue::Cut,
        ));
        assert!(allows_window_clipboard_command(
            &session,
            InputClipboardCommandTypeValue::Paste,
        ));
    }

    /// Reject native linebreak commits for single-line windows sessions.
    #[test]
    fn test_single_line_windows_sessions_reject_native_linebreak_commits() {
        let session = test_windows_text_session(InputTextSessionConfig {
            target: InputWindowTarget { window: None },
            input_type: InputTextInputType::Text,
            is_multiline: false,
            is_secure: false,
        });

        assert!(!allows_window_committed_text(&session, "\n"));
        assert!(!allows_window_committed_text(&session, "\r"));
        assert!(allows_window_committed_text(&session, "x"));
    }

    /// Build one minimal windows text session for policy tests.
    fn test_windows_text_session(config: InputTextSessionConfig) -> WindowsTextRepository {
        WindowsTextRepository {
            console_handle: 0,
            binding: WindowsInputBinding {
                backend: super::input_core::WindowsInputBackend::Window,
                read_mode: InputReadMode::Cooked,
                next_sequence: 1,
                console_button_state: 0,
                pending_console_button_transitions: VecDeque::new(),
                pending_console_records: VecDeque::new(),
                original_mode: None,
                raw_device: None,
                xinput_user_index: None,
                xinput_packet_number: 0,
                xinput_player_index_override: None,
                last_pointer_x: 0.0,
                last_pointer_y: 0.0,
                relative_mode_enabled: false,
                text_active: true,
                text_input_type: config.input_type,
                text_area: None,
                sensor_enabled_kinds: HashSet::new(),
                sensor_effective_configs: HashMap::new(),
            },
            config,
            geometry: None,
            state: InputTextSessionStateValue {
                text: "hello".to_string(),
                selection: InputTextRange {
                    start_offset: 5,
                    end_offset: 5,
                },
                composing: None,
            },
            next_sequence: 1,
            target_window: None,
            target_hwnd: None,
            source: WindowsTextRepositorySource::Window,
            events: VecDeque::new(),
            event_signal: Arc::new(TextRepositoryEventSignal::default()),
            composition_base_state: None,
            pending_high_surrogate: None,
            pending_commit_units_to_ignore: 0,
        }
    }
}

/// Apply one native Win32 committed composition string.
pub(crate) fn notify_win32_window_committed_text(
    runtime_state: &win32_display::Win32RuntimeState,
    window: resource::WindowHandle,
    text: &str,
) -> RuntimeResult<()> {
    update_win32_window_text_state(
        runtime_state,
        window,
        |session| {
            let committed_units = utf16_length(text);
            apply_committed_window_text(session, text)?;
            session.pending_commit_units_to_ignore = committed_units;
            Ok(())
        },
        "destack.input.text.handleWindowMessage",
    )
}

/// Apply one native Win32 composing string update.
pub(crate) fn notify_win32_window_composing_text(
    runtime_state: &win32_display::Win32RuntimeState,
    window: resource::WindowHandle,
    text: &str,
    cursor_offset: Option<u32>,
) -> RuntimeResult<()> {
    update_win32_window_text_state(
        runtime_state,
        window,
        |session| apply_window_composing_text(session, text, cursor_offset),
        "destack.input.text.handleWindowMessage",
    )
}

/// Cancel one native Win32 composition.
pub(crate) fn notify_win32_window_cancel_composition(
    runtime_state: &win32_display::Win32RuntimeState,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    update_win32_window_text_state(
        runtime_state,
        window,
        cancel_window_composition,
        "destack.input.text.handleWindowMessage",
    )
}

/// Persist one text-area hint for one Windows text session.
fn text_set_geometry(
    binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
    area: InputTextGeometry,
    operation: &'static str,
) -> RuntimeResult<()> {
    let updated = binding
        .worker()
        .resources
        .with_entry_mut(session.0, |entry| {
            if entry.kind != ResourceKind::InputTextSession {
                return None;
            }

            if entry.label.as_deref() != Some(TEXT_SESSION_RESOURCE_LABEL) {
                return None;
            }

            let session = entry.payload_mut::<WindowsTextRepository>()?;
            session.binding.text_area = Some(area);
            session.geometry = Some(area);
            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(text_session_not_found(operation, session)),
    }
}

/// Persist renderer-owned text state for one Windows text session.
fn text_set_state(
    binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
    state: InputTextSessionStateValue,
    operation: &'static str,
) -> RuntimeResult<()> {
    validate_text_session_state(&state)?;

    let updated = binding
        .worker()
        .resources
        .with_entry_mut(session.0, |entry| {
            if entry.kind != ResourceKind::InputTextSession {
                return None;
            }

            if entry.label.as_deref() != Some(TEXT_SESSION_RESOURCE_LABEL) {
                return None;
            }

            let session = entry.payload_mut::<WindowsTextRepository>()?;
            session.state = state;
            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(text_session_not_found(operation, session)),
    }
}

/// Read one text session event for one Windows text session.
fn text_read_event(
    binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputTextSessionEvent> {
    let event = binding
        .worker()
        .resources
        .with_entry_mut(session.0, |entry| {
            if entry.kind != ResourceKind::InputTextSession {
                return None;
            }

            if entry.label.as_deref() != Some(TEXT_SESSION_RESOURCE_LABEL) {
                return None;
            }

            let session = entry.payload_mut::<WindowsTextRepository>()?;

            // keep composition reads tied to explicit text-session activation
            if !session.binding.text_active {
                return Some(Err(core_platform::io_would_block(
                    operation,
                    "text session is not active",
                )));
            }

            // read native queued events directly for window-backed sessions
            if session.source == WindowsTextRepositorySource::Window {
                let event = if nonblocking {
                    pop_queued_text_session_event(binding, session, operation)
                } else {
                    wait_for_text_session_event(binding, session, operation).map(Some)
                };

                return Some(event);
            }

            // keep text reads tied to the console backend
            if session.binding.backend != input_core::WindowsInputBackend::Console {
                return Some(Err(RuntimeError::from(PlatformError::not_supported(
                    operation,
                ))
                .boxed()));
            }

            let event = input_event::read_console_text_event(
                binding,
                &mut session.binding,
                session.console_handle,
                nonblocking,
                operation,
            );
            let event = match event {
                Ok(event) => event,
                Err(error) => return Some(Err(error)),
            };

            if let InputEvent::InputTextEvent(value) = event {
                let text = unsafe { value.payload.text.as_str() }.map_err(|error| {
                    RuntimeError::from(PlatformError::invalid_data(format!(
                        "{operation}: text payload is not valid utf-8: {error}"
                    )))
                    .boxed()
                });
                let text = match text {
                    Ok(text) => text.to_string(),
                    Err(error) => return Some(Err(error)),
                };
                let event = committed_text_edit_intent(
                    session,
                    value.metadata.timestamp_ns,
                    text,
                    value.payload.is_composing,
                );

                return Some(Ok(InputTextSessionEvent::from_value(binding, event)));
            }

            Some(Err(RuntimeError::from(PlatformError::not_supported(
                operation,
            ))
            .boxed()))
        });

    match event {
        Some(Some(Ok(event))) => Ok(event),
        Some(Some(Err(error))) => Err(error),
        Some(None) | None => Err(text_session_not_found(operation, session)),
    }
}

/// Open one text input session.
///
/// Open one focused text input session for the active renderer editor and initial text state.
/// Explicit window targets are used for session metadata and geometry hints when one opened window resource is provided.
pub(crate) unsafe fn destack_input_text_open(
    binding: &BindingCallContext,
    out: *mut resource::InputTextSessionHandle,
    config: InputTextSessionConfig,
    state: InputTextSessionState,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // renderer state
    let state = unsafe { state.into_value()? };
    validate_text_session_state(&state)?;

    // target scope
    let target_window = resolve_text_target_window(binding, config, "destack.input.text.open")?;
    let target_hwnd = input_core::resolve_window_target_handle(
        binding,
        config.target,
        "destack.input.text.open",
    )?;

    let open_result = (|| {
        let (pointer_x, pointer_y) = input_core::current_pointer_position();

        // window lane
        if target_window.is_some() && target_hwnd.is_some() {
            let session = WindowsTextRepository {
                console_handle: 0,
                binding: input_core::WindowsInputBinding {
                    backend: input_core::WindowsInputBackend::Window,
                    read_mode: InputReadMode::Cooked,
                    next_sequence: 1,
                    console_button_state: 0,
                    pending_console_button_transitions: VecDeque::new(),
                    pending_console_records: VecDeque::new(),
                    original_mode: None,
                    raw_device: None,
                    xinput_user_index: None,
                    xinput_packet_number: 0,
                    xinput_player_index_override: None,
                    last_pointer_x: pointer_x,
                    last_pointer_y: pointer_y,
                    relative_mode_enabled: false,
                    text_active: true,
                    text_input_type: config.input_type,
                    text_area: None,
                    sensor_enabled_kinds: HashSet::new(),
                    sensor_effective_configs: HashMap::new(),
                },
                config,
                geometry: None,
                state,
                next_sequence: 1,
                target_window,
                target_hwnd,
                source: WindowsTextRepositorySource::Window,
                events: VecDeque::new(),
                event_signal: Arc::new(TextRepositoryEventSignal::default()),
                composition_base_state: None,
                pending_high_surrogate: None,
                pending_commit_units_to_ignore: 0,
            };

            let entry = ResourceEntry::new(ResourceKind::InputTextSession)
                .with_label(TEXT_SESSION_RESOURCE_LABEL)
                .with_handle(target_hwnd.unwrap() as *mut c_void)
                .with_payload(session);
            let resource_id =
                binding
                    .worker()
                    .resources
                    .insert(binding.world(), entry, Some(binding.engine()));

            return Ok(resource::InputTextSessionHandle(resource_id));
        }

        // console lane
        input_core::acquire_console_stream("destack.input.text.open")?;
        let Some((stdin, mode)) = input_core::get_stdin_console_mode()? else {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(crate::platform::diagnostic::PlatformErrorCode::IoNotFound),
                None,
                None,
                Some("destack.input.text.open".to_string()),
                None,
                "console input is not available for this process",
            ))
            .boxed());
        };
        let duplicated = input_core::duplicate_console_handle(stdin)?;

        let session = WindowsTextRepository {
            console_handle: duplicated,
            binding: input_core::WindowsInputBinding {
                backend: input_core::WindowsInputBackend::Console,
                read_mode: input_core::read_mode_from_console_mode(mode),
                next_sequence: 1,
                console_button_state: 0,
                pending_console_button_transitions: VecDeque::new(),
                pending_console_records: VecDeque::new(),
                original_mode: Some(mode),
                raw_device: None,
                xinput_user_index: None,
                xinput_packet_number: 0,
                xinput_player_index_override: None,
                last_pointer_x: pointer_x,
                last_pointer_y: pointer_y,
                relative_mode_enabled: false,
                text_active: true,
                text_input_type: config.input_type,
                text_area: None,
                sensor_enabled_kinds: HashSet::new(),
                sensor_effective_configs: HashMap::new(),
            },
            config,
            geometry: None,
            state,
            next_sequence: 1,
            target_window,
            target_hwnd,
            source: WindowsTextRepositorySource::Console,
            events: VecDeque::new(),
            event_signal: Arc::new(TextRepositoryEventSignal::default()),
            composition_base_state: None,
            pending_high_surrogate: None,
            pending_commit_units_to_ignore: 0,
        };

        let entry = ResourceEntry::new(ResourceKind::InputTextSession)
            .with_label(TEXT_SESSION_RESOURCE_LABEL)
            .with_handle(duplicated as *mut c_void)
            .with_payload(session)
            .with_finalizer(input_core::WindowsInputFinalizer {
                handle: duplicated,
                restore_mode: Some(mode),
                release_console_lane: true,
            });
        let resource_id =
            binding
                .worker()
                .resources
                .insert(binding.world(), entry, Some(binding.engine()));

        Ok(resource::InputTextSessionHandle(resource_id))
    })();

    if open_result.is_err() {
        input_core::WINDOWS_CONSOLE_STREAMS.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
    }

    let session = open_result?;

    // attach window-backed sessions to the native routing map
    if let Some(target_window) = target_window {
        win32_display::activate_window_text_session(binding, target_window, session)?;

        if let Some(target_hwnd) = target_hwnd {
            let runtime_state = win32_display::runtime_state(binding);
            win32_display::synchronize_window_text_session(
                &runtime_state,
                target_window,
                target_hwnd,
                None,
            )?;
        }
    }

    // output
    unsafe {
        *out = session;
    }

    Ok(())
}

/// Close one text input session.
pub(crate) unsafe fn destack_input_text_close(
    binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    let resolved_session = resolve_text_session(binding, session, "destack.input.text.close")?;

    // clear routing before finalization when this session owns one native window lane
    if resolved_session.source == WindowsTextRepositorySource::Window
        && let Some(target_window) = resolved_session.target_window
    {
        win32_display::deactivate_window_text_session(binding, target_window, session)?;
    }

    let removed = binding.worker().resources.remove_and_finalize(
        binding.world(),
        session.0,
        Some(binding.engine()),
    );
    if !removed {
        return Err(text_session_not_found("destack.input.text.close", session));
    }

    Ok(())
}

/// Get text input area.
///
/// Return the currently configured text input geometry hint for one active text input session.
/// Returns `ioWouldBlock` until one renderer geometry hint has been supplied for the session.
pub(crate) unsafe fn destack_input_text_get_geometry(
    binding: &BindingCallContext,
    out: *mut InputTextGeometry,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let session = resolve_text_session(binding, session, "destack.input.text.getGeometry")?;
    let Some(geometry) = session.geometry else {
        return Err(core_platform::io_would_block(
            "destack.input.text.getGeometry",
            "text geometry is not available yet",
        ));
    };

    // output
    unsafe {
        *out = geometry;
    }

    Ok(())
}

/// Read one text session event.
///
/// Windows console backends currently emit committed insert intents from cooked text input.
pub(crate) unsafe fn destack_input_text_read_event(
    binding: &BindingCallContext,
    out: *mut InputTextSessionEvent,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let event = text_read_event(binding, session, false, "destack.input.text.readEvent")?;

    // output
    unsafe {
        *out = event;
    }

    Ok(())
}

/// Set text input area.
pub(crate) unsafe fn destack_input_text_set_geometry(
    binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
    area: InputTextGeometry,
) -> RuntimeResult<()> {
    let resolved_session =
        resolve_text_session(binding, session, "destack.input.text.setGeometry")?;
    if resolved_session.source == WindowsTextRepositorySource::Window
        && let (Some(target_window), Some(target_hwnd)) =
            (resolved_session.target_window, resolved_session.target_hwnd)
    {
        let runtime_state = win32_display::runtime_state(binding);
        win32_display::synchronize_window_text_session(
            &runtime_state,
            target_window,
            target_hwnd,
            Some(area),
        )?;
    }

    text_set_geometry(binding, session, area, "destack.input.text.setGeometry")?;

    Ok(())
}

/// Update one active text input session with renderer-owned state.
pub(crate) unsafe fn destack_input_text_set_state(
    binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
    state: InputTextSessionState,
) -> RuntimeResult<()> {
    let state = unsafe { state.into_value()? };

    let resolved_session = resolve_text_session(binding, session, "destack.input.text.setState")?;
    if resolved_session.source == WindowsTextRepositorySource::Window
        && let (Some(target_window), Some(target_hwnd)) =
            (resolved_session.target_window, resolved_session.target_hwnd)
    {
        let runtime_state = win32_display::runtime_state(binding);
        win32_display::synchronize_window_text_session(
            &runtime_state,
            target_window,
            target_hwnd,
            resolved_session.geometry,
        )?;
    }

    text_set_state(binding, session, state, "destack.input.text.setState")?;

    Ok(())
}

/// Poll one text session event without blocking.
///
/// Windows console backends currently emit committed insert intents from cooked text input.
pub(crate) unsafe fn destack_input_text_try_read_event(
    binding: &BindingCallContext,
    out: *mut InputTextSessionEvent,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let event = text_read_event(binding, session, true, "destack.input.text.tryReadEvent")?;

    // output
    unsafe {
        *out = event;
    }

    Ok(())
}
