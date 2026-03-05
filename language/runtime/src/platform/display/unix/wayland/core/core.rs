use std::collections::{BTreeMap, HashMap};
use std::io::ErrorKind;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};

use wayland_client::backend::WaylandError;
use wayland_client::protocol::{
    wl_buffer, wl_callback, wl_compositor, wl_data_device, wl_data_device_manager, wl_output,
    wl_pointer, wl_region, wl_seat, wl_shm, wl_shm_pool, wl_surface,
};
use wayland_client::{ConnectError, Connection, EventQueue, Proxy, WEnum, delegate_noop};
use wayland_protocols::wp::alpha_modifier::v1::client::{
    wp_alpha_modifier_surface_v1, wp_alpha_modifier_v1,
};
use wayland_protocols::wp::color_management::v1::client::{
    wp_color_management_output_v1, wp_color_manager_v1,
};
use wayland_protocols::wp::cursor_shape::v1::client::{
    wp_cursor_shape_device_v1, wp_cursor_shape_manager_v1,
};
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1;
use wayland_protocols::wp::pointer_constraints::zv1::client::zwp_pointer_constraints_v1;
use wayland_protocols::wp::pointer_warp::v1::client::wp_pointer_warp_v1;
use wayland_protocols::wp::presentation_time::client::wp_presentation;
use wayland_protocols::wp::relative_pointer::zv1::client::{
    zwp_relative_pointer_manager_v1, zwp_relative_pointer_v1,
};
use wayland_protocols::wp::viewporter::client::{wp_viewport, wp_viewporter};
use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_decoration_manager_v1;
use wayland_protocols::xdg::dialog::v1::client::{xdg_dialog_v1, xdg_wm_dialog_v1};
use wayland_protocols::xdg::shell::client::{
    xdg_positioner, xdg_surface, xdg_toplevel, xdg_wm_base,
};
use wayland_protocols::xdg::toplevel_icon::v1::client::{
    xdg_toplevel_icon_manager_v1, xdg_toplevel_icon_v1,
};
use wayland_protocols_wlr::gamma_control::v1::client::zwlr_gamma_control_manager_v1;
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_configuration_head_v1, zwlr_output_manager_v1,
};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::display::{
    DisplayBackend, DisplayEventOverflowPolicy, DisplayMode, DisplayMonitorEventKindMask,
    DisplayOrientation, WindowEventKindMask, WindowLogicalSize, WindowPhysicalSize, WindowPosition,
    WindowVisibility,
};
use crate::platform::{PlatformError, core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::super::constants::*;
use super::super::event::{self, MonitorEventBinding, WindowEventBinding};
use super::super::model::{MonitorSnapshot, WaylandWindowBinding};
use super::super::window;
use super::cursor::{
    apply_pointer_cursor_state, apply_window_cursor_policy, clear_pointer_focus_for_surface,
};

/// Mutable payload for one advertised data offer.
#[derive(Debug, Clone, Default)]
pub(super) struct WaylandDataOfferState {
    /// Offered mime-type set.
    pub(super) mime_types: Vec<String>,
}

/// Mutable payload for one active drop session.
#[derive(Debug, Clone, Default)]
pub(super) struct WaylandDropSessionState {
    /// Offer object id for this active session.
    pub(super) offer: Option<wayland_client::backend::ObjectId>,
    /// Target surface object id for this active session.
    pub(super) surface: Option<wayland_client::backend::ObjectId>,
    /// Last known drop position in window coordinates.
    pub(super) position: Option<WindowPosition>,
    /// Last accepted mime type for this session.
    pub(super) accepted_mime_type: Option<String>,
    /// Whether one drop-started event was emitted for this session.
    pub(super) started: bool,
    /// Whether one compositor drop event is pending payload transfer.
    pub(super) drop_pending: bool,
    /// Last hovered file path used for hover-leave payloads.
    pub(super) last_hovered_path: Option<String>,
}

/// Runtime collector snapshot for one wlr-output-management head object.
#[derive(Debug, Clone, Default)]
pub(super) struct WaylandWlrOutputHeadState {
    /// Logical output name advertised by the compositor.
    pub(super) name: Option<String>,
    /// Enumerated mode object ids for this output head.
    pub(super) mode_ids: Vec<wayland_client::backend::ObjectId>,
    /// Optional adaptive-sync support state for this output head.
    pub(super) adaptive_sync: Option<WaylandWlrAdaptiveSyncState>,
}

/// Runtime collector snapshot for one wlr-output-management mode object.
#[derive(Debug, Clone, Default)]
pub(super) struct WaylandWlrOutputModeState {
    /// Mode width in physical pixels.
    pub(super) width: Option<u32>,
    /// Mode height in physical pixels.
    pub(super) height: Option<u32>,
    /// Mode refresh rate in milli-hertz.
    pub(super) refresh_milli_hz: Option<u32>,
    /// Whether this mode is compositor preferred.
    pub(super) is_preferred: bool,
}

/// Runtime adaptive-sync support state from wlr output management.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WaylandWlrAdaptiveSyncState {
    /// Adaptive sync is currently disabled for this output head.
    Disabled,
    /// Adaptive sync is currently enabled for this output head.
    Enabled,
}

