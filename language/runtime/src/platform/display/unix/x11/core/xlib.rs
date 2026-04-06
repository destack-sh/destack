use std::ffi::{CStr, CString, c_char, c_int, c_long, c_uint, c_ulong, c_void};
use std::mem::MaybeUninit;
use std::ptr::{null, null_mut};

use x11rb::protocol::xproto::KeyPressEvent;
use x11rb::xcb_ffi::XCBConnection;

use crate::platform::input::{InputTextGeometry, InputTextRectangle};

/// Native XCB-backed connection type used by the X11 host.
pub(crate) type X11HostConnection = XCBConnection;

/// Raw Xlib display pointer type.
pub(crate) type X11DisplayHandle = *mut XDisplayOpaque;
/// Raw Xlib input method pointer type.
pub(crate) type X11InputMethodHandle = *mut XInputMethodOpaque;
/// Raw Xlib input context pointer type.
pub(crate) type X11InputContextHandle = *mut XInputContextOpaque;

/// `XlibOwnsEventQueue`.
const XLIB_OWNS_EVENT_QUEUE: c_int = 0;
/// `XCBOwnsEventQueue`.
const XCB_OWNS_EVENT_QUEUE: c_int = 1;

/// `KeyPress`.
const X11_KEY_PRESS_EVENT_TYPE: c_int = 2;
/// `XLookupNone`.
pub(crate) const X11_LOOKUP_NONE: c_int = 1;
/// `XLookupChars`.
pub(crate) const X11_LOOKUP_CHARS: c_int = 2;
/// `XLookupKeySym`.
pub(crate) const X11_LOOKUP_KEYSYM: c_int = 3;
/// `XLookupBoth`.
pub(crate) const X11_LOOKUP_BOTH: c_int = 4;
/// `XBufferOverflow`.
pub(crate) const X11_BUFFER_OVERFLOW: c_int = -1;

/// `XIMPreeditCallbacks`.
const X11_IM_PREEDIT_CALLBACKS: c_ulong = 0x0002;
/// `XIMStatusNothing`.
const X11_IM_STATUS_NOTHING: c_ulong = 0x0400;

/// `clientWindow`.
const XN_CLIENT_WINDOW: &[u8] = b"clientWindow\0";
/// `focusWindow`.
const XN_FOCUS_WINDOW: &[u8] = b"focusWindow\0";
/// `inputStyle`.
const XN_INPUT_STYLE: &[u8] = b"inputStyle\0";
/// `preeditAttributes`.
const XN_PREEDIT_ATTRIBUTES: &[u8] = b"preeditAttributes\0";
/// `preeditDrawCallback`.
const XN_PREEDIT_DRAW_CALLBACK: &[u8] = b"preeditDrawCallback\0";
/// `preeditDoneCallback`.
const XN_PREEDIT_DONE_CALLBACK: &[u8] = b"preeditDoneCallback\0";
/// `preeditCaretCallback`.
const XN_PREEDIT_CARET_CALLBACK: &[u8] = b"preeditCaretCallback\0";
/// `area`.
const XN_AREA: &[u8] = b"area\0";
/// `spotLocation`.
const XN_SPOT_LOCATION: &[u8] = b"spotLocation\0";

/// Opaque `Display`.
#[repr(C)]
pub(crate) struct XDisplayOpaque {
    _private: [u8; 0],
}

/// Opaque `XIM`.
#[repr(C)]
pub(crate) struct XInputMethodOpaque {
    _private: [u8; 0],
}

/// Opaque `XIC`.
#[repr(C)]
pub(crate) struct XInputContextOpaque {
    _private: [u8; 0],
}

/// Native `XPoint`.
#[repr(C)]
#[derive(Clone, Copy)]
struct XPoint {
    x: i16,
    y: i16,
}

/// Native `XRectangle`.
#[repr(C)]
#[derive(Clone, Copy)]
struct XRectangle {
    x: i16,
    y: i16,
    width: u16,
    height: u16,
}

/// Native `XKeyPressedEvent`.
#[repr(C)]
#[derive(Clone, Copy)]
struct XKeyPressedEvent {
    type_: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: X11DisplayHandle,
    window: c_ulong,
    root: c_ulong,
    subwindow: c_ulong,
    time: c_ulong,
    x: c_int,
    y: c_int,
    x_root: c_int,
    y_root: c_int,
    state: c_uint,
    keycode: c_uint,
    same_screen: c_int,
}

