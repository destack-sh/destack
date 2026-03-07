use crate::diagnostic::RuntimeResult;
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;
use windows_sys::Win32::Foundation::{GetLastError, HWND, SetLastError};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GWLP_HWNDPARENT, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    SetWindowPos,
};

use super::core::set_window_long_ptr_checked;
use super::geometry::refresh_window_snapshot;
use crate::platform::display::windows::win32::model::Win32WindowHostState;
use crate::platform::display::windows::win32::{core, event, resource as display_resource};

/// Resolve one referenced relationship window handle to one hwnd.
fn resolve_relationship_hwnd(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    operation: &'static str,
) -> RuntimeResult<HWND> {
    let host_state = display_resource::resolve_window_host_state(context, window, operation)?;
    let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    Ok(host_state.hwnd)
}

/// Resolve the owner relationship for one window host state.
pub(crate) fn owner_relationship(
    host_state: &Win32WindowHostState,
) -> Option<resource::WindowHandle> {
    host_state.transient_for.or(host_state.parent)
}

/// Apply owner relationship style for one window.
pub(crate) fn apply_owner_relationship(
    context: &BindingCallContext,
    host_state: &Win32WindowHostState,
    operation: &'static str,
) -> RuntimeResult<()> {
    let owner_hwnd = if let Some(owner) = owner_relationship(host_state) {
        let owner_hwnd = resolve_relationship_hwnd(context, owner, operation)?;
        if owner_hwnd == host_state.hwnd {
            return Err(core_platform::invalid_argument(
                "window",
                "window relationship cannot target itself",
            ));
        }

        owner_hwnd
    } else {
        0
    };

    set_window_long_ptr_checked(
        host_state.hwnd,
        GWLP_HWNDPARENT,
        owner_hwnd,
        operation,
        "SetWindowLongPtrW",
    )?;
    let status = unsafe {
        SetWindowPos(
            host_state.hwnd,
            0,
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER,
        )
    };
    if status == 0 {
        return Err(core::io_error(
            operation,
            "SetWindowPos",
            "failed to apply window owner relationship",
        ));
    }

    Ok(())
}

/// Set one owner-window enabled state.
fn set_owner_enabled(
    context: &BindingCallContext,
    owner: resource::WindowHandle,
    enabled: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    let owner_hwnd = resolve_relationship_hwnd(context, owner, operation)?;
    let enabled = if enabled { 1 } else { 0 };

    unsafe {
        SetLastError(0);
        let previous = EnableWindow(owner_hwnd, enabled);
        if previous == 0 {
            let error_code = GetLastError();
            if error_code != 0 {
                return Err(core::io_error_with_code(
                    operation,
                    "EnableWindow",
                    error_code as u32,
                    "failed to update owner enabled state",
                ));
            }
        }
    }

    Ok(())
}

/// Apply one modal-owner state transition for one window.
pub(crate) fn apply_modal_owner_transition(
    context: &BindingCallContext,
    previous_owner: Option<resource::WindowHandle>,
    previous_modal: bool,
    next_owner: Option<resource::WindowHandle>,
    next_modal: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    if next_modal && next_owner.is_none() {
        return Err(core_platform::invalid_argument(
            "modal",
            "modal windows require parent or transientFor relationship",
        ));
    }

    if previous_modal
        && let Some(owner) = previous_owner
        && (!next_modal || Some(owner) != next_owner)
    {
        set_owner_enabled(context, owner, true, operation)?;
    }

    if next_modal && let Some(owner) = next_owner {
        set_owner_enabled(context, owner, false, operation)?;
    }

    Ok(())
}

/// Re-enable one modal owner as part of window close cleanup.
pub(crate) fn restore_modal_owner_on_close(
    context: &BindingCallContext,
    host_state: &Win32WindowHostState,
) {
    if !host_state.modal {
        return;
    }

    if let Some(owner) = owner_relationship(host_state)
        && let Err(error) = set_owner_enabled(context, owner, true, "destack.display.window.close")
    {
        core::runtime_state(context).diagnostics.warn(
            "display",
            "destack.display.window.close",
            format!("failed to restore modal owner after close: {error}"),
            None,
        );
    }
}