/// Result state for one pending wlr output configuration request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WaylandOutputConfigurationOutcome {
    /// Compositor accepted and applied this configuration.
    Succeeded,
    /// Compositor rejected this configuration.
    Failed,
    /// Compositor cancelled this configuration because state changed.
    Cancelled,
}

/// Shared completion payload for one pending output configuration request.
#[derive(Debug, Default)]
pub(super) struct WaylandOutputConfigurationState {
    /// Final result state observed from configuration events.
    pub(super) outcome: Mutex<Option<WaylandOutputConfigurationOutcome>>,
}

/// Mutable query payload for one output color-description request.
#[derive(Debug, Clone, Default)]
pub(super) struct WaylandColorDescriptionQueryState {
    /// Whether the image-description object reached ready state.
    pub(super) ready: bool,
    /// Optional failure details from image-description creation.
    pub(super) failed_message: Option<String>,
    /// Optional image-description failure cause value.
    pub(super) failed_cause: Option<u32>,
    /// Whether image-description information delivery completed.
    pub(super) info_done: bool,
    /// Optional named primaries enum value.
    pub(super) primaries_named: Option<u32>,
    /// Optional named transfer-function enum value.
    pub(super) transfer_function_named: Option<u32>,
    /// Optional minimum luminance value multiplied by 10000.
    pub(super) minimum_luminance: Option<u32>,
    /// Optional maximum luminance value.
    pub(super) maximum_luminance: Option<u32>,
    /// Optional reference white luminance value.
    pub(super) reference_luminance: Option<u32>,
    /// Optional target maximum content light-level value.
    pub(super) target_max_cll: Option<u32>,
    /// Optional target maximum frame-average light-level value.
    pub(super) target_max_fall: Option<u32>,
}

/// Mutable query payload for one gamma-control request.
#[derive(Debug, Clone, Default)]
pub(super) struct WaylandGammaControlQueryState {
    /// Optional reported gamma table size.
    pub(super) gamma_size: Option<u32>,
    /// Whether the gamma-control object reported failure.
    pub(super) failed: bool,
}

/// Cached gamma-ramp payload for one display.
#[derive(Debug, Clone, Default)]
pub(super) struct WaylandGammaRampSnapshot {
    /// Cached red channel values.
    pub(super) red: Vec<u16>,
    /// Cached green channel values.
    pub(super) green: Vec<u16>,
    /// Cached blue channel values.
    pub(super) blue: Vec<u16>,
}

