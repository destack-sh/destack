use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::thread;
use std::time::Duration;

use windows_sys::Win32::Foundation::{
    ERROR_CLASS_ALREADY_EXISTS, GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT,
    SetLastError, WPARAM,
};
use windows_sys::Win32::Graphics::Gdi::{ClientToScreen, GetDeviceCaps, LOGPIXELSX, UpdateWindow};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CW_USEDEFAULT, ClipCursor, CreateWindowExW, DefWindowProcW, DestroyWindow,
    DispatchMessageW, FLASHW_TIMERNOFG, FLASHW_TRAY, FLASHWINFO, FlashWindowEx, GWL_EXSTYLE,
    GWL_STYLE, GetClientRect, GetForegroundWindow, GetWindowRect, HWND_NOTOPMOST, HWND_TOPMOST,
    IDC_APPSTARTING, IDC_ARROW, IDC_CROSS, IDC_HAND, IDC_HELP, IDC_IBEAM, IDC_NO, IDC_SIZEALL,
    IDC_SIZENESW, IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE, IDC_WAIT, IsIconic, IsWindowVisible,
    IsZoomed, LoadCursorW, MSG, PM_REMOVE, PeekMessageW, RegisterClassW, SIZE_MAXIMIZED,
    SIZE_MINIMIZED, SW_HIDE, SW_MAXIMIZE, SW_MINIMIZE, SW_SHOW, SW_SHOWNA, SWP_FRAMECHANGED,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetCursor, SetCursorPos,
    SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, SetWindowTextW, ShowCursor, ShowWindow,
    TranslateMessage, WA_INACTIVE, WINDOW_EX_STYLE, WINDOW_STYLE, WM_ACTIVATE, WM_CLOSE,
    WM_DESTROY, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_KILLFOCUS, WM_MOVE, WM_QUIT, WM_SETFOCUS,
    WM_SHOWWINDOW, WM_SIZE, WM_THEMECHANGED, WM_WINDOWPOSCHANGED, WNDCLASSW, WS_CAPTION,
    WS_EX_LAYERED, WS_EX_TOPMOST, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU,
    WS_THICKFRAME,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayBackend, WindowAttentionLevel, WindowCursorIcon, WindowCursorMode, WindowDescriptor,
    WindowLogicalSize, WindowMode, WindowModeOptions, WindowOptions, WindowPhysicalSize,
    WindowPosition, WindowSizeConstraints, WindowState, WindowTheme, WindowVisibility,
};
use crate::platform::{NativeStringRef, core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::model::{ExclusiveModeRestore, Win32WindowBinding};
use super::{core, event, monitor, resource as display_resource};

/// Shared registration state for the Win32 window class.
static WINDOW_CLASS_REGISTERED: OnceLock<()> = OnceLock::new();
/// Shared class-name payload for Win32 window class registration.
static WINDOW_CLASS_NAME: OnceLock<Vec<u16>> = OnceLock::new();
/// Shared global cursor visibility state.
static CURSOR_VISIBLE_STATE: OnceLock<Mutex<Option<bool>>> = OnceLock::new();
/// Monotonic counter used for stable runtime window identifiers.
static NEXT_WINDOW_IDENTIFIER: AtomicU64 = AtomicU64::new(1);
/// Shared runtime registry for one live hwnd to one runtime window binding mapping.
static WINDOW_RUNTIME_REGISTRY: OnceLock<Mutex<HashMap<HWND, WindowRuntimeEntry>>> =
    OnceLock::new();

/// Runtime mapping payload for one live hwnd.
#[derive(Clone)]
struct WindowRuntimeEntry {
    /// Runtime window handle associated with this hwnd.
    window: resource::WindowHandle,
    /// Weak binding reference for this window.
    binding: Weak<Mutex<Win32WindowBinding>>,
}

/// Return one shared global cursor visibility state lock.
fn cursor_visible_state() -> &'static Mutex<Option<bool>> {
    CURSOR_VISIBLE_STATE.get_or_init(|| Mutex::new(None))
}

/// Return one shared runtime hwnd registry lock.
fn window_runtime_registry() -> &'static Mutex<HashMap<HWND, WindowRuntimeEntry>> {
    WINDOW_RUNTIME_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register one live hwnd mapping for runtime window callbacks.
fn register_runtime_window(
    hwnd: HWND,
    window: resource::WindowHandle,
    binding: &Arc<Mutex<Win32WindowBinding>>,
) {
    let mut registry = window_runtime_registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    registry.insert(
        hwnd,
        WindowRuntimeEntry {
            window,
            binding: Arc::downgrade(binding),
        },
    );
}

/// Unregister one live hwnd mapping.
fn unregister_runtime_window(hwnd: HWND) {
    let mut registry = window_runtime_registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let _ = registry.remove(&hwnd);
}

/// Resolve one runtime hwnd entry.
fn runtime_window_entry(hwnd: HWND) -> Option<WindowRuntimeEntry> {
    let registry = window_runtime_registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    registry.get(&hwnd).cloned()
}

/// Return the current host thread identifier.
fn current_thread_id() -> u32 {
    unsafe { GetCurrentThreadId() }
}

/// Ensure the calling thread owns this window binding.
fn ensure_window_thread(
    binding: &Win32WindowBinding,
    operation: &'static str,
) -> RuntimeResult<()> {
    let current = current_thread_id();
    if current == binding.owner_thread_id {
        return Ok(());
    }

    Err(core::invalid_argument(
        "window",
        format!(
            "{operation} must run on owner thread {}, current thread is {current}",
            binding.owner_thread_id
        ),
    ))
}

/// Normalize one window logical-size payload.
fn normalize_logical_size(
    size: WindowLogicalSize,
    field: &'static str,
) -> RuntimeResult<WindowLogicalSize> {
    if !size.width.is_finite() || !size.height.is_finite() {
        return Err(core::invalid_argument(field, "logical size must be finite"));
    }

    if size.width <= 0.0 || size.height <= 0.0 {
        return Err(core::invalid_argument(
            field,
            "logical size dimensions must be greater than zero",
        ));
    }

    Ok(size)
}

