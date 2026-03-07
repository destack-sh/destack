use std::sync::Arc;

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSWindowOrderingMode};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::display::WindowRole;
use crate::platform::resource::WindowHandle;
use crate::runtime::BindingCallContext;

use crate::platform::display::unix::appkit::event::publish_state_deltas;
use crate::platform::display::unix::appkit::model::AppKitWindowHostState;
use crate::platform::display::unix::appkit::{core as appkit_core, resource as display_resource};

/// Return one effective owner relationship with transient priority.
fn owner_relationship(host_state: &AppKitWindowHostState) -> Option<WindowHandle> {
    host_state.transient_for.or(host_state.parent)
}

/// Resolve one owner handle.
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

    // reject self-reference relationships at the host_state boundary
    if owner_window == child_window {
        return Err(core_platform::invalid_argument(
            field,
            format!("{operation}: owner window must differ from source window"),
        ));
    }

    let owner_host_state =
        display_resource::resolve_window_host_state(context, owner_window, operation)?;
    drop(
        owner_host_state
            .lock()
            .unwrap_or_else(|error| error.into_inner()),
    );

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
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setParent",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // snapshot the transition before mutating the host_state
    let previous = host_state.clone();
    let previous_owner = owner_relationship(&host_state);
    let previous_modal = host_state.modal;

    // apply the requested relation in the cached host_state
    host_state.parent = parent;
    host_state.transient_for = None;

    // clear modal state when removing the final owner lane
    if owner_relationship(&host_state).is_none() {
        host_state.modal = false;
    }

    let next_owner = owner_relationship(&host_state);
    let next_modal = host_state.modal;
    let next = host_state.clone();
    drop(host_state);

    // apply the host transition and rollback the cached host_state on failure
    if let Err(error) = apply_host_relationship_state(
        &runtime_state,
        window_handle,
        previous_owner,
        previous_modal,
        next_owner,
        next_modal,
        "destack.display.window.setParent",
    ) {
        let host_state = display_resource::resolve_window_host_state(
            context,
            window_handle,
            "destack.display.window.setParent.rollback",
        )?;
        let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
        *host_state = previous.clone();
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
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setTransientFor",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // snapshot the transition before mutating the host_state
    let previous = host_state.clone();
    let previous_owner = owner_relationship(&host_state);
    let previous_modal = host_state.modal;

    // apply the requested relation in the cached host_state
    host_state.parent = None;
    host_state.transient_for = transient_for;

    // clear modal state when removing the final owner lane
    if owner_relationship(&host_state).is_none() {
        host_state.modal = false;
    }

    let next_owner = owner_relationship(&host_state);
    let next_modal = host_state.modal;
    let next = host_state.clone();
    drop(host_state);

    // apply the host transition and rollback the cached host_state on failure
    if let Err(error) = apply_host_relationship_state(
        &runtime_state,
        window_handle,
        previous_owner,
        previous_modal,
        next_owner,
        next_modal,
        "destack.display.window.setTransientFor",
    ) {
        let host_state = display_resource::resolve_window_host_state(
            context,
            window_handle,
            "destack.display.window.setTransientFor.rollback",
        )?;
        let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
        *host_state = previous.clone();
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
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setModal",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // reject modal transitions for non-toplevel roles
    if host_state.role != WindowRole::Toplevel {
        return Err(core_platform::invalid_argument(
            "modal",
            "modal windows require the toplevel role on AppKit",
        ));
    }

    // reject modal requests without any owner lane
    if modal && owner_relationship(&host_state).is_none() {
        return Err(core_platform::invalid_argument(
            "modal",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // skip exact no-op transitions
    if host_state.modal == modal {
        return Ok(());
    }

    let previous = host_state.clone();
    let previous_owner = owner_relationship(&host_state);
    let previous_modal = host_state.modal;
    host_state.modal = modal;
    let next_owner = owner_relationship(&host_state);
    let next = host_state.clone();
    drop(host_state);

    // apply the host transition and rollback the cached host_state on failure
    if let Err(error) = apply_host_relationship_state(
        &runtime_state,
        window_handle,
        previous_owner,
        previous_modal,
        next_owner,
        modal,
        "destack.display.window.setModal",
    ) {
        let host_state = display_resource::resolve_window_host_state(
            context,
            window_handle,
            "destack.display.window.setModal.rollback",
        )?;
        let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
        *host_state = previous.clone();
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
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setMousePassthrough",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let previous = host_state.clone();
    host_state.mouse_passthrough = passthrough;
    let next = host_state.clone();
    drop(host_state);

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