/// Runtime collector snapshot for one wl_output global.
#[derive(Debug, Clone)]
pub(super) struct WaylandOutputSnapshot {
    /// Stable wl_registry global name.
    pub(super) global_name: u32,
    /// Logical output name from wl_output.name when present.
    pub(super) logical_name: Option<String>,
    /// Human-readable description from wl_output.description when present.
    pub(super) description: Option<String>,
    /// Make token reported by geometry events.
    pub(super) make: Option<String>,
    /// Model token reported by geometry events.
    pub(super) model: Option<String>,
    /// Desktop-space x coordinate.
    pub(super) x: i32,
    /// Desktop-space y coordinate.
    pub(super) y: i32,
    /// Physical width in millimeters.
    pub(super) width_mm: u32,
    /// Physical height in millimeters.
    pub(super) height_mm: u32,
    /// Output scale factor.
    pub(super) scale_factor: u32,
    /// Output transform orientation.
    pub(super) orientation: DisplayOrientation,
    /// Enumerated mode list.
    pub(super) modes: Vec<DisplayMode>,
    /// Current mode when reported.
    pub(super) current_mode: Option<DisplayMode>,
    /// Desktop-preferred mode when reported.
    pub(super) desktop_mode: Option<DisplayMode>,
}

impl WaylandOutputSnapshot {
    /// Create one empty output snapshot for one wl_registry global.
    fn from_global_name(global_name: u32) -> Self {
        Self {
            global_name,
            logical_name: None,
            description: None,
            make: None,
            model: None,
            x: 0,
            y: 0,
            width_mm: 0,
            height_mm: 0,
            scale_factor: 1,
            orientation: DisplayOrientation::Landscape,
            modes: Vec::new(),
            current_mode: None,
            desktop_mode: None,
        }
    }
}

/// Event dispatch token for one wayland window object.
#[derive(Debug, Clone)]
pub(super) struct WaylandWindowDispatchToken {
    /// Stable runtime window identifier.
    pub(super) window_id: String,
    /// Weak handle to the runtime window binding payload.
    pub(super) binding: Weak<Mutex<WaylandWindowBinding>>,
}

/// Shared state for one pending xdg activation token request.
#[derive(Debug)]
pub(super) struct WaylandActivationTokenState {
    /// Resolved token string when compositor responds.
    pub(super) token: Mutex<Option<String>>,
}

