use wayland_client::Proxy;
use wayland_protocols::xdg::dialog::v1::client::xdg_dialog_v1;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::resource::WindowHandle;
use crate::runtime::BindingCallContext;

use super::super::core as backend_core;
use super::{require_xdg_toplevel_id, resolve_window_binding};

/// Resolve one optional owner handle and reject self-relationships.
fn resolve_owner_handle(
    context: &BindingCallContext,
    child_handle: WindowHandle,
    owner_handle: Option<WindowHandle>,
    operation: &'static str,
) -> RuntimeResult<Option<WindowHandle>> {
    let Some(owner_handle) = owner_handle else {
        return Ok(None);
    };

    // reject self-reference owner relationships
    if owner_handle == child_handle {
        return Err(core_platform::invalid_argument(
            "window",
            format!("{operation}: owner window must differ from source window"),
        ));
    }

    // ensure owner handle resolves to one live window on the same owner thread
    resolve_window_binding(context, owner_handle, operation)?;

    Ok(Some(owner_handle))
}

/// Resolve one optional owner handle into one valid xdg-toplevel object id.
fn resolve_owner_toplevel_id(
    context: &BindingCallContext,
    owner_handle: Option<WindowHandle>,
    field: &'static str,
    operation: &'static str,
) -> RuntimeResult<Option<wayland_client::backend::ObjectId>> {
    let Some(owner_handle) = owner_handle else {
        return Ok(None);
    };

    // resolve owner binding and require one toplevel role lane
    let owner_binding = resolve_window_binding(context, owner_handle, operation)?;
    let owner_binding = owner_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let owner_toplevel = owner_binding.host.xdg_toplevel.clone().ok_or_else(|| {
        core_platform::invalid_argument(
            field,
            format!("{operation}: owner window does not expose xdg_toplevel role"),
        )
    })?;
    if owner_toplevel.is_null() {
        return Err(core_platform::invalid_argument(
            field,
            format!("{operation}: owner window xdg_toplevel id is invalid"),
        ));
    }

    Ok(Some(owner_toplevel))
}

/// Apply one optional parent relationship to one xdg_toplevel lane.
fn apply_parent_relationship(
    context: &BindingCallContext,
    window_toplevel: wayland_client::backend::ObjectId,
    owner_toplevel: Option<wayland_client::backend::ObjectId>,
    operation: &'static str,
) -> RuntimeResult<()> {
    backend_core::with_connection_dispatch(
        context,
        operation,
        |connection, event_queue, _dispatch_state| {
            let toplevel =
                backend_core::resolve_xdg_toplevel(connection, window_toplevel.clone(), operation)?;

            let owner_toplevel = match owner_toplevel {
                Some(owner_toplevel) if !owner_toplevel.is_null() => Some(
                    backend_core::resolve_xdg_toplevel(connection, owner_toplevel, operation)?,
                ),
                _ => None,
            };

            toplevel.set_parent(owner_toplevel.as_ref());
            backend_core::flush_queue(event_queue, operation)?;

            Ok(())
        },
    )
}

/// Apply one modal request to one toplevel and optional dialog object.
fn apply_modal_request(
    context: &BindingCallContext,
    toplevel_id: wayland_client::backend::ObjectId,
    dialog_id: Option<wayland_client::backend::ObjectId>,
    modal: bool,
    operation: &'static str,
) -> RuntimeResult<Option<wayland_client::backend::ObjectId>> {
    backend_core::with_connection_dispatch(
        context,
        operation,
        |connection, event_queue, dispatch_state| {
            // resolve one existing dialog object, or create one on first modal use
            let mut dialog_id = dialog_id;
            if modal && dialog_id.is_none() {
                let manager = dispatch_state
                    .dialog_manager
                    .as_ref()
                    .cloned()
                    .ok_or_else(|| core_platform::not_supported(operation))?;
                let toplevel =
                    backend_core::resolve_xdg_toplevel(connection, toplevel_id, operation)?;
                let dialog = manager.get_xdg_dialog(&toplevel, &event_queue.handle(), ());
                dialog_id = Some(dialog.id());
            }

            // apply the requested modal policy through xdg-dialog
            if let Some(dialog_id_value) = dialog_id.as_ref().cloned() {
                let dialog = xdg_dialog_v1::XdgDialogV1::from_id(connection, dialog_id_value)
                    .map_err(|error| {
                        backend_core::io_error(operation, format!("invalid xdg_dialog id: {error}"))
                    })?;

                if modal {
                    dialog.set_modal();
                } else {
                    dialog.unset_modal();
                }
            } else if modal {
                return Err(core_platform::not_supported(operation));
            }

            // flush protocol requests for this modal update
            backend_core::flush_queue(event_queue, operation)?;

            Ok(dialog_id)
        },
    )
}

