use std::sync::Arc;

use x11rb::protocol::xproto::{FocusInEvent, FocusOutEvent, KeyButMask, KeyPressEvent, Keysym};

use super::core::{
    X11ConnectionState, X11PreeditCaretCallbackStruct, X11PreeditDrawCallbackStruct,
    X11RuntimeState, X11TextInputContextState, X11WindowTextCallbackPayload, create_input_context,
    filter_key_press, lookup_utf8, preedit_text_string, runtime_state, set_input_context_focus,
    unset_input_context_focus, update_input_context_geometry,
};
use super::resource::{ensure_window_handle_exists, resolve_window_host_state};
use crate::diagnostic::RuntimeResult;
use crate::platform::input::host::{
    notify_x11_window_clipboard_command, notify_x11_window_composition_event,
    notify_x11_window_edit_intent, notify_x11_window_end_composition,
    resolve_x11_window_text_session,
};
use crate::platform::input::{
    InputClipboardCommandTypeValue, InputEditIntentTypeValue, InputEventAction, InputTextGeometry,
    InputTextSessionConfig, InputTextSessionStateValue,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

/// X11 keysym for backspace.
const XK_BACK_SPACE: Keysym = 0xFF08;
/// X11 keysym for tab.
const XK_TAB: Keysym = 0xFF09;
/// X11 keysym for return.
const XK_RETURN: Keysym = 0xFF0D;
/// X11 keysym for escape.
const XK_ESCAPE: Keysym = 0xFF1B;
/// X11 keysym for delete.
const XK_DELETE: Keysym = 0xFFFF;
/// X11 keysym base for keypad digits.
const XK_KP_0: Keysym = 0xFFB0;
/// X11 keysym end for keypad digits.
const XK_KP_9: Keysym = 0xFFB9;
/// X11 keysym for keypad space.
const XK_KP_SPACE: Keysym = 0xFF80;
/// X11 keysym for keypad tab.
const XK_KP_TAB: Keysym = 0xFF89;
/// X11 keysym for keypad enter.
const XK_KP_ENTER: Keysym = 0xFF8D;
/// X11 keysym for keypad multiply.
const XK_KP_MULTIPLY: Keysym = 0xFFAA;
/// X11 keysym for keypad add.
const XK_KP_ADD: Keysym = 0xFFAB;
/// X11 keysym for keypad separator.
const XK_KP_SEPARATOR: Keysym = 0xFFAC;
/// X11 keysym for keypad subtract.
const XK_KP_SUBTRACT: Keysym = 0xFFAD;
/// X11 keysym for keypad decimal.
const XK_KP_DECIMAL: Keysym = 0xFFAE;
/// X11 keysym for keypad divide.
const XK_KP_DIVIDE: Keysym = 0xFFAF;

/// Return whether one modifier mask contains one flag.
fn has_modifier(mask: KeyButMask, flag: KeyButMask) -> bool {
    (u16::from(mask) & u16::from(flag)) != 0
}

/// Return the cached keysyms for one keycode.
fn keysyms_for_keycode(connection_state: &X11ConnectionState, keycode: u8) -> &[Keysym] {
    let mapping = &connection_state.keyboard_mapping;
    if keycode < mapping.minimum_keycode || keycode > mapping.maximum_keycode {
        return &[];
    }

    let keysyms_per_keycode = mapping.keysyms_per_keycode as usize;
    if keysyms_per_keycode == 0 {
        return &[];
    }

    let keycode_index = (keycode - mapping.minimum_keycode) as usize;
    let start = keycode_index.saturating_mul(keysyms_per_keycode);
    let end = start
        .saturating_add(keysyms_per_keycode)
        .min(mapping.keysyms.len());

    &mapping.keysyms[start..end]
}

/// Return one normalized core keysym for one key press.
fn resolve_key_press_keysym(
    connection_state: &X11ConnectionState,
    event: &KeyPressEvent,
) -> Option<Keysym> {
    let keysyms = keysyms_for_keycode(connection_state, event.detail);
    if keysyms.is_empty() {
        return None;
    }

    let primary = keysyms[0];
    let shifted = keysyms.get(1).copied().filter(|value| *value != 0);
    let is_shifted =
        has_modifier(event.state, KeyButMask::SHIFT) || has_modifier(event.state, KeyButMask::LOCK);

    if is_shifted {
        shifted.or(Some(primary))
    } else {
        Some(primary)
    }
}

/// Map one X11 keysym onto one unicode string when possible.
fn keysym_text(keysym: Keysym) -> Option<String> {
    let character = match keysym {
        0x0020..=0x007E | 0x00A0..=0x00FF => char::from_u32(keysym)?,
        0x0100_0100..=0x0110_FFFF => char::from_u32(keysym - 0x0100_0000)?,
        XK_KP_SPACE => ' ',
        XK_KP_TAB => '\t',
        XK_KP_ENTER => '\n',
        XK_KP_MULTIPLY => '*',
        XK_KP_ADD => '+',
        XK_KP_SEPARATOR => ',',
        XK_KP_SUBTRACT => '-',
        XK_KP_DECIMAL => '.',
        XK_KP_DIVIDE => '/',
        XK_KP_0..=XK_KP_9 => char::from_u32((keysym - XK_KP_0) + ('0' as u32))?,
        _ => return None,
    };

    Some(character.to_string())
}

/// Replace one character range within one preedit string.
fn replace_character_range(text: &mut String, start: usize, length: usize, replacement: &str) {
    let start_byte = text
        .char_indices()
        .nth(start)
        .map(|(byte_index, _)| byte_index)
        .unwrap_or(text.len());
    let end_byte = text
        .char_indices()
        .nth(start.saturating_add(length))
        .map(|(byte_index, _)| byte_index)
        .unwrap_or(text.len());

    text.replace_range(start_byte..end_byte, replacement);
}

/// Resolve one callback payload into the owning runtime and window.
fn callback_state(
    payload: *mut std::ffi::c_void,
) -> Option<(Arc<X11RuntimeState>, resource::WindowHandle)> {
    let payload = unsafe { payload.cast::<X11WindowTextCallbackPayload>().as_ref()? };
    let runtime_state = payload.runtime_state.upgrade()?;

    Some((runtime_state, payload.window))
}

/// Queue one composition update for one X11 window.
fn publish_composition_update(
    runtime_state: &Arc<X11RuntimeState>,
    window: resource::WindowHandle,
    action: InputEventAction,
    text: String,
    caret: i32,
) {
    let _ = notify_x11_window_composition_event(runtime_state, window, action, text, caret, caret);
}

/// Publish one XIM preedit draw callback into the session event queue.
unsafe extern "C" fn x11_preedit_draw_callback(
    _input_method: *mut super::core::XInputMethodOpaque,
    payload: *mut std::ffi::c_void,
    call_data: *mut std::ffi::c_void,
) {
    let Some((runtime_state, window)) = callback_state(payload) else {
        return;
    };
    let Some(draw) = (unsafe { call_data.cast::<X11PreeditDrawCallbackStruct>().as_ref() }) else {
        return;
    };

    // update the cached preedit text before publishing the next lifecycle event
    let (action, text, caret) = {
        let mut text_contexts = runtime_state
            .text_input_contexts
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let Some(text_context) = text_contexts.get_mut(&window) else {
            return;
        };

        let replacement = preedit_text_string(draw.text).unwrap_or_default();
        let change_start = draw.chg_first.max(0) as usize;
        let change_length = draw.chg_length.max(0) as usize;

        replace_character_range(
            &mut text_context.composition_text,
            change_start,
            change_length,
            &replacement,
        );
        text_context.composition_caret = draw.caret.max(0);

        let action = if text_context.is_composing {
            InputEventAction::Update
        } else {
            InputEventAction::Begin
        };
        text_context.is_composing = true;

        (
            action,
            text_context.composition_text.clone(),
            text_context.composition_caret,
        )
    };

    publish_composition_update(&runtime_state, window, action, text, caret);
}

/// Publish one XIM preedit done callback into the session event queue.
unsafe extern "C" fn x11_preedit_done_callback(
    _input_method: *mut super::core::XInputMethodOpaque,
    payload: *mut std::ffi::c_void,
    _call_data: *mut std::ffi::c_void,
) {
    let Some((runtime_state, window)) = callback_state(payload) else {
        return;
    };

    // clear the cached preedit state before signaling the composition end
    let should_end = {
        let mut text_contexts = runtime_state
            .text_input_contexts
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let Some(text_context) = text_contexts.get_mut(&window) else {
            return;
        };

        let should_end = text_context.is_composing || !text_context.composition_text.is_empty();
        text_context.is_composing = false;
        text_context.composition_text.clear();
        text_context.composition_caret = 0;
        should_end
    };

    if should_end {
        let _ = notify_x11_window_end_composition(&runtime_state, window);
    }
}

/// Publish one XIM preedit caret callback into the session event queue.
unsafe extern "C" fn x11_preedit_caret_callback(
    _input_method: *mut super::core::XInputMethodOpaque,
    payload: *mut std::ffi::c_void,
    call_data: *mut std::ffi::c_void,
) {
    let Some((runtime_state, window)) = callback_state(payload) else {
        return;
    };
    let Some(caret) = (unsafe { call_data.cast::<X11PreeditCaretCallbackStruct>().as_ref() })
    else {
        return;
    };

    // keep the published preedit caret aligned with the IME callback state
    let (text, caret) = {
        let mut text_contexts = runtime_state
            .text_input_contexts
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let Some(text_context) = text_contexts.get_mut(&window) else {
            return;
        };
        if !text_context.is_composing {
            return;
        }

        text_context.composition_caret = caret.position.max(0);

        (
            text_context.composition_text.clone(),
            text_context.composition_caret,
        )
    };

    publish_composition_update(
        &runtime_state,
        window,
        InputEventAction::Update,
        text,
        caret,
    );
}

/// Activate one X11 window-backed text session.
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
    let connection_state =
        super::core::connection_state(&runtime_state, "destack.input.text.open")?;

    runtime_state
        .active_text_sessions
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .insert(window, session);

    // replace any stale xic before creating the new active lane
    runtime_state
        .text_input_contexts
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .remove(&window);

    let host_state = resolve_window_host_state(context, window, "destack.input.text.open")?;
    let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let callback_payload = Box::into_raw(Box::new(X11WindowTextCallbackPayload {
        runtime_state: Arc::downgrade(&runtime_state),
        window,
    }));

    // keep the session alive even when XIM is unavailable, and only attach the richer lane when available
    match create_input_context(
        &connection_state.xlib,
        host_state.window,
        callback_payload.cast(),
        Some(x11_preedit_draw_callback),
        Some(x11_preedit_done_callback),
        Some(x11_preedit_caret_callback),
    ) {
        Ok(Some(input_context)) => {
            runtime_state
                .text_input_contexts
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .insert(
                    window,
                    X11TextInputContextState {
                        input_context,
                        callback_payload,
                        is_composing: false,
                        composition_text: String::new(),
                        composition_caret: 0,
                    },
                );
        }
        Ok(None) => unsafe {
            drop(Box::from_raw(callback_payload));
        },
        Err(error) => {
            runtime_state
                .diagnostics
                .warn("display", "destack.input.text.open", error, None);

            unsafe {
                drop(Box::from_raw(callback_payload));
            }
        }
    }

    synchronize_window_text_session(context, window, configuration, state, geometry)
}

