use crate::diagnostic::RuntimeResult;
use crate::platform::display::WindowVisibility;
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::super::{core as appkit_core, resource as display_resource};
use super::cursor::apply_cursor_policy;
use super::{drop, reconcile, runtime};

/// Close one window.
pub(crate) unsafe fn window_close(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    let runtime_state = appkit_core::runtime_state(context);
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.close",
    )?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    runtime::ensure_window_thread(&binding, "destack.display.window.close")?;
    drop(binding);

    appkit_core::with_main_thread_state(&runtime_state, |state| {
        let mut state = state.borrow_mut();

        // cancel any active drop session before dropping the host window
        if let Some(host) = state.windows.remove(&window_handle) {
            drop::cancel_active_drop_session(&runtime_state, window_handle, &host);
            host.window.unregisterDraggedTypes();
            host.window.close();
        }
    });

    apply_cursor_policy(&runtime_state);

    let removed = context
        .agent()
        .resources
        .remove(window_handle.0, Some(context.engine()))
        .is_some();

    // report unknown handles after resource removal
    if !removed {
        return Err(core_platform::io_not_found(
            "destack.display.window.close",
            format!("window handle {} was not found", window_handle.0.0),
        ));
    }

    Ok(())
}

/// Minimize one window.
pub(crate) unsafe fn window_minimize(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    let runtime_state = appkit_core::runtime_state(context);
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.minimize",
    )?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    runtime::ensure_window_thread(&binding, "destack.display.window.minimize")?;
    drop(binding);

    appkit_core::with_window_host(
        &runtime_state,
        window_handle,
        "destack.display.window.minimize",
        |host| {
            host.window.miniaturize(None);
            Ok(())
        },
    )?;

    reconcile::reconcile_host_window_state_with_visibility(
        &runtime_state,
        window_handle,
        Some(WindowVisibility::Minimized),
    )?;

    Ok(())
}

/// Maximize one window.
pub(crate) unsafe fn window_maximize(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    let runtime_state = appkit_core::runtime_state(context);
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.maximize",
    )?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    runtime::ensure_window_thread(&binding, "destack.display.window.maximize")?;
    drop(binding);

    appkit_core::with_window_host(
        &runtime_state,
        window_handle,
        "destack.display.window.maximize",
        |host| {
            if host.window.isMiniaturized() {
                host.window.deminiaturize(None);
            }

            if !host.window.isZoomed() {
                host.window.zoom(None);
            }
            Ok(())
        },
    )?;

    reconcile::reconcile_host_window_state_with_visibility(
        &runtime_state,
        window_handle,
        Some(WindowVisibility::Maximized),
    )?;

    Ok(())
}

/// Restore one window.
pub(crate) unsafe fn window_restore(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    let runtime_state = appkit_core::runtime_state(context);
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.restore",
    )?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    runtime::ensure_window_thread(&binding, "destack.display.window.restore")?;
    let restore_visibility = binding.restored_visibility;
    drop(binding);

    appkit_core::with_window_host(
        &runtime_state,
        window_handle,
        "destack.display.window.restore",
        |host| {
            if host.window.isMiniaturized() {
                host.window.deminiaturize(None);
            }

            if host.window.isZoomed() {
                host.window.zoom(None);
            }

            Ok(())
        },
    )?;

    reconcile::reconcile_host_window_state_with_visibility(
        &runtime_state,
        window_handle,
        Some(restore_visibility),
    )?;

    Ok(())
}