/// Native `XEvent`.
#[repr(C)]
union XEvent {
    type_: c_int,
    xkey: XKeyPressedEvent,
    pad: [c_long; 24],
}

/// Native `XIMCallback`.
#[repr(C)]
pub(crate) struct X11ImCallback {
    /// Callback-owned payload.
    pub(crate) client_data: *mut c_void,
    /// Callback function pointer.
    pub(crate) callback:
        Option<unsafe extern "C" fn(*mut XInputMethodOpaque, *mut c_void, *mut c_void)>,
}

/// Native `XIMText`.
#[repr(C)]
pub(crate) struct X11ImText {
    /// Character count.
    pub(crate) length: u16,
    /// Feedback payload.
    pub(crate) feedback: *mut c_ulong,
    /// Whether the payload is wide-char.
    pub(crate) encoding_is_wchar: c_int,
    /// Text payload.
    pub(crate) string: X11ImTextString,
}

/// Native `XIMText` string union.
#[repr(C)]
pub(crate) union X11ImTextString {
    /// Multibyte payload.
    pub(crate) mbs: *mut c_char,
    /// Wide-char payload.
    pub(crate) wcs: *mut u32,
}

/// Native `XIMPreeditDrawCallbackStruct`.
#[repr(C)]
pub(crate) struct X11PreeditDrawCallbackStruct {
    /// Caret offset within the preedit string.
    pub(crate) caret: c_int,
    /// Starting character offset for the changed span.
    pub(crate) chg_first: c_int,
    /// Character length of the replaced span.
    pub(crate) chg_length: c_int,
    /// Updated preedit text payload.
    pub(crate) text: *mut X11ImText,
}

/// Native `XIMPreeditCaretCallbackStruct`.
#[repr(C)]
pub(crate) struct X11PreeditCaretCallbackStruct {
    /// Caret position within the preedit string.
    pub(crate) position: c_int,
    /// Requested caret direction.
    pub(crate) direction: c_int,
    /// Requested caret style.
    pub(crate) style: c_int,
}

/// Shared Xlib state kept alongside the XCB host connection.
#[derive(Debug)]
pub(crate) struct X11XlibState {
    /// Shared native display pointer.
    pub(crate) display: X11DisplayHandle,
    /// Shared native input method when one was opened successfully.
    pub(crate) input_method: Option<X11InputMethodHandle>,
}

unsafe impl Send for X11XlibState {}
unsafe impl Sync for X11XlibState {}

impl Drop for X11XlibState {
    fn drop(&mut self) {
        // tear down the IM before the shared display closes
        if let Some(input_method) = self.input_method.take() {
            unsafe {
                XCloseIM(input_method);
            }
        }

        // return the event queue to xlib before closing the display
        if !self.display.is_null() {
            unsafe {
                XSetEventQueueOwner(self.display, XLIB_OWNS_EVENT_QUEUE);
                XCloseDisplay(self.display);
            }
        }
    }
}

/// Decoded lookup payload from one XIM key dispatch.
#[derive(Debug, Default)]
pub(crate) struct X11LookupResult {
    /// UTF-8 text returned by the input method.
    pub(crate) text: String,
    /// Resolved keysym when one was produced.
    pub(crate) keysym: Option<u32>,
    /// Raw Xlib lookup status.
    pub(crate) status: c_int,
}