/// Deactivate one X11 window-backed text session.
pub(crate) fn deactivate_window_text_session(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    let runtime_state = runtime_state(context);
    let active_matches = runtime_state
        .active_text_sessions
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if active_matches.get(&window).copied() != Some(session) {
        return Ok(());
    }
    drop(active_matches);

    // end any active composition before the xic is released
    let removed_context = runtime_state
        .text_input_contexts
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .remove(&window);

    if removed_context
        .as_ref()
        .is_some_and(|context| context.is_composing || !context.composition_text.is_empty())
    {
        notify_x11_window_end_composition(&runtime_state, window)?;
    }

    runtime_state
        .active_text_sessions
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .remove(&window);

    Ok(())
}

/// Synchronize one X11 window-backed text session.
pub(crate) fn synchronize_window_text_session(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    configuration: InputTextSessionConfig,
    state: InputTextSessionStateValue,
    geometry: Option<InputTextGeometry>,
) -> RuntimeResult<()> {
    let _ = configuration;
    let _ = state;

    let runtime_state = runtime_state(context);
    let connection_state =
        super::core::connection_state(&runtime_state, "destack.input.text.syncWindowRepository")?;
    let input_context = runtime_state
        .text_input_contexts
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .get(&window)
        .map(|context| context.input_context);

    let Some(input_context) = input_context else {
        return Ok(());
    };
    let Some(geometry) = geometry else {
        return Ok(());
    };

    if let Err(error) =
        update_input_context_geometry(&connection_state.xlib, input_context, geometry)
    {
        runtime_state.diagnostics.warn(
            "display",
            "destack.input.text.syncWindowRepository",
            error,
            None,
        );
    }

    Ok(())
}

