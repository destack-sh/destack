use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};

use windows_sys::Win32::Foundation::{
    ERROR_CLASS_ALREADY_EXISTS, ERROR_SUCCESS, GetLastError, HINSTANCE, HWND, LPARAM, LRESULT,
    POINT, RECT, SetLastError, WPARAM,
};
use windows_sys::Win32::Graphics::Gdi::{ClientToScreen, GetDeviceCaps, LOGPIXELSX, UpdateWindow};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
use windows_sys::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, ReleaseCapture};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CURSOR_SHOWING, CURSORINFO, CW_USEDEFAULT, ClipCursor, CreateWindowExW,
    DefWindowProcW, DestroyWindow, FLASHW_TIMERNOFG, FLASHW_TRAY, FLASHWINFO, FlashWindowEx,
    GWL_EXSTYLE, GWL_STYLE, GWLP_HWNDPARENT, GWLP_USERDATA, GetClientRect, GetCursorInfo,
    GetForegroundWindow, GetWindowLongPtrW, GetWindowRect, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT,
    HTCAPTION, HTLEFT, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT, HWND_NOTOPMOST, HWND_TOP,
    HWND_TOPMOST, ICON_BIG, ICON_SMALL, IDC_APPSTARTING, IDC_ARROW, IDC_CROSS, IDC_HAND, IDC_HELP,
    IDC_IBEAM, IDC_NO, IDC_SIZEALL, IDC_SIZENESW, IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE, IDC_WAIT,
    IsIconic, IsWindow, IsWindowVisible, IsZoomed, LWA_ALPHA, LoadCursorW, PostMessageW,
    RegisterClassW, SIZE_MAXIMIZED, SIZE_MINIMIZED, SPI_GETHIGHCONTRAST, SW_HIDE, SW_MAXIMIZE,
    SW_MINIMIZE, SW_RESTORE, SW_SHOW, SW_SHOWNA, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_NOZORDER, SendMessageW, SetCursor, SetCursorPos, SetForegroundWindow,
    SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, SetWindowTextW, ShowCursor,
    ShowWindow, SystemParametersInfoW, WA_INACTIVE, WINDOW_EX_STYLE, WINDOW_STYLE, WM_ACTIVATE,
    WM_CLOSE, WM_DESTROY, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_KILLFOCUS, WM_MOVE, WM_NCLBUTTONDOWN,
    WM_SETCURSOR, WM_SETFOCUS, WM_SETICON, WM_SETTINGCHANGE, WM_SHOWWINDOW, WM_SIZE, WM_SIZING,
    WM_THEMECHANGED, WM_WINDOWPOSCHANGED, WMSZ_BOTTOM, WMSZ_BOTTOMLEFT, WMSZ_LEFT, WMSZ_RIGHT,
    WMSZ_TOP, WMSZ_TOPLEFT, WMSZ_TOPRIGHT, WNDCLASSW, WS_CAPTION, WS_EX_APPWINDOW, WS_EX_LAYERED,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP,
    WS_SYSMENU, WS_THICKFRAME,
};