/// Runtime-owned wayland connection dispatch state.
#[derive(Debug)]
pub(super) struct WaylandConnectionDispatchState {
    /// Weak runtime state used for event publication lookups.
    pub(super) runtime_state: Weak<WaylandRuntimeState>,
    /// Bound compositor global.
    pub(super) compositor: Option<wl_compositor::WlCompositor>,
    /// Bound xdg shell global.
    pub(super) wm_base: Option<xdg_wm_base::XdgWmBase>,
    /// Bound wl_shm global.
    pub(super) shm: Option<wl_shm::WlShm>,
    /// Bound xdg decoration manager global.
    pub(super) decoration_manager: Option<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
    /// Bound xdg dialog manager global.
    pub(super) dialog_manager: Option<xdg_wm_dialog_v1::XdgWmDialogV1>,
    /// Bound xdg activation manager global.
    pub(super) activation_manager: Option<xdg_activation_v1::XdgActivationV1>,
    /// Bound xdg toplevel icon manager global.
    pub(super) toplevel_icon_manager:
        Option<xdg_toplevel_icon_manager_v1::XdgToplevelIconManagerV1>,
    /// Bound wlr layer-shell manager global for overlay role windows.
    pub(super) layer_shell_manager: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    /// Bound wlr output-manager global for output mode configuration.
    pub(super) wlr_output_manager: Option<zwlr_output_manager_v1::ZwlrOutputManagerV1>,
    /// Last output-manager serial for output configuration requests.
    pub(super) wlr_output_manager_serial: Option<u32>,
    /// Output head snapshots keyed by head object id.
    pub(super) wlr_output_heads_by_id:
        HashMap<wayland_client::backend::ObjectId, WaylandWlrOutputHeadState>,
    /// Output mode snapshots keyed by mode object id.
    pub(super) wlr_output_modes_by_id:
        HashMap<wayland_client::backend::ObjectId, WaylandWlrOutputModeState>,
    /// Bound alpha modifier manager for whole-surface opacity control.
    pub(super) alpha_modifier_manager: Option<wp_alpha_modifier_v1::WpAlphaModifierV1>,
    /// Bound presentation-time manager for frame-pacing feedback.
    pub(super) presentation: Option<wp_presentation::WpPresentation>,
    /// Presentation clock id advertised by the compositor.
    pub(super) presentation_clock_id: Option<u32>,
    /// Bound fractional-scale manager for per-surface preferred-scale events.
    pub(super) fractional_scale_manager:
        Option<wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1>,
    /// Bound viewporter manager for surface destination-size control.
    pub(super) viewporter: Option<wp_viewporter::WpViewporter>,
    /// Bound color manager global for monitor color-state queries.
    pub(super) color_manager: Option<wp_color_manager_v1::WpColorManagerV1>,
    /// Bound color-management output objects keyed by wl_output global name.
    pub(super) color_outputs_by_global:
        HashMap<u32, wp_color_management_output_v1::WpColorManagementOutputV1>,
    /// Bound gamma-control manager global for monitor gamma lanes.
    pub(super) gamma_control_manager:
        Option<zwlr_gamma_control_manager_v1::ZwlrGammaControlManagerV1>,
    /// Bound wl seat global used for pointer serial lanes.
    pub(super) seat: Option<wl_seat::WlSeat>,
    /// Bound wl_data_device_manager global used for drop events.
    pub(super) data_device_manager: Option<wl_data_device_manager::WlDataDeviceManager>,
    /// Bound wl_data_device object for the active seat.
    pub(super) data_device: Option<wl_data_device::WlDataDevice>,
    /// Bound wl_pointer object for the active seat when available.
    pub(super) pointer: Option<wl_pointer::WlPointer>,
    /// Bound cursor-shape manager for compositor-provided cursor themes.
    pub(super) cursor_shape_manager: Option<wp_cursor_shape_manager_v1::WpCursorShapeManagerV1>,
    /// Bound cursor-shape device for the active pointer.
    pub(super) cursor_shape_device: Option<wp_cursor_shape_device_v1::WpCursorShapeDeviceV1>,
    /// Bound pointer-constraints manager for lock and confine lanes.
    pub(super) pointer_constraints_manager:
        Option<zwp_pointer_constraints_v1::ZwpPointerConstraintsV1>,
    /// Bound relative-pointer manager for locked-pointer motion lanes.
    pub(super) relative_pointer_manager:
        Option<zwp_relative_pointer_manager_v1::ZwpRelativePointerManagerV1>,
    /// Bound pointer-warp manager for surface-local cursor warp requests.
    pub(super) pointer_warp_manager: Option<wp_pointer_warp_v1::WpPointerWarpV1>,
    /// Bound relative-pointer object for the active pointer when available.
    pub(super) relative_pointer: Option<zwp_relative_pointer_v1::ZwpRelativePointerV1>,
    /// Last entered wl_surface object id for pointer focus.
    pub(super) pointer_focus_surface: Option<wayland_client::backend::ObjectId>,
    /// Last pointer-enter serial used for cursor shape updates.
    pub(super) last_pointer_enter_serial: Option<u32>,
    /// Last known pointer-button serial for interactive move and resize lanes.
    pub(super) last_pointer_button_serial: Option<u32>,
    /// Bound output globals keyed by registry name.
    pub(super) outputs_by_global: HashMap<u32, wl_output::WlOutput>,
    /// Output snapshots keyed by wl_registry global name.
    pub(super) output_snapshots_by_global: BTreeMap<u32, WaylandOutputSnapshot>,
    /// Data-offer state keyed by offer object id.
    pub(super) data_offer_state_by_id:
        HashMap<wayland_client::backend::ObjectId, WaylandDataOfferState>,
    /// Active drop-session state.
    pub(super) drop_session_state: WaylandDropSessionState,
}

