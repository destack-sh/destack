use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

#[cfg(any(target_os = "android", target_os = "ios"))]
use super::super::PlatformInputState;
use super::core as input_core;

use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(any(target_os = "android", target_os = "ios"))]
use crate::host::HOST_STATUS_FAILED;
#[cfg(any(target_os = "android", target_os = "ios"))]
use crate::host::abi::text::{
    HostTextInputCloseRequest, HostTextInputConfiguration, HostTextInputGeometry,
    HostTextInputGeometryRequest, HostTextInputOpenRequest, HostTextInputState,
    HostTextInputStateRequest, HostTextInputType,
};
#[cfg(target_os = "android")]
use crate::host::os::android::abi::text::ffi::{
    destack_host_android_text_close, destack_host_android_text_open,
    destack_host_android_text_set_geometry, destack_host_android_text_set_state,
};
#[cfg(target_os = "ios")]
use crate::host::os::apple::abi::text::ffi::{
    destack_host_ios_text_close, destack_host_ios_text_open, destack_host_ios_text_set_geometry,
    destack_host_ios_text_set_state,
};
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(target_os = "macos")]
use crate::platform::display::appkit;
#[cfg(target_os = "linux")]
use crate::platform::display::wayland;
#[cfg(target_os = "linux")]
use crate::platform::display::x11;
use crate::platform::input::{
    InputClipboardCommandEventValue, InputClipboardCommandTypeValue, InputDeviceKind,
    InputEditIntentEventValue, InputEditIntentTypeValue, InputEvent, InputEventMetadataValue,
    InputReadMode, InputTextGeometry, InputTextRange, InputTextSessionConfig,
    InputTextSessionEvent, InputTextSessionEventValue, InputTextSessionState,
    InputTextSessionStateEventValue, InputTextSessionStateValue, validation as input_validation,
};
#[cfg(target_os = "linux")]
use crate::platform::input::{
    InputCompositionEventPayloadValue, InputCompositionEventValue, InputEventAction,
};
#[cfg(any(target_os = "android", target_os = "ios"))]
use crate::platform::resource::ResourceBacking;
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{NativeAbiCodec, PlatformError, resource};
use crate::runtime::BindingCallContext;
use parking_lot::{Condvar, Mutex};

#[cfg(any(target_os = "android", target_os = "ios"))]
use super::super::core::text::host_status_result;

/// Resource-table label for active text-session entries.
const TEXT_SESSION_RESOURCE_LABEL: &str = "input.text.session";
/// Stable device identifier used for AppKit-backed text-session events.
#[cfg(target_os = "macos")]
const APPKIT_WINDOW_TEXT_DEVICE_ID: &str = "appkit.window.text";
/// Stable device identifier used for Wayland-backed text-session events.
#[cfg(target_os = "linux")]
const WAYLAND_WINDOW_TEXT_DEVICE_ID: &str = "wayland.window.text";
/// Stable device identifier used for X11-backed text-session events.
#[cfg(target_os = "linux")]
const X11_WINDOW_TEXT_DEVICE_ID: &str = "x11.window.text";

/// Waitable queued-event signal for one text session.
#[derive(Debug, Default)]
struct TextRepositoryEventSignal {
    /// Monotonic wake generation for this session queue.
    generation: Mutex<u64>,
    /// Wake signal for queued session events.
    wake: Condvar,
}

/// Native event source for one Unix text session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnixTextRepositorySource {
    /// Terminal-backed cooked text input.
    UnixTerminal,
    /// Native AppKit window-backed text input.
    #[cfg(target_os = "macos")]
    AppKitWindow,
    /// Native Wayland window-backed text input.
    #[cfg(target_os = "linux")]
    WaylandWindow,
    /// Native X11 window-backed text input.
    #[cfg(target_os = "linux")]
    X11Window,
}

/// Stored text-session payload for one Unix host session.
#[derive(Debug)]
struct UnixTextRepository {
    /// The opened Unix text binding.
    binding: input_core::UnixInputBinding,
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
    /// The native event source used by this session.
    source: UnixTextRepositorySource,
    /// Pending queued session events from native host callbacks.
    events: VecDeque<InputTextSessionEventValue>,
    /// Waitable signal for queued session events.
    event_signal: Arc<TextRepositoryEventSignal>,
}

impl Clone for UnixTextRepository {
    fn clone(&self) -> Self {
        Self {
            binding: input_core::UnixInputBinding {
                descriptor: self.binding.descriptor,
                backend: self.binding.backend,
                read_mode: self.binding.read_mode,
                text_active: self.binding.text_active,
                text_input_type: self.binding.text_input_type,
                text_area: self.binding.text_area,
                gamepad_player_index_override: self.binding.gamepad_player_index_override,
                relative_mode_enabled: self.binding.relative_mode_enabled,
                last_pointer_x: self.binding.last_pointer_x,
                last_pointer_y: self.binding.last_pointer_y,
                sensor_enabled_kinds: self.binding.sensor_enabled_kinds.clone(),
                sensor_effective_configs: self.binding.sensor_effective_configs.clone(),
                device_id: self.binding.device_id.clone(),
                device_kind: self.binding.device_kind,
                next_sequence: self.binding.next_sequence,
                #[cfg(target_os = "linux")]
                linux_modifiers: self.binding.linux_modifiers,
                #[cfg(target_os = "linux")]
                linux_pointer_buttons: self.binding.linux_pointer_buttons,
                #[cfg(target_os = "linux")]
                linux_active_rumble_effect_id: self.binding.linux_active_rumble_effect_id,
                terminal_original_mode: self.binding.terminal_original_mode,
                #[cfg(target_os = "macos")]
                macos_state: None,
            },
            config: self.config,
            geometry: self.geometry,
            state: self.state.clone(),
            next_sequence: self.next_sequence,
            target_window: self.target_window,
            source: self.source,
            events: self.events.clone(),
            event_signal: Arc::clone(&self.event_signal),
        }
    }
}

