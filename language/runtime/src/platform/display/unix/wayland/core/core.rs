use std::sync::{Arc, Mutex, Weak};

use wayland_client::protocol::{
    wl_buffer, wl_callback, wl_compositor, wl_region, wl_shm, wl_shm_pool,
};
use wayland_client::{ConnectError, Connection, EventQueue, delegate_noop};
use wayland_protocols::wp::alpha_modifier::v1::client::{
    wp_alpha_modifier_surface_v1, wp_alpha_modifier_v1,
};
use wayland_protocols::wp::cursor_shape::v1::client::{
    wp_cursor_shape_device_v1, wp_cursor_shape_manager_v1,
};
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1;
use wayland_protocols::wp::pointer_constraints::zv1::client::zwp_pointer_constraints_v1;
use wayland_protocols::wp::pointer_warp::v1::client::wp_pointer_warp_v1;
use wayland_protocols::wp::relative_pointer::zv1::client::zwp_relative_pointer_manager_v1;
use wayland_protocols::wp::viewporter::client::{wp_viewport, wp_viewporter};
use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_decoration_manager_v1;
use wayland_protocols::xdg::dialog::v1::client::{xdg_dialog_v1, xdg_wm_dialog_v1};
use wayland_protocols::xdg::shell::client::xdg_positioner;
use wayland_protocols::xdg::toplevel_icon::v1::client::{
    xdg_toplevel_icon_manager_v1, xdg_toplevel_icon_v1,
};
use wayland_protocols_wlr::gamma_control::v1::client::zwlr_gamma_control_manager_v1;
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_head_v1;

use super::input::WaylandInputState;
use super::output::WaylandOutputState;
use super::registry::WaylandGlobalState;
use super::runtime::WaylandRuntimeState;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::display::unix::wayland::constants::*;
use crate::platform::display::{DisplayMonitorEventKindMask, WindowEventKindMask};
use crate::platform::{PlatformError, core as core_platform, resource};

/// Runtime-owned Wayland connection dispatch state.
#[derive(Debug)]
pub(crate) struct WaylandConnectionDispatchState {
    /// Weak runtime state used for event publication lookups.
    pub(crate) runtime_state: Weak<WaylandRuntimeState>,
    /// Bound global protocol managers.
    pub(crate) globals: WaylandGlobalState,
    /// Output and monitor topology state.
    pub(crate) output: WaylandOutputState,
    /// Input and data-device state.
    pub(crate) input: WaylandInputState,
}

impl WaylandConnectionDispatchState {
    /// Create one empty dispatch state.
    fn from_runtime_state(runtime_state: &Arc<WaylandRuntimeState>) -> Self {
        Self {
            runtime_state: Arc::downgrade(runtime_state),
            globals: WaylandGlobalState::default(),
            output: WaylandOutputState::default(),
            input: WaylandInputState::default(),
        }
    }

    /// Return whether xdg-decoration is available.
    pub(crate) fn supports_window_decorations(&self) -> bool {
        self.globals.decoration_manager.is_some()
    }

    /// Return whether xdg-dialog is available.
    pub(crate) fn supports_window_modal(&self) -> bool {
        self.globals.dialog_manager.is_some()
    }

    /// Return whether xdg-activation is available.
    pub(crate) fn supports_window_activation(&self) -> bool {
        self.globals.activation_manager.is_some()
    }

    /// Return whether xdg-toplevel-icon plus wl_shm are available.
    pub(crate) fn supports_window_icon(&self) -> bool {
        self.globals.toplevel_icon_manager.is_some() && self.globals.shm.is_some()
    }

    /// Return whether whole-window opacity lane is protocol-backed.
    pub(crate) fn supports_window_opacity(&self) -> bool {
        self.globals.alpha_modifier_manager.is_some()
    }

    /// Return whether wl_data_device is available for drop events.
    pub(crate) fn supports_window_drop_events(&self) -> bool {
        self.input.data_device.is_some()
    }

    /// Return whether monitor mode-set lane is protocol-backed.
    pub(crate) fn supports_monitor_mode_set(&self) -> bool {
        self.globals.wlr_output_manager.is_some()
            && self.globals.wlr_output_manager_serial.is_some()
            && !self.output.wlr_output_heads_by_id.is_empty()
    }

    /// Return whether monitor color-state query lanes are protocol-backed.
    pub(crate) fn supports_monitor_color_state(&self) -> bool {
        self.globals.color_manager.is_some() && !self.output.color_outputs_by_global.is_empty()
    }

