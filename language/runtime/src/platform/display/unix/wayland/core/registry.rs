use wayland_client::protocol::{
    wl_compositor, wl_data_device_manager, wl_output, wl_registry, wl_seat, wl_shm,
};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::wp::alpha_modifier::v1::client::wp_alpha_modifier_v1;
use wayland_protocols::wp::color_management::v1::client::wp_color_manager_v1;
use wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_manager_v1;
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1;
use wayland_protocols::wp::pointer_constraints::zv1::client::zwp_pointer_constraints_v1;
use wayland_protocols::wp::pointer_warp::v1::client::wp_pointer_warp_v1;
use wayland_protocols::wp::presentation_time::client::wp_presentation;
use wayland_protocols::wp::relative_pointer::zv1::client::zwp_relative_pointer_manager_v1;
use wayland_protocols::wp::text_input::zv3::client::zwp_text_input_manager_v3;
use wayland_protocols::wp::viewporter::client::wp_viewporter;
use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_decoration_manager_v1;
use wayland_protocols::xdg::dialog::v1::client::xdg_wm_dialog_v1;
use wayland_protocols::xdg::shell::client::xdg_wm_base;
use wayland_protocols::xdg::toplevel_icon::v1::client::xdg_toplevel_icon_manager_v1;
use wayland_protocols_wlr::gamma_control::v1::client::zwlr_gamma_control_manager_v1;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1;

use super::{WaylandConnectionDispatchState, WaylandOutputSnapshot};