/// Build io-would-block for one unavailable text-session state.
fn text_would_block(operation: &'static str, message: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoWouldBlock),
        None,
        Some(libc::EWOULDBLOCK),
        Some(operation.to_string()),
        None,
        message.to_string(),
    ))
    .boxed()
}

/// Build io-not-found for one missing Unix text session.
fn text_session_not_found(
    operation: &'static str,
    session: resource::InputTextSessionHandle,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(operation.to_string()),
        None,
        format!("text session {} not found", session.0.0),
    ))
    .boxed()
}

/// Build io-not-found for one missing explicit window target.
fn window_target_not_found(
    operation: &'static str,
    target: resource::WindowHandle,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(operation.to_string()),
        None,
        format!("window handle {} not found", target.0.0),
    ))
    .boxed()
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
    // keep the default process-scoped target when no explicit window is selected
    if !input_validation::has_explicit_window_target(config.target) {
        return Ok(None);
    }

    let target_window = config
        .target
        .window
        .expect("explicit window targets should carry one handle");
    let resolved = binding
        .worker()
        .resources
        .with_entry(target_window.0, |entry| {
            if entry.kind != ResourceKind::Window {
                return None;
            }

            Some(())
        });

    match resolved.flatten() {
        Some(()) => Ok(Some(target_window)),
        None => Err(window_target_not_found(operation, target_window)),
    }
}

/// Allocate one session event metadata payload.
fn next_text_event_metadata(
    session: &mut UnixTextRepository,
    timestamp_ns: u64,
) -> InputEventMetadataValue {
    let metadata = InputEventMetadataValue {
        timestamp_ns,
        sequence: session.next_sequence,
        device_id: session.binding.device_id.clone(),
        target_window: session.target_window,
    };

    session.next_sequence = session.next_sequence.wrapping_add(1);

    metadata
}

/// Build one committed text edit-intent event.
fn committed_text_edit_intent(
    session: &mut UnixTextRepository,
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

            let session = entry.payload_mut::<UnixTextRepository>()?;
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

/// Resolve one active Unix text session from the resource table.
fn resolve_text_session(
    binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
    operation: &'static str,
) -> RuntimeResult<UnixTextRepository> {
    let resolved = binding.worker().resources.with_entry(session.0, |entry| {
        if entry.kind != ResourceKind::InputTextSession {
            return None;
        }

        if entry.label.as_deref() != Some(TEXT_SESSION_RESOURCE_LABEL) {
            return None;
        }

        entry.payload_cloned::<UnixTextRepository>()
    });

    match resolved.flatten() {
        Some(session) => Ok(session),
        None => Err(text_session_not_found(operation, session)),
    }
}

/// Persist one text-area hint for one Unix text session.
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

            let session = entry.payload_mut::<UnixTextRepository>()?;
            session.binding.text_area = Some(area);
            session.geometry = Some(area);
            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(text_session_not_found(operation, session)),
    }
}

/// Persist renderer-owned text state for one Unix text session.
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

            let session = entry.payload_mut::<UnixTextRepository>()?;
            session.state = state;
            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(text_session_not_found(operation, session)),
    }
}

/// Queue one host-authoritative state event for one AppKit text session.
#[cfg(target_os = "macos")]
pub(crate) fn notify_appkit_window_text_state(
    runtime_state: &appkit::AppKitRuntimeState,
    window: resource::WindowHandle,
    state: InputTextSessionStateValue,
) -> RuntimeResult<()> {
    validate_text_session_state(&state)?;

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

            let session = entry.payload_mut::<UnixTextRepository>()?;
            session.state = state.clone();

            let event = InputTextSessionEventValue::InputTextSessionStateEvent(
                InputTextSessionStateEventValue {
                    kind: "stateChanged".to_string(),
                    metadata: InputEventMetadataValue {
                        timestamp_ns: crate::platform::core::monotonic_now_ns(),
                        sequence: session.next_sequence,
                        device_id: APPKIT_WINDOW_TEXT_DEVICE_ID.to_string(),
                        target_window: session.target_window,
                    },
                    state,
                },
            );

            session.next_sequence = session.next_sequence.wrapping_add(1);
            session.events.push_back(event);
            notify_text_session_event(&session.event_signal);
            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(text_session_not_found(
            "destack.input.text.notifyAppKitState",
            session_handle,
        )),
    }
}

/// Queue one edit-intent event for one AppKit text session.
#[cfg(target_os = "macos")]
pub(crate) fn notify_appkit_window_edit_intent(
    runtime_state: &appkit::AppKitRuntimeState,
    window: resource::WindowHandle,
    input_type: InputEditIntentTypeValue,
    data: Option<String>,
    is_composing: bool,
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

            let session = entry.payload_mut::<UnixTextRepository>()?;
            let event =
                InputTextSessionEventValue::InputEditIntentEvent(InputEditIntentEventValue {
                    kind: "editIntent".to_string(),
                    metadata: InputEventMetadataValue {
                        timestamp_ns: crate::platform::core::monotonic_now_ns(),
                        sequence: session.next_sequence,
                        device_id: APPKIT_WINDOW_TEXT_DEVICE_ID.to_string(),
                        target_window: session.target_window,
                    },
                    input_type,
                    data,
                    target_ranges: vec![session.state.selection],
                    clipboard_items: None,
                    drag: None,
                    is_composing,
                });

            session.next_sequence = session.next_sequence.wrapping_add(1);
            session.events.push_back(event);
            notify_text_session_event(&session.event_signal);
            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(text_session_not_found(
            "destack.input.text.notifyAppKitEditIntent",
            session_handle,
        )),
    }
}