/// Normalize one window physical-size payload.
fn normalize_physical_size(
    size: WindowPhysicalSize,
    field: &'static str,
) -> RuntimeResult<WindowPhysicalSize> {
    if size.width == 0 || size.height == 0 {
        return Err(core::invalid_argument(
            field,
            "physical size dimensions must be greater than zero",
        ));
    }

    Ok(size)
}

/// Clamp one logical-size payload against optional size constraints.
fn clamp_logical_size(
    size: WindowLogicalSize,
    constraints: Option<WindowSizeConstraints>,
) -> WindowLogicalSize {
    let Some(constraints) = constraints else {
        return size;
    };

    let mut width = size.width;
    let mut height = size.height;

    if let Some(min) = constraints.min {
        width = width.max(min.width);
        height = height.max(min.height);
    }

    if let Some(max) = constraints.max {
        width = width.min(max.width);
        height = height.min(max.height);
    }

    WindowLogicalSize { width, height }
}

/// Resolve one win32 scale factor for one window handle.
fn window_scale_factor_milli(hwnd: HWND) -> u32 {
    let hdc = unsafe { windows_sys::Win32::Graphics::Gdi::GetDC(hwnd) };
    if hdc == 0 {
        return 1000;
    }

    let dpi_x = unsafe { GetDeviceCaps(hdc, LOGPIXELSX as i32) };
    unsafe {
        windows_sys::Win32::Graphics::Gdi::ReleaseDC(hwnd, hdc);
    }

    if dpi_x <= 0 {
        return 1000;
    }

    ((dpi_x as u32).saturating_mul(1000) / 96).max(1)
}

/// Convert one logical-size payload into one physical-size payload.
fn logical_to_physical(size: WindowLogicalSize, scale_factor_milli: u32) -> WindowPhysicalSize {
    let scale = if scale_factor_milli == 0 {
        1.0
    } else {
        scale_factor_milli as f64 / 1000.0
    };

    let width = (size.width * scale).round().max(1.0) as u32;
    let height = (size.height * scale).round().max(1.0) as u32;
    WindowPhysicalSize { width, height }
}

/// Convert one physical-size payload into one logical-size payload.
fn physical_to_logical(size: WindowPhysicalSize, scale_factor_milli: u32) -> WindowLogicalSize {
    let scale = if scale_factor_milli == 0 {
        1.0
    } else {
        scale_factor_milli as f64 / 1000.0
    };

    WindowLogicalSize {
        width: (size.width as f64 / scale).max(1.0),
        height: (size.height as f64 / scale).max(1.0),
    }
}

/// Convert one physical dimension into one i32 window-api dimension.
fn dimension_to_i32(value: u32, field: &'static str) -> RuntimeResult<i32> {
    if value > i32::MAX as u32 {
        return Err(core::invalid_argument(
            field,
            "dimension exceeds Win32 i32 range",
        ));
    }

    Ok(value as i32)
}

/// Resolve one non-client outer size from one requested client size.
fn outer_size_from_client_size(
    client_size: WindowPhysicalSize,
    style: WINDOW_STYLE,
    ex_style: WINDOW_EX_STYLE,
    operation: &'static str,
) -> RuntimeResult<(i32, i32)> {
    let width = dimension_to_i32(client_size.width, "size.width")?;
    let height = dimension_to_i32(client_size.height, "size.height")?;
    let mut rectangle = RECT {
        left: 0,
        top: 0,
        right: width,
        bottom: height,
    };

    let status = unsafe { AdjustWindowRectEx(&mut rectangle, style, 0, ex_style) };
    if status == 0 {
        return Err(core::io_error(
            operation,
            "AdjustWindowRectEx",
            "failed to compute non-client window rectangle",
        ));
    }

    let outer_width = (rectangle.right - rectangle.left).max(1);
    let outer_height = (rectangle.bottom - rectangle.top).max(1);
    Ok((outer_width, outer_height))
}

/// Return one ShowWindow command for one visibility state.
fn show_command(visibility: WindowVisibility) -> i32 {
    match visibility {
        WindowVisibility::Visible => SW_SHOW,
        WindowVisibility::Hidden => SW_HIDE,
        WindowVisibility::Minimized => SW_MINIMIZE,
        WindowVisibility::Maximized => SW_MAXIMIZE,
    }
}

/// Return one open-time ShowWindow command that respects focus behavior.
fn show_command_on_open(visibility: WindowVisibility, focus_on_show: bool) -> i32 {
    match visibility {
        WindowVisibility::Visible if !focus_on_show => SW_SHOWNA,
        _ => show_command(visibility),
    }
}

/// Resolve visibility from one live hwnd state.
fn visibility_from_hwnd(hwnd: HWND) -> WindowVisibility {
    if unsafe { IsWindowVisible(hwnd) } == 0 {
        return WindowVisibility::Hidden;
    }

    if unsafe { IsIconic(hwnd) } != 0 {
        return WindowVisibility::Minimized;
    }

    if unsafe { IsZoomed(hwnd) } != 0 {
        return WindowVisibility::Maximized;
    }

    WindowVisibility::Visible
}

/// Build one Win32 style payload from one window binding snapshot.
fn window_style_for_binding(binding: &Win32WindowBinding) -> WINDOW_STYLE {
    let mut style: WINDOW_STYLE = if binding.mode.mode == WindowMode::Windowed {
        WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX
    } else {
        WS_POPUP
    };

    if binding.resizable && binding.mode.mode == WindowMode::Windowed {
        style |= WS_THICKFRAME | WS_MAXIMIZEBOX;
    }

    if !binding.decorated {
        style &= !(WS_CAPTION | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU);
        style |= WS_POPUP;
    }

    style
}

/// Build one Win32 ex-style payload from one window binding snapshot.
fn window_ex_style_for_binding(binding: &Win32WindowBinding) -> WINDOW_EX_STYLE {
    let mut ex_style: WINDOW_EX_STYLE = 0;

    if binding.transparent {
        ex_style |= WS_EX_LAYERED;
    }

    if binding.always_on_top {
        ex_style |= WS_EX_TOPMOST;
    }

    ex_style
}

