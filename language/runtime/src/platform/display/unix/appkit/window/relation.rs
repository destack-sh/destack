use std::sync::Arc;

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSWindowOrderingMode};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::display::WindowRole;
use crate::platform::resource::WindowHandle;
use crate::runtime::BindingCallContext;

use super::super::event::publish_state_deltas;
use super::super::model::AppKitWindowBinding;
use super::super::{core as appkit_core, resource as display_resource};
use super::runtime;

/// Return one effective owner relationship with transient priority.
fn owner_relationship(binding: &AppKitWindowBinding) -> Option<WindowHandle> {
    binding.transient_for.or(binding.parent)
}

/// Resolve one owner handle and enforce same-thread ownership.
fn resolve_owner_handle(
    context: &BindingCallContext,
    child_window: WindowHandle,
    owner_window: Option<WindowHandle>,
    field: &'static str,
    operation: &'static str,
) -> RuntimeResult<Option<WindowHandle>> {
    let Some(owner_window) = owner_window else {
        return Ok(None);
    };

    // reject self-reference relationships at the binding boundary
    if owner_window == child_window {
        return Err(core_platform::invalid_argument(
            field,
            format!("{operation}: owner window must differ from source window"),
        ));
    }

    let owner_binding = display_resource::resolve_window_binding(context, owner_window, operation)?;
    let owner_binding = owner_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    runtime::ensure_window_thread(&owner_binding, operation)?;

    Ok(Some(owner_window))
}