/// Queue one clipboard-command event for one AppKit text session.
#[cfg(target_os = "macos")]
pub(crate) fn notify_appkit_window_clipboard_command(
    runtime_state: &appkit::AppKitRuntimeState,
    window: resource::WindowHandle,
    command: InputClipboardCommandTypeValue,
    is_composing: bool,
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

            let session = entry.payload_mut::<UnixTextRepository>()?;
            let event = InputTextSessionEventValue::InputClipboardCommandEvent(
                InputClipboardCommandEventValue {
                    kind: "clipboardCommand".to_string(),
                    metadata: InputEventMetadataValue {
                        timestamp_ns: crate::platform::core::monotonic_now_ns(),
                        sequence: session.next_sequence,
                        device_id: APPKIT_WINDOW_TEXT_DEVICE_ID.to_string(),
                        target_window: session.target_window,
                    },
                    command,
                    clipboard_items: None,
                    is_composing,
                },
            );

            session.next_sequence = session.next_sequence.wrapping_add(1);
            session.events.push_back(event);
            notify_text_session_event(&session.event_signal);
            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(text_session_not_found(
            "destack.input.text.notifyAppKitClipboardCommand",
            session_handle,
        )),
    }
}

/// Resolve the active AppKit window text session and configuration.
#[cfg(target_os = "macos")]
pub(crate) fn resolve_appkit_window_text_session(
    runtime_state: &appkit::AppKitRuntimeState,
    window: resource::WindowHandle,
) -> RuntimeResult<
    Option<(
        resource::InputTextSessionHandle,
        InputTextSessionConfig,
        InputTextSessionStateValue,
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

            let session = entry.payload_ref::<UnixTextRepository>()?;
            Some((session_handle, session.config, session.state.clone()))
        });

    match session.flatten() {
        Some(session) => Ok(Some(session)),
        None => Err(text_session_not_found(
            "destack.input.text.resolveAppKitWindowRepository",
            session_handle,
        )),
    }
}

/// Queue one composition event for one Wayland text session.
#[cfg(target_os = "linux")]
pub(crate) fn notify_wayland_window_composition_event(
    runtime_state: &wayland::WaylandRuntimeState,
    window: resource::WindowHandle,
    action: InputEventAction,
    text: String,
    selection_start: i32,
    selection_end: i32,
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

            let session = entry.payload_mut::<UnixTextRepository>()?;
            let event =
                InputTextSessionEventValue::InputCompositionEvent(InputCompositionEventValue {
                    kind: "composition".to_string(),
                    metadata: InputEventMetadataValue {
                        timestamp_ns: crate::platform::core::monotonic_now_ns(),
                        sequence: session.next_sequence,
                        device_id: WAYLAND_WINDOW_TEXT_DEVICE_ID.to_string(),
                        target_window: session.target_window,
                    },
                    payload: InputCompositionEventPayloadValue {
                        action,
                        text,
                        selection_start,
                        selection_end,
                    },
                });

            session.next_sequence = session.next_sequence.wrapping_add(1);
            session.events.push_back(event);
            notify_text_session_event(&session.event_signal);
            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(text_session_not_found(
            "destack.input.text.notifyWaylandComposition",
            session_handle,
        )),
    }
}

/// Queue one composition-end event for one Wayland text session.
#[cfg(target_os = "linux")]
pub(crate) fn notify_wayland_window_end_composition(
    runtime_state: &wayland::WaylandRuntimeState,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    notify_wayland_window_composition_event(
        runtime_state,
        window,
        InputEventAction::End,
        String::new(),
        -1,
        -1,
    )
}

/// Queue one edit-intent event for one Wayland text session.
#[cfg(target_os = "linux")]
pub(crate) fn notify_wayland_window_edit_intent(
    runtime_state: &wayland::WaylandRuntimeState,
    window: resource::WindowHandle,
    input_type: InputEditIntentTypeValue,
    data: Option<String>,
    target_ranges: Vec<InputTextRange>,
    is_composing: bool,
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

            let session = entry.payload_mut::<UnixTextRepository>()?;
            let event =
                InputTextSessionEventValue::InputEditIntentEvent(InputEditIntentEventValue {
                    kind: "editIntent".to_string(),
                    metadata: InputEventMetadataValue {
                        timestamp_ns: crate::platform::core::monotonic_now_ns(),
                        sequence: session.next_sequence,
                        device_id: WAYLAND_WINDOW_TEXT_DEVICE_ID.to_string(),
                        target_window: session.target_window,
                    },
                    input_type,
                    data,
                    target_ranges,
                    clipboard_items: None,
                    drag: None,
                    is_composing,
                });

            session.next_sequence = session.next_sequence.wrapping_add(1);
            session.events.push_back(event);
            notify_text_session_event(&session.event_signal);
            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(text_session_not_found(
            "destack.input.text.notifyWaylandEditIntent",
            session_handle,
        )),
    }
}

/// Resolve the active Wayland window text session and configuration.
#[cfg(target_os = "linux")]
pub(crate) fn resolve_wayland_window_text_session(
    runtime_state: &wayland::WaylandRuntimeState,
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

            let session = entry.payload_ref::<UnixTextRepository>()?;
            Some((
                session_handle,
                session.config,
                session.state.clone(),
                session.geometry,
            ))
        });

    match session.flatten() {
        Some(session) => Ok(Some(session)),
        None => Err(text_session_not_found(
            "destack.input.text.resolveWaylandWindowRepository",
            session_handle,
        )),
    }
}

/// Queue one edit-intent event for one X11 text session.
#[cfg(target_os = "linux")]
pub(crate) fn notify_x11_window_composition_event(
    runtime_state: &x11::X11RuntimeState,
    window: resource::WindowHandle,
    action: InputEventAction,
    text: String,
    selection_start: i32,
    selection_end: i32,
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

            let session = entry.payload_mut::<UnixTextRepository>()?;
            let event =
                InputTextSessionEventValue::InputCompositionEvent(InputCompositionEventValue {
                    kind: "composition".to_string(),
                    metadata: InputEventMetadataValue {
                        timestamp_ns: crate::platform::core::monotonic_now_ns(),
                        sequence: session.next_sequence,
                        device_id: X11_WINDOW_TEXT_DEVICE_ID.to_string(),
                        target_window: session.target_window,
                    },
                    payload: InputCompositionEventPayloadValue {
                        action,
                        text,
                        selection_start,
                        selection_end,
                    },
                });

            session.next_sequence = session.next_sequence.wrapping_add(1);
            session.events.push_back(event);
            notify_text_session_event(&session.event_signal);
            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(text_session_not_found(
            "destack.input.text.notifyX11Composition",
            session_handle,
        )),
    }
}