/// Set one mutable window long pointer lane and report Win32 failure correctly.
fn set_window_long_ptr_checked(
    hwnd: HWND,
    index: i32,
    value: isize,
    operation: &'static str,
    syscall: &'static str,
) -> RuntimeResult<()> {
    unsafe {
        SetLastError(0);
        let previous = SetWindowLongPtrW(hwnd, index, value);
        if previous == 0 {
            let error_code = GetLastError();
            if error_code != 0 {
                return Err(core::io_error_with_code(
                    operation,
                    syscall,
                    error_code,
                    "failed to update window style lane",
                ));
            }
        }
    }

    Ok(())
}

/// Apply one desktop rectangle to one window.
fn apply_window_rect(
    binding: &Win32WindowBinding,
    rectangle: RECT,
    operation: &'static str,
) -> RuntimeResult<()> {
    let status = unsafe {
        SetWindowPos(
            binding.hwnd,
            if binding.always_on_top {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            },
            rectangle.left,
            rectangle.top,
            (rectangle.right - rectangle.left).max(1),
            (rectangle.bottom - rectangle.top).max(1),
            SWP_NOACTIVATE,
        )
    };
    if status == 0 {
        return Err(core::io_error(
            operation,
            "SetWindowPos",
            "failed to apply target window rectangle",
        ));
    }

    Ok(())
}

/// Apply one style and ex-style update to one live window.
fn apply_window_style(binding: &Win32WindowBinding, operation: &'static str) -> RuntimeResult<()> {
    let style = window_style_for_binding(binding);
    let ex_style = window_ex_style_for_binding(binding);

    set_window_long_ptr_checked(
        binding.hwnd,
        GWL_STYLE,
        style as isize,
        operation,
        "SetWindowLongPtrW",
    )?;
    set_window_long_ptr_checked(
        binding.hwnd,
        GWL_EXSTYLE,
        ex_style as isize,
        operation,
        "SetWindowLongPtrW",
    )?;

    let status = unsafe {
        SetWindowPos(
            binding.hwnd,
            if binding.always_on_top {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            },
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
        )
    };
    if status == 0 {
        return Err(core::io_error(
            operation,
            "SetWindowPos",
            "failed to apply window style",
        ));
    }

    Ok(())
}

/// Refresh one cached window state snapshot from one live hwnd.
fn refresh_window_snapshot(binding: &mut Win32WindowBinding) {
    let mut window_rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let mut client_rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };

    if unsafe { GetWindowRect(binding.hwnd, &mut window_rect) } != 0 {
        binding.position = WindowPosition {
            x: window_rect.left,
            y: window_rect.top,
        };
    }

    if unsafe { GetClientRect(binding.hwnd, &mut client_rect) } != 0 {
        let width = (client_rect.right - client_rect.left).max(1) as u32;
        let height = (client_rect.bottom - client_rect.top).max(1) as u32;
        binding.size_physical = WindowPhysicalSize { width, height };
    }

    binding.scale_factor_milli = window_scale_factor_milli(binding.hwnd);
    binding.size_logical = physical_to_logical(binding.size_physical, binding.scale_factor_milli);
    binding.visibility = visibility_from_hwnd(binding.hwnd);
    binding.focused = unsafe { GetForegroundWindow() } == binding.hwnd;
    binding.occluded = matches!(
        binding.visibility,
        WindowVisibility::Hidden | WindowVisibility::Minimized
    );
}

/// Resolve one class-name payload for Win32 window registration.
fn window_class_name() -> &'static [u16] {
    WINDOW_CLASS_NAME
        .get_or_init(|| core_platform::wide_with_nul("destack_display_win32"))
        .as_slice()
}

/// Apply one window-message snapshot mutation and publish state deltas.
fn apply_window_message_snapshot(
    entry: &WindowRuntimeEntry,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) {
    let Some(binding) = entry.binding.upgrade() else {
        return;
    };

    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    let previous = binding.clone();

    if message == WM_MOVE {
        binding.position = WindowPosition {
            x: (lparam as u32 & 0xFFFF) as i16 as i32,
            y: ((lparam as u32 >> 16) & 0xFFFF) as i16 as i32,
        };
    }

    if message == WM_SIZE {
        let width = (lparam as usize & 0xFFFF) as u16 as u32;
        let height = ((lparam as usize >> 16) & 0xFFFF) as u16 as u32;
        if width > 0 && height > 0 {
            binding.size_physical = WindowPhysicalSize { width, height };
            binding.size_logical =
                physical_to_logical(binding.size_physical, binding.scale_factor_milli);
        }

        if wparam == SIZE_MINIMIZED as usize {
            binding.visibility = WindowVisibility::Minimized;
        } else if wparam == SIZE_MAXIMIZED as usize {
            binding.visibility = WindowVisibility::Maximized;
        } else if binding.visibility != WindowVisibility::Hidden {
            binding.visibility = WindowVisibility::Visible;
        }
    }

    if message == WM_ACTIVATE {
        let activation = (wparam & 0xFFFF) as u32;
        binding.focused = activation != WA_INACTIVE;
    }

    if message == WM_SETFOCUS {
        binding.focused = true;
    }

    if message == WM_KILLFOCUS {
        binding.focused = false;
    }

    if message == WM_SHOWWINDOW {
        if wparam == 0 {
            binding.visibility = WindowVisibility::Hidden;
        } else if binding.visibility == WindowVisibility::Hidden {
            binding.visibility = WindowVisibility::Visible;
        }
    }

    if message == WM_DPICHANGED {
        let dpi_x = (wparam & 0xFFFF) as u32;
        if dpi_x > 0 {
            binding.scale_factor_milli = dpi_x.saturating_mul(1000).max(1) / 96;
        }

        if lparam != 0 {
            let recommended = unsafe { *(lparam as *const RECT) };
            let _ = unsafe {
                SetWindowPos(
                    binding.hwnd,
                    0,
                    recommended.left,
                    recommended.top,
                    (recommended.right - recommended.left).max(1),
                    (recommended.bottom - recommended.top).max(1),
                    SWP_NOACTIVATE | SWP_NOZORDER,
                )
            };
        }
    }

    if message == WM_THEMECHANGED {
        binding.theme = WindowTheme::Unknown;
    }

    if message == WM_DESTROY {
        binding.destroyed_emitted = true;
        binding.visibility = WindowVisibility::Hidden;
        binding.focused = false;
        binding.occluded = true;
    }

    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    event::publish_state_deltas(entry.window, &previous, &next);
}

