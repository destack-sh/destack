use std::ffi::c_void;
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex};

use windows_sys::Win32::Foundation::{
    DRAGDROP_E_ALREADYREGISTERED, DRAGDROP_E_NOTREGISTERED, E_POINTER, POINTL, RPC_E_CHANGED_MODE,
    S_FALSE, S_OK,
};
use windows_sys::Win32::System::Com::{
    DVASPECT_CONTENT, FORMATETC, IDataObject, STGMEDIUM, TYMED_HGLOBAL,
};
use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
use windows_sys::Win32::System::Ole::{
    CF_HDROP, CF_UNICODETEXT, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE,
    DROPEFFECT_NONE, IDropTarget, OleInitialize, OleUninitialize, RegisterDragDrop,
    ReleaseStgMedium, RevokeDragDrop,
};
use windows_sys::Win32::UI::Shell::{DragQueryFileW, HDROP};
use windows_sys::core::{GUID, HRESULT};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::display::WindowPosition;
use crate::platform::{PlatformError, core as core_platform, resource};

use crate::platform::display::windows::win32::core::Win32RuntimeState;
use crate::platform::display::windows::win32::event;
use crate::platform::display::windows::win32::model::Win32WindowHostState;

/// COM interface identifier for `IDropTarget`.
const IID_IDROPTARGET: GUID = GUID::from_u128(0x00000122_0000_0000_c000_000000000046);
/// Allowed drop effects mask.
const ALLOWED_DROP_EFFECTS: u32 = DROPEFFECT_COPY | DROPEFFECT_MOVE | DROPEFFECT_LINK;

/// Mutable state for one active drop session.
#[derive(Debug, Default)]
struct DropSessionState {
    /// Whether one drag session is active over this window.
    is_active: bool,
    /// Whether this session currently has one accepted payload type.
    accepts_payload: bool,
    /// Last hovered path payload from `fileHovered`.
    last_hover_path_utf16: Option<Vec<u16>>,
    /// Last hovered position payload from `fileHovered`.
    last_hover_position: Option<WindowPosition>,
}

/// Runtime-owned drop target callback object.
#[repr(C)]
struct WindowDropTargetCallback {
    /// COM callback vtable.
    vtable: *const WindowDropTargetVTable,
    /// Manual COM reference count.
    reference_count: AtomicU32,
    /// Runtime window handle for event metadata.
    window: resource::WindowHandle,
    /// Shared window-event runtime state.
    runtime_state: Arc<Win32RuntimeState>,
    /// Mutable drop-session state.
    session_state: Mutex<DropSessionState>,
}

/// Vtable shape for `IDropTarget`.
#[repr(C)]
struct WindowDropTargetVTable {
    /// QueryInterface callback.
    query_interface:
        unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    /// AddRef callback.
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    /// Release callback.
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    /// DragEnter callback.
    drag_enter: unsafe extern "system" fn(
        *mut c_void,
        IDataObject,
        u32,
        POINTL,
        *mut DROPEFFECT,
    ) -> HRESULT,
    /// DragOver callback.
    drag_over: unsafe extern "system" fn(*mut c_void, u32, POINTL, *mut DROPEFFECT) -> HRESULT,
    /// DragLeave callback.
    drag_leave: unsafe extern "system" fn(*mut c_void) -> HRESULT,
    /// Drop callback.
    drop_data: unsafe extern "system" fn(
        *mut c_void,
        IDataObject,
        u32,
        POINTL,
        *mut DROPEFFECT,
    ) -> HRESULT,
}

/// Vtable prefix for `IDataObject` callbacks used by this module.
#[repr(C)]
struct DataObjectVTablePrefix {
    /// QueryInterface callback.
    query_interface:
        unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    /// AddRef callback.
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    /// Release callback.
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    /// GetData callback.
    get_data: unsafe extern "system" fn(*mut c_void, *const FORMATETC, *mut STGMEDIUM) -> HRESULT,
}

