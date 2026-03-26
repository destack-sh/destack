use std::ffi::c_void;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Globalization::HIMC;
use windows_sys::Win32::UI::Input::Ime::{
    CANDIDATEFORM, CFS_EXCLUDE, CFS_POINT, COMPOSITIONFORM, GCS_COMPSTR, GCS_CURSORPOS,
    GCS_RESULTSTR, ImmGetCompositionStringW, ImmGetContext, ImmReleaseContext,
    ImmSetCandidateWindow, ImmSetCompositionWindow,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    WM_CHAR, WM_COPY, WM_CUT, WM_IME_CHAR, WM_IME_COMPOSITION, WM_IME_ENDCOMPOSITION, WM_KEYDOWN,
    WM_PASTE, WM_UNICHAR,
};

use super::core::{self as win32_core, Win32RuntimeState};
use crate::diagnostic::RuntimeResult;
use crate::platform::input::host::{
    notify_win32_window_cancel_composition, notify_win32_window_clipboard_command,
    notify_win32_window_committed_text, notify_win32_window_composing_text,
    notify_win32_window_edit_intent, notify_win32_window_text_message,
    resolve_win32_window_text_session,
};
use crate::platform::input::{
    InputClipboardCommandTypeValue, InputEditIntentTypeValue, InputTextGeometry,
    InputTextRectangle, InputTextSessionConfig, InputTextSessionStateValue,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

/// Sentinel `WM_UNICHAR` probe used to detect unicode support.
const WM_UNICHAR_NOCHAR: u32 = 0xFFFF;

/// Project one local rectangle into one host-space rectangle.
fn project_text_rectangle(rectangle: InputTextRectangle, geometry: InputTextGeometry) -> RECT {
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

    let min_x = first_x.min(second_x).min(third_x).min(fourth_x).floor() as i32;
    let min_y = first_y.min(second_y).min(third_y).min(fourth_y).floor() as i32;
    let max_x = first_x.max(second_x).max(third_x).max(fourth_x).ceil() as i32;
    let max_y = first_y.max(second_y).max(third_y).max(fourth_y).ceil() as i32;

    RECT {
        left: min_x,
        top: min_y,
        right: max_x.max(min_x + 1),
        bottom: max_y.max(min_y + 1),
    }
}

/// Return one host-space caret rectangle for IME placement.
fn ime_rectangle(geometry: InputTextGeometry) -> RECT {
    let rectangle = geometry
        .composing_rectangle
        .or(geometry.caret_rectangle)
        .unwrap_or(geometry.editor_rectangle);

    project_text_rectangle(rectangle, geometry)
}

/// One scoped IMM context.
struct Win32ImeContext {
    /// The target native window.
    hwnd: HWND,
    /// The current input context.
    himc: HIMC,
}

impl Win32ImeContext {
    /// Acquire one IMM context for one hwnd.
    fn current(hwnd: HWND) -> Option<Self> {
        let himc = unsafe { ImmGetContext(hwnd) };

        if himc == 0 {
            return None;
        }

        Some(Self { hwnd, himc })
    }

    /// Read one wide composition string.
    fn composition_string(&self, mode: u32) -> Option<String> {
        let size = unsafe { ImmGetCompositionStringW(self.himc, mode, std::ptr::null_mut(), 0) };

        if size < 0 {
            return None;
        }

        if size == 0 {
            return Some(String::new());
        }

        let mut buffer = vec![0u8; size as usize];
        let copied = unsafe {
            ImmGetCompositionStringW(
                self.himc,
                mode,
                buffer.as_mut_ptr() as *mut c_void,
                size as u32,
            )
        };

        if copied < 0 {
            return None;
        }

        let units = unsafe {
            std::slice::from_raw_parts(
                buffer.as_ptr() as *const u16,
                copied as usize / std::mem::size_of::<u16>(),
            )
        };

        String::from_utf16(units).ok()
    }

    /// Read one IME cursor offset in UTF-16 units.
    fn cursor_offset(&self) -> Option<u32> {
        let offset =
            unsafe { ImmGetCompositionStringW(self.himc, GCS_CURSORPOS, std::ptr::null_mut(), 0) };

        if offset < 0 {
            return None;
        }

        Some(offset as u32)
    }

    /// Apply one renderer geometry hint to the native IME windows.
    fn set_cursor_rectangle(&self, geometry: InputTextGeometry) {
        let rectangle = ime_rectangle(geometry);
        let point = POINT {
            x: rectangle.left,
            y: rectangle.bottom,
        };
        let candidate = CANDIDATEFORM {
            dwIndex: 0,
            dwStyle: CFS_EXCLUDE,
            ptCurrentPos: point,
            rcArea: rectangle,
        };
        let composition = COMPOSITIONFORM {
            dwStyle: CFS_POINT,
            ptCurrentPos: point,
            rcArea: rectangle,
        };

        unsafe {
            ImmSetCandidateWindow(self.himc, &candidate);
            ImmSetCompositionWindow(self.himc, &composition);
        }
    }
}

impl Drop for Win32ImeContext {
    fn drop(&mut self) {
        unsafe {
            ImmReleaseContext(self.hwnd, self.himc);
        }
    }
}

/// Activate one Win32 window-backed text session.
pub(crate) fn activate_window_text_session(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    let runtime_state = win32_core::runtime_state(binding);
    runtime_state
        .active_text_sessions
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .insert(window, session);

    Ok(())
}

/// Deactivate one Win32 window-backed text session.
pub(crate) fn deactivate_window_text_session(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    let runtime_state = win32_core::runtime_state(binding);
    let mut active_text_sessions = runtime_state
        .active_text_sessions
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    if active_text_sessions.get(&window).copied() == Some(session) {
        active_text_sessions.remove(&window);
    }

    Ok(())
}

/// Resolve one active Win32 window text session and its current state.
pub(crate) fn resolve_window_text_session(
    runtime_state: &Win32RuntimeState,
    window: resource::WindowHandle,
) -> RuntimeResult<
    Option<(
        resource::InputTextSessionHandle,
        InputTextSessionConfig,
        InputTextSessionStateValue,
        Option<InputTextGeometry>,
    )>,
> {
    resolve_win32_window_text_session(runtime_state, window)
}

/// Synchronize one Win32 window text session with the native IME cursor area.
pub(crate) fn synchronize_window_text_session(
    runtime_state: &Win32RuntimeState,
    window: resource::WindowHandle,
    hwnd: HWND,
    geometry: Option<InputTextGeometry>,
) -> RuntimeResult<()> {
    let geometry = if let Some(geometry) = geometry {
        Some(geometry)
    } else {
        let Some((_, _, _, geometry)) = resolve_window_text_session(runtime_state, window)? else {
            return Ok(());
        };

        geometry
    };
    let Some(geometry) = geometry else {
        return Ok(());
    };
    let Some(ime_context) = Win32ImeContext::current(hwnd) else {
        return Ok(());
    };

    ime_context.set_cursor_rectangle(geometry);
    Ok(())
}

/// Handle one native Win32 window text message.
pub(crate) fn handle_window_text_message(
    runtime_state: &Win32RuntimeState,
    window: resource::WindowHandle,
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> RuntimeResult<Option<LRESULT>> {
    // keep native IME windows aligned with renderer geometry during composition
    if message == WM_IME_COMPOSITION {
        synchronize_window_text_session(runtime_state, window, hwnd, None)?;
    }

    // committed text
    if matches!(message, WM_CHAR | WM_IME_CHAR | WM_UNICHAR) {
        if message == WM_UNICHAR && wparam as u32 == WM_UNICHAR_NOCHAR {
            return Ok(Some(1));
        }

        notify_win32_window_text_message(runtime_state, window, message, wparam)?;
        return Ok(Some(0));
    }

    // edit intents
    if message == WM_KEYDOWN {
        let input_type = match wparam as u16 {
            value if value == windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_BACK => {
                Some(InputEditIntentTypeValue::DeleteContentBackward)
            }
            value if value == windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_DELETE => {
                Some(InputEditIntentTypeValue::DeleteContentForward)
            }
            value if value == windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_RETURN => {
                Some(InputEditIntentTypeValue::InsertLineBreak)
            }
            _ => None,
        };

        if let Some(input_type) = input_type {
            notify_win32_window_edit_intent(runtime_state, window, input_type, None)?;
            return Ok(Some(0));
        }
    }

    // clipboard commands
    if matches!(message, WM_COPY | WM_CUT | WM_PASTE) {
        let command = match message {
            WM_COPY => InputClipboardCommandTypeValue::Copy,
            WM_CUT => InputClipboardCommandTypeValue::Cut,
            WM_PASTE => InputClipboardCommandTypeValue::Paste,
            _ => unreachable!(),
        };

        notify_win32_window_clipboard_command(runtime_state, window, command)?;
        return Ok(Some(0));
    }

    // ime composition
    if message == WM_IME_COMPOSITION {
        let Some(ime_context) = Win32ImeContext::current(hwnd) else {
            return Ok(Some(0));
        };

        if (lparam as u32 & GCS_RESULTSTR) != 0
            && let Some(text) = ime_context.composition_string(GCS_RESULTSTR)
        {
            notify_win32_window_committed_text(runtime_state, window, &text)?;
        }

        if (lparam as u32 & GCS_COMPSTR) != 0
            && let Some(text) = ime_context.composition_string(GCS_COMPSTR)
        {
            notify_win32_window_composing_text(
                runtime_state,
                window,
                &text,
                ime_context.cursor_offset(),
            )?;
        }

        return Ok(Some(0));
    }

    // clear preedit when one composition ends without a result string
    if message == WM_IME_ENDCOMPOSITION {
        notify_win32_window_cancel_composition(runtime_state, window)?;
        return Ok(Some(0));
    }

    Ok(None)
}