/// Bound global objects and protocol managers for one wayland connection lane.
#[derive(Debug, Default)]
pub(crate) struct WaylandGlobalState {
    /// Bound compositor global.
    pub(crate) compositor: Option<wl_compositor::WlCompositor>,
    /// Bound xdg shell global.
    pub(crate) wm_base: Option<xdg_wm_base::XdgWmBase>,
    /// Bound wl_shm global.
    pub(crate) shm: Option<wl_shm::WlShm>,
    /// Bound xdg decoration manager global.
    pub(crate) decoration_manager: Option<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
    /// Bound xdg dialog manager global.
    pub(crate) dialog_manager: Option<xdg_wm_dialog_v1::XdgWmDialogV1>,
    /// Bound xdg activation manager global.
    pub(crate) activation_manager: Option<xdg_activation_v1::XdgActivationV1>,
    /// Bound xdg toplevel icon manager global.
    pub(crate) toplevel_icon_manager:
        Option<xdg_toplevel_icon_manager_v1::XdgToplevelIconManagerV1>,
    /// Bound wlr layer-shell manager global for overlay role windows.
    pub(crate) layer_shell_manager: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    /// Bound wlr output-manager global for output mode configuration.
    pub(crate) wlr_output_manager: Option<zwlr_output_manager_v1::ZwlrOutputManagerV1>,
    /// Last output-manager serial for output configuration requests.
    pub(crate) wlr_output_manager_serial: Option<u32>,
    /// Bound alpha modifier manager for whole-surface opacity control.
    pub(crate) alpha_modifier_manager: Option<wp_alpha_modifier_v1::WpAlphaModifierV1>,
    /// Bound presentation-time manager for frame-pacing feedback.
    pub(crate) presentation: Option<wp_presentation::WpPresentation>,
    /// Presentation clock id advertised by the compositor.
    pub(crate) presentation_clock_id: Option<u32>,
    /// Bound fractional-scale manager for per-surface preferred-scale events.
    pub(crate) fractional_scale_manager:
        Option<wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1>,
    /// Bound viewporter manager for surface destination-size control.
    pub(crate) viewporter: Option<wp_viewporter::WpViewporter>,
    /// Bound color manager global for monitor color-state queries.
    pub(crate) color_manager: Option<wp_color_manager_v1::WpColorManagerV1>,
    /// Bound gamma-control manager global for monitor gamma lanes.
    pub(crate) gamma_control_manager:
        Option<zwlr_gamma_control_manager_v1::ZwlrGammaControlManagerV1>,
    /// Bound text-input manager global for seat-scoped IME sessions.
    pub(crate) text_input_manager: Option<zwp_text_input_manager_v3::ZwpTextInputManagerV3>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for WaylandConnectionDispatchState {
    /// Handle wl_registry events.
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _connection: &Connection,
        queue_handle: &QueueHandle<Self>,
    ) {
        match event {
            // bind supported globals as they appear
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => match interface.as_str() {
                "wl_compositor" => {
                    let compositor = registry.bind::<wl_compositor::WlCompositor, _, _>(
                        name,
                        version.min(4),
                        queue_handle,
                        (),
                    );
                    state.globals.compositor = Some(compositor);
                }
                "xdg_wm_base" => {
                    let wm_base = registry.bind::<xdg_wm_base::XdgWmBase, _, _>(
                        name,
                        version.min(6),
                        queue_handle,
                        (),
                    );
                    state.globals.wm_base = Some(wm_base);
                }
                "wl_shm" => {
                    let shm = registry.bind::<wl_shm::WlShm, _, _>(
                        name,
                        version.min(1),
                        queue_handle,
                        (),
                    );
                    state.globals.shm = Some(shm);
                }
                "zxdg_decoration_manager_v1" => {
                    let manager = registry
                        .bind::<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, _, _>(
                            name,
                            version.min(1),
                            queue_handle,
                            (),
                        );
                    state.globals.decoration_manager = Some(manager);
                }
                "xdg_wm_dialog_v1" => {
                    let manager = registry.bind::<xdg_wm_dialog_v1::XdgWmDialogV1, _, _>(
                        name,
                        version.min(1),
                        queue_handle,
                        (),
                    );
                    state.globals.dialog_manager = Some(manager);
                }
                "xdg_activation_v1" => {
                    let manager = registry.bind::<xdg_activation_v1::XdgActivationV1, _, _>(
                        name,
                        version.min(1),
                        queue_handle,
                        (),
                    );
                    state.globals.activation_manager = Some(manager);
                }
                "xdg_toplevel_icon_manager_v1" => {
                    let manager = registry
                        .bind::<xdg_toplevel_icon_manager_v1::XdgToplevelIconManagerV1, _, _>(
                            name,
                            version.min(1),
                            queue_handle,
                            (),
                        );
                    state.globals.toplevel_icon_manager = Some(manager);
                }
                "zwlr_layer_shell_v1" => {
                    let manager = registry.bind::<zwlr_layer_shell_v1::ZwlrLayerShellV1, _, _>(
                        name,
                        version.min(4),
                        queue_handle,
                        (),
                    );
                    state.globals.layer_shell_manager = Some(manager);
                }
                "zwlr_output_manager_v1" => {
                    let manager = registry
                        .bind::<zwlr_output_manager_v1::ZwlrOutputManagerV1, _, _>(
                            name,
                            version.min(4),
                            queue_handle,
                            (),
                        );
                    state.globals.wlr_output_manager = Some(manager);
                }
                "wp_alpha_modifier_v1" => {
                    let manager = registry.bind::<wp_alpha_modifier_v1::WpAlphaModifierV1, _, _>(
                        name,
                        version.min(1),
                        queue_handle,
                        (),
                    );
                    state.globals.alpha_modifier_manager = Some(manager);
                }
                "wp_presentation" => {
                    let presentation = registry.bind::<wp_presentation::WpPresentation, _, _>(
                        name,
                        version.min(2),
                        queue_handle,
                        (),
                    );
                    state.globals.presentation = Some(presentation);
                }
                "wp_fractional_scale_manager_v1" => {
                    let manager = registry
                        .bind::<wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1, _, _>(
                        name,
                        version.min(1),
                        queue_handle,
                        (),
                    );
                    state.globals.fractional_scale_manager = Some(manager);
                }
                "wp_viewporter" => {
                    let viewporter = registry.bind::<wp_viewporter::WpViewporter, _, _>(
                        name,
                        version.min(1),
                        queue_handle,
                        (),
                    );
                    state.globals.viewporter = Some(viewporter);
                }
                "zwp_text_input_manager_v3" => {
                    let manager = registry
                        .bind::<zwp_text_input_manager_v3::ZwpTextInputManagerV3, _, _>(
                            name,
                            version.min(1),
                            queue_handle,
                            (),
                        );
                    state.globals.text_input_manager = Some(manager.clone());

                    if let Some(seat) = state.input.seat.as_ref().cloned()
                        && state.input.text_input.is_none()
                    {
                        let text_input = manager.get_text_input(&seat, queue_handle, ());
                        state.input.text_input = Some(text_input);
                    }
                }
                "wp_color_manager_v1" => {
                    let manager = registry.bind::<wp_color_manager_v1::WpColorManagerV1, _, _>(
                        name,
                        version.min(2),
                        queue_handle,
                        (),
                    );
                    state.globals.color_manager = Some(manager.clone());

                    for (global_name, output) in state.output.outputs_by_global.iter() {
                        let color_output = manager.get_output(output, queue_handle, *global_name);
                        state
                            .output
                            .color_outputs_by_global
                            .insert(*global_name, color_output);
                    }
                }
                "wl_seat" => {
                    let seat = registry.bind::<wl_seat::WlSeat, _, _>(
                        name,
                        version.min(7),
                        queue_handle,
                        (),
                    );
                    state.input.seat = Some(seat.clone());

                    if let Some(data_device_manager) =
                        state.input.data_device_manager.as_ref().cloned()
                    {
                        let data_device =
                            data_device_manager.get_data_device(&seat, queue_handle, ());
                        state.input.data_device = Some(data_device);
                    }

                    if let Some(text_input_manager) =
                        state.globals.text_input_manager.as_ref().cloned()
                        && state.input.text_input.is_none()
                    {
                        let text_input = text_input_manager.get_text_input(&seat, queue_handle, ());
                        state.input.text_input = Some(text_input);
                    }
                }
                "wl_data_device_manager" => {
                    let manager = registry
                        .bind::<wl_data_device_manager::WlDataDeviceManager, _, _>(
                            name,
                            version.min(3),
                            queue_handle,
                            (),
                        );
                    state.input.data_device_manager = Some(manager.clone());

                    if let Some(seat) = state.input.seat.as_ref().cloned() {
                        let data_device = manager.get_data_device(&seat, queue_handle, ());
                        state.input.data_device = Some(data_device);
                    }
                }
                "wp_cursor_shape_manager_v1" => {
                    let manager = registry
                        .bind::<wp_cursor_shape_manager_v1::WpCursorShapeManagerV1, _, _>(
                            name,
                            version.min(2),
                            queue_handle,
                            (),
                        );
                    state.input.cursor_shape_manager = Some(manager.clone());

                    if let Some(pointer) = state.input.pointer.as_ref().cloned() {
                        let shape_device = manager.get_pointer(&pointer, queue_handle, ());
                        state.input.cursor_shape_device = Some(shape_device);
                    }
                }
                "zwp_pointer_constraints_v1" => {
                    let manager = registry
                        .bind::<zwp_pointer_constraints_v1::ZwpPointerConstraintsV1, _, _>(
                            name,
                            version.min(1),
                            queue_handle,
                            (),
                        );
                    state.input.pointer_constraints_manager = Some(manager);
                }
                "zwp_relative_pointer_manager_v1" => {
                    let manager = registry
                        .bind::<zwp_relative_pointer_manager_v1::ZwpRelativePointerManagerV1, _, _>(
                        name,
                        version.min(1),
                        queue_handle,
                        (),
                    );
                    state.input.relative_pointer_manager = Some(manager.clone());

                    if let Some(pointer) = state.input.pointer.as_ref().cloned() {
                        let relative_pointer =
                            manager.get_relative_pointer(&pointer, queue_handle, ());
                        state.input.relative_pointer = Some(relative_pointer);
                    }
                }
                "wp_pointer_warp_v1" => {
                    let manager = registry.bind::<wp_pointer_warp_v1::WpPointerWarpV1, _, _>(
                        name,
                        version.min(1),
                        queue_handle,
                        (),
                    );
                    state.input.pointer_warp_manager = Some(manager);
                }
                "zwlr_gamma_control_manager_v1" => {
                    let manager = registry
                        .bind::<zwlr_gamma_control_manager_v1::ZwlrGammaControlManagerV1, _, _>(
                            name,
                            version.min(1),
                            queue_handle,
                            (),
                        );
                    state.globals.gamma_control_manager = Some(manager);
                }
                "wl_output" => {
                    let output = registry.bind::<wl_output::WlOutput, _, _>(
                        name,
                        version.min(4),
                        queue_handle,
                        name,
                    );
                    state.output.outputs_by_global.insert(name, output);
                    state
                        .output
                        .output_snapshots_by_global
                        .entry(name)
                        .or_insert_with(|| WaylandOutputSnapshot::from_global_name(name));

                    if let Some(color_manager) = state.globals.color_manager.as_ref().cloned()
                        && let Some(output) = state.output.outputs_by_global.get(&name)
                    {
                        let color_output = color_manager.get_output(output, queue_handle, name);
                        state
                            .output
                            .color_outputs_by_global
                            .insert(name, color_output);
                    }

                    // wl_output globals change the published monitor topology
                    state.output.mark_monitor_topology_dirty();
                }
                _ => {}
            },
            // remove disappearing output lanes from lookup state
            wl_registry::Event::GlobalRemove { name } => {
                state.output.outputs_by_global.remove(&name);
                state.output.output_snapshots_by_global.remove(&name);

                if let Some(color_output) = state.output.color_outputs_by_global.remove(&name) {
                    color_output.destroy();
                }

                // removed outputs change the published monitor topology
                state.output.mark_monitor_topology_dirty();
            }
            _ => {}
        }
    }
}