// shared vtable for window drop target callbacks
core_platform::define_com_callback_vtable!(
    static = WINDOW_DROP_TARGET_VTABLE,
    type = WindowDropTargetVTable,
    value = WindowDropTargetVTable,
    query_interface = drop_target_query_interface,
    add_ref = drop_target_add_ref,
    release = drop_target_release,
    methods = {
        drag_enter = drop_target_drag_enter,
        drag_over = drop_target_drag_over,
        drag_leave = drop_target_drag_leave,
        drop_data = drop_target_drop_data
    }
);

core_platform::define_com_iunknown_methods!(
    object = WindowDropTargetCallback,
    from_raw = drop_target_from_raw,
    query_interface = drop_target_query_interface,
    add_ref = drop_target_add_ref,
    release = drop_target_release,
    interfaces = [IID_IDROPTARGET]
);

/// Return whether one HRESULT indicates success.
fn hresult_succeeded(status: HRESULT) -> bool {
    status >= 0
}

/// Build one HRESULT-backed runtime I/O error.
fn hresult_io_error(
    operation: &'static str,
    syscall: &'static str,
    status: HRESULT,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        Some(status),
        Some(operation.to_string()),
        None,
        format!("{syscall} failed: {} ({status:#x})", message.into()),
    ))
    .boxed()
}

/// Create one runtime-owned drop target callback object.
fn create_drop_target_callback(
    window: resource::WindowHandle,
    runtime_state: Arc<Win32RuntimeState>,
) -> *mut c_void {
    let callback = Box::new(WindowDropTargetCallback {
        vtable: &WINDOW_DROP_TARGET_VTABLE,
        reference_count: AtomicU32::new(1),
        window,
        runtime_state,
        session_state: Mutex::new(DropSessionState::default()),
    });

    Box::into_raw(callback) as *mut c_void
}

/// Release one drop target callback pointer.
fn release_drop_target_callback(callback_pointer: *mut c_void) {
    unsafe {
        core_platform::com_release_with(callback_pointer, drop_target_release);
    }
}

/// Build one `FORMATETC` payload for one clipboard format.
fn format_for_clipboard(format: u16) -> FORMATETC {
    FORMATETC {
        cfFormat: format,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT,
        lindex: -1,
        tymed: TYMED_HGLOBAL as u32,
    }
}

/// Return one point payload as one window position payload.
fn window_position_from_point(point: POINTL) -> WindowPosition {
    WindowPosition {
        x: point.x,
        y: point.y,
    }
}

/// Resolve one effective drop effect from source capabilities and payload support.
fn resolved_drop_effect(source_effect: u32, accepts_payload: bool) -> u32 {
    // reject drop lanes when payload is unsupported
    if !accepts_payload {
        return DROPEFFECT_NONE;
    }

    // mask source capabilities to supported win32 effects
    let source_effect = source_effect & ALLOWED_DROP_EFFECTS;
    if source_effect == 0 {
        return DROPEFFECT_NONE;
    }

    // prioritize copy over move over link
    if (source_effect & DROPEFFECT_COPY) != 0 {
        return DROPEFFECT_COPY;
    }

    if (source_effect & DROPEFFECT_MOVE) != 0 {
        return DROPEFFECT_MOVE;
    }

    DROPEFFECT_LINK
}

/// Set one COM drag and drop effect payload.
fn set_drop_effect(effect: *mut DROPEFFECT, accepts_payload: bool) {
    // ignore null output pointers from host callback
    if effect.is_null() {
        return;
    }

    // resolve effective drop effect and write it back to host lane
    let source_effect = unsafe { *effect };
    let resolved_effect = resolved_drop_effect(source_effect, accepts_payload);
    unsafe {
        *effect = resolved_effect;
    }
}

/// Read one `IDataObject::GetData` payload into one `STGMEDIUM`.
unsafe fn data_object_get_data(
    data_object: IDataObject,
    format: &FORMATETC,
    medium: &mut STGMEDIUM,
) -> HRESULT {
    if data_object.is_null() {
        return E_POINTER;
    }

    let vtable = unsafe { *(data_object as *mut *const DataObjectVTablePrefix) };
    if vtable.is_null() {
        return E_POINTER;
    }

    unsafe {
        ((*vtable).get_data)(
            data_object,
            format as *const FORMATETC,
            medium as *mut STGMEDIUM,
        )
    }
}