impl WaylandConnectionDispatchState {
    /// Create one empty dispatch state.
    fn from_runtime_state(runtime_state: &Arc<WaylandRuntimeState>) -> Self {
        Self {
            runtime_state: Arc::downgrade(runtime_state),
            compositor: None,
            wm_base: None,
            shm: None,
            decoration_manager: None,
            dialog_manager: None,
            activation_manager: None,
            toplevel_icon_manager: None,
            layer_shell_manager: None,
            wlr_output_manager: None,
            wlr_output_manager_serial: None,
            wlr_output_heads_by_id: HashMap::new(),
            wlr_output_modes_by_id: HashMap::new(),
            alpha_modifier_manager: None,
            presentation: None,
            presentation_clock_id: None,
            fractional_scale_manager: None,
            viewporter: None,
            color_manager: None,
            color_outputs_by_global: HashMap::new(),
            gamma_control_manager: None,
            seat: None,
            data_device_manager: None,
            data_device: None,
            pointer: None,
            cursor_shape_manager: None,
            cursor_shape_device: None,
            pointer_constraints_manager: None,
            relative_pointer_manager: None,
            pointer_warp_manager: None,
            relative_pointer: None,
            pointer_focus_surface: None,
            last_pointer_enter_serial: None,
            last_pointer_button_serial: None,
            outputs_by_global: HashMap::new(),
            output_snapshots_by_global: BTreeMap::new(),
            data_offer_state_by_id: HashMap::new(),
            drop_session_state: WaylandDropSessionState::default(),
        }
    }

    /// Return whether xdg-decoration is available.
    pub(super) fn supports_window_decorations(&self) -> bool {
        self.decoration_manager.is_some()
    }

    /// Return whether xdg-dialog is available.
    pub(super) fn supports_window_modal(&self) -> bool {
        self.dialog_manager.is_some()
    }

    /// Return whether xdg-activation is available.
    pub(super) fn supports_window_activation(&self) -> bool {
        self.activation_manager.is_some()
    }

    /// Return whether xdg-toplevel-icon plus wl_shm are available.
    pub(super) fn supports_window_icon(&self) -> bool {
        self.toplevel_icon_manager.is_some() && self.shm.is_some()
    }

    /// Return whether whole-window opacity lane is protocol-backed.
    pub(super) fn supports_window_opacity(&self) -> bool {
        self.alpha_modifier_manager.is_some()
    }

    /// Return whether wl_data_device is available for drop events.
    pub(super) fn supports_window_drop_events(&self) -> bool {
        self.data_device.is_some()
    }

    /// Return whether monitor mode-set lane is protocol-backed.
    pub(super) fn supports_monitor_mode_set(&self) -> bool {
        self.wlr_output_manager.is_some()
            && self.wlr_output_manager_serial.is_some()
            && !self.wlr_output_heads_by_id.is_empty()
    }

    /// Return whether monitor color-state query lanes are protocol-backed.
    pub(super) fn supports_monitor_color_state(&self) -> bool {
        self.color_manager.is_some() && !self.color_outputs_by_global.is_empty()
    }

    /// Return whether monitor gamma-control lanes are protocol-backed.
    pub(super) fn supports_monitor_gamma_control(&self) -> bool {
        self.gamma_control_manager.is_some() && !self.outputs_by_global.is_empty()
    }

    /// Return whether cursor icon and visibility lanes are protocol-backed.
    pub(super) fn supports_cursor_shape(&self) -> bool {
        self.cursor_shape_device.is_some()
    }

    /// Return whether cursor lock lane is protocol-backed.
    pub(super) fn supports_cursor_lock(&self) -> bool {
        self.pointer_constraints_manager.is_some()
            && self.pointer.is_some()
            && self.supports_cursor_shape()
    }

    /// Return whether cursor confine lane is protocol-backed.
    pub(super) fn supports_cursor_confine(&self) -> bool {
        self.pointer_constraints_manager.is_some()
            && self.pointer.is_some()
            && self.supports_cursor_shape()
    }

