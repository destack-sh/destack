use wayland_client::backend::ObjectId;

use crate::platform::display::{
    DisplayBackend, DisplayMode, DisplayOrientation, DisplaySupportStatus, WindowAspectRatio,
    WindowChromeKind, WindowCursorIcon, WindowCursorMode, WindowLogicalSize, WindowModeOptions,
    WindowPhysicalSize, WindowPosition, WindowRole, WindowSafeAreaInsets, WindowSizeConstraints,
    WindowTheme, WindowVisibility,
};
use crate::platform::resource;

/// Stored descriptor payload with owned strings.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DisplayDescriptorSnapshot {
    /// Resolved backend that produced this descriptor.
    pub(crate) backend: DisplayBackend,
    /// Stable runtime display identifier.
    pub(crate) id: String,
    /// Host display name.
    pub(crate) name: String,
    /// Whether this display is primary.
    pub(crate) primary: bool,
    /// Display origin x coordinate in desktop space.
    pub(crate) x: i32,
    /// Display origin y coordinate in desktop space.
    pub(crate) y: i32,
    /// Display width in physical pixels.
    pub(crate) width_px: u32,
    /// Display height in physical pixels.
    pub(crate) height_px: u32,
    /// Work-area origin x coordinate in desktop space.
    pub(crate) work_area_x: i32,
    /// Work-area origin y coordinate in desktop space.
    pub(crate) work_area_y: i32,
    /// Work-area width in physical pixels.
    pub(crate) work_area_width_px: u32,
    /// Work-area height in physical pixels.
    pub(crate) work_area_height_px: u32,
    /// Physical width in millimeters.
    pub(crate) width_mm: u32,
    /// Physical height in millimeters.
    pub(crate) height_mm: u32,
    /// Scale factor in milli-scale units.
    pub(crate) scale_factor_milli: u32,
    /// Current display orientation.
    pub(crate) orientation: DisplayOrientation,
    /// Built-in panel support status.
    pub(crate) builtin_panel: DisplaySupportStatus,
    /// Variable-refresh support status.
    pub(crate) variable_refresh_support: DisplaySupportStatus,
    /// HDR support status.
    pub(crate) hdr_support: DisplaySupportStatus,
}

/// Snapshot payload for one Wayland monitor endpoint.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MonitorSnapshot {
    /// Descriptor payload for this monitor.
    pub(crate) descriptor: DisplayDescriptorSnapshot,
    /// Current active mode payload.
    pub(crate) current_mode: DisplayMode,
    /// Desktop mode payload.
    pub(crate) desktop_mode: DisplayMode,
    /// Enumerated host mode set.
    pub(crate) modes: Vec<DisplayMode>,
}

/// Resource payload for one opened display handle.
#[derive(Debug, Clone)]
pub(crate) struct WaylandDisplayHostState {
    /// Stable monitor identifier.
    pub(crate) id: String,
}

/// Runtime payload for one Wayland host window lane.
#[derive(Debug, Clone)]
pub(crate) struct WaylandWindowHost {
    /// Native wayland `wl_surface` object id.
    pub(crate) surface: ObjectId,
    /// Native wayland `xdg_surface` object id.
    pub(crate) xdg_surface: Option<ObjectId>,
    /// Native wayland `xdg_toplevel` object id.
    pub(crate) xdg_toplevel: Option<ObjectId>,
    /// Native wayland `xdg_popup` object id.
    pub(crate) xdg_popup: Option<ObjectId>,
    /// Native wayland `zwlr_layer_surface_v1` object id.
    pub(crate) layer_surface: Option<ObjectId>,
    /// Native wayland `zxdg_toplevel_decoration_v1` object id when available.
    pub(crate) xdg_decoration: Option<ObjectId>,
    /// Native wayland `xdg_dialog_v1` object id when available.
    pub(crate) xdg_dialog: Option<ObjectId>,
    /// Native wayland `wp_alpha_modifier_surface_v1` object id when available.
    pub(crate) alpha_modifier_surface: Option<ObjectId>,
    /// Native wayland `wp_fractional_scale_v1` object id when available.
    pub(crate) fractional_scale: Option<ObjectId>,
    /// Native wayland `wp_viewport` object id when available.
    pub(crate) viewport: Option<ObjectId>,
    /// Native wayland `zwp_locked_pointer_v1` object id when available.
    pub(crate) locked_pointer: Option<ObjectId>,
    /// Native wayland `zwp_confined_pointer_v1` object id when available.
    pub(crate) confined_pointer: Option<ObjectId>,
}

/// Resource payload for one opened Wayland window handle.
#[derive(Debug, Clone)]
pub(crate) struct WaylandWindowHostState {
    /// Stable runtime identifier.
    pub(crate) id: String,
    /// Host payload identity.
    pub(crate) host: WaylandWindowHost,
    /// Current host-visible title.
    pub(crate) title: String,
    /// Current window role.
    pub(crate) role: WindowRole,
    /// Current mode configuration.
    pub(crate) mode: WindowModeOptions,
    /// Pending mode request awaiting compositor confirmation.
    pub(crate) pending_mode: Option<WindowModeOptions>,
    /// Current display association.
    pub(crate) display: Option<resource::DisplayHandle>,
    /// Whether this window is resizable.
    pub(crate) resizable: bool,
    /// Whether this window uses host decorations.
    pub(crate) decorated: bool,
    /// Current window chrome style.
    pub(crate) chrome: WindowChromeKind,
    /// Whether this window is currently visible in task switching surfaces.
    pub(crate) taskbar_visible: bool,
    /// Whether this window requested compositor transparency.
    pub(crate) transparent: bool,
    /// Current whole-window opacity in `[0.0, 1.0]`.
    pub(crate) opacity: f64,
    /// Whether this window is currently always-on-top.
    pub(crate) always_on_top: bool,
    /// Current parent window relationship.
    pub(crate) parent: Option<resource::WindowHandle>,
    /// Current transient-owner window relationship.
    pub(crate) transient_for: Option<resource::WindowHandle>,
    /// Whether this window is currently modal.
    pub(crate) modal: bool,
    /// Whether this window is currently mouse-passthrough.
    pub(crate) mouse_passthrough: bool,
    /// Current aspect-ratio lock.
    pub(crate) aspect_ratio: Option<WindowAspectRatio>,
    /// Current visibility state.
    pub(crate) visibility: WindowVisibility,
    /// Current optional logical size constraints.
    pub(crate) constraints: Option<WindowSizeConstraints>,
    /// Current cursor visibility state.
    pub(crate) cursor_visible: bool,
    /// Current cursor interaction mode.
    pub(crate) cursor_mode: WindowCursorMode,
    /// Current cursor icon selector.
    pub(crate) cursor_icon: WindowCursorIcon,
    /// Current desktop position.
    pub(crate) position: WindowPosition,
    /// Current logical size.
    pub(crate) size_logical: WindowLogicalSize,
    /// Current physical size.
    pub(crate) size_physical: WindowPhysicalSize,
    /// Current scale factor.
    pub(crate) scale_factor_milli: u32,
    /// Current keyboard focus state.
    pub(crate) focused: bool,
    /// Current safe-area insets when available.
    pub(crate) safe_area_insets: Option<WindowSafeAreaInsets>,
    /// Current theme value.
    pub(crate) theme: WindowTheme,
    /// Whether closeRequested was already emitted for this window lifetime.
    pub(crate) close_requested_emitted: bool,
    /// Whether destroyed was already emitted for this window lifetime.
    pub(crate) destroyed_emitted: bool,
}