/// Read one dropped file list from one data-object payload.
fn drop_file_paths(data_object: IDataObject) -> Option<Vec<Vec<u16>>> {
    // request file list payload via cf hdrop
    let format = format_for_clipboard(CF_HDROP);
    let mut medium = unsafe { std::mem::zeroed::<STGMEDIUM>() };
    let status = unsafe { data_object_get_data(data_object, &format, &mut medium) };
    if !hresult_succeeded(status) {
        return None;
    }

    // decode file paths from hglobal payload when available
    let paths = if medium.tymed != TYMED_HGLOBAL as u32 {
        None
    } else {
        let hglobal = unsafe { medium.u.hGlobal };
        if hglobal.is_null() {
            None
        } else {
            let hdrop = hglobal as HDROP;
            let count = unsafe { DragQueryFileW(hdrop, u32::MAX, std::ptr::null_mut(), 0) };
            let mut paths = Vec::with_capacity(count as usize);

            // read each dropped path as utf16 payload
            for index in 0..count {
                let length = unsafe { DragQueryFileW(hdrop, index, std::ptr::null_mut(), 0) };
                if length == 0 {
                    continue;
                }

                let mut wide = vec![0u16; length as usize + 1];
                let actual =
                    unsafe { DragQueryFileW(hdrop, index, wide.as_mut_ptr(), wide.len() as u32) };
                if actual == 0 {
                    continue;
                }

                wide.truncate(actual as usize);
                paths.push(wide);
            }

            Some(paths)
        }
    };

    // release storage medium before returning
    unsafe {
        ReleaseStgMedium(&mut medium);
    }

    paths
}

/// Read one dropped text payload from one data-object payload.
fn drop_text(data_object: IDataObject) -> Option<String> {
    // request unicode text payload via clipboard format
    let format = format_for_clipboard(CF_UNICODETEXT);
    let mut medium = unsafe { std::mem::zeroed::<STGMEDIUM>() };
    let status = unsafe { data_object_get_data(data_object, &format, &mut medium) };
    if !hresult_succeeded(status) {
        return None;
    }

    // decode utf16 text from hglobal payload when available
    let text = if medium.tymed != TYMED_HGLOBAL as u32 {
        None
    } else {
        let hglobal = unsafe { medium.u.hGlobal };
        if hglobal.is_null() {
            None
        } else {
            let pointer = unsafe { GlobalLock(hglobal) } as *const u16;
            if pointer.is_null() {
                None
            } else {
                // map nul terminated utf16 data into one string payload
                let size_bytes = unsafe { GlobalSize(hglobal) };
                let unit_count = size_bytes / std::mem::size_of::<u16>();
                let units = unsafe { std::slice::from_raw_parts(pointer, unit_count) };
                let end = units
                    .iter()
                    .position(|value| *value == 0)
                    .unwrap_or(units.len());
                let text = String::from_utf16_lossy(&units[..end]);
                unsafe {
                    let _ = GlobalUnlock(hglobal);
                }
                Some(text)
            }
        }
    };

    // release storage medium before returning
    unsafe {
        ReleaseStgMedium(&mut medium);
    }

    text
}

/// Publish one drop-started event when this session has not started yet.
fn publish_drop_started(callback: &WindowDropTargetCallback, state: &mut DropSessionState) {
    if state.is_active {
        return;
    }

    state.is_active = true;
    event::publish_window_drop_started_event(&callback.runtime_state, callback.window);
}

/// Handle one drag-enter callback.
unsafe extern "system" fn drop_target_drag_enter(
    this: *mut c_void,
    data_object: IDataObject,
    _grf_key_state: u32,
    point: POINTL,
    effect: *mut DROPEFFECT,
) -> HRESULT {
    let callback = unsafe { drop_target_from_raw(this) };
    let callback = unsafe { callback.as_ref() };
    let position = Some(window_position_from_point(point));
    let file_paths = drop_file_paths(data_object).unwrap_or_default();
    let first_path = file_paths.first().cloned();
    let accepts_payload = !file_paths.is_empty() || drop_text(data_object).is_some();
    let mut session_state = callback
        .session_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    publish_drop_started(callback, &mut session_state);
    session_state.accepts_payload = accepts_payload;

    if let Some(path_utf16) = first_path {
        let should_emit = session_state.last_hover_path_utf16.as_ref() != Some(&path_utf16)
            || session_state.last_hover_position != position;
        session_state.last_hover_path_utf16 = Some(path_utf16.clone());
        session_state.last_hover_position = position;

        if should_emit {
            event::publish_window_file_hovered_event(
                &callback.runtime_state,
                callback.window,
                Some(path_utf16),
                position,
            );
        }
    } else {
        session_state.last_hover_path_utf16 = None;
        session_state.last_hover_position = position;
    }

    drop(session_state);
    set_drop_effect(effect, accepts_payload);
    S_OK
}