    /// Return whether cursor warp lane is protocol-backed.
    pub(super) fn supports_cursor_warp(&self) -> bool {
        self.pointer_warp_manager.is_some() && self.pointer.is_some()
    }

    /// Return whether native window drag interactions are protocol-backed.
    pub(super) fn supports_window_drag_interaction(&self) -> bool {
        self.seat.is_some()
    }

    /// Return whether popup window role support is protocol-backed.
    pub(super) fn supports_window_role_popup(&self) -> bool {
        self.wm_base.is_some()
    }

    /// Return whether overlay window role support is protocol-backed.
    pub(super) fn supports_window_role_overlay(&self) -> bool {
        self.layer_shell_manager.is_some()
    }
}

/// Runtime-owned wayland connection lane.
#[derive(Debug)]
pub(super) struct WaylandConnectionState {
    /// Shared wayland connection for this runtime.
    pub(super) connection: Connection,
    /// Event queue bound to this runtime.
    event_queue: Mutex<EventQueue<WaylandConnectionDispatchState>>,
    /// Dispatch payload for queue callbacks.
    dispatch_state: Mutex<WaylandConnectionDispatchState>,
}

impl WaylandConnectionState {
    /// Create one initialized wayland connection lane.
    fn from_runtime_state(
        runtime_state: &Arc<WaylandRuntimeState>,
        operation: &'static str,
    ) -> RuntimeResult<Self> {
        // establish one wayland connection from process environment
        let connection = Connection::connect_to_env().map_err(|error| match error {
            // no compositor endpoint: report notSupported for this backend
            ConnectError::NoCompositor => core_platform::not_supported(operation),
            // otherwise surface one explicit io error payload
            _ => io_error(operation, format!("wayland connect failed: {error}")),
        })?;

        // create one queue and seed registry global discovery
        let mut event_queue = connection.new_event_queue();
        let queue_handle = event_queue.handle();
        let _registry = connection.display().get_registry(&queue_handle, ());

        // dispatch initial globals and resolve compositor + xdg shell lanes
        let mut dispatch_state = WaylandConnectionDispatchState::from_runtime_state(runtime_state);
        event_queue
            .roundtrip(&mut dispatch_state)
            .map_err(|error| {
                io_error(
                    operation,
                    format!("wayland registry roundtrip failed: {error}"),
                )
            })?;
        event_queue
            .roundtrip(&mut dispatch_state)
            .map_err(|error| {
                io_error(
                    operation,
                    format!("wayland global roundtrip failed: {error}"),
                )
            })?;

        // reject when required globals are not available
        if dispatch_state.compositor.is_none() || dispatch_state.wm_base.is_none() {
            return Err(core_platform::not_supported(operation));
        }

        Ok(Self {
            connection,
            event_queue: Mutex::new(event_queue),
            dispatch_state: Mutex::new(dispatch_state),
        })
    }
}

/// Convert one fractional-scale preferred value into one milli-scale factor.
pub(super) fn scale_factor_milli_from_fractional_scale(preferred_scale: u32) -> u32 {
    /// Fractional-scale protocol denominator units.
    const FRACTIONAL_SCALE_DENOMINATOR: u64 = 120;
    /// Milli-scale conversion numerator.
    const MILLI_SCALE_NUMERATOR: u64 = 1000;

    // normalize zero inputs to one unit scale
    let preferred_scale = preferred_scale.max(1) as u64;

    // convert and round to nearest milli-scale
    let scaled = preferred_scale
        .saturating_mul(MILLI_SCALE_NUMERATOR)
        .saturating_add(FRACTIONAL_SCALE_DENOMINATOR / 2)
        / FRACTIONAL_SCALE_DENOMINATOR;

    scaled.clamp(1, u32::MAX as u64) as u32
}

