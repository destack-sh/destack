use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::resource::WindowHandle;
use crate::runtime::BindingCallContext;

use super::{apply_window_transient_owner, set_net_wm_state};
use crate::platform::display::unix::x11::{core, event, resource as display_resource};

/// Resolve one optional owner handle into one x11 window id.
fn resolve_owner_window(
    binding: &BindingCallContext,
    child_handle: WindowHandle,
    owner_handle: Option<WindowHandle>,
    operation: &'static str,
) -> RuntimeResult<Option<u32>> {
    let Some(owner_handle) = owner_handle else {
        return Ok(None);
    };

    // reject self-ownership relationships
    if owner_handle == child_handle {
        return Err(core_platform::invalid_argument(
            "window",
            format!("{operation}: owner window must differ from source window"),
        ));
    }

    // resolve owner host state and return its native x11 window id
    let owner_host_state =
        display_resource::resolve_window_host_state(binding, owner_handle, operation)?;
    let owner_host_state = owner_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    Ok(Some(owner_host_state.window))
}

/// Resolve one effective transient-owner relation with transient priority.
fn resolve_effective_owner_window(
    binding: &BindingCallContext,
    child_handle: WindowHandle,
    parent: Option<WindowHandle>,
    transient_for: Option<WindowHandle>,
    operation: &'static str,
) -> RuntimeResult<Option<u32>> {
    // prefer transient owner when both lanes are configured
    if transient_for.is_some() {
        return resolve_owner_window(binding, child_handle, transient_for, operation);
    }

    // otherwise use parent relationship
    resolve_owner_window(binding, child_handle, parent, operation)
}

/// Set one window modal state.
pub(crate) unsafe fn window_set_modal(
    binding: &BindingCallContext,
    window_handle: WindowHandle,
    modal: bool,
) -> RuntimeResult<()> {
    // resolve runtime and mutate the target window host state
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setModal")?;
    let resolved_host_state = display_resource::resolve_window_host_state(
        binding,
        window_handle,
        "destack.display.window.setModal",
    )?;
    let mut resolved_host_state = resolved_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let previous = resolved_host_state.clone();

    // reject modal state when no owner relationship exists
    if modal && resolved_host_state.parent.is_none() && resolved_host_state.transient_for.is_none()
    {
        return Err(core_platform::invalid_argument(
            "modal",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // apply one modal state mutation before updating the snapshot
    set_net_wm_state(
        connection_state.as_ref(),
        resolved_host_state.window,
        connection_state.atoms.net_wm_state_modal,
        modal,
    )?;
    resolved_host_state.modal = modal;
    let current = resolved_host_state.clone();
    drop(resolved_host_state);

    // publish all affected state deltas
    event::publish_state_deltas(&runtime_state, window_handle, &previous, &current);

    Ok(())
}

/// Set one window parent relationship.
pub(crate) unsafe fn window_set_parent(
    binding: &BindingCallContext,
    window_handle: WindowHandle,
    parent: Option<WindowHandle>,
) -> RuntimeResult<()> {
    // resolve runtime and child host state lanes
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setParent")?;
    let child_host_state = display_resource::resolve_window_host_state(
        binding,
        window_handle,
        "destack.display.window.setParent",
    )?;
    let mut child_host_state = child_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let previous = child_host_state.clone();

    // resolve the next effective owner relationship
    let owner_window = resolve_effective_owner_window(
        binding,
        window_handle,
        parent,
        child_host_state.transient_for,
        "destack.display.window.setParent",
    )?;
    if child_host_state.modal && owner_window.is_none() {
        return Err(core_platform::invalid_argument(
            "parent",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // apply one transient-owner update before mutating the snapshot
    apply_window_transient_owner(
        connection_state.as_ref(),
        child_host_state.window,
        owner_window,
        "destack.display.window.setParent",
    )?;
    if child_host_state.modal {
        set_net_wm_state(
            connection_state.as_ref(),
            child_host_state.window,
            connection_state.atoms.net_wm_state_modal,
            true,
        )?;
    }

    // update cached parent lane
    child_host_state.parent = parent;
    let current = child_host_state.clone();
    drop(child_host_state);

    // publish all affected state deltas
    event::publish_state_deltas(&runtime_state, window_handle, &previous, &current);

    Ok(())
}

/// Set one window transient relationship.
pub(crate) unsafe fn window_set_transient_for(
    binding: &BindingCallContext,
    window_handle: WindowHandle,
    transient_for: Option<WindowHandle>,
) -> RuntimeResult<()> {
    // resolve runtime and child host state state
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setTransientFor")?;
    let child_host_state = display_resource::resolve_window_host_state(
        binding,
        window_handle,
        "destack.display.window.setTransientFor",
    )?;
    let mut child_host_state = child_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let previous = child_host_state.clone();

    // resolve the next effective owner relationship
    let owner_window = resolve_effective_owner_window(
        binding,
        window_handle,
        child_host_state.parent,
        transient_for,
        "destack.display.window.setTransientFor",
    )?;
    if child_host_state.modal && owner_window.is_none() {
        return Err(core_platform::invalid_argument(
            "transientFor",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // apply one transient-owner update before mutating the snapshot
    apply_window_transient_owner(
        connection_state.as_ref(),
        child_host_state.window,
        owner_window,
        "destack.display.window.setTransientFor",
    )?;
    if child_host_state.modal {
        set_net_wm_state(
            connection_state.as_ref(),
            child_host_state.window,
            connection_state.atoms.net_wm_state_modal,
            true,
        )?;
    }

    // update cached transient lane
    child_host_state.transient_for = transient_for;
    let current = child_host_state.clone();
    drop(child_host_state);

    // publish all affected state deltas
    event::publish_state_deltas(&runtime_state, window_handle, &previous, &current);

    Ok(())
}
