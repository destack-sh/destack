use std::sync::Arc;

use wayland_client::backend::ObjectId;
use wayland_client::protocol::wl_surface;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, delegate_noop};
use wayland_protocols::wp::text_input::zv3::client::zwp_text_input_manager_v3;
use wayland_protocols::wp::text_input::zv3::client::zwp_text_input_v3::{
    self, ContentHint, ContentPurpose, Event as TextInputEvent,
};

use super::{
    WaylandConnectionDispatchState, WaylandRuntimeState, runtime_state, with_connection_dispatch,
};
use crate::diagnostic::RuntimeResult;
use crate::platform::display::unix::wayland::resource::ensure_window_handle_exists;
use crate::platform::display::unix::wayland::window::resolve_window_host_state;
use crate::platform::input::host::{
    notify_wayland_window_composition_event, notify_wayland_window_edit_intent,
    notify_wayland_window_end_composition, resolve_wayland_window_text_session,
};
use crate::platform::input::{
    InputEditIntentTypeValue, InputEventAction, InputTextGeometry, InputTextInputType,
    InputTextRange, InputTextRectangle, InputTextSessionConfig, InputTextSessionStateValue,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

/// Project one local rectangle into one target-space rectangle.
fn project_target_rectangle(
    geometry: InputTextGeometry,
    rectangle: InputTextRectangle,
) -> InputTextRectangle {
    let transform = geometry.local_to_target_transform;

    let first_x = transform.xx * rectangle.x + transform.xy * rectangle.y + transform.tx;
    let first_y = transform.yx * rectangle.x + transform.yy * rectangle.y + transform.ty;
    let second_x =
        transform.xx * (rectangle.x + rectangle.width) + transform.xy * rectangle.y + transform.tx;
    let second_y =
        transform.yx * (rectangle.x + rectangle.width) + transform.yy * rectangle.y + transform.ty;
    let third_x =
        transform.xx * rectangle.x + transform.xy * (rectangle.y + rectangle.height) + transform.tx;
    let third_y =
        transform.yx * rectangle.x + transform.yy * (rectangle.y + rectangle.height) + transform.ty;
    let fourth_x = transform.xx * (rectangle.x + rectangle.width)
        + transform.xy * (rectangle.y + rectangle.height)
        + transform.tx;
    let fourth_y = transform.yx * (rectangle.x + rectangle.width)
        + transform.yy * (rectangle.y + rectangle.height)
        + transform.ty;

    let min_x = first_x.min(second_x).min(third_x).min(fourth_x);
    let min_y = first_y.min(second_y).min(third_y).min(fourth_y);
    let max_x = first_x.max(second_x).max(third_x).max(fourth_x);
    let max_y = first_y.max(second_y).max(third_y).max(fourth_y);

    InputTextRectangle {
        x: min_x.floor(),
        y: min_y.floor(),
        width: (max_x - min_x).ceil().max(1.0),
        height: (max_y - min_y).ceil().max(1.0),
    }
}

/// Convert one UTF-16 code-unit offset into one UTF-8 byte offset.
fn utf16_offset_to_utf8_offset(text: &str, offset: u32) -> Option<usize> {
    if offset == 0 {
        return Some(0);
    }

    let mut utf16_offset = 0u32;
    for (utf8_offset, character) in text.char_indices() {
        if utf16_offset == offset {
            return Some(utf8_offset);
        }

        utf16_offset = utf16_offset.saturating_add(character.len_utf16() as u32);
        if utf16_offset == offset {
            return Some(utf8_offset + character.len_utf8());
        }
    }

    (utf16_offset == offset).then_some(text.len())
}

/// Convert one UTF-8 byte offset into one UTF-16 code-unit offset.
fn utf8_offset_to_utf16_offset(text: &str, offset: usize) -> Option<u32> {
    if offset > text.len() || !text.is_char_boundary(offset) {
        return None;
    }

    let prefix = &text[..offset];
    Some(prefix.encode_utf16().count() as u32)
}

/// Build one surrounding-text payload when one valid UTF-8 window can be described.
fn surrounding_text_payload(state: &InputTextSessionStateValue) -> Option<(String, i32, i32)> {
    const MAX_SURROUNDING_TEXT_BYTES: usize = 4000;

    // skip surrounding-text sync while renderer state still contains one active preedit span
    if state.composing.is_some() {
        return None;
    }

    let selection_start = utf16_offset_to_utf8_offset(&state.text, state.selection.start_offset)?;
    let selection_end = utf16_offset_to_utf8_offset(&state.text, state.selection.end_offset)?;

    if state.text.len() <= MAX_SURROUNDING_TEXT_BYTES {
        return Some((
            state.text.clone(),
            selection_end as i32,
            selection_start as i32,
        ));
    }

    let selection_length = selection_end.saturating_sub(selection_start);
    if selection_length > MAX_SURROUNDING_TEXT_BYTES {
        return None;
    }

    let remaining_context = MAX_SURROUNDING_TEXT_BYTES.saturating_sub(selection_length);
    let preferred_prefix = remaining_context / 2;
    let preferred_suffix = remaining_context.saturating_sub(preferred_prefix);

    let mut start = selection_start.saturating_sub(preferred_prefix);
    let mut end = selection_end
        .saturating_add(preferred_suffix)
        .min(state.text.len());

    while start > 0 && !state.text.is_char_boundary(start) {
        start -= 1;
    }

    while end < state.text.len() && !state.text.is_char_boundary(end) {
        end += 1;
    }

    while end.saturating_sub(start) > MAX_SURROUNDING_TEXT_BYTES {
        if start < selection_start {
            start += 1;
            while start < selection_start && !state.text.is_char_boundary(start) {
                start += 1;
            }
        } else if end > selection_end {
            end -= 1;
            while end > selection_end && !state.text.is_char_boundary(end) {
                end -= 1;
            }
        } else {
            return None;
        }
    }

    let text = state.text.get(start..end)?.to_string();
    let cursor = selection_end.saturating_sub(start) as i32;
    let anchor = selection_start.saturating_sub(start) as i32;

    Some((text, cursor, anchor))
}

/// Convert one renderer geometry hint into one compositor cursor rectangle.
fn cursor_rectangle(geometry: Option<InputTextGeometry>) -> Option<(i32, i32, i32, i32)> {
    let geometry = geometry?;
    let rectangle = geometry
        .caret_rectangle
        .or(geometry.composing_rectangle)
        .unwrap_or(geometry.editor_rectangle);
    let rectangle = project_target_rectangle(geometry, rectangle);

    Some((
        rectangle.x.round() as i32,
        rectangle.y.round() as i32,
        rectangle.width.round().max(1.0) as i32,
        rectangle.height.round().max(1.0) as i32,
    ))
}

/// Map one runtime text-input configuration onto one Wayland content-type pair.
fn content_type(configuration: InputTextSessionConfig) -> (ContentHint, ContentPurpose) {
    let mut hint = ContentHint::None;

    if configuration.is_multiline {
        hint |= ContentHint::Multiline;
    }

    if configuration.is_secure {
        hint |= ContentHint::SensitiveData;
        hint |= ContentHint::HiddenText;
    }

    let purpose = if configuration.is_secure {
        ContentPurpose::Password
    } else {
        match configuration.input_type {
            InputTextInputType::Text => ContentPurpose::Normal,
            InputTextInputType::Number => ContentPurpose::Number,
            InputTextInputType::Email => ContentPurpose::Email,
            InputTextInputType::Url => ContentPurpose::Url,
            InputTextInputType::Password => ContentPurpose::Password,
            InputTextInputType::Phone => ContentPurpose::Phone,
            InputTextInputType::Search => ContentPurpose::Normal,
        }
    };

    (hint, purpose)
}

/// Resolve one UTF-16 deletion range from one Wayland surrounding-text request.
fn delete_target_range(
    state: &InputTextSessionStateValue,
    before_length: u32,
    after_length: u32,
) -> Option<InputTextRange> {
    let base_range = state.composing.unwrap_or(state.selection);
    let base_start = utf16_offset_to_utf8_offset(&state.text, base_range.start_offset)?;
    let base_end = utf16_offset_to_utf8_offset(&state.text, base_range.end_offset)?;

    let start = base_start.checked_sub(before_length as usize)?;
    let end = base_end.checked_add(after_length as usize)?;
    if end > state.text.len() {
        return None;
    }

    if !state.text.is_char_boundary(start) || !state.text.is_char_boundary(end) {
        return None;
    }

    Some(InputTextRange {
        start_offset: utf8_offset_to_utf16_offset(&state.text, start)?,
        end_offset: utf8_offset_to_utf16_offset(&state.text, end)?,
    })
}

/// Commit one buffered text-input state snapshot to the compositor.
fn apply_text_input_state(
    dispatch_state: &mut WaylandConnectionDispatchState,
    text_input: &zwp_text_input_v3::ZwpTextInputV3,
    configuration: InputTextSessionConfig,
    state: &InputTextSessionStateValue,
    geometry: Option<InputTextGeometry>,
) {
    let (hint, purpose) = content_type(configuration);

    // re-enable so the compositor treats this renderer state as the current focused editor
    text_input.enable();
    text_input.set_content_type(hint, purpose);

    // send surrounding text when the renderer can describe it without leaking unstable preedit
    if let Some((text, cursor, anchor)) = surrounding_text_payload(state) {
        text_input.set_surrounding_text(text, cursor, anchor);
    }

    // send one candidate anchor when the renderer has usable geometry
    if let Some((x, y, width, height)) = cursor_rectangle(geometry) {
        text_input.set_cursor_rectangle(x, y, width, height);
    }

    text_input.commit();
    dispatch_state.input.text_input_commit_serial = dispatch_state
        .input
        .text_input_commit_serial
        .wrapping_add(1);
}

/// Resolve one Wayland surface id for one runtime window handle.
fn window_surface_id(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    operation: &'static str,
) -> RuntimeResult<ObjectId> {
    let host_state = resolve_window_host_state(context, window, operation)?;
    let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    Ok(host_state.host.surface.clone())
}

/// Activate one Wayland window-backed text session.
pub(crate) fn activate_window_text_session(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    session: resource::InputTextSessionHandle,
    configuration: InputTextSessionConfig,
    state: InputTextSessionStateValue,
    geometry: Option<InputTextGeometry>,
) -> RuntimeResult<()> {
    ensure_window_handle_exists(context, window, "destack.input.text.open")?;
    let runtime_state = runtime_state(context);

    runtime_state
        .active_text_sessions
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .insert(window, session);

    synchronize_window_text_session(context, window, configuration, state, geometry)
}

/// Deactivate one Wayland window-backed text session.
pub(crate) fn deactivate_window_text_session(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    let runtime_state = runtime_state(context);
    let mut active_sessions = runtime_state
        .active_text_sessions
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    if active_sessions.get(&window).copied() != Some(session) {
        return Ok(());
    }

    active_sessions.remove(&window);
    drop(active_sessions);

    let surface_id = window_surface_id(context, window, "destack.input.text.close")?;
    with_connection_dispatch(
        context,
        "destack.input.text.close",
        |_connection, _event_queue, dispatch_state| {
            let Some(text_input) = dispatch_state.input.text_input.as_ref() else {
                return Ok(());
            };

            if dispatch_state.input.text_input_focus_surface.as_ref() != Some(&surface_id) {
                return Ok(());
            }

            text_input.disable();
            text_input.commit();
            dispatch_state.input.text_input_commit_serial = dispatch_state
                .input
                .text_input_commit_serial
                .wrapping_add(1);
            dispatch_state.input.active_preedit = None;
            dispatch_state.input.pending_preedit = None;
            dispatch_state.input.pending_commit = None;
            dispatch_state.input.pending_delete = None;
            Ok(())
        },
    )
}

/// Synchronize one Wayland window-backed text session into the compositor text-input lane.
pub(crate) fn synchronize_window_text_session(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    configuration: InputTextSessionConfig,
    state: InputTextSessionStateValue,
    geometry: Option<InputTextGeometry>,
) -> RuntimeResult<()> {
    let surface_id = window_surface_id(context, window, "destack.input.text.syncWindowSession")?;

    with_connection_dispatch(
        context,
        "destack.input.text.syncWindowSession",
        |_connection, _event_queue, dispatch_state| {
            let Some(text_input) = dispatch_state.input.text_input.as_ref().cloned() else {
                return Err(core_platform::not_supported(
                    "destack.input.text.syncWindowSession",
                ));
            };

            if dispatch_state.input.text_input_focus_surface.as_ref() != Some(&surface_id) {
                return Ok(());
            }

            apply_text_input_state(dispatch_state, &text_input, configuration, &state, geometry);
            Ok(())
        },
    )
}

/// Flush one Wayland done batch into one runtime text session.
fn publish_done_batch(
    runtime_state: &Arc<WaylandRuntimeState>,
    surface: &wl_surface::WlSurface,
    dispatch_state: &mut WaylandConnectionDispatchState,
) {
    let Some(token) = runtime_state.window_token_from_surface(&surface.id()) else {
        dispatch_state.input.pending_preedit = None;
        dispatch_state.input.pending_commit = None;
        dispatch_state.input.pending_delete = None;
        dispatch_state.input.active_preedit = None;
        return;
    };
    let Some(window_handle) = runtime_state.window_handle_from_id(&token.window_id) else {
        dispatch_state.input.pending_preedit = None;
        dispatch_state.input.pending_commit = None;
        dispatch_state.input.pending_delete = None;
        dispatch_state.input.active_preedit = None;
        return;
    };
    let Some((_, _configuration, state, _geometry)) =
        resolve_wayland_window_text_session(runtime_state, window_handle)
            .ok()
            .flatten()
    else {
        dispatch_state.input.pending_preedit = None;
        dispatch_state.input.pending_commit = None;
        dispatch_state.input.pending_delete = None;
        dispatch_state.input.active_preedit = None;
        return;
    };

    let previous_preedit = dispatch_state.input.active_preedit.take();
    let next_preedit = dispatch_state.input.pending_preedit.take();
    let pending_delete = dispatch_state.input.pending_delete.take();
    let pending_commit = dispatch_state.input.pending_commit.take();
    let is_composing = previous_preedit.is_some() || next_preedit.is_some();

    // apply the compositor-requested deletion around the current selection
    if let Some(delete) = pending_delete
        && let Some(range) = delete_target_range(&state, delete.before_length, delete.after_length)
    {
        let _ = notify_wayland_window_edit_intent(
            runtime_state,
            window_handle,
            InputEditIntentTypeValue::DeleteContent,
            None,
            vec![range],
            is_composing,
        );
    }

    // publish one composition commit before the committed insertion intent
    if let Some(text) = pending_commit
        && !text.is_empty()
    {
        let _ = notify_wayland_window_composition_event(
            runtime_state,
            window_handle,
            InputEventAction::Commit,
            text.clone(),
            -1,
            -1,
        );

        let input_type = if matches!(text.as_str(), "\n" | "\r") {
            InputEditIntentTypeValue::InsertLineBreak
        } else {
            InputEditIntentTypeValue::InsertText
        };
        let _ = notify_wayland_window_edit_intent(
            runtime_state,
            window_handle,
            input_type,
            Some(text),
            vec![state.selection],
            is_composing,
        );
    }

    // publish one preedit lifecycle transition for the current compositor batch
    match (previous_preedit, next_preedit) {
        (None, Some(preedit)) => {
            let _ = notify_wayland_window_composition_event(
                runtime_state,
                window_handle,
                InputEventAction::Begin,
                preedit.text.clone(),
                preedit.selection_start,
                preedit.selection_end,
            );
            dispatch_state.input.active_preedit = Some(preedit);
        }
        (Some(_), Some(preedit)) => {
            let _ = notify_wayland_window_composition_event(
                runtime_state,
                window_handle,
                InputEventAction::Update,
                preedit.text.clone(),
                preedit.selection_start,
                preedit.selection_end,
            );
            dispatch_state.input.active_preedit = Some(preedit);
        }
        (Some(_), None) => {
            let _ = notify_wayland_window_end_composition(runtime_state, window_handle);
        }
        (None, None) => {}
    }
}

delegate_noop!(WaylandConnectionDispatchState: zwp_text_input_manager_v3::ZwpTextInputManagerV3);

impl Dispatch<zwp_text_input_v3::ZwpTextInputV3, ()> for WaylandConnectionDispatchState {
    /// Handle zwp_text_input_v3 events.
    fn event(
        state: &mut Self,
        text_input: &zwp_text_input_v3::ZwpTextInputV3,
        event: TextInputEvent,
        _data: &(),
        connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        match event {
            // track compositor text focus and push the current session snapshot on enter
            TextInputEvent::Enter { surface } => {
                state.input.text_input_focus_surface = Some(surface.id());

                let Some(runtime_state) = state.runtime_state.upgrade() else {
                    return;
                };
                let Some(token) = runtime_state.window_token_from_surface(&surface.id()) else {
                    return;
                };
                let Some(window_handle) = runtime_state.window_handle_from_id(&token.window_id)
                else {
                    return;
                };
                let Some((_session, configuration, session_state, geometry)) =
                    resolve_wayland_window_text_session(&runtime_state, window_handle)
                        .ok()
                        .flatten()
                else {
                    return;
                };

                apply_text_input_state(state, text_input, configuration, &session_state, geometry);
            }

            // clear pending preedit state and disable the seat text-input lane on leave
            TextInputEvent::Leave { surface: _ } => {
                if let Some(runtime_state) = state.runtime_state.upgrade()
                    && let Some(surface_id) = state.input.text_input_focus_surface.as_ref()
                    && let Some(token) = runtime_state.window_token_from_surface(surface_id)
                    && let Some(window_handle) =
                        runtime_state.window_handle_from_id(&token.window_id)
                    && state.input.active_preedit.take().is_some()
                {
                    let _ = notify_wayland_window_end_composition(&runtime_state, window_handle);
                }

                state.input.text_input_focus_surface = None;
                state.input.pending_preedit = None;
                state.input.pending_commit = None;
                state.input.pending_delete = None;
                text_input.disable();
                text_input.commit();
                state.input.text_input_commit_serial =
                    state.input.text_input_commit_serial.wrapping_add(1);
            }

            // buffer preedit updates until done so they follow the protocol batch semantics
            TextInputEvent::PreeditString {
                text,
                cursor_begin,
                cursor_end,
            } => {
                let text = text.unwrap_or_default();
                if text.is_empty() {
                    state.input.pending_preedit = None;
                    return;
                }

                state.input.pending_preedit = Some(super::WaylandTextPreeditState {
                    text,
                    selection_start: cursor_begin,
                    selection_end: cursor_end,
                });
            }

            // buffer committed text until done so delete and preedit ordering stays correct
            TextInputEvent::CommitString { text } => {
                state.input.pending_commit = text.filter(|text| !text.is_empty());
            }

            // buffer delete requests until done so they apply against the same batch
            TextInputEvent::DeleteSurroundingText {
                before_length,
                after_length,
            } => {
                state.input.pending_delete = Some(super::WaylandTextDeleteState {
                    before_length,
                    after_length,
                });
            }

            // flush one pending batch into the runtime text-session queue
            TextInputEvent::Done { serial: _ } => {
                let Some(runtime_state) = state.runtime_state.upgrade() else {
                    return;
                };
                let Some(surface_id) = state.input.text_input_focus_surface.as_ref() else {
                    return;
                };
                let Ok(surface) = wl_surface::WlSurface::from_id(connection, surface_id.clone())
                else {
                    return;
                };

                publish_done_batch(&runtime_state, &surface, state);
            }

            _ => {}
        }
    }
}