/// Handle one X11 focus-in event for one window-backed text session.
pub(crate) fn handle_window_text_focus_in(
    runtime_state: &X11RuntimeState,
    connection_state: &X11ConnectionState,
    window: resource::WindowHandle,
    _event: &FocusInEvent,
) -> RuntimeResult<()> {
    let input_context = runtime_state
        .text_input_contexts
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .get(&window)
        .map(|context| context.input_context);

    let Some(input_context) = input_context else {
        return Ok(());
    };

    set_input_context_focus(input_context);

    if let Some((_, _, _, geometry)) = resolve_x11_window_text_session(runtime_state, window)?
        && let Some(geometry) = geometry
    {
        if let Err(error) =
            update_input_context_geometry(&connection_state.xlib, input_context, geometry)
        {
            runtime_state.diagnostics.warn(
                "display",
                "destack.display.window.eventRead",
                error,
                None,
            );
        }
    }

    Ok(())
}

/// Handle one X11 focus-out event for one window-backed text session.
pub(crate) fn handle_window_text_focus_out(
    runtime_state: &X11RuntimeState,
    _connection_state: &X11ConnectionState,
    window: resource::WindowHandle,
    _event: &FocusOutEvent,
) -> RuntimeResult<()> {
    let removed_text = {
        let mut text_contexts = runtime_state
            .text_input_contexts
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let Some(text_context) = text_contexts.get_mut(&window) else {
            return Ok(());
        };

        unset_input_context_focus(text_context.input_context);

        if !text_context.is_composing && text_context.composition_text.is_empty() {
            return Ok(());
        }

        text_context.is_composing = false;
        text_context.composition_text.clear();
        text_context.composition_caret = 0;
        true
    };

    if removed_text {
        notify_x11_window_end_composition(runtime_state, window)?;
    }

    Ok(())
}