#[link(name = "X11")]
unsafe extern "C" {
    fn XOpenDisplay(name: *const c_char) -> X11DisplayHandle;
    fn XCloseDisplay(display: X11DisplayHandle) -> c_int;
    fn XDefaultScreen(display: X11DisplayHandle) -> c_int;
    fn XSetLocaleModifiers(modifiers: *const c_char) -> *mut c_char;
    fn XOpenIM(
        display: X11DisplayHandle,
        resource_database: *mut c_void,
        resource_name: *mut c_char,
        resource_class: *mut c_char,
    ) -> X11InputMethodHandle;
    fn XCloseIM(input_method: X11InputMethodHandle) -> c_int;
    fn XCreateIC(input_method: X11InputMethodHandle, ...) -> X11InputContextHandle;
    fn XDestroyIC(input_context: X11InputContextHandle);
    fn XSetICFocus(input_context: X11InputContextHandle);
    fn XUnsetICFocus(input_context: X11InputContextHandle);
    fn XSetICValues(input_context: X11InputContextHandle, ...) -> *mut c_char;
    fn Xutf8LookupString(
        input_context: X11InputContextHandle,
        event: *mut XKeyPressedEvent,
        buffer_return: *mut c_char,
        bytes_buffer: c_int,
        keysym_return: *mut c_ulong,
        status_return: *mut c_int,
    ) -> c_int;
    fn XFilterEvent(event: *mut XEvent, window: c_ulong) -> c_int;
    fn XVaCreateNestedList(unused: c_int, ...) -> *mut c_void;
    fn XFree(data: *mut c_void) -> c_int;
}

#[link(name = "X11-xcb")]
unsafe extern "C" {
    fn XGetXCBConnection(display: X11DisplayHandle) -> *mut c_void;
    fn XSetEventQueueOwner(display: X11DisplayHandle, owner: c_int);
}

/// Open one shared Xlib display and wrap its XCB connection for x11rb.
pub(crate) fn open_host_connection(
    operation: &'static str,
) -> Result<(X11HostConnection, usize, X11XlibState), String> {
    let locale = CString::new("").expect("empty locale modifier should be valid");

    // initialize the process locale so Xutf8 paths actually operate on utf8 text
    unsafe {
        libc::setlocale(libc::LC_CTYPE, locale.as_ptr());
        XSetLocaleModifiers(locale.as_ptr());
    }

    // open the display first so the IM and XCB layers share one native endpoint
    let display = unsafe { XOpenDisplay(null()) };
    if display.is_null() {
        return Err(format!("{operation}: XOpenDisplay failed"));
    }

    let screen_index = unsafe { XDefaultScreen(display) };
    if screen_index < 0 {
        unsafe {
            XCloseDisplay(display);
        }

        return Err(format!("{operation}: XDefaultScreen failed"));
    }

    let raw_xcb_connection = unsafe { XGetXCBConnection(display) };
    if raw_xcb_connection.is_null() {
        unsafe {
            XCloseDisplay(display);
        }

        return Err(format!("{operation}: XGetXCBConnection failed"));
    }

    unsafe {
        XSetEventQueueOwner(display, XCB_OWNS_EVENT_QUEUE);
    }

    let connection = unsafe { XCBConnection::from_raw_xcb_connection(raw_xcb_connection, false) }
        .map_err(|error| {
        unsafe {
            XSetEventQueueOwner(display, XLIB_OWNS_EVENT_QUEUE);
            XCloseDisplay(display);
        }

        format!("{operation}: x11rb xcb connection setup failed: {error}")
    })?;

    let input_method = unsafe { XOpenIM(display, null_mut(), null_mut(), null_mut()) };

    Ok((
        connection,
        screen_index as usize,
        X11XlibState {
            display,
            input_method: (!input_method.is_null()).then_some(input_method),
        },
    ))
}