/// Drain pending window-thread messages and dispatch them through the registered wndproc.
pub(super) fn pump_window_messages() {
    loop {
        let mut message = unsafe { std::mem::zeroed::<MSG>() };
        let has_message = unsafe { PeekMessageW(&mut message, 0, 0, 0, PM_REMOVE) } != 0;
        if !has_message {
            break;
        }

        if message.message == WM_QUIT {
            continue;
        }

        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

/// Wndproc for display windows.
unsafe extern "system" fn display_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_CLOSE
        && let Some(entry) = runtime_window_entry(hwnd)
    {
        if let Some(binding) = entry.binding.upgrade() {
            let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
            if !binding.close_requested_emitted {
                binding.close_requested_emitted = true;
                drop(binding);
                event::publish_window_close_requested_event(entry.window);
            }
        }

        return 0;
    }

    if message == WM_DESTROY
        && let Some(entry) = runtime_window_entry(hwnd)
    {
        let mut should_emit_destroyed = true;
        if let Some(binding) = entry.binding.upgrade() {
            let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
            should_emit_destroyed = !binding.destroyed_emitted;
            binding.destroyed_emitted = true;
        }

        apply_window_message_snapshot(&entry, message, wparam, lparam);
        if should_emit_destroyed {
            event::publish_window_destroyed_event(entry.window);
        }
        unregister_runtime_window(hwnd);
        return 0;
    }

    if matches!(
        message,
        WM_MOVE
            | WM_SIZE
            | WM_SETFOCUS
            | WM_KILLFOCUS
            | WM_THEMECHANGED
            | WM_DPICHANGED
            | WM_DISPLAYCHANGE
            | WM_ACTIVATE
            | WM_SHOWWINDOW
            | WM_WINDOWPOSCHANGED
    ) && let Some(entry) = runtime_window_entry(hwnd)
    {
        apply_window_message_snapshot(&entry, message, wparam, lparam);
    }

    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

/// Ensure the display window class is registered.
fn ensure_window_class_registered() -> RuntimeResult<()> {
    if WINDOW_CLASS_REGISTERED.get().is_some() {
        return Ok(());
    }

    let instance = unsafe { GetModuleHandleW(std::ptr::null()) } as HINSTANCE;
    if instance == 0 {
        return Err(core::io_error(
            "destack.display.window.open",
            "GetModuleHandleW",
            "failed to resolve module handle",
        ));
    }

    let class_name = window_class_name();
    let class = WNDCLASSW {
        style: 0,
        lpfnWndProc: Some(display_window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance,
        hIcon: 0,
        hCursor: 0,
        hbrBackground: 0,
        lpszMenuName: std::ptr::null(),
        lpszClassName: class_name.as_ptr(),
    };

    let atom = unsafe { RegisterClassW(&class) };
    if atom == 0 {
        let code = core_platform::last_error_code() as u32;
        if code != ERROR_CLASS_ALREADY_EXISTS {
            return Err(core::io_error_with_code(
                "destack.display.window.open",
                "RegisterClassW",
                code,
                "failed to register display window class",
            ));
        }
    }

    WINDOW_CLASS_REGISTERED.get_or_init(|| ());
    Ok(())
}

/// Resolve one monitor handle to one display identifier for exclusive-fullscreen updates.
fn mode_target_display(
    mode: WindowModeOptions,
    current_display: Option<resource::DisplayHandle>,
) -> Option<resource::DisplayHandle> {
    if mode.display.is_some() {
        return mode.display;
    }

    if mode.mode == WindowMode::ExclusiveFullscreen {
        return current_display;
    }

    current_display
}

/// Restore one captured exclusive-fullscreen display mode snapshot.
fn restore_exclusive_mode(
    restore: &ExclusiveModeRestore,
    operation: &'static str,
) -> RuntimeResult<()> {
    monitor::apply_monitor_mode_by_id(&restore.display_id, restore.mode, operation)?;
    event::publish_mode_changed_event(&restore.display_id, restore.mode);
    if let Some(snapshot) = monitor::monitor_snapshot_by_id(&restore.display_id)? {
        event::publish_descriptor_changed_event(
            &snapshot.descriptor,
            core::DISPLAY_CHANGED_MASK_BOUNDS
                | core::DISPLAY_CHANGED_MASK_WORKAREA
                | core::DISPLAY_CHANGED_MASK_SCALE
                | core::DISPLAY_CHANGED_MASK_ORIENTATION,
        );
    }
    Ok(())
}

/// Apply one window-mode transition and associated exclusive-display state changes.
fn apply_mode_options(
    context: &BindingCallContext,
    binding: &mut Win32WindowBinding,
    mode: WindowModeOptions,
    operation: &'static str,
) -> RuntimeResult<()> {
    let target_display = mode_target_display(mode, binding.display);
    if mode.mode == WindowMode::ExclusiveFullscreen {
        let Some(display) = target_display else {
            return Err(core::invalid_argument(
                "mode.display",
                "exclusive fullscreen requires one display target",
            ));
        };

        let display_id = display_resource::resolve_display_id(context, display, operation)?;

        if let Some(restore) = binding.exclusive_restore.as_ref()
            && restore.display_id != display_id
        {
            let restore = restore.clone();
            restore_exclusive_mode(&restore, operation)?;
            binding.exclusive_restore = None;
        }

        if binding.exclusive_restore.is_none() {
            let snapshot = monitor::monitor_snapshot_by_id(&display_id)?.ok_or_else(|| {
                core::not_found(
                    operation,
                    format!("display id '{display_id}' is no longer available"),
                )
            })?;

            binding.exclusive_restore = Some(ExclusiveModeRestore {
                display_id: display_id.clone(),
                mode: snapshot.current_mode,
            });
        }

        if let Some(display_mode) = mode.display_mode {
            monitor::apply_monitor_mode_by_id(&display_id, display_mode, operation)?;
            event::publish_mode_changed_event(&display_id, display_mode);
            if let Some(snapshot) = monitor::monitor_snapshot_by_id(&display_id)? {
                event::publish_descriptor_changed_event(
                    &snapshot.descriptor,
                    core::DISPLAY_CHANGED_MASK_BOUNDS
                        | core::DISPLAY_CHANGED_MASK_WORKAREA
                        | core::DISPLAY_CHANGED_MASK_SCALE
                        | core::DISPLAY_CHANGED_MASK_ORIENTATION,
                );
            }
        }
    } else if let Some(restore) = binding.exclusive_restore.clone() {
        restore_exclusive_mode(&restore, operation)?;
        binding.exclusive_restore = None;
    }

    binding.mode = mode;
    binding.display = target_display;
    apply_window_style(binding, operation)?;

    if let Some(rectangle) = monitor::mode_target_rect(context, mode, binding.display)? {
        apply_window_rect(binding, rectangle, operation)?;
    }

    Ok(())
}

/// Set one cursor visibility lane.
fn set_cursor_visibility(visible: bool) {
    let mut state = cursor_visible_state()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if *state == Some(visible) {
        return;
    }

    unsafe {
        if visible {
            let mut display_count = ShowCursor(1);
            while display_count < 0 {
                display_count = ShowCursor(1);
            }
        } else {
            let mut display_count = ShowCursor(0);
            while display_count >= 0 {
                display_count = ShowCursor(0);
            }
        }
    }

    *state = Some(visible);
}

/// Restore global cursor state when one window is closed.
fn restore_cursor_after_close(binding: &Win32WindowBinding) {
    if matches!(
        binding.cursor_mode,
        WindowCursorMode::Confined | WindowCursorMode::Locked
    ) {
        unsafe {
            ClipCursor(std::ptr::null());
        }
    }

    if !binding.cursor_visible || binding.cursor_mode == WindowCursorMode::Hidden {
        set_cursor_visibility(true);
    }
}

/// Apply one cursor interaction mode for one window.
fn apply_cursor_mode(
    hwnd: HWND,
    mode: WindowCursorMode,
    operation: &'static str,
) -> RuntimeResult<()> {
    match mode {
        WindowCursorMode::Normal | WindowCursorMode::Hidden => unsafe {
            ClipCursor(std::ptr::null());
            Ok(())
        },
        WindowCursorMode::Confined => {
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if unsafe { GetClientRect(hwnd, &mut rect) } == 0 {
                return Err(core::io_error(
                    operation,
                    "GetClientRect",
                    "failed to query client rectangle",
                ));
            }

            let mut top_left = POINT {
                x: rect.left,
                y: rect.top,
            };
            let mut bottom_right = POINT {
                x: rect.right,
                y: rect.bottom,
            };

            if unsafe { ClientToScreen(hwnd, &mut top_left) } == 0
                || unsafe { ClientToScreen(hwnd, &mut bottom_right) } == 0
            {
                return Err(core::io_error(
                    operation,
                    "ClientToScreen",
                    "failed to map client rectangle",
                ));
            }

            rect.left = top_left.x;
            rect.top = top_left.y;
            rect.right = bottom_right.x;
            rect.bottom = bottom_right.y;

            if unsafe { ClipCursor(&rect) } == 0 {
                return Err(core::io_error(
                    operation,
                    "ClipCursor",
                    "failed to confine cursor",
                ));
            }

            Ok(())
        }
        WindowCursorMode::Locked => {
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if unsafe { GetClientRect(hwnd, &mut rect) } == 0 {
                return Err(core::io_error(
                    operation,
                    "GetClientRect",
                    "failed to query client rectangle",
                ));
            }

            let center = POINT {
                x: rect.left + (rect.right - rect.left) / 2,
                y: rect.top + (rect.bottom - rect.top) / 2,
            };
            let mut center_screen = center;
            if unsafe { ClientToScreen(hwnd, &mut center_screen) } == 0 {
                return Err(core::io_error(
                    operation,
                    "ClientToScreen",
                    "failed to map cursor lock point",
                ));
            }

            let status = unsafe { SetCursorPos(center_screen.x, center_screen.y) };
            if status == 0 {
                return Err(core::io_error(
                    operation,
                    "SetCursorPos",
                    "failed to lock cursor position",
                ));
            }

            let lock_rect = RECT {
                left: center_screen.x,
                top: center_screen.y,
                right: center_screen.x.saturating_add(1),
                bottom: center_screen.y.saturating_add(1),
            };
            if unsafe { ClipCursor(&lock_rect) } == 0 {
                return Err(core::io_error(
                    operation,
                    "ClipCursor",
                    "failed to lock cursor region",
                ));
            }

            Ok(())
        }
    }
}

/// Resolve one Win32 cursor name selector from one cursor icon enum.
fn cursor_name(icon: WindowCursorIcon) -> *const u16 {
    match icon {
        WindowCursorIcon::Default => IDC_ARROW,
        WindowCursorIcon::Pointer => IDC_HAND,
        WindowCursorIcon::Text => IDC_IBEAM,
        WindowCursorIcon::Crosshair => IDC_CROSS,
        WindowCursorIcon::Grab | WindowCursorIcon::Grabbing | WindowCursorIcon::AllScroll => {
            IDC_SIZEALL
        }
        WindowCursorIcon::Move => IDC_SIZEALL,
        WindowCursorIcon::EResize
        | WindowCursorIcon::WResize
        | WindowCursorIcon::EwResize
        | WindowCursorIcon::ColResize => IDC_SIZEWE,
        WindowCursorIcon::NResize
        | WindowCursorIcon::SResize
        | WindowCursorIcon::NsResize
        | WindowCursorIcon::RowResize => IDC_SIZENS,
        WindowCursorIcon::NeResize | WindowCursorIcon::SwResize | WindowCursorIcon::NeswResize => {
            IDC_SIZENESW
        }
        WindowCursorIcon::NwResize | WindowCursorIcon::SeResize | WindowCursorIcon::NwseResize => {
            IDC_SIZENWSE
        }
        WindowCursorIcon::Wait => IDC_WAIT,
        WindowCursorIcon::Progress => IDC_APPSTARTING,
        WindowCursorIcon::NotAllowed => IDC_NO,
        WindowCursorIcon::ZoomIn | WindowCursorIcon::ZoomOut => IDC_ARROW,
        WindowCursorIcon::Help => IDC_HELP,
    }
}

/// Open one window.
pub(crate) unsafe fn window_open(
    context: &BindingCallContext,
    out: *mut resource::WindowHandle,
    options: WindowOptions,
) -> RuntimeResult<()> {
    core::ensure_out(out, "out")?;

    ensure_window_class_registered()?;

    let title = unsafe { options.title.as_str()? }.to_string();
    let requested_size_logical =
        normalize_logical_size(options.size_logical, "options.sizeLogical")?;

    let scale_factor_milli = 1000u32;
    let constrained_logical_size = clamp_logical_size(requested_size_logical, options.constraints);
    let size_physical = logical_to_physical(constrained_logical_size, scale_factor_milli);

    if let Some(display) = options.display {
        let _ =
            display_resource::resolve_display_id(context, display, "destack.display.window.open")?;
    }

    let initial_mode = options.mode;
    let mut provisional_binding = Win32WindowBinding {
        id: format!(
            "win32-window-{}",
            NEXT_WINDOW_IDENTIFIER.fetch_add(1, Ordering::Relaxed)
        ),
        hwnd: 0,
        owner_thread_id: current_thread_id(),
        title: title.clone(),
        mode: initial_mode,
        display: options.display,
        resizable: options.resizable,
        decorated: options.decorated,
        transparent: options.transparent,
        always_on_top: options.always_on_top,
        visibility: options.visibility,
        constraints: options.constraints,
        cursor_visible: true,
        cursor_mode: WindowCursorMode::Normal,
        cursor_icon: WindowCursorIcon::Default,
        position: options.position.unwrap_or(WindowPosition {
            x: CW_USEDEFAULT,
            y: CW_USEDEFAULT,
        }),
        size_logical: constrained_logical_size,
        size_physical,
        scale_factor_milli,
        focused: options.focus_on_show,
        occluded: false,
        theme: WindowTheme::Unknown,
        exclusive_restore: None,
        close_requested_emitted: false,
        destroyed_emitted: false,
    };

    let style = window_style_for_binding(&provisional_binding);
    let ex_style = window_ex_style_for_binding(&provisional_binding);
    let (outer_width, outer_height) = outer_size_from_client_size(
        provisional_binding.size_physical,
        style,
        ex_style,
        "destack.display.window.open",
    )?;

    let class_name = window_class_name();
    let title_wide = core::wide_with_nul("title", &title)?;
    let hwnd = unsafe {
        CreateWindowExW(
            ex_style,
            class_name.as_ptr(),
            title_wide.as_ptr(),
            style,
            provisional_binding.position.x,
            provisional_binding.position.y,
            outer_width,
            outer_height,
            0,
            0,
            GetModuleHandleW(std::ptr::null()) as HINSTANCE,
            std::ptr::null(),
        )
    };
    if hwnd == 0 {
        return Err(core::io_error(
            "destack.display.window.open",
            "CreateWindowExW",
            "failed to create window",
        ));
    }

    provisional_binding.hwnd = hwnd;
    if let Err(error) = apply_mode_options(
        context,
        &mut provisional_binding,
        initial_mode,
        "destack.display.window.open",
    ) {
        unsafe {
            DestroyWindow(hwnd);
        }
        return Err(error);
    }

    provisional_binding.scale_factor_milli = window_scale_factor_milli(provisional_binding.hwnd);
    refresh_window_snapshot(&mut provisional_binding);

    let binding = Arc::new(Mutex::new(provisional_binding));
    let entry = display_resource::window_resource_entry(hwnd, Arc::clone(&binding));
    let resource_id = context.runtime().resources.insert(entry);
    let handle = resource::WindowHandle(resource_id);
    register_runtime_window(hwnd, handle, &binding);

    if options.visibility != WindowVisibility::Hidden {
        unsafe {
            ShowWindow(
                hwnd,
                show_command_on_open(options.visibility, options.focus_on_show),
            );
            UpdateWindow(hwnd);
        }
    }

    if options.focus_on_show && options.visibility == WindowVisibility::Visible {
        unsafe {
            let _ = SetForegroundWindow(hwnd);
        }
    }

    event::publish_window_created_event(handle);

    let previous = {
        let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
        Win32WindowBinding {
            visibility: WindowVisibility::Hidden,
            focused: false,
            ..binding.clone()
        }
    };
    let next = {
        let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
        refresh_window_snapshot(&mut binding);
        binding.clone()
    };
    if previous.visibility != next.visibility
        || previous.position != next.position
        || previous.size_logical != next.size_logical
        || previous.size_physical != next.size_physical
        || previous.scale_factor_milli != next.scale_factor_milli
        || previous.focused != next.focused
        || previous.occluded != next.occluded
        || previous.theme != next.theme
    {
        event::publish_state_deltas(handle, &previous, &next);
    }

    unsafe {
        *out = handle;
    }

    Ok(())
}

/// Close one window.
pub(crate) unsafe fn window_close(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let binding =
        display_resource::resolve_window_binding(context, window, "destack.display.window.close")?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.close")?;

    if let Some(restore) = binding.exclusive_restore.clone() {
        restore_exclusive_mode(&restore, "destack.display.window.close")?;
        binding.exclusive_restore = None;
    }

    restore_cursor_after_close(&binding);
    let hwnd = binding.hwnd;

    let should_emit_close_requested = !binding.close_requested_emitted;
    if should_emit_close_requested {
        binding.close_requested_emitted = true;
    }
    drop(binding);

    if should_emit_close_requested {
        event::publish_window_close_requested_event(window);
    }

    if unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(hwnd) } != 0 {
        unsafe {
            DestroyWindow(hwnd);
        }
        pump_window_messages();
    }

    let binding =
        display_resource::resolve_window_binding(context, window, "destack.display.window.close")?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    if !binding.destroyed_emitted {
        binding.destroyed_emitted = true;
        drop(binding);
        event::publish_window_destroyed_event(window);
        unregister_runtime_window(hwnd);
    } else {
        drop(binding);
    }

    let removed = context.runtime().resources.remove_and_finalize(window.0);
    if !removed {
        return Err(core::not_found(
            "destack.display.window.close",
            format!("window handle {} was not found", window.0.0),
        ));
    }

    Ok(())
}

/// Read descriptor metadata for one window.
pub(crate) unsafe fn window_descriptor(
    context: &BindingCallContext,
    out: *mut WindowDescriptor,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    core::ensure_out(out, "out")?;
    pump_window_messages();

    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.descriptor",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    refresh_window_snapshot(&mut binding);

    let descriptor = WindowDescriptor {
        backend: DisplayBackend::Win32,
        id: context.store_string(&binding.id),
        title: context.store_string(&binding.title),
        mode: binding.mode,
        display: binding.display,
        resizable: binding.resizable,
        decorated: binding.decorated,
        transparent: binding.transparent,
        always_on_top: binding.always_on_top,
    };

    unsafe {
        *out = descriptor;
    }

    Ok(())
}

/// Read one window state snapshot.
pub(crate) unsafe fn window_state(
    context: &BindingCallContext,
    out: *mut WindowState,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    core::ensure_out(out, "out")?;
    pump_window_messages();

    let binding =
        display_resource::resolve_window_binding(context, window, "destack.display.window.state")?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    refresh_window_snapshot(&mut binding);

    let state = WindowState {
        backend: DisplayBackend::Win32,
        position: binding.position,
        size_logical: binding.size_logical,
        size_physical: binding.size_physical,
        scale_factor_milli: binding.scale_factor_milli,
        visibility: binding.visibility,
        display: binding.display,
        focused: binding.focused,
        occluded: binding.occluded,
        theme: binding.theme,
        always_on_top: binding.always_on_top,
    };

    unsafe {
        *out = state;
    }

    let _ = context;
    Ok(())
}

/// Set one window title string.
pub(crate) unsafe fn window_set_title(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    title: NativeStringRef,
) -> RuntimeResult<()> {
    let title = unsafe { title.as_str()? }.to_string();
    let title_wide = core::wide_with_nul("title", &title)?;

    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setTitle",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setTitle")?;
    let status = unsafe { SetWindowTextW(binding.hwnd, title_wide.as_ptr()) };
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.setTitle",
            "SetWindowTextW",
            "failed to set window title",
        ));
    }

    binding.title = title;

    Ok(())
}