/// Dispatch one higher-level edit or clipboard command from one keysym/text pair.
fn dispatch_text_result(
    runtime_state: &X11RuntimeState,
    window: resource::WindowHandle,
    selection: crate::platform::input::InputTextRange,
    keysym: Option<Keysym>,
    text: Option<String>,
    event: &KeyPressEvent,
    is_composing: bool,
) -> RuntimeResult<bool> {
    let is_control = has_modifier(event.state, KeyButMask::CONTROL);
    let is_alt = has_modifier(event.state, KeyButMask::MOD1);
    let is_super = has_modifier(event.state, KeyButMask::MOD4);

    // clipboard shortcuts
    if is_control && !is_alt && !is_super {
        let command = match keysym {
            Some(0x0043 | 0x0063) => Some(InputClipboardCommandTypeValue::Copy),
            Some(0x0058 | 0x0078) => Some(InputClipboardCommandTypeValue::Cut),
            Some(0x0056 | 0x0076) => Some(InputClipboardCommandTypeValue::Paste),
            _ => None,
        };

        if let Some(command) = command {
            notify_x11_window_clipboard_command(runtime_state, window, command, is_composing)?;
            return Ok(true);
        }
    }

    // edit intents
    let input_type = match keysym {
        Some(XK_BACK_SPACE) => Some(InputEditIntentTypeValue::DeleteContentBackward),
        Some(XK_DELETE) => Some(InputEditIntentTypeValue::DeleteContentForward),
        _ => None,
    };

    if let Some(input_type) = input_type {
        notify_x11_window_edit_intent(
            runtime_state,
            window,
            input_type,
            None,
            vec![selection],
            is_composing,
        )?;
        return Ok(true);
    }

    // text insertion
    if is_control || is_alt || is_super || keysym == Some(XK_ESCAPE) {
        return Ok(false);
    }

    if let Some(text) = text
        && !text.is_empty()
    {
        let input_type = if matches!(text.as_str(), "\n" | "\r") {
            InputEditIntentTypeValue::InsertLineBreak
        } else {
            InputEditIntentTypeValue::InsertText
        };
        let data = matches!(input_type, InputEditIntentTypeValue::InsertText).then_some(text);

        notify_x11_window_edit_intent(
            runtime_state,
            window,
            input_type,
            data,
            vec![selection],
            is_composing,
        )?;
        return Ok(true);
    }

    if matches!(keysym, Some(XK_RETURN | XK_KP_ENTER)) {
        notify_x11_window_edit_intent(
            runtime_state,
            window,
            InputEditIntentTypeValue::InsertLineBreak,
            None,
            vec![selection],
            is_composing,
        )?;
        return Ok(true);
    }

    if keysym == Some(XK_TAB)
        && let Some(text) = keysym_text(XK_TAB)
    {
        notify_x11_window_edit_intent(
            runtime_state,
            window,
            InputEditIntentTypeValue::InsertText,
            Some(text),
            vec![selection],
            is_composing,
        )?;
        return Ok(true);
    }

    Ok(false)
}