/// Apply one native owner and modal relationship transition.
pub(crate) fn apply_host_relationship_state(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: WindowHandle,
    previous_owner: Option<WindowHandle>,
    previous_modal: bool,
    next_owner: Option<WindowHandle>,
    next_modal: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    // reject modal transitions without any owner lane
    if next_modal && next_owner.is_none() {
        return Err(core_platform::invalid_argument(
            "modal",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // skip exact no-op transitions
    if previous_owner == next_owner && previous_modal == next_modal {
        return Ok(());
    }

    appkit_core::with_main_thread_state(runtime_state, |state| {
        let state = state.borrow();
        let child = state
            .windows
            .get(&window_handle)
            .ok_or_else(|| appkit_core::window_not_found(operation, window_handle))?;
        let previous_owner_handle = previous_owner;
        let next_owner_handle = next_owner;

        let previous_owner = previous_owner
            .map(|owner| {
                state
                    .windows
                    .get(&owner)
                    .ok_or_else(|| appkit_core::window_not_found(operation, owner))
            })
            .transpose()?;
        let next_owner = next_owner
            .map(|owner| {
                state
                    .windows
                    .get(&owner)
                    .ok_or_else(|| appkit_core::window_not_found(operation, owner))
            })
            .transpose()?;

        // detach one active sheet before reconfiguring relationships
        if previous_modal && let Some(sheet_parent) = child.window.sheetParent() {
            sheet_parent.endSheet(&child.window);
        }

        // detach one non-modal child relationship when it changes
        if !previous_modal
            && let Some(previous_owner) = previous_owner
            && (previous_owner_handle != next_owner_handle || next_modal)
        {
            previous_owner.window.removeChildWindow(&child.window);
        }

        // apply one modal sheet relationship when requested
        if next_modal {
            let next_owner = next_owner.ok_or_else(|| {
                core_platform::invalid_argument(
                    "modal",
                    "modal windows require parent or transientFor relationship",
                )
            })?;
            let mtm =
                MainThreadMarker::new().ok_or_else(|| core_platform::not_supported(operation))?;
            let application = NSApplication::sharedApplication(mtm);

            #[allow(deprecated)]
            unsafe {
                application.beginSheet_modalForWindow_modalDelegate_didEndSelector_contextInfo(
                    &child.window,
                    &next_owner.window,
                    None,
                    None,
                    std::ptr::null_mut(),
                );
            }

            return Ok(());
        }

        // otherwise attach one non-modal child relationship when requested
        if let Some(next_owner) = next_owner
            && (previous_owner_handle != next_owner_handle || previous_modal)
        {
            unsafe {
                next_owner
                    .window
                    .addChildWindow_ordered(&child.window, NSWindowOrderingMode::Above);
            }
        }

        Ok(())
    })
}

/// Set one window parent relationship.
pub(crate) unsafe fn window_set_parent(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    parent: Option<WindowHandle>,
) -> RuntimeResult<()> {
    let parent = resolve_owner_handle(
        context,
        window_handle,
        parent,
        "parent",
        "destack.display.window.setParent",
    )?;

    let runtime_state = appkit_core::runtime_state(context);
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setParent",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    runtime::ensure_window_thread(&binding, "destack.display.window.setParent")?;

    // snapshot the transition before mutating the binding
    let previous = binding.clone();
    let previous_owner = owner_relationship(&binding);
    let previous_modal = binding.modal;

    // apply the requested relation in the cached binding
    binding.parent = parent;
    binding.transient_for = None;

    // clear modal state when removing the final owner lane
    if owner_relationship(&binding).is_none() {
        binding.modal = false;
    }

    let next_owner = owner_relationship(&binding);
    let next_modal = binding.modal;
    let next = binding.clone();
    drop(binding);

    // apply the host transition and rollback the cached binding on failure
    if let Err(error) = apply_host_relationship_state(
        &runtime_state,
        window_handle,
        previous_owner,
        previous_modal,
        next_owner,
        next_modal,
        "destack.display.window.setParent",
    ) {
        let binding = display_resource::resolve_window_binding(
            context,
            window_handle,
            "destack.display.window.setParent.rollback",
        )?;
        let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
        *binding = previous.clone();
        return Err(error);
    }

    publish_state_deltas(&runtime_state, window_handle, &previous, &next);

    Ok(())
}

/// Set one window transient-owner relationship.
pub(crate) unsafe fn window_set_transient_for(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    transient_for: Option<WindowHandle>,
) -> RuntimeResult<()> {
    let transient_for = resolve_owner_handle(
        context,
        window_handle,
        transient_for,
        "transientFor",
        "destack.display.window.setTransientFor",
    )?;

    let runtime_state = appkit_core::runtime_state(context);
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setTransientFor",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    runtime::ensure_window_thread(&binding, "destack.display.window.setTransientFor")?;

    // snapshot the transition before mutating the binding
    let previous = binding.clone();
    let previous_owner = owner_relationship(&binding);
    let previous_modal = binding.modal;

    // apply the requested relation in the cached binding
    binding.parent = None;
    binding.transient_for = transient_for;

    // clear modal state when removing the final owner lane
    if owner_relationship(&binding).is_none() {
        binding.modal = false;
    }

    let next_owner = owner_relationship(&binding);
    let next_modal = binding.modal;
    let next = binding.clone();
    drop(binding);

    // apply the host transition and rollback the cached binding on failure
    if let Err(error) = apply_host_relationship_state(
        &runtime_state,
        window_handle,
        previous_owner,
        previous_modal,
        next_owner,
        next_modal,
        "destack.display.window.setTransientFor",
    ) {
        let binding = display_resource::resolve_window_binding(
            context,
            window_handle,
            "destack.display.window.setTransientFor.rollback",
        )?;
        let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
        *binding = previous.clone();
        return Err(error);
    }

    publish_state_deltas(&runtime_state, window_handle, &previous, &next);

    Ok(())
}

/// Set one window modal state.
pub(crate) unsafe fn window_set_modal(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    modal: bool,
) -> RuntimeResult<()> {
    let runtime_state = appkit_core::runtime_state(context);
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setModal",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    runtime::ensure_window_thread(&binding, "destack.display.window.setModal")?;

    // reject modal transitions for non-toplevel roles
    if binding.role != WindowRole::Toplevel {
        return Err(core_platform::invalid_argument(
            "modal",
            "modal windows require the toplevel role on AppKit",
        ));
    }

    // reject modal requests without any owner lane
    if modal && owner_relationship(&binding).is_none() {
        return Err(core_platform::invalid_argument(
            "modal",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // skip exact no-op transitions
    if binding.modal == modal {
        return Ok(());
    }

    let previous = binding.clone();
    let previous_owner = owner_relationship(&binding);
    let previous_modal = binding.modal;
    binding.modal = modal;
    let next_owner = owner_relationship(&binding);
    let next = binding.clone();
    drop(binding);

    // apply the host transition and rollback the cached binding on failure
    if let Err(error) = apply_host_relationship_state(
        &runtime_state,
        window_handle,
        previous_owner,
        previous_modal,
        next_owner,
        modal,
        "destack.display.window.setModal",
    ) {
        let binding = display_resource::resolve_window_binding(
            context,
            window_handle,
            "destack.display.window.setModal.rollback",
        )?;
        let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
        *binding = previous.clone();
        return Err(error);
    }

    publish_state_deltas(&runtime_state, window_handle, &previous, &next);

    Ok(())
}

/// Set one window mouse-passthrough state.
pub(crate) unsafe fn window_set_mouse_passthrough(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    passthrough: bool,
) -> RuntimeResult<()> {
    let runtime_state = appkit_core::runtime_state(context);
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setMousePassthrough",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    runtime::ensure_window_thread(&binding, "destack.display.window.setMousePassthrough")?;
    let previous = binding.clone();
    binding.mouse_passthrough = passthrough;
    let next = binding.clone();
    drop(binding);

    appkit_core::with_window_host(
        &runtime_state,
        window_handle,
        "destack.display.window.setMousePassthrough",
        |host| {
            host.window.setIgnoresMouseEvents(passthrough);
            Ok(())
        },
    )?;

    publish_state_deltas(&runtime_state, window_handle, &previous, &next);

    Ok(())
}