    /// Return whether monitor gamma-control lanes are protocol-backed.
    pub(crate) fn supports_monitor_gamma_control(&self) -> bool {
        self.globals.gamma_control_manager.is_some() && !self.output.outputs_by_global.is_empty()
    }

    /// Return whether cursor icon and visibility lanes are protocol-backed.
    pub(crate) fn supports_cursor_shape(&self) -> bool {
        self.input.cursor_shape_device.is_some()
    }

    /// Return whether cursor lock lane is protocol-backed.
    pub(crate) fn supports_cursor_lock(&self) -> bool {
        self.input.pointer_constraints_manager.is_some()
            && self.input.pointer.is_some()
            && self.supports_cursor_shape()
    }

    /// Return whether cursor confine lane is protocol-backed.
    pub(crate) fn supports_cursor_confine(&self) -> bool {
        self.input.pointer_constraints_manager.is_some()
            && self.input.pointer.is_some()
            && self.supports_cursor_shape()
    }

    /// Return whether cursor warp lane is protocol-backed.
    pub(crate) fn supports_cursor_warp(&self) -> bool {
        self.input.pointer_warp_manager.is_some() && self.input.pointer.is_some()
    }

    /// Return whether native window drag interactions are protocol-backed.
    pub(crate) fn supports_window_drag_interaction(&self) -> bool {
        self.input.seat.is_some()
    }

    /// Return whether popup window role support is protocol-backed.
    pub(crate) fn supports_window_role_popup(&self) -> bool {
        self.globals.wm_base.is_some()
    }

    /// Return whether overlay window role support is protocol-backed.
    pub(crate) fn supports_window_role_overlay(&self) -> bool {
        self.globals.layer_shell_manager.is_some()
    }
}

/// Runtime-owned Wayland connection lane.
#[derive(Debug)]
pub(crate) struct WaylandConnectionState {
    /// Shared wayland connection for this runtime.
    pub(crate) connection: Connection,
    /// Event queue bound to this runtime.
    pub(crate) event_queue: Mutex<EventQueue<WaylandConnectionDispatchState>>,
    /// Dispatch payload for queue callbacks.
    pub(crate) dispatch_state: Mutex<WaylandConnectionDispatchState>,
}

impl WaylandConnectionState {
    /// Create one initialized wayland connection lane.
    pub(crate) fn from_runtime_state(
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
        if dispatch_state.globals.compositor.is_none() || dispatch_state.globals.wm_base.is_none() {
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
pub(crate) fn scale_factor_milli_from_fractional_scale(preferred_scale: u32) -> u32 {
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

/// Resolve one monitor-event kind bit mask from one optional value.
pub(crate) fn monitor_kind_mask(value: Option<DisplayMonitorEventKindMask>) -> u32 {
    value.map_or(DISPLAY_MONITOR_EVENT_KIND_MASK_ALL, |value| value.0)
}

/// Resolve one window-event kind bit mask from one optional value.
pub(crate) fn window_kind_mask(value: Option<WindowEventKindMask>) -> u64 {
    value.map_or(WINDOW_EVENT_KIND_MASK_ALL, |value| value.0)
}

/// Validate one monitor-event kind-mask payload.
pub(crate) fn validate_monitor_event_kind_mask(
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
pub(crate) fn validate_window_event_kind_mask(
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
pub(crate) fn overflow_error(operation: &'static str) -> Box<RuntimeError> {
    core_platform::io_busy(
        operation,
        "event queue overflowed while overflow policy is error",
    )
}

/// Build one ioNotFound error for one missing window handle.
pub(crate) fn window_not_found(
    operation: &'static str,
    handle: resource::WindowHandle,
) -> Box<RuntimeError> {
    core_platform::io_not_found(
        operation,
        format!("window handle {} was not found", handle.0.local_id),
    )
}

/// Build one ioNotFound error for one missing display handle.
pub(crate) fn display_not_found(
    operation: &'static str,
    handle: resource::DisplayHandle,
) -> Box<RuntimeError> {
    core_platform::io_not_found(
        operation,
        format!("display handle {} was not found", handle.0.local_id),
    )
}

/// Build one io error for wayland operations.
pub(crate) fn io_error(operation: &'static str, message: impl Into<String>) -> Box<RuntimeError> {
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