/// Set one window visibility state.
pub(crate) unsafe fn window_set_visibility(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    visibility: WindowVisibility,
) -> RuntimeResult<()> {
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setVisibility",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setVisibility")?;
    let previous = binding.clone();

    unsafe {
        ShowWindow(binding.hwnd, show_command(visibility));
        UpdateWindow(binding.hwnd);
    }

    binding.visibility = visibility;
    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    event::publish_state_deltas(window, &previous, &next);

    Ok(())
}

/// Set one window position.
pub(crate) unsafe fn window_set_position(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setPosition",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setPosition")?;
    let previous = binding.clone();

    let status = unsafe {
        SetWindowPos(
            binding.hwnd,
            0,
            position.x,
            position.y,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOSIZE | SWP_NOZORDER,
        )
    };
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.setPosition",
            "SetWindowPos",
            "failed to set window position",
        ));
    }

    binding.position = position;
    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    event::publish_state_deltas(window, &previous, &next);

    Ok(())
}

/// Set one logical window size.
pub(crate) unsafe fn window_set_size_logical(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowLogicalSize,
) -> RuntimeResult<()> {
    let size = normalize_logical_size(size, "size")?;

    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setSizeLogical",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setSizeLogical")?;
    let previous = binding.clone();

    let clamped_size = clamp_logical_size(size, binding.constraints);
    let size_physical = logical_to_physical(clamped_size, binding.scale_factor_milli);
    let style = window_style_for_binding(&binding);
    let ex_style = window_ex_style_for_binding(&binding);
    let (outer_width, outer_height) = outer_size_from_client_size(
        size_physical,
        style,
        ex_style,
        "destack.display.window.setSizeLogical",
    )?;

    let status = unsafe {
        SetWindowPos(
            binding.hwnd,
            0,
            0,
            0,
            outer_width,
            outer_height,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOZORDER,
        )
    };
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.setSizeLogical",
            "SetWindowPos",
            "failed to set logical window size",
        ));
    }

    binding.size_logical = clamped_size;
    binding.size_physical = size_physical;
    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    event::publish_state_deltas(window, &previous, &next);

    Ok(())
}