/// Set one window modal state.
pub(crate) unsafe fn window_set_modal(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    modal: bool,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let resolved_host_state = display_resource::resolve_window_host_state(
        binding,
        window,
        "destack.display.window.setModal",
    )?;
    let mut resolved_host_state = resolved_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // reject modal transitions without an owner relationship
    if modal && owner_relationship(&resolved_host_state).is_none() {
        return Err(core_platform::invalid_argument(
            "modal",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // snapshot state before mutation, for event and rollback lanes
    let previous = resolved_host_state.clone();
    let previous_owner = owner_relationship(&resolved_host_state);
    let previous_modal = resolved_host_state.modal;
    resolved_host_state.modal = modal;
    let next_owner = owner_relationship(&resolved_host_state);

    // apply the host owner transition and roll back window host state on failure
    if let Err(error) = apply_modal_owner_transition(
        binding,
        previous_owner,
        previous_modal,
        next_owner,
        resolved_host_state.modal,
        "destack.display.window.setModal",
    ) {
        resolved_host_state.modal = previous_modal;
        return Err(error);
    }

    // publish state deltas after host mutation succeeds
    refresh_window_snapshot(&mut resolved_host_state);
    let next = resolved_host_state.clone();
    drop(resolved_host_state);

    let runtime_state = core::runtime_state(binding);
    event::publish_state_deltas(&runtime_state, window, &previous, &next);

    Ok(())
}

/// Set one parent-window relationship.
pub(crate) unsafe fn window_set_parent(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    parent: Option<resource::WindowHandle>,
) -> RuntimeResult<()> {
    // reject self-parent relationship
    if parent == Some(window) {
        return Err(core_platform::invalid_argument(
            "parent",
            "window cannot be parent of itself",
        ));
    }

    // resolve and validate the target window host state
    let resolved_host_state = display_resource::resolve_window_host_state(
        binding,
        window,
        "destack.display.window.setParent",
    )?;
    let mut resolved_host_state = resolved_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    // snapshot state before mutation, for event and rollback lanes
    let previous = resolved_host_state.clone();
    let previous_owner = owner_relationship(&resolved_host_state);
    let previous_modal = resolved_host_state.modal;
    let next_owner = resolved_host_state.transient_for.or(parent);

    // reject modal state without an owner relationship
    if resolved_host_state.modal && next_owner.is_none() {
        return Err(core_platform::invalid_argument(
            "parent",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // apply relationship update and rollback host owner lane on failure
    resolved_host_state.parent = parent;
    if let Err(error) = apply_owner_relationship(
        binding,
        &resolved_host_state,
        "destack.display.window.setParent",
    ) {
        resolved_host_state.parent = previous.parent;
        return Err(error);
    }

    if let Err(error) = apply_modal_owner_transition(
        binding,
        previous_owner,
        previous_modal,
        next_owner,
        resolved_host_state.modal,
        "destack.display.window.setParent",
    ) {
        let transition_reversal = apply_modal_owner_transition(
            binding,
            next_owner,
            resolved_host_state.modal,
            previous_owner,
            previous_modal,
            "destack.display.window.setParent.rollback",
        );

        resolved_host_state.parent = previous.parent;
        let relationship_reversal = apply_owner_relationship(
            binding,
            &resolved_host_state,
            "destack.display.window.setParent.rollback",
        );

        // keep primary error: rollback is best effort during failure unwind
        if transition_reversal.is_err() || relationship_reversal.is_err() {
            binding.warn(
                "display",
                "destack.display.window.setParent.rollback",
                "rollback failed while unwinding modal transition",
                None,
            );
        }

        return Err(error);
    }

    // publish state deltas after host mutation succeeds
    refresh_window_snapshot(&mut resolved_host_state);
    let next = resolved_host_state.clone();
    drop(resolved_host_state);

    let runtime_state = core::runtime_state(binding);
    event::publish_state_deltas(&runtime_state, window, &previous, &next);

    Ok(())
}

/// Set one transient-owner relationship.
pub(crate) unsafe fn window_set_transient_for(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    transientfor: Option<resource::WindowHandle>,
) -> RuntimeResult<()> {
    // reject self-transient relationship
    if transientfor == Some(window) {
        return Err(core_platform::invalid_argument(
            "transientFor",
            "window cannot be transient owner of itself",
        ));
    }

    // resolve and validate the target window host state
    let resolved_host_state = display_resource::resolve_window_host_state(
        binding,
        window,
        "destack.display.window.setTransientFor",
    )?;
    let mut resolved_host_state = resolved_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    // snapshot state before mutation, for event and rollback lanes
    let previous = resolved_host_state.clone();
    let previous_owner = owner_relationship(&resolved_host_state);
    let previous_modal = resolved_host_state.modal;
    let next_owner = transientfor.or(resolved_host_state.parent);

    // reject modal state without an owner relationship
    if resolved_host_state.modal && next_owner.is_none() {
        return Err(core_platform::invalid_argument(
            "transientFor",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // apply relationship update and rollback host owner lane on failure
    resolved_host_state.transient_for = transientfor;
    if let Err(error) = apply_owner_relationship(
        binding,
        &resolved_host_state,
        "destack.display.window.setTransientFor",
    ) {
        resolved_host_state.transient_for = previous.transient_for;
        return Err(error);
    }

    if let Err(error) = apply_modal_owner_transition(
        binding,
        previous_owner,
        previous_modal,
        next_owner,
        resolved_host_state.modal,
        "destack.display.window.setTransientFor",
    ) {
        let transition_reversal = apply_modal_owner_transition(
            binding,
            next_owner,
            resolved_host_state.modal,
            previous_owner,
            previous_modal,
            "destack.display.window.setTransientFor.rollback",
        );

        resolved_host_state.transient_for = previous.transient_for;
        let relationship_reversal = apply_owner_relationship(
            binding,
            &resolved_host_state,
            "destack.display.window.setTransientFor.rollback",
        );

        // keep primary error: rollback is best effort during failure unwind
        if transition_reversal.is_err() || relationship_reversal.is_err() {
            binding.warn(
                "display",
                "destack.display.window.setTransientFor.rollback",
                "rollback failed while unwinding modal transition",
                None,
            );
        }

        return Err(error);
    }

    // publish state deltas after host mutation succeeds
    refresh_window_snapshot(&mut resolved_host_state);
    let next = resolved_host_state.clone();
    drop(resolved_host_state);

    let runtime_state = core::runtime_state(binding);
    event::publish_state_deltas(&runtime_state, window, &previous, &next);

    Ok(())
}
