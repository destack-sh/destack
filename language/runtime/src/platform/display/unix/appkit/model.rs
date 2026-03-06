use std::thread::ThreadId;

use crate::platform::display::{
    DisplayBackend, DisplayMode, DisplayOrientation, DisplaySupportStatus, WindowAspectRatio,
    WindowChromeKind, WindowCursorIcon, WindowCursorMode, WindowLogicalSize, WindowModeOptions,
    WindowOcclusionState, WindowPhysicalSize, WindowPosition, WindowRole, WindowSafeAreaInsets,
    WindowSizeConstraints, WindowTheme, WindowVisibility,
};
use crate::platform::resource;

/// Stored descriptor payload with owned strings.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DisplayDescriptorSnapshot {
    /// The backend that produced this descriptor.
    pub(crate) backend: DisplayBackend,
    /// The stable runtime display identifier.
    pub(crate) id: String,
    /// The host display name.
    pub(crate) name: String,
    /// Whether this display is primary.
    pub(crate) primary: bool,
    /// The display origin x coordinate in desktop space.
    pub(crate) x: i32,
    /// The display origin y coordinate in desktop space.
    pub(crate) y: i32,
    /// The display width in physical pixels.
    pub(crate) width_px: u32,
    /// The display height in physical pixels.
    pub(crate) height_px: u32,
    /// The work-area origin x coordinate in desktop space.
    pub(crate) work_area_x: i32,
    /// The work-area origin y coordinate in desktop space.
    pub(crate) work_area_y: i32,
    /// The work-area width in physical pixels.
    pub(crate) work_area_width_px: u32,
    /// The work-area height in physical pixels.
    pub(crate) work_area_height_px: u32,
    /// The physical width in millimeters.
    pub(crate) width_mm: u32,
    /// The physical height in millimeters.
    pub(crate) height_mm: u32,
    /// The scale factor in milli-scale units.
    pub(crate) scale_factor_milli: u32,
    /// The current display orientation.
    pub(crate) orientation: DisplayOrientation,
    /// The built-in panel support status.
    pub(crate) builtin_panel: DisplaySupportStatus,
    /// The variable-refresh support status.
    pub(crate) variable_refresh_support: DisplaySupportStatus,
    /// The HDR support status.
    pub(crate) hdr_support: DisplaySupportStatus,
}

/// Snapshot payload for one AppKit monitor endpoint.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MonitorSnapshot {
    /// The descriptor payload for this monitor.
    pub(crate) descriptor: DisplayDescriptorSnapshot,
    /// The current active mode payload.
    pub(crate) current_mode: DisplayMode,
    /// The desktop mode payload.
    pub(crate) desktop_mode: DisplayMode,
    /// The enumerated host mode set.
    pub(crate) modes: Vec<DisplayMode>,
}

/// Resource payload for one opened monitor handle.
#[derive(Debug, Clone)]
pub(crate) struct AppKitDisplayBinding {
    /// The stable monitor identifier.
    pub(crate) id: String,
}

/// Resource payload for one opened window handle.
#[derive(Debug, Clone)]
pub(crate) struct AppKitWindowBinding {
    /// The stable runtime identifier.
    pub(crate) id: String,
    /// The owner thread identifier that created this window.
    pub(crate) owner_thread_id: ThreadId,
    /// The current host-visible title.
    pub(crate) title: String,
    /// The current window role.
    pub(crate) role: WindowRole,
    /// The current mode configuration.
    pub(crate) mode: WindowModeOptions,
    /// The current display association.
    pub(crate) display: Option<resource::DisplayHandle>,
    /// Whether this window is resizable.
    pub(crate) resizable: bool,
    /// Whether this window uses host decorations.
    pub(crate) decorated: bool,
    /// The current window chrome style.
    pub(crate) chrome: WindowChromeKind,
    /// Whether this window is currently visible in task switching surfaces.
    pub(crate) taskbar_visible: bool,
    /// Whether this window requested compositor transparency.
    pub(crate) transparent: bool,
    /// The current whole-window opacity in `[0.0, 1.0]`.
    pub(crate) opacity: f64,
    /// Whether this window is currently always-on-top.
    pub(crate) always_on_top: bool,
    /// The current parent window relationship.
    pub(crate) parent: Option<resource::WindowHandle>,
    /// The current transient-owner window relationship.
    pub(crate) transient_for: Option<resource::WindowHandle>,
    /// Whether this window is currently modal.
    pub(crate) modal: bool,
    /// Whether this window is currently mouse-passthrough.
    pub(crate) mouse_passthrough: bool,
    /// The current aspect-ratio lock.
    pub(crate) aspect_ratio: Option<WindowAspectRatio>,
    /// The current visibility state.
    pub(crate) visibility: WindowVisibility,
    /// The requested visibility state while AppKit host propagation is still pending.
    pub(crate) requested_visibility: Option<WindowVisibility>,
    /// The non-minimized visibility state restored after one minimize transition.
    pub(crate) restored_visibility: WindowVisibility,
    /// The current optional logical size constraints.
    pub(crate) constraints: Option<WindowSizeConstraints>,
    /// The current cursor visibility state.
    pub(crate) cursor_visible: bool,
    /// The current cursor interaction mode.
    pub(crate) cursor_mode: WindowCursorMode,
    /// The current cursor icon selector.
    pub(crate) cursor_icon: WindowCursorIcon,
    /// The current desktop position.
    pub(crate) position: WindowPosition,
    /// The current logical size.
    pub(crate) size_logical: WindowLogicalSize,
    /// The current physical size.
    pub(crate) size_physical: WindowPhysicalSize,
    /// The current scale factor.
    pub(crate) scale_factor_milli: u32,
    /// The current keyboard focus state.
    pub(crate) focused: bool,
    /// The current occlusion state.
    pub(crate) occlusion: WindowOcclusionState,
    /// The current safe-area insets when available.
    pub(crate) safe_area_insets: Option<WindowSafeAreaInsets>,
    /// The current theme value.
    pub(crate) theme: WindowTheme,
}