/// Queue one composition-end event for one X11 text session.
#[cfg(target_os = "linux")]
pub(crate) fn notify_x11_window_end_composition(
    runtime_state: &x11::X11RuntimeState,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    notify_x11_window_composition_event(
        runtime_state,
        window,
        InputEventAction::End,
        String::new(),
        -1,
        -1,
    )
}

/// Queue one edit-intent event for one X11 text session.
#[cfg(target_os = "linux")]
pub(crate) fn notify_x11_window_edit_intent(
    runtime_state: &x11::X11RuntimeState,
    window: resource::WindowHandle,
    input_type: InputEditIntentTypeValue,
    data: Option<String>,
    target_ranges: Vec<InputTextRange>,
    is_composing: bool,
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

            let session = entry.payload_mut::<UnixTextRepository>()?;
            let event =
                InputTextSessionEventValue::InputEditIntentEvent(InputEditIntentEventValue {
                    kind: "editIntent".to_string(),
                    metadata: InputEventMetadataValue {
                        timestamp_ns: crate::platform::core::monotonic_now_ns(),
                        sequence: session.next_sequence,
                        device_id: X11_WINDOW_TEXT_DEVICE_ID.to_string(),
                        target_window: session.target_window,
                    },
                    input_type,
                    data,
                    target_ranges,
                    clipboard_items: None,
                    drag: None,
                    is_composing,
                });

            session.next_sequence = session.next_sequence.wrapping_add(1);
            session.events.push_back(event);
            notify_text_session_event(&session.event_signal);
            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(text_session_not_found(
            "destack.input.text.notifyX11EditIntent",
            session_handle,
        )),
    }
}

/// Queue one clipboard-command event for one X11 text session.
#[cfg(target_os = "linux")]
pub(crate) fn notify_x11_window_clipboard_command(
    runtime_state: &x11::X11RuntimeState,
    window: resource::WindowHandle,
    command: InputClipboardCommandTypeValue,
    is_composing: bool,
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

            let session = entry.payload_mut::<UnixTextRepository>()?;
            let event = InputTextSessionEventValue::InputClipboardCommandEvent(
                InputClipboardCommandEventValue {
                    kind: "clipboardCommand".to_string(),
                    metadata: InputEventMetadataValue {
                        timestamp_ns: crate::platform::core::monotonic_now_ns(),
                        sequence: session.next_sequence,
                        device_id: X11_WINDOW_TEXT_DEVICE_ID.to_string(),
                        target_window: session.target_window,
                    },
                    command,
                    clipboard_items: None,
                    is_composing,
                },
            );

            session.next_sequence = session.next_sequence.wrapping_add(1);
            session.events.push_back(event);
            notify_text_session_event(&session.event_signal);
            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(text_session_not_found(
            "destack.input.text.notifyX11ClipboardCommand",
            session_handle,
        )),
    }
}

/// Resolve the active X11 window text session and configuration.
#[cfg(target_os = "linux")]
pub(crate) fn resolve_x11_window_text_session(
    runtime_state: &x11::X11RuntimeState,
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

            let session = entry.payload_ref::<UnixTextRepository>()?;
            Some((
                session_handle,
                session.config,
                session.state.clone(),
                session.geometry,
            ))
        });

    match session.flatten() {
        Some(session) => Ok(Some(session)),
        None => Err(text_session_not_found(
            "destack.input.text.resolveX11WindowRepository",
            session_handle,
        )),
    }
}

/// Read one text session event for one Unix text session.
fn read_text_session_event(
    binding: &BindingCallContext,
    session_handle: resource::InputTextSessionHandle,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputTextSessionEvent> {
    let event = binding
        .worker()
        .resources
        .with_entry_mut(session_handle.0, |entry| {
            if entry.kind != ResourceKind::InputTextSession {
                return None;
            }

            if entry.label.as_deref() != Some(TEXT_SESSION_RESOURCE_LABEL) {
                return None;
            }

            let session = entry.payload_mut::<UnixTextRepository>()?;

            // require cooked mode so terminal bytes are decoded as text payloads
            if session.binding.read_mode != InputReadMode::Cooked {
                return Some(Err(RuntimeError::from(PlatformError::not_supported(
                    operation,
                ))
                .boxed()));
            }

            // require one active text session before composition reads
            if !session.binding.text_active {
                return Some(Err(text_would_block(
                    operation,
                    "text session is not active",
                )));
            }

            // read native queued events directly for window-backed sessions
            #[cfg(target_os = "macos")]
            let is_window_session = session.source == UnixTextRepositorySource::AppKitWindow;
            #[cfg(target_os = "linux")]
            let is_window_session = matches!(
                session.source,
                UnixTextRepositorySource::WaylandWindow | UnixTextRepositorySource::X11Window
            );
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            let is_window_session = false;

            if is_window_session {
                let event = if nonblocking {
                    pop_queued_text_session_event(binding, session_handle, operation)
                } else {
                    wait_for_text_session_event(binding, session_handle, operation).map(Some)
                };

                return Some(event);
            }

            let Some(descriptor) = session.binding.descriptor else {
                return Some(Err(RuntimeError::from(PlatformError::io_with(
                    Some(PlatformErrorCode::IoNotFound),
                    None,
                    None,
                    Some(operation.to_string()),
                    None,
                    "text session is missing one terminal descriptor".to_string(),
                ))
                .boxed()));
            };

            // skip non-text terminal events until one committed text payload arrives
            loop {
                let event = input_core::read_terminal_event(
                    binding,
                    descriptor,
                    &session.binding.device_id,
                    nonblocking,
                    session.binding.read_mode,
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

                    return Some(Ok(Some(InputTextSessionEvent::from_value(binding, event))));
                }
            }
        });

    match event {
        Some(Some(Ok(Some(event)))) => Ok(event),
        Some(Some(Ok(None))) => Err(text_would_block(
            operation,
            "text session has no queued event",
        )),
        Some(Some(Err(error))) => Err(error),
        Some(None) | None => Err(text_session_not_found(operation, session_handle)),
    }
}

/// Return the shared `platform.input` state for this binding.
#[cfg(any(target_os = "android", target_os = "ios"))]
fn input_state(binding: &BindingCallContext) -> RuntimeResult<&PlatformInputState> {
    let state = &binding.worker().platform_state.input;
    state.bootstrap_host_text_state(binding)?;

    Ok(state)
}

/// Validate one attached-host text-open request.
#[cfg(any(target_os = "android", target_os = "ios"))]
fn validate_host_text_open(
    config: InputTextSessionConfig,
    state: &InputTextSessionStateValue,
) -> RuntimeResult<()> {
    // host-backed mobile text follows one focused host target
    if input_validation::has_explicit_window_target(config.target) {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.input.text.open")).boxed(),
        );
    }

    validate_text_session_state(state)
}

