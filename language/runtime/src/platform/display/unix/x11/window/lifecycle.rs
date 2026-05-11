use std::sync::Arc;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::ConnectionExt as XprotoConnectionExt;

use crate::diagnostic::RuntimeResult;
use crate::platform::display::WindowModeOptions;
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::cursor;
use crate::platform::display::unix::x11::model::ExclusiveModeRestore;
use crate::platform::display::unix::x11::{core, event, monitor, resource as display_resource};

/// Apply one exclusive fullscreen monitor mode request and return restore metadata.
pub(crate) fn apply_exclusive_mode(
    context: &BindingCallContext,
    runtime_state: &Arc<core::X11RuntimeState>,
    mode: WindowModeOptions,
    operation: &'static str,
) -> RuntimeResult<Option<ExclusiveModeRestore>> {
    // skip monitor mode mutation when this mode is not exclusive fullscreen
    let Some(display) = super::mode_display(mode) else {
        return Ok(None);
    };
    let Some(requested_mode) = super::mode_display_mode(mode) else {
        return Ok(None);
    };

    // resolve one stable display id and current monitor mode
    let display_id = display_resource::resolve_display_id(context, display, operation)?;
    let Some(snapshot) = monitor::monitor_snapshot_by_display_id(context, &display_id)? else {
        return Err(core_platform::io_not_found(
            operation,
            format!("display id '{display_id}' is no longer available"),
        ));
    };
    let previous_mode = snapshot.current_mode;
    if previous_mode == requested_mode {
        return Ok(None);
    }

    // apply one mode transition and publish monitor mode change
    let current_mode =
        monitor::apply_monitor_mode_by_display_id(context, &display_id, requested_mode, operation)?;
    let current_snapshot = monitor::monitor_snapshot_by_display_id(context, &display_id)?;
    event::publish_monitor_mode_changed(
        runtime_state,
        &display_id,
        Some(previous_mode),
        current_mode,
    );
    if let Some(current_snapshot) = current_snapshot
        && current_snapshot.descriptor != snapshot.descriptor
    {
        event::publish_monitor_descriptor_changed(
            runtime_state,
            Some(snapshot.descriptor),
            current_snapshot.descriptor,
        );
    }
    event::refresh_monitor_topology_cache(context)?;

    Ok(Some(ExclusiveModeRestore {
        display_id,
        previous_mode,
    }))
}

/// Restore one monitor mode captured by one exclusive fullscreen transition.
pub(crate) fn restore_exclusive_mode(
    context: &BindingCallContext,
    runtime_state: &Arc<core::X11RuntimeState>,
    restore: &ExclusiveModeRestore,
    operation: &'static str,
) -> RuntimeResult<()> {
    // capture previous mode when the display is still available
    let previous_snapshot = monitor::monitor_snapshot_by_display_id(context, &restore.display_id)?;
    let previous_mode = previous_snapshot
        .as_ref()
        .map(|snapshot| snapshot.current_mode);
    let previous_descriptor = previous_snapshot.map(|snapshot| snapshot.descriptor);

    // apply one restore transition and publish monitor mode delta
    let current_mode = monitor::apply_monitor_mode_by_display_id(
        context,
        &restore.display_id,
        restore.previous_mode,
        operation,
    )?;
    let current_snapshot = monitor::monitor_snapshot_by_display_id(context, &restore.display_id)?;
    if previous_mode == Some(current_mode) {
        return Ok(());
    }

    event::publish_monitor_mode_changed(
        runtime_state,
        &restore.display_id,
        previous_mode,
        current_mode,
    );
    if let (Some(previous_descriptor), Some(current_snapshot)) =
        (previous_descriptor, current_snapshot)
        && current_snapshot.descriptor != previous_descriptor
    {
        event::publish_monitor_descriptor_changed(
            runtime_state,
            Some(previous_descriptor),
            current_snapshot.descriptor,
        );
    }
    event::refresh_monitor_topology_cache(context)?;

    Ok(())
}

/// Close one window.
pub(crate) unsafe fn window_close(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve runtime and host_state lanes
    let runtime_state = core::runtime_state(context);
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.close",
    )?;
    let mut binding_snapshot = {
        let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
        host_state.clone()
    };

    // release cursor state and restore any exclusive mode ownership
    let connection_state = core::connection_state(&runtime_state, "destack.display.window.close")?;
    cursor::release_window_cursor(
        connection_state.as_ref(),
        &mut binding_snapshot,
        "destack.display.window.close",
    )?;
    if let Some(restore) = binding_snapshot.exclusive_restore.clone() {
        restore_exclusive_mode(
            context,
            &runtime_state,
            &restore,
            "destack.display.window.close",
        )?;
        binding_snapshot.exclusive_restore = None;
    }

    // destroy the native window when it was not already destroyed by the host
    if !binding_snapshot.destroyed_emitted {
        let cookie = connection_state
            .connection
            .destroy_window(binding_snapshot.window)
            .map_err(|error| {
                core::io_error(
                    "destack.display.window.close",
                    format!("destroy_window failed: {error}"),
                )
            })?;
        cookie.check().map_err(|error| {
            core::io_error(
                "destack.display.window.close",
                format!("destroy_window check failed: {error}"),
            )
        })?;
        connection_state.connection.flush().map_err(|error| {
            core::io_error(
                "destack.display.window.close",
                format!("flush failed: {error}"),
            )
        })?;
    }

    // remove the runtime mapping and resource entry
    runtime_state.unregister_xid(binding_snapshot.window);
    let removed_text_context = runtime_state
        .text_input_contexts
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .remove(&window_handle);
    let removed = context
        .worker()
        .resources
        .remove(context.world(), window_handle.0, Some(context.engine()))
        .is_some();
    if !removed {
        return Err(core::window_not_found(
            "destack.display.window.close",
            window_handle,
        ));
    }

    // publish destroyed exactly once for this close path
    if removed_text_context
        .as_ref()
        .is_some_and(|context| context.is_composing || !context.composition_text.is_empty())
    {
        crate::platform::input::host::notify_x11_window_end_composition(
            &runtime_state,
            window_handle,
        )?;
    }

    runtime_state
        .active_text_sessions
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .remove(&window_handle);

    if !binding_snapshot.destroyed_emitted {
        event::publish_window_destroyed(&runtime_state, window_handle);
    }

    Ok(())
}