/// Set one physical window size.
pub(crate) unsafe fn window_set_size_physical(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowPhysicalSize,
) -> RuntimeResult<()> {
    let size = normalize_physical_size(size, "size")?;

    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setSizePhysical",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setSizePhysical")?;
    let previous = binding.clone();

    let size_logical = physical_to_logical(size, binding.scale_factor_milli);
    let clamped_logical = clamp_logical_size(size_logical, binding.constraints);
    let clamped_physical = logical_to_physical(clamped_logical, binding.scale_factor_milli);
    let style = window_style_for_binding(&binding);
    let ex_style = window_ex_style_for_binding(&binding);
    let (outer_width, outer_height) = outer_size_from_client_size(
        clamped_physical,
        style,
        ex_style,
        "destack.display.window.setSizePhysical",
    )?;

    let status = unsafe {
        SetWindowPos(
            binding.hwnd,
            0,
            0,
            0,
            outer_width,
            outer_height,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOZORDER,
        )
    };
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.setSizePhysical",
            "SetWindowPos",
            "failed to set physical window size",
        ));
    }

    binding.size_logical = clamped_logical;
    binding.size_physical = clamped_physical;
    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    event::publish_state_deltas(window, &previous, &next);

    Ok(())
}

/// Set logical size constraints.
pub(crate) unsafe fn window_set_size_constraints(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    constraints: Option<WindowSizeConstraints>,
) -> RuntimeResult<()> {
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setSizeConstraints",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setSizeConstraints")?;
    binding.constraints = constraints;

    Ok(())
}