/// Call the target host text-open callback.
#[cfg(any(target_os = "android", target_os = "ios"))]
unsafe fn host_text_open(session_handle: u64, request: HostTextInputOpenRequest) -> u32 {
    #[cfg(target_os = "android")]
    {
        return unsafe { destack_host_android_text_open(session_handle, request) };
    }

    #[cfg(target_os = "ios")]
    {
        return unsafe { destack_host_ios_text_open(session_handle, request) };
    }

    #[allow(unreachable_code)]
    HOST_STATUS_FAILED
}

/// Call the target host text-close callback.
#[cfg(any(target_os = "android", target_os = "ios"))]
pub(crate) unsafe fn host_text_close(
    session_handle: u64,
    request: HostTextInputCloseRequest,
) -> u32 {
    #[cfg(target_os = "android")]
    {
        return unsafe { destack_host_android_text_close(session_handle, request) };
    }

    #[cfg(target_os = "ios")]
    {
        return unsafe { destack_host_ios_text_close(session_handle, request) };
    }

    #[allow(unreachable_code)]
    HOST_STATUS_FAILED
}

/// Call the target host text-geometry callback.
#[cfg(any(target_os = "android", target_os = "ios"))]
unsafe fn host_text_set_geometry(
    session_handle: u64,
    request: crate::host::abi::text::HostTextInputGeometryRequest,
) -> u32 {
    #[cfg(target_os = "android")]
    {
        return unsafe { destack_host_android_text_set_geometry(session_handle, request) };
    }

    #[cfg(target_os = "ios")]
    {
        return unsafe { destack_host_ios_text_set_geometry(session_handle, request) };
    }

    #[allow(unreachable_code)]
    HOST_STATUS_FAILED
}

/// Call the target host text-state callback.
#[cfg(any(target_os = "android", target_os = "ios"))]
unsafe fn host_text_set_state(session_handle: u64, request: HostTextInputStateRequest) -> u32 {
    #[cfg(target_os = "android")]
    {
        return unsafe { destack_host_android_text_set_state(session_handle, request) };
    }

    #[cfg(target_os = "ios")]
    {
        return unsafe { destack_host_ios_text_set_state(session_handle, request) };
    }

    #[allow(unreachable_code)]
    HOST_STATUS_FAILED
}

