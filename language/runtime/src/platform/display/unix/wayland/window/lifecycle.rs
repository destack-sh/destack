use wayland_client::Proxy;
use wayland_protocols::wp::alpha_modifier::v1::client::wp_alpha_modifier_surface_v1;
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1;
use wayland_protocols::wp::pointer_constraints::zv1::client::{
    zwp_confined_pointer_v1, zwp_locked_pointer_v1,
};
use wayland_protocols::wp::viewporter::client::wp_viewport;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1;
use wayland_protocols::xdg::dialog::v1::client::xdg_dialog_v1;
use wayland_protocols::xdg::shell::client::xdg_popup;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1;

use crate::diagnostic::RuntimeResult;
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::{drop, resolve_window_host_state};
use crate::platform::display::unix::wayland::{core as wayland_core, event};

/// Close one window.
pub(crate) unsafe fn window_close(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve target window host state and snapshot host object ids
    let host_state =
        resolve_window_host_state(context, window_handle, "destack.display.window.close")?;
    let (
        window_id,
        host_surface,
        host_xdg_surface,
        host_xdg_toplevel,
        host_xdg_popup,
        host_layer_surface,
        host_xdg_decoration,
        host_xdg_dialog,
        host_alpha_modifier_surface,
        host_fractional_scale,
        host_viewport,
        host_locked_pointer,
        host_confined_pointer,
        should_publish_close_requested,
    ) = {
        let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

        // clear transient drop session state before closing this window
        drop::reset_drop_state(context, &mut host_state)?;

        let should_publish_close_requested = !host_state.close_requested_emitted;
        host_state.close_requested_emitted = true;
        host_state.destroyed_emitted = true;

        (
            host_state.id.clone(),
            host_state.host.surface.clone(),
            host_state.host.xdg_surface.clone(),
            host_state.host.xdg_toplevel.clone(),
            host_state.host.xdg_popup.clone(),
            host_state.host.layer_surface.clone(),
            host_state.host.xdg_decoration.clone(),
            host_state.host.xdg_dialog.clone(),
            host_state.host.alpha_modifier_surface.clone(),
            host_state.host.fractional_scale.clone(),
            host_state.host.viewport.clone(),
            host_state.host.locked_pointer.clone(),
            host_state.host.confined_pointer.clone(),
            should_publish_close_requested,
        )
    };
    let host_surface_for_destroy = host_surface.clone();

    // destroy native wayland objects for this window
    wayland_core::with_connection_dispatch(
        context,
        "destack.display.window.close",
        |connection, event_queue, dispatch_state| {
            wayland_core::clear_pointer_focus_for_surface(dispatch_state, &host_surface);

            if let Some(host_xdg_dialog) = host_xdg_dialog.clone()
                && !host_xdg_dialog.is_null()
                && let Ok(proxy) = xdg_dialog_v1::XdgDialogV1::from_id(connection, host_xdg_dialog)
            {
                proxy.destroy();
            }

            if let Some(host_xdg_decoration) = host_xdg_decoration.clone()
                && !host_xdg_decoration.is_null()
                && let Ok(proxy) = zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1::from_id(
                    connection,
                    host_xdg_decoration,
                )
            {
                proxy.destroy();
            }

            if let Some(host_alpha_modifier_surface) = host_alpha_modifier_surface.clone()
                && !host_alpha_modifier_surface.is_null()
                && let Ok(proxy) = wp_alpha_modifier_surface_v1::WpAlphaModifierSurfaceV1::from_id(
                    connection,
                    host_alpha_modifier_surface,
                )
            {
                proxy.destroy();
            }

            if let Some(host_fractional_scale) = host_fractional_scale.clone()
                && !host_fractional_scale.is_null()
                && let Ok(proxy) = wp_fractional_scale_v1::WpFractionalScaleV1::from_id(
                    connection,
                    host_fractional_scale,
                )
            {
                proxy.destroy();
            }

            if let Some(host_viewport) = host_viewport.clone()
                && !host_viewport.is_null()
                && let Ok(proxy) = wp_viewport::WpViewport::from_id(connection, host_viewport)
            {
                proxy.destroy();
            }

            if let Some(host_locked_pointer) = host_locked_pointer.clone()
                && !host_locked_pointer.is_null()
                && let Ok(proxy) = zwp_locked_pointer_v1::ZwpLockedPointerV1::from_id(
                    connection,
                    host_locked_pointer,
                )
            {
                proxy.destroy();
            }

            if let Some(host_confined_pointer) = host_confined_pointer.clone()
                && !host_confined_pointer.is_null()
                && let Ok(proxy) = zwp_confined_pointer_v1::ZwpConfinedPointerV1::from_id(
                    connection,
                    host_confined_pointer,
                )
            {
                proxy.destroy();
            }

            if let Some(host_xdg_popup) = host_xdg_popup.clone()
                && !host_xdg_popup.is_null()
                && let Ok(proxy) = xdg_popup::XdgPopup::from_id(connection, host_xdg_popup)
            {
                proxy.destroy();
            }

            if let Some(host_layer_surface) = host_layer_surface.clone()
                && !host_layer_surface.is_null()
                && let Ok(proxy) = zwlr_layer_surface_v1::ZwlrLayerSurfaceV1::from_id(
                    connection,
                    host_layer_surface,
                )
            {
                proxy.destroy();
            }

            if let Some(host_xdg_toplevel) = host_xdg_toplevel.clone()
                && !host_xdg_toplevel.is_null()
                && let Ok(proxy) = wayland_core::resolve_xdg_toplevel(
                    connection,
                    host_xdg_toplevel,
                    "destack.display.window.close",
                )
            {
                proxy.destroy();
            }

            if let Some(host_xdg_surface) = host_xdg_surface.clone()
                && !host_xdg_surface.is_null()
                && let Ok(proxy) = wayland_core::resolve_xdg_surface(
                    connection,
                    host_xdg_surface,
                    "destack.display.window.close",
                )
            {
                proxy.destroy();
            }

            if !host_surface.is_null()
                && let Ok(proxy) = wayland_core::resolve_wl_surface(
                    connection,
                    host_surface_for_destroy,
                    "destack.display.window.close",
                )
            {
                proxy.destroy();
            }

            wayland_core::flush_queue(event_queue, "destack.display.window.close")?;

            Ok(())
        },
    )?;

    // remove backend window id mappings
    let runtime_state = wayland_core::runtime_state(context);
    runtime_state.unregister_window_handle(&window_id);
    runtime_state.unregister_window_token(&host_surface);

    // remove the resource entry and reject stale handles
    let removed = context
        .worker()
        .resources
        .remove(&context.world(), window_handle.0, Some(context.engine()))
        .is_some();
    if !removed {
        return Err(wayland_core::window_not_found(
            "destack.display.window.close",
            window_handle,
        ));
    }

    // publish close requested once, then publish destroyed
    if should_publish_close_requested {
        event::publish_window_close_requested(&runtime_state, window_handle);
    }
    event::publish_window_destroyed(&runtime_state, window_handle);

    Ok(())
}