/// Handle one drag-over callback.
unsafe extern "system" fn drop_target_drag_over(
    this: *mut c_void,
    _grf_key_state: u32,
    point: POINTL,
    effect: *mut DROPEFFECT,
) -> HRESULT {
    let callback = unsafe { drop_target_from_raw(this) };
    let callback = unsafe { callback.as_ref() };
    let position = Some(window_position_from_point(point));
    let mut session_state = callback
        .session_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let accepts_payload = session_state.accepts_payload;
    let path = session_state.last_hover_path_utf16.clone();
    let should_emit_hover = path.is_some() && session_state.last_hover_position != position;
    session_state.last_hover_position = position;
    drop(session_state);

    if should_emit_hover {
        event::publish_window_file_hovered_event(
            &callback.runtime_state,
            callback.window,
            path,
            position,
        );
    }

    set_drop_effect(effect, accepts_payload);
    S_OK
}

/// Handle one drag-leave callback.
unsafe extern "system" fn drop_target_drag_leave(this: *mut c_void) -> HRESULT {
    // resolve callback and lock drag session state
    let callback = unsafe { drop_target_from_raw(this) };
    let callback = unsafe { callback.as_ref() };
    let mut session_state = callback
        .session_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if !session_state.is_active {
        return S_OK;
    }

    // capture previous hover payload and clear session
    let previous_path = session_state.last_hover_path_utf16.clone();
    let position = session_state.last_hover_position;
    *session_state = DropSessionState::default();
    drop(session_state);

    // publish hover leave when one hover payload exists
    if previous_path.is_some() || position.is_some() {
        event::publish_window_file_hover_left_event(
            &callback.runtime_state,
            callback.window,
            previous_path,
            position,
        );
    }

    // close drop session as cancelled
    event::publish_window_drop_cancelled_event(&callback.runtime_state, callback.window);
    S_OK
}

/// Handle one drop callback.
unsafe extern "system" fn drop_target_drop_data(
    this: *mut c_void,
    data_object: IDataObject,
    _grf_key_state: u32,
    point: POINTL,
    effect: *mut DROPEFFECT,
) -> HRESULT {
    let callback = unsafe { drop_target_from_raw(this) };
    let callback = unsafe { callback.as_ref() };
    let position = Some(window_position_from_point(point));
    let file_paths = drop_file_paths(data_object).unwrap_or_default();
    let text_payload = drop_text(data_object);
    let accepts_payload = !file_paths.is_empty() || text_payload.is_some();
    let mut session_state = callback
        .session_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    publish_drop_started(callback, &mut session_state);
    let previous_path = session_state.last_hover_path_utf16.clone();
    let previous_position = session_state.last_hover_position.or(position);
    *session_state = DropSessionState::default();
    drop(session_state);

    if previous_path.is_some() || previous_position.is_some() {
        event::publish_window_file_hover_left_event(
            &callback.runtime_state,
            callback.window,
            previous_path,
            previous_position,
        );
    }

    if !accepts_payload {
        event::publish_window_drop_cancelled_event(&callback.runtime_state, callback.window);
        set_drop_effect(effect, false);
        return S_OK;
    }

    for path_utf16 in file_paths {
        event::publish_window_file_dropped_event(
            &callback.runtime_state,
            callback.window,
            Some(path_utf16),
            position,
        );
    }

    if let Some(text) = text_payload {
        event::publish_window_text_dropped_event(
            &callback.runtime_state,
            callback.window,
            text,
            position,
        );
    }

    event::publish_window_drop_completed_event(&callback.runtime_state, callback.window);
    set_drop_effect(effect, true);
    S_OK
}