/// Open one text input session.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
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

    // wayland window lane
    #[cfg(target_os = "linux")]
    if let Some(target_window) = target_window {
        let (device_id, source) = if wayland::ensure_window_handle_exists(
            binding,
            target_window,
            "destack.input.text.open",
        )
        .is_ok()
        {
            (
                WAYLAND_WINDOW_TEXT_DEVICE_ID.to_string(),
                UnixTextRepositorySource::WaylandWindow,
            )
        } else if x11::ensure_window_handle_exists(
            binding,
            target_window,
            "destack.input.text.open",
        )
        .is_ok()
        {
            (
                X11_WINDOW_TEXT_DEVICE_ID.to_string(),
                UnixTextRepositorySource::X11Window,
            )
        } else {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.input.text.open",
            ))
            .boxed());
        };

        let session = UnixTextRepository {
            binding: input_core::UnixInputBinding {
                descriptor: None,
                backend: input_core::UnixInputBackend::Platform,
                read_mode: InputReadMode::Cooked,
                text_active: true,
                text_input_type: config.input_type,
                text_area: None,
                gamepad_player_index_override: None,
                relative_mode_enabled: false,
                last_pointer_x: 0.0,
                last_pointer_y: 0.0,
                sensor_enabled_kinds: HashSet::new(),
                sensor_effective_configs: HashMap::new(),
                device_id,
                device_kind: InputDeviceKind::Keyboard,
                next_sequence: 1,
                #[cfg(target_os = "linux")]
                linux_modifiers: 0,
                #[cfg(target_os = "linux")]
                linux_pointer_buttons: 0,
                #[cfg(target_os = "linux")]
                linux_active_rumble_effect_id: None,
                terminal_original_mode: None,
                #[cfg(target_os = "macos")]
                macos_state: input_core::initial_macos_state(
                    input_core::UnixInputBackend::Platform,
                ),
            },
            config,
            geometry: None,
            state: state.clone(),
            next_sequence: 1,
            target_window: Some(target_window),
            source,
            events: VecDeque::new(),
            event_signal: Arc::new(TextRepositoryEventSignal::default()),
        };
        let entry = ResourceEntry::new(ResourceKind::InputTextSession)
            .with_label(TEXT_SESSION_RESOURCE_LABEL)
            .with_payload(session);
        let resource_id =
            binding
                .worker()
                .resources
                .insert(&binding.world(), entry, Some(binding.engine()));
        let session_handle = resource::InputTextSessionHandle(resource_id);

        let activate = match source {
            UnixTextRepositorySource::WaylandWindow => wayland::activate_window_text_session(
                binding,
                target_window,
                session_handle,
                config,
                state,
                None,
            ),
            UnixTextRepositorySource::X11Window => x11::activate_window_text_session(
                binding,
                target_window,
                session_handle,
                config,
                state,
                None,
            ),
            UnixTextRepositorySource::UnixTerminal => unreachable!(),
            #[cfg(target_os = "macos")]
            UnixTextRepositorySource::AppKitWindow => unreachable!(),
        };
        if let Err(error) = activate {
            let _ = binding.worker().resources.remove_and_finalize(
                &binding.world(),
                session_handle.0,
                Some(binding.engine()),
            );
            return Err(error);
        }

        unsafe {
            *out = session_handle;
        }
        return Ok(());
    }

    // appkit window lane
    #[cfg(target_os = "macos")]
    if let Some(target_window) = target_window {
        let runtime_state = appkit::runtime_state(binding);
        let session = UnixTextRepository {
            binding: input_core::UnixInputBinding {
                descriptor: None,
                backend: input_core::UnixInputBackend::Platform,
                read_mode: InputReadMode::Cooked,
                text_active: true,
                text_input_type: config.input_type,
                text_area: None,
                gamepad_player_index_override: None,
                relative_mode_enabled: false,
                last_pointer_x: 0.0,
                last_pointer_y: 0.0,
                sensor_enabled_kinds: HashSet::new(),
                sensor_effective_configs: HashMap::new(),
                device_id: APPKIT_WINDOW_TEXT_DEVICE_ID.to_string(),
                device_kind: InputDeviceKind::Keyboard,
                next_sequence: 1,
                #[cfg(target_os = "linux")]
                linux_modifiers: 0,
                #[cfg(target_os = "linux")]
                linux_pointer_buttons: 0,
                #[cfg(target_os = "linux")]
                linux_active_rumble_effect_id: None,
                terminal_original_mode: None,
                #[cfg(target_os = "macos")]
                macos_state: input_core::initial_macos_state(
                    input_core::UnixInputBackend::Platform,
                ),
            },
            config,
            geometry: None,
            state: state.clone(),
            next_sequence: 1,
            target_window: Some(target_window),
            source: UnixTextRepositorySource::AppKitWindow,
            events: VecDeque::new(),
            event_signal: Arc::new(TextRepositoryEventSignal::default()),
        };
        let entry = ResourceEntry::new(ResourceKind::InputTextSession)
            .with_label(TEXT_SESSION_RESOURCE_LABEL)
            .with_payload(session);
        let resource_id =
            binding
                .worker()
                .resources
                .insert(&binding.world(), entry, Some(binding.engine()));
        let session_handle = resource::InputTextSessionHandle(resource_id);

        let activate = appkit::activate_window_text_session(
            &runtime_state,
            target_window,
            session_handle,
            config,
            state,
            None,
        );
        if let Err(error) = activate {
            let _ = binding.worker().resources.remove_and_finalize(
                &binding.world(),
                session_handle.0,
                Some(binding.engine()),
            );
            return Err(error);
        }

        unsafe {
            *out = session_handle;
        }
        return Ok(());
    }

    // tty binding
    let spec = input_core::normalize_unix_input_spec(input_core::UNIX_INPUT_TTY_ID)?;
    if spec.backend != input_core::UnixInputBackend::UnixTerminal {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.input.text.open")).boxed(),
        );
    }

    let descriptor = if spec.path.is_empty() {
        None
    } else {
        Some(input_core::open_input_descriptor(&spec.path)?)
    };
    let terminal_original_mode = match descriptor {
        Some(descriptor) => Some(input_core::read_terminal_mode(descriptor)?),
        None => None,
    };

    let session = UnixTextRepository {
        binding: input_core::UnixInputBinding {
            descriptor,
            backend: spec.backend,
            read_mode: InputReadMode::Cooked,
            text_active: true,
            text_input_type: config.input_type,
            text_area: None,
            gamepad_player_index_override: None,
            relative_mode_enabled: false,
            last_pointer_x: 0.0,
            last_pointer_y: 0.0,
            sensor_enabled_kinds: HashSet::new(),
            sensor_effective_configs: HashMap::new(),
            device_id: spec.device_id,
            device_kind: InputDeviceKind::Keyboard,
            next_sequence: 1,
            #[cfg(target_os = "linux")]
            linux_modifiers: 0,
            #[cfg(target_os = "linux")]
            linux_pointer_buttons: 0,
            #[cfg(target_os = "linux")]
            linux_active_rumble_effect_id: None,
            terminal_original_mode,
            #[cfg(target_os = "macos")]
            macos_state: input_core::initial_macos_state(spec.backend),
        },
        config,
        geometry: None,
        state,
        next_sequence: 1,
        target_window,
        source: UnixTextRepositorySource::UnixTerminal,
        events: VecDeque::new(),
        event_signal: Arc::new(TextRepositoryEventSignal::default()),
    };

    let entry = ResourceEntry::new(ResourceKind::InputTextSession)
        .with_label(TEXT_SESSION_RESOURCE_LABEL)
        .with_payload(session);
    let entry = if let Some(descriptor) = descriptor {
        entry.with_finalizer(input_core::InputDeviceFinalizer {
            fd: descriptor,
            restore_terminal_mode: terminal_original_mode,
        })
    } else {
        entry
    };
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));

    // output
    unsafe {
        *out = resource::InputTextSessionHandle(resource_id);
    }

    Ok(())
}