delegate_noop!(WaylandConnectionDispatchState: wl_shm::WlShm);
delegate_noop!(WaylandConnectionDispatchState: wl_shm_pool::WlShmPool);
delegate_noop!(WaylandConnectionDispatchState: wl_buffer::WlBuffer);
delegate_noop!(WaylandConnectionDispatchState: wl_callback::WlCallback);
delegate_noop!(WaylandConnectionDispatchState: wl_compositor::WlCompositor);
delegate_noop!(WaylandConnectionDispatchState: wl_region::WlRegion);
delegate_noop!(
    WaylandConnectionDispatchState: wp_cursor_shape_manager_v1::WpCursorShapeManagerV1
);
delegate_noop!(WaylandConnectionDispatchState: wp_cursor_shape_device_v1::WpCursorShapeDeviceV1);
delegate_noop!(WaylandConnectionDispatchState: wp_alpha_modifier_v1::WpAlphaModifierV1);
delegate_noop!(
    WaylandConnectionDispatchState: wp_alpha_modifier_surface_v1::WpAlphaModifierSurfaceV1
);
delegate_noop!(
    WaylandConnectionDispatchState: wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1
);
delegate_noop!(WaylandConnectionDispatchState: wp_viewporter::WpViewporter);
delegate_noop!(WaylandConnectionDispatchState: wp_viewport::WpViewport);
delegate_noop!(
    WaylandConnectionDispatchState: zwp_pointer_constraints_v1::ZwpPointerConstraintsV1
);
delegate_noop!(WaylandConnectionDispatchState: wp_pointer_warp_v1::WpPointerWarpV1);
delegate_noop!(
    WaylandConnectionDispatchState: zwp_relative_pointer_manager_v1::ZwpRelativePointerManagerV1
);
delegate_noop!(
    WaylandConnectionDispatchState: zxdg_decoration_manager_v1::ZxdgDecorationManagerV1
);
delegate_noop!(WaylandConnectionDispatchState: xdg_wm_dialog_v1::XdgWmDialogV1);
delegate_noop!(WaylandConnectionDispatchState: xdg_dialog_v1::XdgDialogV1);
delegate_noop!(WaylandConnectionDispatchState: xdg_activation_v1::XdgActivationV1);
delegate_noop!(WaylandConnectionDispatchState: xdg_positioner::XdgPositioner);
delegate_noop!(
    WaylandConnectionDispatchState: xdg_toplevel_icon_manager_v1::XdgToplevelIconManagerV1
);
delegate_noop!(WaylandConnectionDispatchState: xdg_toplevel_icon_v1::XdgToplevelIconV1);
delegate_noop!(WaylandConnectionDispatchState: zwlr_layer_shell_v1::ZwlrLayerShellV1);
delegate_noop!(
    WaylandConnectionDispatchState: zwlr_gamma_control_manager_v1::ZwlrGammaControlManagerV1
);
delegate_noop!(WaylandConnectionDispatchState: zwlr_layer_surface_v1::ZwlrLayerSurfaceV1);
delegate_noop!(
    WaylandConnectionDispatchState: zwlr_output_configuration_head_v1::ZwlrOutputConfigurationHeadV1
);

/// Allocate one stable host window identifier for this runtime.
pub(super) fn next_window_host_id(context: &BindingCallContext) -> u64 {
    let runtime_state = runtime_state(context);
    runtime_state
        .next_window_host_id
        .fetch_add(1, Ordering::Relaxed)
}

/// Resolve one queue capacity from open options and runtime defaults.
pub(super) fn resolved_queue_capacity(context: &BindingCallContext, requested: u32) -> usize {
    // use runtime default when request value is zero
    if requested == 0 {
        let configured = context
            .runtime()
            .module_options
            .display
            .default_event_queue_capacity;
        return core_platform::option_u64_to_usize_or_min(
            configured,
            DEFAULT_EVENT_QUEUE_CAPACITY,
            1,
        );
    }

    requested as usize
}

/// Resolve one wait-slice interval for blocking window-event reads.
pub(super) fn window_event_wait_slice_ns(context: &BindingCallContext) -> u64 {
    let configured = context
        .runtime()
        .module_options
        .display
        .window_event_wait_slice_ns;
    core_platform::option_u64_or_min(configured, DEFAULT_EVENT_WAIT_SLICE_NS, 1)
}