/// Register one drop target for one live window host state.
pub(crate) fn register_window_drop_target(
    host_state: &mut Win32WindowHostState,
    window: resource::WindowHandle,
    runtime_state: &Arc<Win32RuntimeState>,
    operation: &'static str,
) -> RuntimeResult<()> {
    if host_state.drop_target_callback != 0 {
        return Ok(());
    }

    let ole_status = unsafe { OleInitialize(std::ptr::null()) };
    let ole_initialized = if ole_status == S_OK || ole_status == S_FALSE {
        true
    } else if ole_status == RPC_E_CHANGED_MODE {
        false
    } else {
        return Err(hresult_io_error(
            operation,
            "OleInitialize",
            ole_status,
            "failed to initialize OLE for window drop target",
        ));
    };

    let callback_pointer = create_drop_target_callback(window, Arc::clone(runtime_state));
    let mut register_status =
        unsafe { RegisterDragDrop(host_state.hwnd, callback_pointer as IDropTarget) };
    if register_status == DRAGDROP_E_ALREADYREGISTERED {
        let _ = unsafe { RevokeDragDrop(host_state.hwnd) };
        register_status =
            unsafe { RegisterDragDrop(host_state.hwnd, callback_pointer as IDropTarget) };
    }
    if !hresult_succeeded(register_status) {
        release_drop_target_callback(callback_pointer);
        if ole_initialized {
            unsafe {
                OleUninitialize();
            }
        }
        return Err(hresult_io_error(
            operation,
            "RegisterDragDrop",
            register_status,
            "failed to register window drop target",
        ));
    }

    host_state.drop_target_callback = callback_pointer as usize;
    host_state.drop_target_ole_initialized = ole_initialized;
    Ok(())
}

/// Unregister one drop target from one live window host state.
pub(crate) fn unregister_window_drop_target(host_state: &mut Win32WindowHostState) {
    let callback_pointer = host_state.drop_target_callback as *mut c_void;
    let ole_initialized = host_state.drop_target_ole_initialized;
    host_state.drop_target_callback = 0;
    host_state.drop_target_ole_initialized = false;

    if !callback_pointer.is_null() {
        let revoke_status = unsafe { RevokeDragDrop(host_state.hwnd) };
        if revoke_status != DRAGDROP_E_NOTREGISTERED && !hresult_succeeded(revoke_status) {
            // ignore teardown errors during best effort cleanup
        }

        release_drop_target_callback(callback_pointer);
    }

    if ole_initialized {
        unsafe {
            OleUninitialize();
        }
    }
}

#[cfg(test)]
mod tests {
    use windows_sys::Win32::System::Ole::{
        DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE, DROPEFFECT_NONE,
    };

    use super::resolved_drop_effect;

    /// Drop effect resolution should reject unsupported payloads.
    #[test]
    fn test_resolved_drop_effect_rejects_unsupported_payloads() {
        assert_eq!(
            resolved_drop_effect(DROPEFFECT_COPY, false),
            DROPEFFECT_NONE
        );
    }

    /// Drop effect resolution should prefer copy then move then link.
    #[test]
    fn test_resolved_drop_effect_prefers_copy_move_link_order() {
        assert_eq!(resolved_drop_effect(DROPEFFECT_COPY, true), DROPEFFECT_COPY);
        assert_eq!(resolved_drop_effect(DROPEFFECT_MOVE, true), DROPEFFECT_MOVE);
        assert_eq!(resolved_drop_effect(DROPEFFECT_LINK, true), DROPEFFECT_LINK);
        assert_eq!(
            resolved_drop_effect(DROPEFFECT_MOVE | DROPEFFECT_LINK, true),
            DROPEFFECT_MOVE,
        );
        assert_eq!(
            resolved_drop_effect(DROPEFFECT_COPY | DROPEFFECT_MOVE, true),
            DROPEFFECT_COPY,
        );
        assert_eq!(resolved_drop_effect(0, true), DROPEFFECT_NONE);
    }
}
