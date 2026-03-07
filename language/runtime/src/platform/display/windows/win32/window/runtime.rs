use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};

use windows_sys::Win32::Foundation::{GetLastError, HWND, SetLastError};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GWLP_USERDATA, GetWindowLongPtrW, IsWindow, SetWindowLongPtrW,
};

use crate::diagnostic::{DiagnosticStore, RuntimeResult};
use crate::platform::display::WindowCursorMode;
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::super::model::Win32WindowBinding;
use super::super::{core, event};

/// Runtime-owned mutable state for win32 window bindings.
#[derive(Debug)]
pub(crate) struct WindowRuntimeState {
    /// Shared global cursor visibility state.
    pub(super) cursor_visible_state: Mutex<Option<bool>>,
    /// Per-window cursor policy lanes used to derive process-global cursor state.
    pub(super) cursor_policy_by_window: Mutex<HashMap<resource::WindowHandle, CursorPolicyState>>,
    /// Monotonic counter used for stable cursor policy ordering.
    pub(super) next_cursor_policy_sequence: AtomicU64,
    /// Monotonic counter used for stable runtime window identifiers.
    next_window_identifier: AtomicU64,
    /// Runtime diagnostics store for callback and best-effort lanes.
    pub(super) diagnostics: Arc<DiagnosticStore>,
}

impl Default for WindowRuntimeState {
    /// Create one default window runtime state.
    fn default() -> Self {
        Self::new(Arc::new(DiagnosticStore::default()))
    }
}

impl WindowRuntimeState {
    /// Create one window runtime state with explicit diagnostics storage.
    fn new(diagnostics: Arc<DiagnosticStore>) -> Self {
        Self {
            cursor_visible_state: Mutex::new(None),
            cursor_policy_by_window: Mutex::new(HashMap::new()),
            next_cursor_policy_sequence: AtomicU64::new(1),
            next_window_identifier: AtomicU64::new(1),
            diagnostics,
        }
    }
}

/// Per-window cursor policy snapshot.
#[derive(Debug, Clone, Copy)]
pub(super) struct CursorPolicyState {
    /// Host window handle associated with this policy.
    pub(super) hwnd: HWND,
    /// Per-window cursor visibility preference.
    pub(super) cursor_visible: bool,
    /// Per-window cursor mode preference.
    pub(super) cursor_mode: WindowCursorMode,
    /// Monotonic sequence used for most-recent policy ordering.
    pub(super) sequence: u64,
}

/// Runtime mapping payload for one live hwnd.
#[derive(Clone)]
pub(super) struct WindowRuntimeEntry {
    /// Runtime window handle associated with this hwnd.
    pub(super) window: resource::WindowHandle,
    /// Weak binding reference for this window.
    pub(super) binding: Weak<Mutex<Win32WindowBinding>>,
    /// Runtime-owned display event stream state.
    pub(super) event_runtime_state: Arc<event::DisplayEventRuntimeState>,
    /// Runtime-owned window state for cursor/global cleanup lanes.
    pub(super) window_runtime_state: Arc<WindowRuntimeState>,
}

/// Return runtime-owned win32 window state.
pub(super) fn window_runtime_state(context: &BindingCallContext) -> Arc<WindowRuntimeState> {
    let diagnostics = Arc::clone(&context.runtime().diagnostics);
    context
        .runtime()
        .platform_state
        .display
        .window_runtime_state(|| WindowRuntimeState::new(diagnostics))
}

/// Allocate one stable runtime window identifier.
pub(super) fn next_window_identifier(context: &BindingCallContext) -> u64 {
    let runtime_state = window_runtime_state(context);
    runtime_state
        .next_window_identifier
        .fetch_add(1, Ordering::Relaxed)
}

/// Allocate one stable cursor-policy sequence number.
pub(super) fn next_cursor_policy_sequence(runtime_state: &Arc<WindowRuntimeState>) -> u64 {
    runtime_state
        .next_cursor_policy_sequence
        .fetch_add(1, Ordering::Relaxed)
}

/// Register one live hwnd mapping for runtime window callbacks.
pub(super) fn register_runtime_window(
    hwnd: HWND,
    entry: WindowRuntimeEntry,
    operation: &'static str,
) -> RuntimeResult<()> {
    let entry = Box::new(entry);
    let entry = Box::into_raw(entry);
    let previous = unsafe {
        SetLastError(0);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, entry as isize)
    };

    // evaluate this condition
    if previous == 0 {
        let error_code = unsafe { GetLastError() };

        // evaluate this condition
        if error_code != 0 {
            unsafe {
                drop(Box::from_raw(entry));
            }
            return Err(core::io_error_with_code(
                operation,
                "SetWindowLongPtrW",
                error_code as u32,
                "failed to register runtime window entry",
            ));
        }
    }

    Ok(())
}

/// Unregister one live hwnd mapping.
pub(super) fn unregister_runtime_window(hwnd: HWND) {
    let pointer = unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) } as *mut WindowRuntimeEntry;

    // evaluate this condition
    if pointer.is_null() {
        return;
    }

    unsafe {
        drop(Box::from_raw(pointer));
    }
}

/// Resolve one runtime hwnd entry.
pub(super) fn runtime_window_entry(hwnd: HWND) -> Option<WindowRuntimeEntry> {
    let pointer = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut WindowRuntimeEntry;

    // evaluate this condition
    if pointer.is_null() {
        return None;
    }

    let entry = unsafe { &*pointer };
    Some(entry.clone())
}

/// Return the current host thread identifier.
pub(super) fn current_thread_id() -> u32 {
    unsafe { GetCurrentThreadId() }
}

/// Ensure the calling thread owns this window binding.
pub(super) fn ensure_window_thread(
    binding: &Win32WindowBinding,
    operation: &'static str,
) -> RuntimeResult<()> {
    let current = current_thread_id();

    // evaluate this condition
    if current == binding.owner_thread_id {
        return Ok(());
    }

    Err(core_platform::invalid_argument(
        "window",
        format!(
            "{operation} must run on owner thread {}, current thread is {current}",
            binding.owner_thread_id
        ),
    ))
}

/// Resolve whether one live window handle is still valid.
pub(super) fn is_live_hwnd(hwnd: HWND) -> bool {
    unsafe { IsWindow(hwnd) != 0 }
}