/// Resolve one monitor-event kind bit mask from one optional value.
pub(super) fn monitor_kind_mask(value: Option<DisplayMonitorEventKindMask>) -> u32 {
    value.map_or(DISPLAY_MONITOR_EVENT_KIND_MASK_ALL, |value| value.0)
}

/// Resolve one window-event kind bit mask from one optional value.
pub(super) fn window_kind_mask(value: Option<WindowEventKindMask>) -> u64 {
    value.map_or(WINDOW_EVENT_KIND_MASK_ALL, |value| value.0)
}

/// Validate one monitor-event kind-mask payload.
pub(super) fn validate_monitor_event_kind_mask(
    value: u32,
    field: &'static str,
) -> RuntimeResult<()> {
    let unsupported_bits = value & !DISPLAY_MONITOR_EVENT_KIND_MASK_ALL;
    if unsupported_bits == 0 {
        return Ok(());
    }

    Err(core_platform::invalid_argument(
        field,
        format!("unsupported monitor event kind bits: 0x{unsupported_bits:x}"),
    ))
}

/// Validate one window-event kind-mask payload.
pub(super) fn validate_window_event_kind_mask(
    value: u64,
    field: &'static str,
) -> RuntimeResult<()> {
    let unsupported_bits = value & !WINDOW_EVENT_KIND_MASK_ALL;
    if unsupported_bits == 0 {
        return Ok(());
    }

    Err(core_platform::invalid_argument(
        field,
        format!("unsupported window event kind bits: 0x{unsupported_bits:x}"),
    ))
}

/// Build one overflow error for display event queues.
pub(super) fn overflow_error(operation: &'static str) -> Box<RuntimeError> {
    core_platform::io_busy(
        operation,
        "event queue overflowed while overflow policy is error",
    )
}

/// Build one ioNotFound error for one missing window handle.
pub(super) fn window_not_found(
    operation: &'static str,
    handle: resource::WindowHandle,
) -> Box<RuntimeError> {
    core_platform::io_not_found(
        operation,
        format!("window handle {} was not found", handle.0.0),
    )
}

/// Build one ioNotFound error for one missing display handle.
pub(super) fn display_not_found(
    operation: &'static str,
    handle: resource::DisplayHandle,
) -> Box<RuntimeError> {
    core_platform::io_not_found(
        operation,
        format!("display handle {} was not found", handle.0.0),
    )
}

/// Build one io error for wayland operations.
pub(super) fn io_error(operation: &'static str, message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Push one event into one queue under overflow policy.
pub(super) fn push_with_overflow<T>(
    queue: &mut std::collections::VecDeque<T>,
    queue_capacity: usize,
    overflow_policy: DisplayEventOverflowPolicy,
    overflow_error_pending: &mut bool,
    dropped_count: &mut u64,
    value: T,
) {
    // append directly while capacity remains
    if queue.len() < queue_capacity {
        queue.push_back(value);
        return;
    }

    // apply policy for full queue state
    match overflow_policy {
        DisplayEventOverflowPolicy::DropNewest => {
            *dropped_count = dropped_count.saturating_add(1);
        }
        DisplayEventOverflowPolicy::DropOldest => {
            let _ = queue.pop_front();
            *dropped_count = dropped_count.saturating_add(1);
            queue.push_back(value);
        }
        DisplayEventOverflowPolicy::Error => {
            *overflow_error_pending = true;
            *dropped_count = dropped_count.saturating_add(1);
        }
    }
}

/// Drain stale weak entries and skip one identity from one weak registry.
pub(super) fn retain_live_without_identity<T>(registry: &mut Vec<Weak<T>>, identity: usize) {
    registry.retain(|weak| {
        let Some(strong) = weak.upgrade() else {
            return false;
        };

        Arc::as_ptr(&strong) as usize != identity
    });
}