/// Handle one X11 key press for one window-backed text session.
pub(crate) fn handle_window_text_key_press(
    runtime_state: &X11RuntimeState,
    connection_state: &X11ConnectionState,
    window: resource::WindowHandle,
    event: &KeyPressEvent,
) -> RuntimeResult<bool> {
    let Some((_, _, state, _)) = resolve_x11_window_text_session(runtime_state, window)? else {
        return Ok(false);
    };

    let (input_context, is_composing) = {
        let text_contexts = runtime_state
            .text_input_contexts
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let text_context = text_contexts.get(&window);

        (
            text_context.map(|context| context.input_context),
            text_context.is_some_and(|context| context.is_composing),
        )
    };

    // prefer the native XIM lane whenever one active context exists
    if let Some(input_context) = input_context {
        if filter_key_press(&connection_state.xlib, event) {
            return Ok(true);
        }

        match lookup_utf8(&connection_state.xlib, input_context, event) {
            Ok(lookup) => {
                if dispatch_text_result(
                    runtime_state,
                    window,
                    state.selection,
                    lookup.keysym,
                    (!lookup.text.is_empty()).then_some(lookup.text),
                    event,
                    is_composing,
                )? {
                    return Ok(true);
                }
            }
            Err(error) => {
                runtime_state.diagnostics.warn(
                    "display",
                    "destack.display.window.eventRead",
                    error,
                    None,
                );
            }
        }
    }

    // fall back to keysym-based text insertion when XIM is unavailable
    let Some(keysym) = resolve_key_press_keysym(connection_state, event) else {
        return Ok(false);
    };

    dispatch_text_result(
        runtime_state,
        window,
        state.selection,
        Some(keysym),
        keysym_text(keysym),
        event,
        is_composing,
    )
}