/// Close one text input session.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub(crate) unsafe fn destack_input_text_close(
    binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    // existence
    let resolved = resolve_text_session(binding, session, "destack.input.text.close")?;

    // detach one active appkit window session before the resource disappears
    #[cfg(target_os = "macos")]
    if resolved.source == UnixTextRepositorySource::AppKitWindow
        && let Some(target_window) = resolved.target_window
    {
        let runtime_state = appkit::runtime_state(binding);
        appkit::deactivate_window_text_session(&runtime_state, target_window, session)?;
    }

    // detach one active wayland window session before the resource disappears
    #[cfg(target_os = "linux")]
    if resolved.source == UnixTextRepositorySource::WaylandWindow
        && let Some(target_window) = resolved.target_window
    {
        wayland::deactivate_window_text_session(binding, target_window, session)?;
    }

    #[cfg(target_os = "linux")]
    if resolved.source == UnixTextRepositorySource::X11Window
        && let Some(target_window) = resolved.target_window
    {
        x11::deactivate_window_text_session(binding, target_window, session)?;
    }

    // remove and finalize
    let removed = binding.worker().resources.remove_and_finalize(
        &binding.world(),
        session.0,
        Some(binding.engine()),
    );
    if !removed {
        return Err(text_session_not_found("destack.input.text.close", session));
    }

    Ok(())
}

/// Get text input area.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
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
        return Err(text_would_block(
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

/// Read one pending text session event for one active text input session.
/// Unix terminal backends currently emit committed insert intents from cooked text input.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub(crate) unsafe fn destack_input_text_read_event(
    binding: &BindingCallContext,
    out: *mut InputTextSessionEvent,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let event = read_text_session_event(binding, session, false, "destack.input.text.readEvent")?;

    // output
    unsafe {
        *out = event;
    }

    Ok(())
}

/// Set one text input area and cursor position hint for one active text input session.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub(crate) unsafe fn destack_input_text_set_geometry(
    binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
    area: InputTextGeometry,
) -> RuntimeResult<()> {
    #[cfg(target_os = "macos")]
    {
        let resolved = resolve_text_session(binding, session, "destack.input.text.setGeometry")?;
        if resolved.source == UnixTextRepositorySource::AppKitWindow
            && let Some(target_window) = resolved.target_window
        {
            let runtime_state = appkit::runtime_state(binding);
            appkit::synchronize_window_text_session(
                &runtime_state,
                target_window,
                resolved.config,
                resolved.state,
                Some(area),
            )?;
        }
    }

    #[cfg(target_os = "linux")]
    {
        let resolved = resolve_text_session(binding, session, "destack.input.text.setGeometry")?;
        if resolved.source == UnixTextRepositorySource::WaylandWindow
            && let Some(target_window) = resolved.target_window
        {
            wayland::synchronize_window_text_session(
                binding,
                target_window,
                resolved.config,
                resolved.state,
                Some(area),
            )?;
        }

        if resolved.source == UnixTextRepositorySource::X11Window
            && let Some(target_window) = resolved.target_window
        {
            x11::synchronize_window_text_session(
                binding,
                target_window,
                resolved.config,
                resolved.state,
                Some(area),
            )?;
        }
    }

    text_set_geometry(binding, session, area, "destack.input.text.setGeometry")?;

    Ok(())
}

/// Update one active text input session with renderer-owned state.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub(crate) unsafe fn destack_input_text_set_state(
    binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
    state: InputTextSessionState,
) -> RuntimeResult<()> {
    let state = unsafe { state.into_value()? };

    #[cfg(target_os = "macos")]
    {
        let resolved = resolve_text_session(binding, session, "destack.input.text.setState")?;
        if resolved.source == UnixTextRepositorySource::AppKitWindow
            && let Some(target_window) = resolved.target_window
        {
            let runtime_state = appkit::runtime_state(binding);
            appkit::synchronize_window_text_session(
                &runtime_state,
                target_window,
                resolved.config,
                state.clone(),
                resolved.geometry,
            )?;
        }
    }

    #[cfg(target_os = "linux")]
    {
        let resolved = resolve_text_session(binding, session, "destack.input.text.setState")?;
        if resolved.source == UnixTextRepositorySource::WaylandWindow
            && let Some(target_window) = resolved.target_window
        {
            wayland::synchronize_window_text_session(
                binding,
                target_window,
                resolved.config,
                state.clone(),
                resolved.geometry,
            )?;
        }

        if resolved.source == UnixTextRepositorySource::X11Window
            && let Some(target_window) = resolved.target_window
        {
            x11::synchronize_window_text_session(
                binding,
                target_window,
                resolved.config,
                state.clone(),
                resolved.geometry,
            )?;
        }
    }

    text_set_state(binding, session, state, "destack.input.text.setState")?;

    Ok(())
}

/// Poll one pending text session event for one active text input session.
/// Unix terminal backends currently emit committed insert intents from cooked text input.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub(crate) unsafe fn destack_input_text_try_read_event(
    binding: &BindingCallContext,
    out: *mut InputTextSessionEvent,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let event = read_text_session_event(binding, session, true, "destack.input.text.tryReadEvent")?;

    // output
    unsafe {
        *out = event;
    }

    Ok(())
}

/// Open one text input session.
#[cfg(any(target_os = "android", target_os = "ios"))]
pub(crate) unsafe fn destack_input_text_open(
    binding: &BindingCallContext,
    out: *mut resource::InputTextSessionHandle,
    config: InputTextSessionConfig,
    state: InputTextSessionState,
) -> RuntimeResult<()> {
    // output
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // state
    let state = unsafe { state.into_value()? };
    validate_host_text_open(config, &state)?;

    let state_store = input_state(binding)?;
    let host_session_id = binding.host().host_session_id().0;

    // resource
    let entry = ResourceEntry::new(ResourceKind::InputTextSession)
        .with_label(TEXT_SESSION_RESOURCE_LABEL)
        .with_backing(ResourceBacking::Host);
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));
    let session = resource::InputTextSessionHandle(resource_id);
    let session_id = resource_id.0;

    // runtime state
    state_store.insert_host_text_session(session_id, config.target.window, state.clone());

    // cleanup
    binding
        .worker()
        .resources
        .with_entry_mut(resource_id, |entry| {
            entry.finalizer =
                Some(state_store.host_text_session_finalizer(host_session_id, session_id));
        });

    // host open
    let request = HostTextInputOpenRequest {
        configuration: HostTextInputConfiguration {
            session_id,
            input_type: <HostTextInputType as NativeAbiCodec>::from_value(
                binding,
                config.input_type,
            ),
            is_multiline: config.is_multiline,
            is_secure: config.is_secure,
        },
        state: <HostTextInputState as NativeAbiCodec>::from_value(binding, state.clone()),
    };
    let status = unsafe { host_text_open(host_session_id, request) };
    if let Err(error) = host_status_result(status, "destack.input.text.open", "open") {
        state_store.remove_host_text_session(session_id);
        let _ = binding.worker().resources.remove(
            &binding.world(),
            resource_id,
            Some(binding.engine()),
        );
        return Err(error);
    }

    unsafe {
        *out = session;
    }

    Ok(())
}

