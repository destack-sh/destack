use std::thread::ThreadId;

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
pub(super) struct DisplayDescriptorSnapshot {
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
    /// Built-in panel support status.
    pub(super) builtin_panel: DisplaySupportStatus,
    /// Variable-refresh support status.
    pub(super) variable_refresh_support: DisplaySupportStatus,
    /// HDR support status.
    pub(super) hdr_support: DisplaySupportStatus,
}

/// Snapshot payload for one wayland monitor endpoint.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct MonitorSnapshot {
    /// Descriptor payload for this monitor.
    pub(super) descriptor: DisplayDescriptorSnapshot,
    /// Current active mode payload.
    pub(super) current_mode: DisplayMode,
    /// Desktop mode payload.
    pub(super) desktop_mode: DisplayMode,
    /// Enumerated host mode set.
    pub(super) modes: Vec<DisplayMode>,
}

/// Resource payload for one opened display handle.
#[derive(Debug, Clone)]
pub(super) struct WaylandDisplayBinding {
    /// Stable monitor identifier.
    pub(super) id: String,
}

/// Runtime payload for one wayland host window lane.
#[derive(Debug, Clone)]
pub(super) struct WaylandWindowHost {
    /// Stable host-surface identity.
    pub(super) id: u64,
    /// Native wayland `wl_surface` object id.
    pub(super) surface: ObjectId,
    /// Native wayland `xdg_surface` object id.
    pub(super) xdg_surface: Option<ObjectId>,
    /// Native wayland `xdg_toplevel` object id.
    pub(super) xdg_toplevel: Option<ObjectId>,
    /// Native wayland `xdg_popup` object id.
    pub(super) xdg_popup: Option<ObjectId>,
    /// Native wayland `zwlr_layer_surface_v1` object id.
    pub(super) layer_surface: Option<ObjectId>,
    /// Native wayland `zxdg_toplevel_decoration_v1` object id when available.
    pub(super) xdg_decoration: Option<ObjectId>,
    /// Native wayland `xdg_dialog_v1` object id when available.
    pub(super) xdg_dialog: Option<ObjectId>,
    /// Native wayland `wp_alpha_modifier_surface_v1` object id when available.
    pub(super) alpha_modifier_surface: Option<ObjectId>,
    /// Native wayland `wp_fractional_scale_v1` object id when available.
    pub(super) fractional_scale: Option<ObjectId>,
    /// Native wayland `wp_viewport` object id when available.
    pub(super) viewport: Option<ObjectId>,
    /// Native wayland `zwp_locked_pointer_v1` object id when available.
    pub(super) locked_pointer: Option<ObjectId>,
    /// Native wayland `zwp_confined_pointer_v1` object id when available.
    pub(super) confined_pointer: Option<ObjectId>,
}

/// Resource payload for one opened wayland window handle.
#[derive(Debug, Clone)]
pub(super) struct WaylandWindowBinding {
    /// Stable runtime identifier.
    pub(super) id: String,
    /// Host payload identity.
    pub(super) host: WaylandWindowHost,
    /// Owner thread identifier that created this window.
    pub(super) owner_thread_id: ThreadId,
    /// Current host-visible title.
    pub(super) title: String,
    /// Current window role.
    pub(super) role: WindowRole,
    /// Current mode configuration.
    pub(super) mode: WindowModeOptions,
    /// Current display association.
    pub(super) display: Option<resource::DisplayHandle>,
    /// Whether this window is resizable.
    pub(super) resizable: bool,
    /// Whether this window uses host decorations.
    pub(super) decorated: bool,
    /// Current window chrome style.
    pub(super) chrome: WindowChromeKind,
    /// Whether this window is currently visible in task switching surfaces.
    pub(super) taskbar_visible: bool,
    /// Whether this window requested compositor transparency.
    pub(super) transparent: bool,
    /// Current whole-window opacity in `[0.0, 1.0]`.
    pub(super) opacity: f64,
    /// Whether this window is currently always-on-top.
    pub(super) always_on_top: bool,
    /// Current parent window relationship.
    pub(super) parent: Option<resource::WindowHandle>,
    /// Current transient-owner window relationship.
    pub(super) transient_for: Option<resource::WindowHandle>,
    /// Whether this window is currently modal.
    pub(super) modal: bool,
    /// Whether this window is currently mouse-passthrough.
    pub(super) mouse_passthrough: bool,
    /// Current aspect-ratio lock.
    pub(super) aspect_ratio: Option<WindowAspectRatio>,
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
    /// Current safe-area insets when available.
    pub(super) safe_area_insets: Option<WindowSafeAreaInsets>,
    /// Current theme value.
    pub(super) theme: WindowTheme,
    /// Whether closeRequested was already emitted for this window lifetime.
    pub(super) close_requested_emitted: bool,
    /// Whether destroyed was already emitted for this window lifetime.
    pub(super) destroyed_emitted: bool,
}
