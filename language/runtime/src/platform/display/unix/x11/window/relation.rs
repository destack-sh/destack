use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::resource::WindowHandle;
use crate::runtime::BindingCallContext;

use super::super::super::{core, resource as display_resource};
use super::{apply_window_transient_owner, ensure_window_thread, set_net_wm_state};

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

    // resolve owner binding and return its native x11 window id
    let owner_binding = display_resource::resolve_window_binding(binding, owner_handle, operation)?;
    let owner_binding = owner_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&owner_binding, operation)?;

    Ok(Some(owner_binding.window))
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
    // resolve runtime and mutate resolved_binding state
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setModal")?;
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window_handle,
        "destack.display.window.setModal",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.setModal")?;

    // reject modal state when no owner relationship exists
    if modal && resolved_binding.parent.is_none() && resolved_binding.transient_for.is_none() {
        return Err(core_platform::invalid_argument(
            "modal",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // apply one modal state mutation before updating the snapshot
    set_net_wm_state(
        connection_state.as_ref(),
        resolved_binding.window,
        connection_state.atoms.net_wm_state_modal,
        modal,
    )?;
    resolved_binding.modal = modal;

    Ok(())
}

/// Set one window parent relationship.
pub(crate) unsafe fn window_set_parent(
    binding: &BindingCallContext,
    window_handle: WindowHandle,
    parent: Option<WindowHandle>,
) -> RuntimeResult<()> {
    // resolve runtime and child binding lanes
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setParent")?;
    let child_binding = display_resource::resolve_window_binding(
        binding,
        window_handle,
        "destack.display.window.setParent",
    )?;
    let mut child_binding = child_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&child_binding, "destack.display.window.setParent")?;

    // resolve the next effective owner relationship
    let owner_window = resolve_effective_owner_window(
        binding,
        window_handle,
        parent,
        child_binding.transient_for,
        "destack.display.window.setParent",
    )?;
    // evaluate this condition
    if child_binding.modal && owner_window.is_none() {
        return Err(core_platform::invalid_argument(
            "parent",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // apply one transient-owner update before mutating the snapshot
    apply_window_transient_owner(
        connection_state.as_ref(),
        child_binding.window,
        owner_window,
        "destack.display.window.setParent",
    )?;
    // evaluate this condition
    if child_binding.modal {
        set_net_wm_state(
            connection_state.as_ref(),
            child_binding.window,
            connection_state.atoms.net_wm_state_modal,
            true,
        )?;
    }

    // update cached parent lane
    child_binding.parent = parent;

    Ok(())
}

/// Set one window transient relationship.
pub(crate) unsafe fn window_set_transient_for(
    binding: &BindingCallContext,
    window_handle: WindowHandle,
    transient_for: Option<WindowHandle>,
) -> RuntimeResult<()> {
    // resolve runtime and child binding state
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setTransientFor")?;
    let child_binding = display_resource::resolve_window_binding(
        binding,
        window_handle,
        "destack.display.window.setTransientFor",
    )?;
    let mut child_binding = child_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&child_binding, "destack.display.window.setTransientFor")?;

    // resolve the next effective owner relationship
    let owner_window = resolve_effective_owner_window(
        binding,
        window_handle,
        child_binding.parent,
        transient_for,
        "destack.display.window.setTransientFor",
    )?;
    // evaluate this condition
    if child_binding.modal && owner_window.is_none() {
        return Err(core_platform::invalid_argument(
            "transientFor",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // apply one transient-owner update before mutating the snapshot
    apply_window_transient_owner(
        connection_state.as_ref(),
        child_binding.window,
        owner_window,
        "destack.display.window.setTransientFor",
    )?;
    // evaluate this condition
    if child_binding.modal {
        set_net_wm_state(
            connection_state.as_ref(),
            child_binding.window,
            connection_state.atoms.net_wm_state_modal,
            true,
        )?;
    }

    // update cached transient lane
    child_binding.transient_for = transient_for;

    Ok(())
}