use crate::diagnostic::{AgentDiagnosticStore, RuntimeResult};
use crate::platform::display::{
    DisplayBackend, DisplayMode, WindowAspectRatio, WindowAttentionLevel, WindowChromeKind,
    WindowCursorIcon, WindowCursorMode, WindowDescriptor, WindowIconSet, WindowLogicalSize,
    WindowModeOptions, WindowOcclusionState, WindowOptions, WindowPhysicalSize, WindowPosition,
    WindowResizeEdge, WindowSizeConstraints, WindowState, WindowTheme, WindowVisibility,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::{BindingCallContext, NativeStringRef};

#[path = "action.rs"]
mod action;
#[path = "appearance.rs"]
mod appearance;
#[path = "cursor.rs"]
mod cursor;
#[path = "drop.rs"]
mod drop;
#[path = "geometry.rs"]
mod geometry;
#[path = "icon.rs"]
mod icon;
#[path = "lifecycle.rs"]
mod lifecycle;
#[path = "relation.rs"]
mod relation;
#[path = "state.rs"]
mod state;

pub(crate) use action::*;
pub(crate) use appearance::*;
pub(crate) use cursor::*;
pub(crate) use geometry::*;
pub(crate) use lifecycle::*;
pub(crate) use relation::*;
pub(crate) use state::*;

use self::drop::unregister_window_drop_target;
use self::icon::{
    best_icon_index, create_hicon, decode_window_icons, destroy_owned_icons, icon_target_dimensions,
};
use super::super::model::{ExclusiveModeRestore, Win32WindowBinding};
use super::super::{core, event, monitor, resource as display_resource};

/// Win32 color-plane selector used for icon creation.
const WINDOW_ICON_COLOR_PLANES: u8 = 1;
/// Win32 bits-per-pixel selector used for icon creation.
const WINDOW_ICON_BITS_PER_PIXEL: u8 = 32;
/// Default small icon dimension when system metrics are unavailable.
const WINDOW_ICON_SMALL_DEFAULT: u32 = 16;
/// Default big icon dimension when system metrics are unavailable.
const WINDOW_ICON_BIG_DEFAULT: u32 = 32;

/// Runtime-owned mutable state for win32 window bindings.
#[derive(Debug)]
pub(crate) struct WindowRuntimeState {
    /// Shared global cursor visibility state.
    cursor_visible_state: Mutex<Option<bool>>,
    /// Per-window cursor policy lanes used to derive process-global cursor state.
    cursor_policy_by_window: Mutex<HashMap<resource::WindowHandle, CursorPolicyState>>,
    /// Monotonic counter used for stable cursor policy ordering.
    next_cursor_policy_sequence: AtomicU64,
    /// Monotonic counter used for stable runtime window identifiers.
    next_window_identifier: AtomicU64,
    /// Runtime diagnostics store for callback and best-effort lanes.
    diagnostics: Arc<AgentDiagnosticStore>,
}

impl Default for WindowRuntimeState {
    /// Create one default window runtime state.
    fn default() -> Self {
        Self::new(Arc::new(AgentDiagnosticStore::default()))
    }
}

impl WindowRuntimeState {
    /// Create one window runtime state with explicit diagnostics storage.
    fn new(diagnostics: Arc<AgentDiagnosticStore>) -> Self {
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
struct CursorPolicyState {
    /// Host window handle associated with this policy.
    hwnd: HWND,
    /// Per-window cursor visibility preference.
    cursor_visible: bool,
    /// Per-window cursor mode preference.
    cursor_mode: WindowCursorMode,
    /// Monotonic sequence used for most-recent policy ordering.
    sequence: u64,
}

/// Runtime mapping payload for one live hwnd.
#[derive(Clone)]
struct WindowRuntimeEntry {
    /// Runtime window handle associated with this hwnd.
    window: resource::WindowHandle,
    /// Weak binding reference for this window.
    binding: Weak<Mutex<Win32WindowBinding>>,
    /// Runtime-owned display event stream state.
    event_runtime_state: Arc<event::DisplayEventRuntimeState>,
    /// Runtime-owned window state for cursor/global cleanup lanes.
    window_runtime_state: Arc<WindowRuntimeState>,
}

/// Return runtime-owned win32 window state.
fn window_runtime_state(binding: &BindingCallContext) -> Arc<WindowRuntimeState> {
    let diagnostics = Arc::clone(&binding.agent().diagnostic);
    binding
        .agent()
        .platform_state
        .display
        .window_runtime_state(|| WindowRuntimeState::new(diagnostics))
}

/// Allocate one stable runtime window identifier.
fn next_window_identifier(binding: &BindingCallContext) -> u64 {
    let runtime_state = window_runtime_state(binding);
    runtime_state
        .next_window_identifier
        .fetch_add(1, Ordering::Relaxed)
}

/// Allocate one stable cursor-policy sequence number.
fn next_cursor_policy_sequence(runtime_state: &Arc<WindowRuntimeState>) -> u64 {
    runtime_state
        .next_cursor_policy_sequence
        .fetch_add(1, Ordering::Relaxed)
}

/// Resolve whether one cursor mode requests process-global cursor clipping.
fn is_clipped_cursor_mode(mode: WindowCursorMode) -> bool {
    matches!(mode, WindowCursorMode::Confined | WindowCursorMode::Locked)
}

/// Resolve one process-global cursor visibility target from per-window policies.
fn desired_cursor_visibility_from_policies(
    policies: &HashMap<resource::WindowHandle, CursorPolicyState>,
) -> bool {
    !policies
        .values()
        .any(|policy| !policy.cursor_visible || policy.cursor_mode == WindowCursorMode::Hidden)
}

/// Resolve one most-recent cursor-clip policy from per-window policies.
fn desired_cursor_clip_policy(
    policies: &HashMap<resource::WindowHandle, CursorPolicyState>,
) -> Option<CursorPolicyState> {
    policies
        .values()
        .filter(|policy| is_clipped_cursor_mode(policy.cursor_mode))
        .max_by_key(|policy| policy.sequence)
        .copied()
}

/// Synchronize process-global cursor state from per-window policies.
fn refresh_cursor_policy(runtime_state: &Arc<WindowRuntimeState>) -> RuntimeResult<()> {
    // retain valid windows and compute desired global cursor state
    let (desired_visible, desired_clip) = {
        let mut policies = runtime_state
            .cursor_policy_by_window
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        policies.retain(|_, policy| unsafe { IsWindow(policy.hwnd) } != 0);

        let desired_visible = desired_cursor_visibility_from_policies(&policies);
        let desired_clip = desired_cursor_clip_policy(&policies);
        (desired_visible, desired_clip)
    };

    // synchronize process-global cursor visibility
    set_cursor_visibility(runtime_state, desired_visible);

    // synchronize process-global cursor clip lane
    if let Some(policy) = desired_clip {
        apply_cursor_mode(
            policy.hwnd,
            policy.cursor_mode,
            "destack.display.window.cursorPolicy.sync",
        )?;
    } else {
        unsafe {
            ClipCursor(std::ptr::null());
        }
    }

    Ok(())
}

/// Synchronize process-global cursor state and suppress best-effort failures.
fn refresh_cursor_policy_best_effort(runtime_state: &Arc<WindowRuntimeState>) {
    // evaluate this condition
    if let Err(error) = refresh_cursor_policy(runtime_state) {
        runtime_state.diagnostics.warn(
            "display",
            "destack.display.window.cursorPolicy.sync",
            format!("cursor cleanup sync failed: {error}"),
            None,
        );
    }
}

/// Insert or update one window cursor policy snapshot.
fn upsert_cursor_policy(
    runtime_state: &Arc<WindowRuntimeState>,
    window: resource::WindowHandle,
    binding: &Win32WindowBinding,
) {
    let sequence = next_cursor_policy_sequence(runtime_state);
    let mut policies = runtime_state
        .cursor_policy_by_window
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    policies.insert(
        window,
        CursorPolicyState {
            hwnd: binding.hwnd,
            cursor_visible: binding.cursor_visible,
            cursor_mode: binding.cursor_mode,
            sequence,
        },
    );
}

/// Remove one window cursor policy snapshot.
fn remove_cursor_policy(runtime_state: &Arc<WindowRuntimeState>, window: resource::WindowHandle) {
    let mut policies = runtime_state
        .cursor_policy_by_window
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    policies.remove(&window);
}

/// Register one live hwnd mapping for runtime window callbacks.
fn register_runtime_window(
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
fn unregister_runtime_window(hwnd: HWND) {
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
fn runtime_window_entry(hwnd: HWND) -> Option<WindowRuntimeEntry> {
    let pointer = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut WindowRuntimeEntry;
    // evaluate this condition
    if pointer.is_null() {
        return None;
    }

    let entry = unsafe { &*pointer };
    Some(entry.clone())
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

/// Normalize one window logical-size payload.
fn normalize_logical_size(
    size: WindowLogicalSize,
    field: &'static str,
) -> RuntimeResult<WindowLogicalSize> {
    // evaluate this condition
    if !size.width.is_finite() || !size.height.is_finite() {
        return Err(core_platform::invalid_argument(
            field,
            "logical size must be finite",
        ));
    }

    // evaluate this condition
    if size.width <= 0.0 || size.height <= 0.0 {
        return Err(core_platform::invalid_argument(
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
    // evaluate this condition
    if size.width == 0 || size.height == 0 {
        return Err(core_platform::invalid_argument(
            field,
            "physical size dimensions must be greater than zero",
        ));
    }

    Ok(size)
}

/// Normalize one whole-window opacity payload.
fn normalize_opacity(opacity: f64, field: &'static str) -> RuntimeResult<f64> {
    // validate finite value
    if !opacity.is_finite() {
        return Err(core_platform::invalid_argument(
            field,
            "opacity must be finite",
        ));
    }

    // validate normalized range
    if !(0.0..=1.0).contains(&opacity) {
        return Err(core_platform::invalid_argument(
            field,
            "opacity must be between 0.0 and 1.0 inclusive",
        ));
    }

    Ok(opacity)
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

    // evaluate this condition
    if let Some(min) = constraints.min {
        width = width.max(min.width);
        height = height.max(min.height);
    }

    // evaluate this condition
    if let Some(max) = constraints.max {
        width = width.min(max.width);
        height = height.min(max.height);
    }

    WindowLogicalSize { width, height }
}

/// Resolve one win32 scale factor for one window handle.
fn window_scale_factor_milli(hwnd: HWND) -> u32 {
    // open one device context for dpi query
    let hdc = unsafe { windows_sys::Win32::Graphics::Gdi::GetDC(hwnd) };
    // evaluate this condition
    if hdc == 0 {
        return 1000;
    }

    // query and release dpi lane
    let dpi_x = unsafe { GetDeviceCaps(hdc, LOGPIXELSX as i32) };
    unsafe {
        windows_sys::Win32::Graphics::Gdi::ReleaseDC(hwnd, hdc);
    }

    // normalize scale factor output
    if dpi_x <= 0 {
        return 1000;
    }

    ((dpi_x as u32).saturating_mul(1000) / 96).max(1)
}

/// Convert one logical-size payload into one physical-size payload.
fn logical_to_physical(size: WindowLogicalSize, scale_factor_milli: u32) -> WindowPhysicalSize {
    // resolve numeric scale from milli factor
    let scale = if scale_factor_milli == 0 {
        1.0
    } else {
        scale_factor_milli as f64 / 1000.0
    };

    // convert logical dimensions to clamped physical pixels
    let width = (size.width * scale).round().max(1.0) as u32;
    let height = (size.height * scale).round().max(1.0) as u32;
    WindowPhysicalSize { width, height }
}

/// Convert one physical-size payload into one logical-size payload.
fn physical_to_logical(size: WindowPhysicalSize, scale_factor_milli: u32) -> WindowLogicalSize {
    // resolve numeric scale from milli factor
    let scale = if scale_factor_milli == 0 {
        1.0
    } else {
        scale_factor_milli as f64 / 1000.0
    };

    // convert physical pixels to clamped logical dimensions
    WindowLogicalSize {
        width: (size.width as f64 / scale).max(1.0),
        height: (size.height as f64 / scale).max(1.0),
    }
}

/// Convert one utf8 string into one nul-terminated utf16 buffer.
fn wide_with_nul(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Resolve one apps-use-light-theme preference from the current user profile.
fn apps_use_light_theme() -> Option<bool> {
    // read windows theme personalization value
    let key = wide_with_nul("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    let value_name = wide_with_nul("AppsUseLightTheme");

    let mut data = 0u32;
    let mut data_size = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value_name.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&mut data as *mut u32).cast(),
            &mut data_size,
        )
    };
    // return none when the registry lane is unavailable
    if status != ERROR_SUCCESS {
        return None;
    }

    Some(data != 0)
}

/// Resolve whether the host high-contrast accessibility lane is enabled.
fn high_contrast_enabled() -> Option<bool> {
    // query high contrast accessibility state
    let mut high_contrast = HIGHCONTRASTW {
        cbSize: std::mem::size_of::<HIGHCONTRASTW>() as u32,
        dwFlags: 0,
        lpszDefaultScheme: std::ptr::null_mut(),
    };
    let status = unsafe {
        SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            high_contrast.cbSize,
            (&mut high_contrast as *mut HIGHCONTRASTW).cast(),
            0,
        )
    };
    // return none when host query fails
    if status == 0 {
        return None;
    }

    Some((high_contrast.dwFlags & HCF_HIGHCONTRASTON) != 0)
}

/// Resolve one theme value from host preference state.
fn theme_from_preferences(high_contrast: bool, prefers_light: Option<bool>) -> WindowTheme {
    // prioritize high contrast mapping
    if high_contrast {
        // evaluate this condition
        if prefers_light == Some(true) {
            return WindowTheme::HighContrastLight;
        }

        return WindowTheme::HighContrastDark;
    }

    // otherwise map light and dark preference lanes
    match prefers_light {
        Some(true) => WindowTheme::Light,
        Some(false) => WindowTheme::Dark,
        None => WindowTheme::Unknown,
    }
}

/// Resolve one current window theme from host preference state.
fn current_window_theme() -> WindowTheme {
    let high_contrast = high_contrast_enabled().unwrap_or(false);
    let prefers_light = apps_use_light_theme();
    theme_from_preferences(high_contrast, prefers_light)
}

/// Convert one physical dimension into one i32 window-api dimension.
fn dimension_to_i32(value: u32, field: &'static str) -> RuntimeResult<i32> {
    // evaluate this condition
    if value > i32::MAX as u32 {
        return Err(core_platform::invalid_argument(
            field,
            "dimension exceeds Win32 i32 range",
        ));
    }

    Ok(value as i32)
}

/// Resolve one non-client outer size from one requested client size.
fn outer_size_from_client_size(
    hwnd: HWND,
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

    let dpi = unsafe { GetDpiForWindow(hwnd) };
    let status = if dpi > 0 {
        unsafe { AdjustWindowRectExForDpi(&mut rectangle, style, 0, ex_style, dpi) }
    } else {
        unsafe { AdjustWindowRectEx(&mut rectangle, style, 0, ex_style) }
    };
    // evaluate this condition
    if status == 0 {
        let syscall = if dpi > 0 {
            "AdjustWindowRectExForDpi"
        } else {
            "AdjustWindowRectEx"
        };
        return Err(core::io_error(
            operation,
            syscall,
            "failed to compute non-client window rectangle",
        ));
    }

    let outer_width = (rectangle.right - rectangle.left).max(1);
    let outer_height = (rectangle.bottom - rectangle.top).max(1);
    Ok((outer_width, outer_height))
}

/// Return one ShowWindow command for one visibility state.
fn show_command(visibility: WindowVisibility) -> i32 {
    // resolve this variant
    match visibility {
        WindowVisibility::Visible => SW_SHOW,
        WindowVisibility::Hidden => SW_HIDE,
        WindowVisibility::Minimized => SW_MINIMIZE,
        WindowVisibility::Maximized => SW_MAXIMIZE,
    }
}

/// Return one open-time ShowWindow command that respects focus behavior.
fn show_command_on_open(visibility: WindowVisibility, focus_on_show: bool) -> i32 {
    // resolve this variant
    match visibility {
        WindowVisibility::Visible if !focus_on_show => SW_SHOWNA,
        _ => show_command(visibility),
    }
}

/// Return whether one mode payload resolves to windowed.
fn is_windowed_mode(mode: WindowModeOptions) -> bool {
    matches!(mode, WindowModeOptions::WindowWindowedModeOptions(_))
}

/// Return whether one mode payload resolves to exclusive fullscreen.
fn is_exclusive_mode(mode: WindowModeOptions) -> bool {
    matches!(
        mode,
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(_)
    )
}

/// Resolve one mode payload to its optional preferred display handle.
fn mode_display(mode: WindowModeOptions) -> Option<resource::DisplayHandle> {
    // resolve this variant
    match mode {
        WindowModeOptions::WindowWindowedModeOptions(_) => None,
        WindowModeOptions::WindowBorderlessModeOptions(options) => options.display,
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(options) => Some(options.display),
    }
}

/// Resolve one mode payload to its optional preferred exclusive mode.
fn mode_display_mode(mode: WindowModeOptions) -> Option<DisplayMode> {
    // resolve this variant
    match mode {
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(options) => options.display_mode,
        _ => None,
    }
}

/// Compare two mode payloads by semantic fields, excluding discriminator string handles.
fn same_window_mode(left: WindowModeOptions, right: WindowModeOptions) -> bool {
    // compare mode payload by semantic fields
    match (left, right) {
        (
            WindowModeOptions::WindowWindowedModeOptions(_),
            WindowModeOptions::WindowWindowedModeOptions(_),
        ) => true,
        (
            WindowModeOptions::WindowBorderlessModeOptions(left),
            WindowModeOptions::WindowBorderlessModeOptions(right),
        ) => left.display == right.display,
        (
            WindowModeOptions::WindowExclusiveFullscreenModeOptions(left),
            WindowModeOptions::WindowExclusiveFullscreenModeOptions(right),
        ) => left.display == right.display && left.display_mode == right.display_mode,
        _ => false,
    }
}

/// Resolve visibility from one live hwnd state.
fn visibility_from_hwnd(hwnd: HWND) -> WindowVisibility {
    // hidden windows are always hidden
    if unsafe { IsWindowVisible(hwnd) } == 0 {
        return WindowVisibility::Hidden;
    }

    // iconic windows map to minimized visibility
    if unsafe { IsIconic(hwnd) } != 0 {
        return WindowVisibility::Minimized;
    }

    // zoomed windows map to maximized visibility
    if unsafe { IsZoomed(hwnd) } != 0 {
        return WindowVisibility::Maximized;
    }

    WindowVisibility::Visible
}

/// Build one Win32 style payload from one window binding snapshot.
fn window_style_for_binding(binding: &Win32WindowBinding) -> WINDOW_STYLE {
    // seed style from mode and chrome lanes
    let mut style: WINDOW_STYLE = if !is_windowed_mode(binding.mode) {
        WS_POPUP
    } else {
        // resolve this variant
        match binding.chrome {
            WindowChromeKind::Standard => WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            WindowChromeKind::Tool => WS_CAPTION | WS_SYSMENU,
            WindowChromeKind::Popup => WS_POPUP,
        }
    };

    // apply resize affordances for decorated windowed windows
    if binding.resizable
        && is_windowed_mode(binding.mode)
        && binding.chrome != WindowChromeKind::Popup
    {
        style |= WS_THICKFRAME | WS_MAXIMIZEBOX;
    }

    // strip decorations when decorated lane is disabled
    if !binding.decorated {
        style &= !(WS_CAPTION | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU);
        style |= WS_POPUP;
    }

    style
}

/// Build one Win32 ex-style payload from one window binding snapshot.
fn window_ex_style_for_binding(binding: &Win32WindowBinding) -> WINDOW_EX_STYLE {
    // seed empty ex style payload
    let mut ex_style: WINDOW_EX_STYLE = 0;

    // enable layered style for transparency lanes
    if binding.transparent || binding.opacity < 1.0 || binding.mouse_passthrough {
        ex_style |= WS_EX_LAYERED;
    }

    // enable input passthrough flag when requested
    if binding.mouse_passthrough {
        ex_style |= WS_EX_TRANSPARENT;
    }

    // apply always on top style lane
    if binding.always_on_top {
        ex_style |= WS_EX_TOPMOST;
    }

    // route taskbar and tool window behavior
    if !binding.taskbar_visible || binding.chrome == WindowChromeKind::Tool {
        ex_style |= WS_EX_TOOLWINDOW;
    } else {
        ex_style |= WS_EX_APPWINDOW;
    }

    ex_style
}

/// Apply one layered alpha payload for one window.
fn apply_window_alpha(binding: &Win32WindowBinding, operation: &'static str) -> RuntimeResult<()> {
    // skip alpha call when layered style is not active
    let ex_style = window_ex_style_for_binding(binding);
    // evaluate this condition
    if (ex_style & WS_EX_LAYERED) == 0 {
        return Ok(());
    }

    // apply normalized opacity as win32 alpha channel
    let alpha = (binding.opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    let status = unsafe { SetLayeredWindowAttributes(binding.hwnd, 0, alpha, LWA_ALPHA) };
    // evaluate this condition
    if status == 0 {
        return Err(core::io_error(
            operation,
            "SetLayeredWindowAttributes",
            "failed to apply window opacity",
        ));
    }

    Ok(())
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
        // evaluate this condition
        if previous == 0 {
            let error_code = GetLastError();
            // evaluate this condition
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
            // evaluate this condition
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
    // evaluate this condition
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
    // compute target style lanes
    let style = window_style_for_binding(binding);
    let ex_style = window_ex_style_for_binding(binding);

    // update style and ex style slots
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

    // commit frame changes through set window pos
    let status = unsafe {
        SetWindowPos(
            binding.hwnd,
            // evaluate this condition
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
    // evaluate this condition
    if status == 0 {
        return Err(core::io_error(
            operation,
            "SetWindowPos",
            "failed to apply window style",
        ));
    }

    // apply layered opacity lane when needed
    apply_window_alpha(binding, operation)?;

    Ok(())
}

/// Refresh one cached window state snapshot from one live hwnd.
fn refresh_window_snapshot(binding: &mut Win32WindowBinding) {
    // query current outer and client rectangles
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

    // refresh cached position from outer rect
    if unsafe { GetWindowRect(binding.hwnd, &mut window_rect) } != 0 {
        window_binding.position = WindowPosition {
            x: window_rect.left,
            y: window_rect.top,
        };
    }

    // refresh cached physical size from client rect
    if unsafe { GetClientRect(window_binding.hwnd, &mut client_rect) } != 0 {
        let width = (client_rect.right - client_rect.left).max(1) as u32;
        let height = (client_rect.bottom - client_rect.top).max(1) as u32;
        window_binding.size_physical = WindowPhysicalSize { width, height };
    }

    // refresh derived state lanes
    window_binding.scale_factor_milli = window_scale_factor_milli(binding.hwnd);
    binding.size_logical = physical_to_logical(
        window_binding.size_physical,
        window_binding.scale_factor_milli,
    );
    window_binding.visibility = visibility_from_hwnd(binding.hwnd);
    window_binding.focused = unsafe { GetForegroundWindow() } == binding.hwnd;
    window_binding.theme = current_window_theme();
}

/// Return whether one sizing-edge code anchors width from the left side.
fn sizing_edge_has_left_anchor(edge: u32) -> bool {
    matches!(edge, WMSZ_LEFT | WMSZ_TOPLEFT | WMSZ_BOTTOMLEFT)
}

/// Return whether one sizing-edge code anchors height from the top side.
fn sizing_edge_has_top_anchor(edge: u32) -> bool {
    matches!(edge, WMSZ_TOP | WMSZ_TOPLEFT | WMSZ_TOPRIGHT)
}

/// Return whether one sizing-edge code is a pure horizontal resize edge.
fn sizing_edge_is_horizontal(edge: u32) -> bool {
    matches!(edge, WMSZ_LEFT | WMSZ_RIGHT)
}

/// Return whether one sizing-edge code is a pure vertical resize edge.
fn sizing_edge_is_vertical(edge: u32) -> bool {
    matches!(edge, WMSZ_TOP | WMSZ_BOTTOM)
}

/// Set one outer-rect width according to one interactive sizing edge.
fn set_outer_width_for_edge(rect: &mut RECT, edge: u32, width: i32) {
    // evaluate this condition
    if sizing_edge_has_left_anchor(edge) {
        rect.left = rect.right.saturating_sub(width);
    } else {
        rect.right = rect.left.saturating_add(width);
    }
}

/// Set one outer-rect height according to one interactive sizing edge.
fn set_outer_height_for_edge(rect: &mut RECT, edge: u32, height: i32) {
    // evaluate this condition
    if sizing_edge_has_top_anchor(edge) {
        rect.top = rect.bottom.saturating_sub(height);
    } else {
        rect.bottom = rect.top.saturating_add(height);
    }
}

/// Apply one aspect-ratio lock to one live WM_SIZING rectangle update.
fn apply_aspect_ratio_on_sizing(entry: &WindowRuntimeEntry, edge: WPARAM, rect_ptr: LPARAM) {
    // abort when the sizing rectangle pointer is absent
    if rect_ptr == 0 {
        return;
    }

    // upgrade binding and lock mutable state
    let Some(binding) = entry.binding.upgrade() else {
        return;
    };
    let window_binding = match binding.try_lock() {
        Ok(binding) => binding,
        Err(std::sync::TryLockError::WouldBlock) => return,
        Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
    };
    let Some(aspect_ratio) = window_binding.aspect_ratio else {
        return;
    };

    // load current frame and client metrics
    let edge = edge as u32;
    let rect = unsafe { &mut *(rect_ptr as *mut RECT) };

    let mut outer_rect = RECT {
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
    // evaluate this condition
    if unsafe { GetWindowRect(window_binding.hwnd, &mut outer_rect) } == 0 {
        return;
    }
    // evaluate this condition
    if unsafe { GetClientRect(window_binding.hwnd, &mut client_rect) } == 0 {
        return;
    }

    // compute proposed client size and ratio adjusted target
    let frame_width =
        ((outer_rect.right - outer_rect.left) - (client_rect.right - client_rect.left)).max(0);
    let frame_height =
        ((outer_rect.bottom - outer_rect.top) - (client_rect.bottom - client_rect.top)).max(0);

    let proposed_outer_width = (rect.right - rect.left).max(1);
    let proposed_outer_height = (rect.bottom - rect.top).max(1);
    let proposed_client_width = (proposed_outer_width - frame_width).max(1);
    let proposed_client_height = (proposed_outer_height - frame_height).max(1);

    let numerator = aspect_ratio.numerator as f64;
    let denominator = aspect_ratio.denominator as f64;
    let from_width = ((proposed_client_width as f64 * denominator) / numerator).round() as i32;
    let from_height = ((proposed_client_height as f64 * numerator) / denominator).round() as i32;
    let from_width = from_width.max(1);
    let from_height = from_height.max(1);

    let (target_width, target_height) = if sizing_edge_is_horizontal(edge) {
        (proposed_client_width, from_width)
    } else if sizing_edge_is_vertical(edge) {
        (from_height, proposed_client_height)
    } else {
        let height_error = from_width.abs_diff(proposed_client_height);
        let width_error = from_height.abs_diff(proposed_client_width);
        // evaluate this condition
        if height_error <= width_error {
            (proposed_client_width, from_width)
        } else {
            (from_height, proposed_client_height)
        }
    };

    // clamp through logical constraints and map back to outer rect
    let target_physical = WindowPhysicalSize {
        width: target_width as u32,
        height: target_height as u32,
    };
    let target_logical =
        physical_to_logical(target_physical, window_window_binding.scale_factor_milli);
    let clamped_logical = clamp_logical_size(target_logical, window_binding.constraints);
    let clamped_physical =
        logical_to_physical(clamped_logical, window_window_binding.scale_factor_milli);

    let target_outer_width = (clamped_physical.width as i32)
        .saturating_add(frame_width)
        .max(1);
    let target_outer_height = (clamped_physical.height as i32)
        .saturating_add(frame_height)
        .max(1);
    set_outer_width_for_edge(rect, edge, target_outer_width);
    set_outer_height_for_edge(rect, edge, target_outer_height);
}

/// Resolve one class-name payload for Win32 window registration.
fn window_class_name() -> Vec<u16> {
    core_platform::wide_with_nul("destack_display_win32")
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

    let mut window_binding = match binding.try_lock() {
        Ok(binding) => binding,
        Err(std::sync::TryLockError::WouldBlock) => return,
        Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
    };
    let previous = window_binding.clone();
    let mut monitor_topology_changed = false;

    // evaluate this condition
    if message == WM_MOVE {
        window_binding.position = WindowPosition {
            x: (lparam as u32 & 0xFFFF) as i16 as i32,
            y: ((lparam as u32 >> 16) & 0xFFFF) as i16 as i32,
        };
    }

    // evaluate this condition
    if message == WM_SIZE {
        let width = (lparam as usize & 0xFFFF) as u16 as u32;
        let height = ((lparam as usize >> 16) & 0xFFFF) as u16 as u32;
        // evaluate this condition
        if width > 0 && height > 0 {
            window_binding.size_physical = WindowPhysicalSize { width, height };
            binding.size_logical = physical_to_logical(
                window_binding.size_physical,
                window_binding.scale_factor_milli,
            );
        }

        // evaluate this condition
        if wparam == SIZE_MINIMIZED as usize {
            window_binding.visibility = WindowVisibility::Minimized;
        } else if wparam == SIZE_MAXIMIZED as usize {
            window_binding.visibility = WindowVisibility::Maximized;
        } else if window_binding.visibility != WindowVisibility::Hidden {
            window_binding.visibility = WindowVisibility::Visible;
        }
    }

    // evaluate this condition
    if message == WM_ACTIVATE {
        let activation = (wparam & 0xFFFF) as u32;
        window_binding.focused = activation != WA_INACTIVE;
    }

    // evaluate this condition
    if message == WM_SETFOCUS {
        window_binding.focused = true;
    }

    // evaluate this condition
    if message == WM_KILLFOCUS {
        window_binding.focused = false;
    }

    // evaluate this condition
    if message == WM_SHOWWINDOW {
        // evaluate this condition
        if wparam == 0 {
            window_binding.visibility = WindowVisibility::Hidden;
        } else if window_binding.visibility == WindowVisibility::Hidden {
            window_binding.visibility = WindowVisibility::Visible;
        }
    }

    // evaluate this condition
    if message == WM_DPICHANGED {
        let dpi_x = (wparam & 0xFFFF) as u32;
        // evaluate this condition
        if dpi_x > 0 {
            window_binding.scale_factor_milli = dpi_x.saturating_mul(1000).max(1) / 96;
        }

        // evaluate this condition
        if lparam != 0 {
            let recommended = unsafe { *(lparam as *const RECT) };
            let status = unsafe {
                SetWindowPos(
                    window_binding.hwnd,
                    0,
                    recommended.left,
                    recommended.top,
                    (recommended.right - recommended.left).max(1),
                    (recommended.bottom - recommended.top).max(1),
                    SWP_NOACTIVATE | SWP_NOZORDER,
                )
            };
            // evaluate this condition
            if status == 0 {
                let error_code = core_platform::last_error_code() as u32;
                entry.window_runtime_state.diagnostics.warn(
                    "display",
                    "destack.display.window.wndproc",
                    "WM_DPICHANGED SetWindowPos failed while applying recommended rectangle",
                    Some(error_code),
                );
            }
        }
    }

    // evaluate this condition
    if message == WM_THEMECHANGED || message == WM_SETTINGCHANGE {
        window_binding.theme = current_window_theme();
    }

    // evaluate this condition
    if message == WM_DISPLAYCHANGE {
        monitor_topology_changed = true;
    }

    // evaluate this condition
    if message == WM_DESTROY {
        window_window_binding.destroyed_emitted = true;
        window_binding.visibility = WindowVisibility::Hidden;
        window_binding.focused = false;
    }

    refresh_window_snapshot(&mut window_binding);
    let next = window_binding.clone();
    drop(window_binding);

    // evaluate this condition
    if monitor_topology_changed
        && let Err(error) = event::publish_monitor_topology_deltas(&entry.event_runtime_state)
    {
        entry.window_runtime_state.diagnostics.warn(
            "display",
            "destack.display.window.messageDispatch.monitorTopology",
            format!("failed to publish monitor topology deltas: {error}"),
            None,
        );
    }

    event::publish_state_deltas(&entry.event_runtime_state, entry.window, &previous, &next);
}

/// Drain pending window-thread messages and dispatch them through the registered wndproc.
pub(in super::super) fn pump_window_messages(binding: &BindingCallContext) -> RuntimeResult<()> {
    // drain pending thread messages while preserving host-managed quit lifecycle
    binding.host().pump_pending_thread_messages(true)?;

    Ok(())
}

/// Wndproc for display windows.
unsafe extern "system" fn display_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // apply live aspect ratio constraints during interactive sizing
    if message == WM_SIZING
        && let Some(entry) = runtime_window_entry(hwnd)
    {
        apply_aspect_ratio_on_sizing(&entry, wparam, lparam);
        return 1;
    }

    // intercept close request and publish close requested once
    if message == WM_CLOSE
        && let Some(entry) = runtime_window_entry(hwnd)
    {
        let is_forced_close = wparam == core::WINDOW_CLOSE_FORCE_WPARAM;

        // evaluate this condition
        if let Some(binding) = entry.binding.upgrade() {
            let mut window_binding = binding.lock().unwrap_or_else(|error| error.into_inner());
            // evaluate this condition
            if !window_binding.close_requested_emitted {
                window_binding.close_requested_emitted = true;
                drop(window_binding);
                event::publish_window_close_requested_event(
                    &entry.event_runtime_state,
                    entry.window,
                );
            }
        }

        // evaluate this condition
        if is_forced_close {
            let status = unsafe { DestroyWindow(hwnd) };
            // evaluate this condition
            if status != 0 {
                return 0;
            }

            let error_code = core_platform::last_error_code() as u32;
            entry.window_runtime_state.diagnostics.warn(
                "display",
                "destack.display.window.wndproc",
                "forced close DestroyWindow failed, falling back to DefWindowProcW",
                Some(error_code),
            );
            return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
        }

        return 0;
    }

    // cleanup runtime entry and publish destroyed on teardown
    if message == WM_DESTROY
        && let Some(entry) = runtime_window_entry(hwnd)
    {
        let mut should_emit_destroyed = true;
        // evaluate this condition
        if let Some(binding) = entry.binding.upgrade() {
            let mut window_binding = binding.lock().unwrap_or_else(|error| error.into_inner());
            unregister_window_drop_target(&mut window_binding);
            should_emit_destroyed = !window_binding.destroyed_emitted;
            window_window_binding.destroyed_emitted = true;
        }

        apply_window_message_snapshot(&entry, message, wparam, lparam);
        // evaluate this condition
        if should_emit_destroyed {
            event::publish_window_destroyed_event(&entry.event_runtime_state, entry.window);
        }
        remove_cursor_policy(&entry.window_runtime_state, entry.window);
        refresh_cursor_policy_best_effort(&entry.window_runtime_state);
        unregister_runtime_window(hwnd);
        return 0;
    }

    // apply stored cursor icon for this window during host cursor updates
    if message == WM_SETCURSOR
        && let Some(entry) = runtime_window_entry(hwnd)
        && let Some(binding) = entry.binding.upgrade()
    {
        let window_binding = binding.lock().unwrap_or_else(|error| error.into_inner());
        let cursor = unsafe { LoadCursorW(0, cursor_name(window_binding.cursor_icon)) };
        // evaluate this condition
        if cursor != 0 {
            unsafe {
                SetCursor(cursor);
            }
            return 1;
        }
    }

    // mirror host message deltas into cached runtime state
    if matches!(
        message,
        WM_MOVE
            | WM_SIZE
            | WM_SETFOCUS
            | WM_KILLFOCUS
            | WM_THEMECHANGED
            | WM_SETTINGCHANGE
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
    // resolve process module instance for class registration
    let instance = unsafe { GetModuleHandleW(std::ptr::null()) } as HINSTANCE;
    // evaluate this condition
    if instance == 0 {
        return Err(core::io_error(
            "destack.display.window.open",
            "GetModuleHandleW",
            "failed to resolve module handle",
        ));
    }

    // build and register one window class
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
    // tolerate duplicate class registration by name
    if atom == 0 {
        let code = core_platform::last_error_code() as u32;
        // evaluate this condition
        if code != ERROR_CLASS_ALREADY_EXISTS {
            return Err(core::io_error_with_code(
                "destack.display.window.open",
                "RegisterClassW",
                code,
                "failed to register display window class",
            ));
        }
    }

    Ok(())
}

/// Resolve one monitor handle to one display identifier for exclusive-fullscreen updates.
fn mode_target_display(
    mode: WindowModeOptions,
    current_display: Option<resource::DisplayHandle>,
) -> Option<resource::DisplayHandle> {
    // evaluate this condition
    if let Some(display) = mode_display(mode) {
        return Some(display);
    }

    // evaluate this condition
    if is_exclusive_mode(mode) {
        return current_display;
    }

    current_display
}

/// Restore one captured exclusive-fullscreen display mode snapshot.
fn restore_exclusive_mode(
    binding: &BindingCallContext,
    restore: &ExclusiveModeRestore,
    operation: &'static str,
    emit_events: bool,
) -> RuntimeResult<()> {
    monitor::apply_monitor_mode_by_id(&restore.display_id, restore.mode, operation)?;
    // evaluate this condition
    if emit_events {
        event::publish_mode_changed_event(binding, &restore.display_id, restore.mode);
        // evaluate this condition
        if let Some(snapshot) = monitor::monitor_snapshot_by_id(&restore.display_id)? {
            event::publish_descriptor_changed_event(
                binding,
                &snapshot.descriptor,
                core::DISPLAY_CHANGED_MASK_BOUNDS
                    | core::DISPLAY_CHANGED_MASK_WORKAREA
                    | core::DISPLAY_CHANGED_MASK_SCALE
                    | core::DISPLAY_CHANGED_MASK_ORIENTATION,
            );
        }
    }

    event::refresh_monitor_topology_cache(binding)?;
    Ok(())
}

/// Apply one window-mode transition and associated exclusive-display state changes.
fn apply_mode_options(
    binding_2: &BindingCallContext,
    binding: &mut Win32WindowBinding,
    mode: WindowModeOptions,
    operation: &'static str,
    emit_monitor_events: bool,
) -> RuntimeResult<()> {
    let target_display = mode_target_display(mode, binding.display);
    // evaluate this condition
    if is_exclusive_mode(mode) {
        let Some(display) = target_display else {
            return Err(core_platform::invalid_argument(
                "mode.display",
                "exclusive fullscreen requires one display target",
            ));
        };

        let display_id = display_resource::resolve_display_id(binding_2, display, operation)?;

        // evaluate this condition
        if let Some(restore) = binding.exclusive_restore.as_ref()
            && restore.display_id != display_id
        {
            let restore = restore.clone();
            restore_exclusive_mode(binding_2, &restore, operation, emit_monitor_events)?;
            binding.exclusive_restore = None;
        }

        // evaluate this condition
        if binding.exclusive_restore.is_none() {
            let snapshot = monitor::monitor_snapshot_by_id(&display_id)?.ok_or_else(|| {
                core_platform::io_not_found(
                    operation,
                    format!("display id '{display_id}' is no longer available"),
                )
            })?;

            binding.exclusive_restore = Some(ExclusiveModeRestore {
                display_id: display_id.clone(),
                mode: snapshot.current_mode,
            });
        }

        // evaluate this condition
        if let Some(display_mode) = mode_display_mode(mode) {
            monitor::apply_monitor_mode_by_id(&display_id, display_mode, operation)?;
            // evaluate this condition
            if emit_monitor_events {
                event::publish_mode_changed_event(binding_2, &display_id, display_mode);
                // evaluate this condition
                if let Some(snapshot) = monitor::monitor_snapshot_by_id(&display_id)? {
                    event::publish_descriptor_changed_event(
                        binding_2,
                        &snapshot.descriptor,
                        core::DISPLAY_CHANGED_MASK_BOUNDS
                            | core::DISPLAY_CHANGED_MASK_WORKAREA
                            | core::DISPLAY_CHANGED_MASK_SCALE
                            | core::DISPLAY_CHANGED_MASK_ORIENTATION,
                    );
                }
            }
        }
    } else if let Some(restore) = binding.exclusive_restore.clone() {
        restore_exclusive_mode(binding_2, &restore, operation, emit_monitor_events)?;
        binding.exclusive_restore = None;
    }

    binding.mode = mode;
    binding.display = target_display;
    apply_window_style(binding, operation)?;

    // evaluate this condition
    if let Some(rectangle) =
        monitor::mode_target_rect(binding_2, mode, binding.display, binding.hwnd, operation)?
    {
        apply_window_rect(binding, rectangle, operation)?;
    }

    event::refresh_monitor_topology_cache(binding_2)?;
    Ok(())
}

/// Set one cursor visibility lane.
fn host_cursor_visible() -> Option<bool> {
    let mut cursor = CURSORINFO {
        cbSize: std::mem::size_of::<CURSORINFO>() as u32,
        flags: 0,
        hCursor: 0,
        ptScreenPos: POINT { x: 0, y: 0 },
    };
    let status = unsafe { GetCursorInfo(&mut cursor) };
    // evaluate this condition
    if status == 0 {
        return None;
    }

    Some((cursor.flags & CURSOR_SHOWING) != 0)
}

/// Set one cursor visibility lane.
fn set_cursor_visibility(runtime_state: &Arc<WindowRuntimeState>, visible: bool) {
    // skip host calls when cursor visibility already matches
    let mut state = runtime_state
        .cursor_visible_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    // evaluate this condition
    if *state == Some(visible) && host_cursor_visible() == Some(visible) {
        return;
    }

    // drive win32 global cursor display counter toward target visibility
    unsafe {
        // evaluate this condition
        if visible {
            let mut display_count = ShowCursor(1);
            // iterate while this condition holds
            while display_count < 0 {
                display_count = ShowCursor(1);
            }
        } else {
            let mut display_count = ShowCursor(0);
            // iterate while this condition holds
            while display_count >= 0 {
                display_count = ShowCursor(0);
            }
        }
    }

    *state = Some(host_cursor_visible().unwrap_or(visible));
}

/// Restore global cursor state when one window is closed.
fn restore_cursor_after_close(
    runtime_state: &Arc<WindowRuntimeState>,
    window: resource::WindowHandle,
) {
    remove_cursor_policy(runtime_state, window);
    refresh_cursor_policy_best_effort(runtime_state);
}

/// Resolve one referenced relationship window handle to one hwnd.
fn resolve_relationship_hwnd(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    operation: &'static str,
) -> RuntimeResult<HWND> {
    let resolved_binding = display_resource::resolve_window_binding(binding, window, operation)?;
    let resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    Ok(resolved_binding.hwnd)
}

/// Resolve the owner relationship for one window binding.
fn owner_relationship(binding: &Win32WindowBinding) -> Option<resource::WindowHandle> {
    binding.transient_for.or(binding.parent)
}

/// Apply owner relationship style for one window.
fn apply_owner_relationship(
    binding_2: &BindingCallContext,
    binding: &Win32WindowBinding,
    operation: &'static str,
) -> RuntimeResult<()> {
    let owner_hwnd = if let Some(owner) = owner_relationship(binding) {
        let owner_hwnd = resolve_relationship_hwnd(binding_2, owner, operation)?;
        // evaluate this condition
        if owner_hwnd == binding.hwnd {
            return Err(core_platform::invalid_argument(
                "window",
                "window relationship cannot target itself",
            ));
        }

        owner_hwnd
    } else {
        0
    };

    set_window_long_ptr_checked(
        binding.hwnd,
        GWLP_HWNDPARENT,
        owner_hwnd,
        operation,
        "SetWindowLongPtrW",
    )?;
    let status = unsafe {
        SetWindowPos(
            binding.hwnd,
            0,
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER,
        )
    };
    // evaluate this condition
    if status == 0 {
        return Err(core::io_error(
            operation,
            "SetWindowPos",
            "failed to apply window owner relationship",
        ));
    }

    Ok(())
}

/// Set one owner-window enabled state.
fn set_owner_enabled(
    binding: &BindingCallContext,
    owner: resource::WindowHandle,
    enabled: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    let owner_hwnd = resolve_relationship_hwnd(binding, owner, operation)?;
    let enabled = if enabled { 1 } else { 0 };

    unsafe {
        SetLastError(0);
        let previous = EnableWindow(owner_hwnd, enabled);
        // evaluate this condition
        if previous == 0 {
            let error_code = GetLastError();
            // evaluate this condition
            if error_code != 0 {
                return Err(core::io_error_with_code(
                    operation,
                    "EnableWindow",
                    error_code as u32,
                    "failed to update owner enabled state",
                ));
            }
        }
    }

    Ok(())
}

/// Apply one modal-owner state transition for one window.
fn apply_modal_owner_transition(
    binding: &BindingCallContext,
    previous_owner: Option<resource::WindowHandle>,
    previous_modal: bool,
    next_owner: Option<resource::WindowHandle>,
    next_modal: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    // evaluate this condition
    if next_modal && next_owner.is_none() {
        return Err(core_platform::invalid_argument(
            "modal",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // evaluate this condition
    if previous_modal
        && let Some(owner) = previous_owner
        && (!next_modal || Some(owner) != next_owner)
    {
        set_owner_enabled(binding, owner, true, operation)?;
    }

    // evaluate this condition
    if next_modal && let Some(owner) = next_owner {
        set_owner_enabled(binding, owner, false, operation)?;
    }

    Ok(())
}

/// Re-enable one modal owner as part of window close cleanup.
fn restore_modal_owner_on_close(binding_2: &BindingCallContext, binding: &Win32WindowBinding) {
    // evaluate this condition
    if !binding.modal {
        return;
    }

    // evaluate this condition
    if let Some(owner) = owner_relationship(binding) {
        let _ = set_owner_enabled(binding_2, owner, true, "destack.display.window.close");
    }
}

/// Map one resize edge selector to one non-client hit-test value.
fn resize_hit_test(edge: WindowResizeEdge) -> usize {
    // map binding edge enum to win32 non client hit test value
    match edge {
        WindowResizeEdge::North => HTTOP as usize,
        WindowResizeEdge::South => HTBOTTOM as usize,
        WindowResizeEdge::East => HTRIGHT as usize,
        WindowResizeEdge::West => HTLEFT as usize,
        WindowResizeEdge::NorthEast => HTTOPRIGHT as usize,
        WindowResizeEdge::NorthWest => HTTOPLEFT as usize,
        WindowResizeEdge::SouthEast => HTBOTTOMRIGHT as usize,
        WindowResizeEdge::SouthWest => HTBOTTOMLEFT as usize,
    }
}

/// Apply one cursor interaction mode for one window.
fn apply_cursor_mode(
    hwnd: HWND,
    mode: WindowCursorMode,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve this variant
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
            // evaluate this condition
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

            // evaluate this condition
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

            // evaluate this condition
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
            // evaluate this condition
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
            // evaluate this condition
            if unsafe { ClientToScreen(hwnd, &mut center_screen) } == 0 {
                return Err(core::io_error(
                    operation,
                    "ClientToScreen",
                    "failed to map cursor lock point",
                ));
            }

            let status = unsafe { SetCursorPos(center_screen.x, center_screen.y) };
            // evaluate this condition
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
            // evaluate this condition
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
    // map binding cursor icon enum to win32 cursor selector
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