/// Close one text input session.
#[cfg(any(target_os = "android", target_os = "ios"))]
pub(crate) unsafe fn destack_input_text_close(
    binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    let exists = binding.worker().resources.with_entry(session.0, |entry| {
        entry.kind == ResourceKind::InputTextSession
            && entry.label.as_deref() == Some(TEXT_SESSION_RESOURCE_LABEL)
    });
    if exists != Some(true) {
        return Err(text_session_not_found("destack.input.text.close", session));
    }

    let state_store = input_state(binding)?;
    let host_session_id = binding.host().host_session_id().0;
    let request = HostTextInputCloseRequest {
        session_id: session.0.0,
    };
    let status = unsafe { host_text_close(host_session_id, request) };
    host_status_result(status, "destack.input.text.close", "close")?;

    if let Some(queue) = state_store.remove_host_text_session(session.0.0) {
        queue.close();
    }

    let _ = binding
        .worker()
        .resources
        .remove(&binding.world(), session.0, Some(binding.engine()));

    Ok(())
}

/// Get text input area.
#[cfg(any(target_os = "android", target_os = "ios"))]
pub(crate) unsafe fn destack_input_text_get_geometry(
    binding: &BindingCallContext,
    out: *mut InputTextGeometry,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let state = input_state(binding)?;
    let Some(geometry) = state.host_text_session_geometry(session.0.0) else {
        return Err(RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoWouldBlock),
            None,
            Some(libc::EWOULDBLOCK),
            Some("destack.input.text.getGeometry".to_string()),
            None,
            "text geometry is not available yet".to_string(),
        ))
        .boxed());
    };

    unsafe {
        *out = geometry;
    }

    Ok(())
}

/// Read one pending text session event for one active text input session.
#[cfg(any(target_os = "android", target_os = "ios"))]
pub(crate) unsafe fn destack_input_text_read_event(
    binding: &BindingCallContext,
    out: *mut InputTextSessionEvent,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    binding.advance_wait_progress()?;

    let state_store = input_state(binding)?;
    let Some(queue) = state_store.host_text_session_queue(session.0.0) else {
        return Err(text_session_not_found(
            "destack.input.text.readEvent",
            session,
        ));
    };

    let event = binding.wait_for_binding_result(
        "destack.input.text.readEvent",
        "timed out waiting for text-session event",
        u64::MAX,
        || {
            if queue.is_closed() {
                return Err(text_session_not_found(
                    "destack.input.text.readEvent",
                    session,
                ));
            }

            Ok(queue.try_take())
        },
        |duration| queue.wait_once(duration),
    )?;
    let event = InputTextSessionEvent::from_value(binding, event);

    unsafe {
        *out = event;
    }

    Ok(())
}

/// Set one text input area and cursor position hint for one active text input session.
#[cfg(any(target_os = "android", target_os = "ios"))]
pub(crate) unsafe fn destack_input_text_set_geometry(
    binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
    area: InputTextGeometry,
) -> RuntimeResult<()> {
    let state_store = input_state(binding)?;
    let host_session_id = binding.host().host_session_id().0;

    let request = HostTextInputGeometryRequest {
        session_id: session.0.0,
        geometry: <HostTextInputGeometry as NativeAbiCodec>::from_value(binding, area),
    };
    let status = unsafe { host_text_set_geometry(host_session_id, request) };
    host_status_result(status, "destack.input.text.setGeometry", "set geometry")?;

    state_store.set_host_text_session_geometry(session.0.0, area)?;

    Ok(())
}

/// Update one active text input session with renderer-owned state.
#[cfg(any(target_os = "android", target_os = "ios"))]
pub(crate) unsafe fn destack_input_text_set_state(
    binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
    state: InputTextSessionState,
) -> RuntimeResult<()> {
    let state_store = input_state(binding)?;
    let host_session_id = binding.host().host_session_id().0;
    let state = unsafe { state.into_value()? };
    validate_text_session_state(&state)?;

    let request = HostTextInputStateRequest {
        session_id: session.0.0,
        state: <HostTextInputState as NativeAbiCodec>::from_value(binding, state.clone()),
    };
    let status = unsafe { host_text_set_state(host_session_id, request) };
    host_status_result(status, "destack.input.text.setState", "set state")?;

    state_store.set_host_text_session_state(session.0.0, state)?;

    Ok(())
}

/// Poll one pending text session event for one active text input session.
#[cfg(any(target_os = "android", target_os = "ios"))]
pub(crate) unsafe fn destack_input_text_try_read_event(
    binding: &BindingCallContext,
    out: *mut InputTextSessionEvent,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    binding.advance_wait_progress()?;

    let state_store = input_state(binding)?;
    let Some(queue) = state_store.host_text_session_queue(session.0.0) else {
        return Err(text_session_not_found(
            "destack.input.text.tryReadEvent",
            session,
        ));
    };
    let Some(event) = queue.try_take() else {
        return Err(text_would_block(
            "destack.input.text.tryReadEvent",
            "no pending text-session event is available",
        ));
    };
    let event = InputTextSessionEvent::from_value(binding, event);

    unsafe {
        *out = event;
    }

    Ok(())
}