/// Set window resizable state.
pub(crate) unsafe fn window_set_resizable(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    resizable: bool,
) -> RuntimeResult<()> {
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setResizable",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setResizable")?;
    binding.resizable = resizable;
    apply_window_style(&binding, "destack.display.window.setResizable")?;

    Ok(())
}

/// Set window decoration state.
pub(crate) unsafe fn window_set_decorated(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    decorated: bool,
) -> RuntimeResult<()> {
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setDecorated",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setDecorated")?;
    binding.decorated = decorated;
    apply_window_style(&binding, "destack.display.window.setDecorated")?;

    Ok(())
}

/// Set always-on-top state.
pub(crate) unsafe fn window_set_always_on_top(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    alwaysontop: bool,
) -> RuntimeResult<()> {
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setAlwaysOnTop",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setAlwaysOnTop")?;

    let status = unsafe {
        SetWindowPos(
            binding.hwnd,
            if alwaysontop {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            },
            0,
            0,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
        )
    };
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.setAlwaysOnTop",
            "SetWindowPos",
            "failed to update topmost state",
        ));
    }

    binding.always_on_top = alwaysontop;

    Ok(())
}

/// Set one cursor visibility lane.
pub(crate) unsafe fn window_set_cursor_visible(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setCursorVisible",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setCursorVisible")?;

    set_cursor_visibility(visible);
    binding.cursor_visible = visible;

    Ok(())
}

