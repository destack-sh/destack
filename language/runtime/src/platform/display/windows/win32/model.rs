use windows_sys::Win32::Foundation::HWND;

use crate::platform::display::{
    DisplayBackend, DisplayMode, DisplayOrientation, WindowCursorIcon, WindowCursorMode,
    WindowLogicalSize, WindowModeOptions, WindowPhysicalSize, WindowPosition,
    WindowSizeConstraints, WindowTheme, WindowVisibility,
};
use crate::platform::resource;

/// Stored descriptor payload with owned strings.
#[derive(Debug, Clone)]
pub(super) struct DisplayDescriptorOwned {
    /// Resolved backend that produced this descriptor.
    pub(super) backend: DisplayBackend,
    /// Stable runtime display identifier.
    pub(super) id: String,
    /// Host display name.
    pub(super) name: String,
    /// Whether this display is primary.
    pub(super) primary: bool,
    /// Display origin x coordinate in desktop space.
    pub(super) x: i32,
    /// Display origin y coordinate in desktop space.
    pub(super) y: i32,
    /// Display width in physical pixels.
    pub(super) width_px: u32,
    /// Display height in physical pixels.
    pub(super) height_px: u32,
    /// Work-area origin x coordinate in desktop space.
    pub(super) work_area_x: i32,
    /// Work-area origin y coordinate in desktop space.
    pub(super) work_area_y: i32,
    /// Work-area width in physical pixels.
    pub(super) work_area_width_px: u32,
    /// Work-area height in physical pixels.
    pub(super) work_area_height_px: u32,
    /// Physical width in millimeters.
    pub(super) width_mm: u32,
    /// Physical height in millimeters.
    pub(super) height_mm: u32,
    /// Scale factor in milli-scale units.
    pub(super) scale_factor_milli: u32,
    /// Current display orientation.
    pub(super) orientation: DisplayOrientation,
    /// Whether this display is one built-in panel.
    pub(super) is_builtin: bool,
    /// Whether this display reports variable refresh support.
    pub(super) supports_variable_refresh: bool,
    /// Whether this display reports hdr support.
    pub(super) supports_hdr: bool,
}

/// Snapshot payload for one Win32 monitor endpoint.
#[derive(Debug, Clone)]
pub(super) struct MonitorSnapshot {
    /// Descriptor payload for this monitor.
    pub(super) descriptor: DisplayDescriptorOwned,
    /// Current active mode payload.
    pub(super) current_mode: DisplayMode,
    /// Desktop mode payload.
    pub(super) desktop_mode: DisplayMode,
    /// Enumerated host mode set.
    pub(super) modes: Vec<DisplayMode>,
}

/// Resource payload for one opened monitor handle.
#[derive(Debug, Clone)]
pub(super) struct Win32DisplayBinding {
    /// Stable monitor identifier.
    pub(super) id: String,
}

/// Captured desktop mode snapshot used to restore exclusive fullscreen transitions.
#[derive(Debug, Clone)]
pub(super) struct ExclusiveModeRestore {
    /// Monitor identifier associated with this restore snapshot.
    pub(super) display_id: String,
    /// Mode to apply when leaving exclusive fullscreen.
    pub(super) mode: DisplayMode,
}

/// Resource payload for one opened window handle.
#[derive(Debug, Clone)]
pub(super) struct Win32WindowBinding {
    /// Stable runtime identifier.
    pub(super) id: String,
    /// Native Win32 window handle.
    pub(super) hwnd: HWND,
    /// Owner thread identifier that created this window.
    pub(super) owner_thread_id: u32,
    /// Current host-visible title.
    pub(super) title: String,
    /// Current mode configuration.
    pub(super) mode: WindowModeOptions,
    /// Current display association.
    pub(super) display: Option<resource::DisplayHandle>,
    /// Whether this window is resizable.
    pub(super) resizable: bool,
    /// Whether this window uses host decorations.
    pub(super) decorated: bool,
    /// Whether this window requested compositor transparency.
    pub(super) transparent: bool,
    /// Whether this window is currently always-on-top.
    pub(super) always_on_top: bool,
    /// Current visibility state.
    pub(super) visibility: WindowVisibility,
    /// Current optional logical size constraints.
    pub(super) constraints: Option<WindowSizeConstraints>,
    /// Current cursor visibility state.
    pub(super) cursor_visible: bool,
    /// Current cursor interaction mode.
    pub(super) cursor_mode: WindowCursorMode,
    /// Current cursor icon selector.
    pub(super) cursor_icon: WindowCursorIcon,
    /// Current desktop position.
    pub(super) position: WindowPosition,
    /// Current logical size.
    pub(super) size_logical: WindowLogicalSize,
    /// Current physical size.
    pub(super) size_physical: WindowPhysicalSize,
    /// Current scale factor.
    pub(super) scale_factor_milli: u32,
    /// Current keyboard focus state.
    pub(super) focused: bool,
    /// Current occlusion state.
    pub(super) occluded: bool,
    /// Current theme value.
    pub(super) theme: WindowTheme,
    /// Optional restore snapshot for exclusive fullscreen transitions.
    pub(super) exclusive_restore: Option<ExclusiveModeRestore>,
    /// Whether closeRequested was already emitted for this window lifetime.
    pub(super) close_requested_emitted: bool,
    /// Whether destroyed was already emitted for this window lifetime.
    pub(super) destroyed_emitted: bool,
}