/// Create one XIM-backed input context for one window.
pub(crate) fn create_input_context(
    xlib_state: &X11XlibState,
    window: u32,
    client_data: *mut c_void,
    draw_callback: Option<unsafe extern "C" fn(*mut XInputMethodOpaque, *mut c_void, *mut c_void)>,
    done_callback: Option<unsafe extern "C" fn(*mut XInputMethodOpaque, *mut c_void, *mut c_void)>,
    caret_callback: Option<unsafe extern "C" fn(*mut XInputMethodOpaque, *mut c_void, *mut c_void)>,
) -> Result<Option<X11InputContextHandle>, String> {
    let Some(input_method) = xlib_state.input_method else {
        return Ok(None);
    };

    let draw_callback = X11ImCallback {
        client_data,
        callback: draw_callback,
    };
    let done_callback = X11ImCallback {
        client_data,
        callback: done_callback,
    };
    let caret_callback = X11ImCallback {
        client_data,
        callback: caret_callback,
    };

    // build the callback attribute list first so the IC can own the preedit lane
    let preedit_attributes = unsafe {
        XVaCreateNestedList(
            0,
            XN_PREEDIT_DRAW_CALLBACK.as_ptr().cast::<c_char>(),
            &draw_callback,
            XN_PREEDIT_DONE_CALLBACK.as_ptr().cast::<c_char>(),
            &done_callback,
            XN_PREEDIT_CARET_CALLBACK.as_ptr().cast::<c_char>(),
            &caret_callback,
            null::<c_void>(),
        )
    };
    if preedit_attributes.is_null() {
        return Err("destack.input.text.open: XVaCreateNestedList failed".to_string());
    }

    let input_context = unsafe {
        XCreateIC(
            input_method,
            XN_INPUT_STYLE.as_ptr().cast::<c_char>(),
            X11_IM_PREEDIT_CALLBACKS | X11_IM_STATUS_NOTHING,
            XN_CLIENT_WINDOW.as_ptr().cast::<c_char>(),
            window as c_ulong,
            XN_FOCUS_WINDOW.as_ptr().cast::<c_char>(),
            window as c_ulong,
            XN_PREEDIT_ATTRIBUTES.as_ptr().cast::<c_char>(),
            preedit_attributes,
            null::<c_void>(),
        )
    };

    unsafe {
        XFree(preedit_attributes);
    }

    if input_context.is_null() {
        return Err("destack.input.text.open: XCreateIC failed".to_string());
    }

    Ok(Some(input_context))
}

/// Destroy one native XIM input context.
pub(crate) fn destroy_input_context(input_context: X11InputContextHandle) {
    if input_context.is_null() {
        return;
    }

    unsafe {
        XDestroyIC(input_context);
    }
}

/// Focus one native XIM input context.
pub(crate) fn set_input_context_focus(input_context: X11InputContextHandle) {
    if input_context.is_null() {
        return;
    }

    unsafe {
        XSetICFocus(input_context);
    }
}

/// Unfocus one native XIM input context.
pub(crate) fn unset_input_context_focus(input_context: X11InputContextHandle) {
    if input_context.is_null() {
        return;
    }

    unsafe {
        XUnsetICFocus(input_context);
    }
}

/// Return whether one key press was consumed by the active input method.
pub(crate) fn filter_key_press(xlib_state: &X11XlibState, event: &KeyPressEvent) -> bool {
    let mut xevent = key_press_xevent(xlib_state.display, event);

    unsafe { XFilterEvent(&mut xevent, event.event.into()) != 0 }
}

/// Query one UTF-8 lookup result from one native XIM input context.
pub(crate) fn lookup_utf8(
    xlib_state: &X11XlibState,
    input_context: X11InputContextHandle,
    event: &KeyPressEvent,
) -> Result<X11LookupResult, String> {
    let mut key_event = key_press_struct(xlib_state.display, event);
    let mut status = X11_LOOKUP_NONE;
    let mut keysym = 0 as c_ulong;
    let mut buffer = vec![0u8; 64];

    let mut copied = unsafe {
        Xutf8LookupString(
            input_context,
            &mut key_event,
            buffer.as_mut_ptr().cast::<c_char>(),
            buffer.len() as c_int,
            &mut keysym,
            &mut status,
        )
    };

    if status == X11_BUFFER_OVERFLOW {
        buffer.resize(copied.max(0) as usize + 1, 0);
        copied = unsafe {
            Xutf8LookupString(
                input_context,
                &mut key_event,
                buffer.as_mut_ptr().cast::<c_char>(),
                buffer.len() as c_int,
                &mut keysym,
                &mut status,
            )
        };
    }

    if copied < 0 {
        return Err("destack.display.window.eventRead: Xutf8LookupString failed".to_string());
    }

    let copied = copied as usize;
    let text = String::from_utf8_lossy(&buffer[..copied]).into_owned();

    Ok(X11LookupResult {
        text,
        keysym: (keysym != 0).then_some(keysym as u32),
        status,
    })
}