/// Set one cursor icon selector.
pub(crate) unsafe fn window_set_cursor_icon(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    icon: WindowCursorIcon,
) -> RuntimeResult<()> {
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setCursorIcon",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setCursorIcon")?;

    let cursor = unsafe { LoadCursorW(0, cursor_name(icon)) };
    if cursor == 0 {
        return Err(core::io_error(
            "destack.display.window.setCursorIcon",
            "LoadCursorW",
            "failed to load cursor icon",
        ));
    }

    unsafe {
        SetCursor(cursor);
    }
    binding.cursor_icon = icon;

    Ok(())
}

/// Set one cursor position lane.
pub(crate) unsafe fn window_set_cursor_position(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setCursorPosition",
    )?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setCursorPosition")?;

    let mut point = POINT {
        x: position.x,
        y: position.y,
    };
    if unsafe { ClientToScreen(binding.hwnd, &mut point) } == 0 {
        return Err(core::io_error(
            "destack.display.window.setCursorPosition",
            "ClientToScreen",
            "failed to map cursor position",
        ));
    }

    let status = unsafe { SetCursorPos(point.x, point.y) };
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.setCursorPosition",
            "SetCursorPos",
            "failed to set cursor position",
        ));
    }

    Ok(())
}

/// Set one cursor interaction mode.
pub(crate) unsafe fn window_set_cursor_mode(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowCursorMode,
) -> RuntimeResult<()> {
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setCursorMode",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setCursorMode")?;

    apply_cursor_mode(binding.hwnd, mode, "destack.display.window.setCursorMode")?;

    if mode == WindowCursorMode::Hidden {
        set_cursor_visibility(false);
        binding.cursor_visible = false;
    } else if mode == WindowCursorMode::Normal {
        set_cursor_visibility(true);
        binding.cursor_visible = true;
    }

    binding.cursor_mode = mode;

    Ok(())
}

/// Request user attention for one window.
pub(crate) unsafe fn window_request_attention(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    level: WindowAttentionLevel,
) -> RuntimeResult<()> {
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.requestAttention",
    )?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.requestAttention")?;

    let flash_count = if level == WindowAttentionLevel::Critical {
        7
    } else {
        3
    };
    let info = FLASHWINFO {
        cbSize: std::mem::size_of::<FLASHWINFO>() as u32,
        hwnd: binding.hwnd,
        dwFlags: FLASHW_TRAY | FLASHW_TIMERNOFG,
        uCount: flash_count,
        dwTimeout: 0,
    };

    let status = unsafe { FlashWindowEx(&info) };
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.requestAttention",
            "FlashWindowEx",
            "failed to request user attention",
        ));
    }

    Ok(())
}

/// Request one redraw for one window.
pub(crate) unsafe fn window_request_refresh(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.requestRefresh",
    )?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    drop(binding);
    event::publish_window_refresh_event(window);

    Ok(())
}

/// Set one window mode.
pub(crate) unsafe fn window_set_mode(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowModeOptions,
) -> RuntimeResult<()> {
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setMode",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setMode")?;
    let previous = binding.clone();
    apply_mode_options(
        context,
        &mut binding,
        mode,
        "destack.display.window.setMode",
    )?;

    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    event::publish_window_mode_event(window, mode);
    if previous.display != next.display {
        event::publish_window_display_event(window, next.display);
    }
    event::publish_state_deltas(window, &previous, &next);

    Ok(())
}

/// Present one frame interval marker.
pub(crate) unsafe fn window_vsync_wait(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    pump_window_messages();

    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.vsyncWait",
    )?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());

    if timeoutns == 0 {
        return Err(core::io_would_block(
            "destack.display.window.vsyncWait",
            "vsync wait timed out",
        ));
    }
    drop(binding);

    let sleep_ns = timeoutns.min(core::FALLBACK_VSYNC_INTERVAL_NS);
    thread::sleep(Duration::from_nanos(sleep_ns));

    Ok(())
}