/// Set one window modal state.
pub(crate) unsafe fn window_set_modal(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    modal: bool,
) -> RuntimeResult<()> {
    // resolve target window binding and enforce owner-thread affinity
    let binding =
        resolve_window_binding(context, window_handle, "destack.display.window.setModal")?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());

    // reject modal requests when no owner relationship exists
    if modal && binding.parent.is_none() && binding.transient_for.is_none() {
        return Err(core_platform::invalid_argument(
            "modal",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // skip no-op modal transitions
    if binding.modal == modal {
        return Ok(());
    }

    // apply modal state through optional xdg-dialog support
    let toplevel_id = require_xdg_toplevel_id(&binding, "destack.display.window.setModal")?;
    let next_dialog_id = apply_modal_request(
        context,
        toplevel_id,
        binding.host.xdg_dialog.clone(),
        modal,
        "destack.display.window.setModal",
    )?;
    binding.host.xdg_dialog = next_dialog_id;
    binding.modal = modal;

    Ok(())
}

/// Set one window parent relationship.
pub(crate) unsafe fn window_set_parent(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    parent: Option<WindowHandle>,
) -> RuntimeResult<()> {
    // resolve and validate the requested owner handle
    let parent = resolve_owner_handle(
        context,
        window_handle,
        parent,
        "destack.display.window.setParent",
    )?;

    // resolve target window binding and enforce owner-thread affinity
    let binding =
        resolve_window_binding(context, window_handle, "destack.display.window.setParent")?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());

    // apply parent relation through xdg_toplevel
    let window_toplevel_id = require_xdg_toplevel_id(&binding, "destack.display.window.setParent")?;
    let parent_toplevel = resolve_owner_toplevel_id(
        context,
        parent,
        "parent",
        "destack.display.window.setParent",
    )?;
    apply_parent_relationship(
        context,
        window_toplevel_id,
        parent_toplevel,
        "destack.display.window.setParent",
    )?;

    // clear modal state when both owner lanes are absent
    if parent.is_none() && binding.transient_for.is_none() {
        let toplevel_id = require_xdg_toplevel_id(&binding, "destack.display.window.setParent")?;
        let next_dialog_id = apply_modal_request(
            context,
            toplevel_id,
            binding.host.xdg_dialog.clone(),
            false,
            "destack.display.window.setParent",
        )?;
        binding.host.xdg_dialog = next_dialog_id;
        binding.modal = false;
    }

    // update parent relationship snapshot
    binding.parent = parent;

    Ok(())
}

/// Set one window transient-owner relationship.
pub(crate) unsafe fn window_set_transient_for(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    transient_for: Option<WindowHandle>,
) -> RuntimeResult<()> {
    // resolve and validate the requested owner handle
    let transient_for = resolve_owner_handle(
        context,
        window_handle,
        transient_for,
        "destack.display.window.setTransientFor",
    )?;

    // resolve target window binding and enforce owner-thread affinity
    let binding = resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setTransientFor",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());

    // apply transient-owner relation through xdg_toplevel parent lane
    let window_toplevel_id =
        require_xdg_toplevel_id(&binding, "destack.display.window.setTransientFor")?;
    let owner_toplevel = resolve_owner_toplevel_id(
        context,
        transient_for,
        "transientFor",
        "destack.display.window.setTransientFor",
    )?;
    apply_parent_relationship(
        context,
        window_toplevel_id,
        owner_toplevel,
        "destack.display.window.setTransientFor",
    )?;

    // clear modal state when both owner lanes are absent
    if transient_for.is_none() && binding.parent.is_none() {
        let toplevel_id =
            require_xdg_toplevel_id(&binding, "destack.display.window.setTransientFor")?;
        let next_dialog_id = apply_modal_request(
            context,
            toplevel_id,
            binding.host.xdg_dialog.clone(),
            false,
            "destack.display.window.setTransientFor",
        )?;
        binding.host.xdg_dialog = next_dialog_id;
        binding.modal = false;
    }

    // update transient relationship snapshot
    binding.transient_for = transient_for;

    Ok(())
}

/// Set one window mouse-passthrough state.
pub(crate) unsafe fn window_set_mouse_passthrough(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    passthrough: bool,
) -> RuntimeResult<()> {
    // resolve target window binding and enforce owner-thread affinity
    let binding = resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setMousePassthrough",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());

    // apply mouse input region policy through wl_surface
    let surface_id = binding.host.surface.clone();
    let runtime_state = backend_core::runtime_state(context);
    let window_token = backend_core::window_token_from_surface(&runtime_state, &surface_id)
        .ok_or_else(|| {
            backend_core::io_error(
                "destack.display.window.setMousePassthrough",
                "missing wayland window dispatch token",
            )
        })?;
    backend_core::with_connection_dispatch(
        context,
        "destack.display.window.setMousePassthrough",
        |connection, event_queue, dispatch_state| {
            let surface = backend_core::resolve_wl_surface(
                connection,
                surface_id,
                "destack.display.window.setMousePassthrough",
            )?;

            // apply empty input region for passthrough windows
            if passthrough {
                let compositor = dispatch_state.compositor.as_ref().cloned().ok_or_else(|| {
                    core_platform::not_supported("destack.display.window.setMousePassthrough")
                })?;
                let queue_handle = event_queue.handle();
                let region = compositor.create_region(&queue_handle, ());
                surface.set_input_region(Some(&region));
                region.destroy();
            }
            // otherwise reset to compositor default input region
            else {
                surface.set_input_region(None);
            }

            backend_core::request_surface_presentation_feedback(
                dispatch_state,
                event_queue,
                &surface,
                window_token.clone(),
            );
            surface.commit();
            backend_core::flush_queue(event_queue, "destack.display.window.setMousePassthrough")?;

            Ok(())
        },
    )?;

    // update passthrough snapshot
    binding.mouse_passthrough = passthrough;

    Ok(())
}