/// Apply one renderer geometry hint to one native XIM context.
pub(crate) fn update_input_context_geometry(
    _xlib_state: &X11XlibState,
    input_context: X11InputContextHandle,
    geometry: InputTextGeometry,
) -> Result<(), String> {
    let caret_rectangle = geometry
        .composing_rectangle
        .or(geometry.caret_rectangle)
        .unwrap_or(geometry.editor_rectangle);
    let caret_rectangle = project_rectangle(geometry, caret_rectangle);
    let editor_rectangle = project_rectangle(geometry, geometry.editor_rectangle);

    let spot = XPoint {
        x: clamp_i16(caret_rectangle.x.round()),
        y: clamp_i16((caret_rectangle.y + caret_rectangle.height).round()),
    };
    let area = XRectangle {
        x: clamp_i16(editor_rectangle.x.round()),
        y: clamp_i16(editor_rectangle.y.round()),
        width: clamp_u16(editor_rectangle.width.round().max(1.0)),
        height: clamp_u16(editor_rectangle.height.round().max(1.0)),
    };

    let preedit_attributes = unsafe {
        XVaCreateNestedList(
            0,
            XN_SPOT_LOCATION.as_ptr().cast::<c_char>(),
            &spot,
            XN_AREA.as_ptr().cast::<c_char>(),
            &area,
            null::<c_void>(),
        )
    };
    if preedit_attributes.is_null() {
        return Err(
            "destack.input.text.syncWindowRepository: XVaCreateNestedList failed".to_string(),
        );
    }

    let error = unsafe {
        XSetICValues(
            input_context,
            XN_PREEDIT_ATTRIBUTES.as_ptr().cast::<c_char>(),
            preedit_attributes,
            null::<c_void>(),
        )
    };

    unsafe {
        XFree(preedit_attributes);
    }

    if error.is_null() {
        return Ok(());
    }

    let message = unsafe { CStr::from_ptr(error).to_string_lossy().into_owned() };

    Err(format!(
        "destack.input.text.syncWindowRepository: XSetICValues failed: {message}"
    ))
}

/// Return one multibyte preedit string from one XIM callback payload.
pub(crate) fn preedit_text_string(text: *const X11ImText) -> Option<String> {
    if text.is_null() {
        return None;
    }

    let text = unsafe { &*text };
    if text.length == 0 {
        return Some(String::new());
    }

    if text.encoding_is_wchar != 0 {
        let codepoints =
            unsafe { std::slice::from_raw_parts(text.string.wcs, text.length as usize) };
        let text = codepoints
            .iter()
            .filter_map(|codepoint| char::from_u32(*codepoint))
            .collect::<String>();

        return Some(text);
    }

    if text.string.mbs.is_null() {
        return Some(String::new());
    }

    let bytes =
        unsafe { std::slice::from_raw_parts(text.string.mbs.cast::<u8>(), text.length as usize) };

    Some(String::from_utf8_lossy(bytes).into_owned())
}

/// Build one synthetic xlib key event.
fn key_press_xevent(display: X11DisplayHandle, event: &KeyPressEvent) -> XEvent {
    let mut xevent = MaybeUninit::<XEvent>::zeroed();
    unsafe {
        xevent.as_mut_ptr().write(XEvent {
            xkey: key_press_struct(display, event),
        });
        xevent.assume_init()
    }
}

/// Build one synthetic xlib key event payload.
fn key_press_struct(display: X11DisplayHandle, event: &KeyPressEvent) -> XKeyPressedEvent {
    XKeyPressedEvent {
        type_: X11_KEY_PRESS_EVENT_TYPE,
        serial: event.sequence.into(),
        send_event: 0,
        display,
        window: event.event.into(),
        root: event.root.into(),
        subwindow: event.child.into(),
        time: event.time.into(),
        x: i32::from(event.event_x),
        y: i32::from(event.event_y),
        x_root: i32::from(event.root_x),
        y_root: i32::from(event.root_y),
        state: u16::from(event.state).into(),
        keycode: event.detail.into(),
        same_screen: if event.same_screen { 1 } else { 0 },
    }
}

/// Project one local text rectangle into target space.
fn project_rectangle(
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

/// Clamp one floating-point coordinate into the Xlib `short` range.
fn clamp_i16(value: f64) -> i16 {
    value.clamp(i16::MIN as f64, i16::MAX as f64) as i16
}

/// Clamp one floating-point extent into the Xlib `unsigned short` range.
fn clamp_u16(value: f64) -> u16 {
    value.clamp(1.0, u16::MAX as f64) as u16
}
