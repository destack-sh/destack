use crate::diagnostic::RuntimeResult;
use crate::platform::display::{DisplayMode, WindowModeOptions};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::core::{apply_window_rect, apply_window_style};
use crate::platform::display::windows::win32::model::{ExclusiveModeRestore, Win32WindowHostState};
use crate::platform::display::windows::win32::{
    core, event, monitor, resource as display_resource,
};

/// Return whether one mode payload resolves to windowed.
pub(crate) fn is_windowed_mode(mode: WindowModeOptions) -> bool {
    matches!(mode, WindowModeOptions::WindowWindowedModeOptions(_))
}

/// Return whether one mode payload resolves to exclusive fullscreen.
pub(crate) fn is_exclusive_mode(mode: WindowModeOptions) -> bool {
    matches!(
        mode,
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(_)
    )
}

/// Resolve one mode payload to its optional preferred display handle.
pub(crate) fn mode_display(mode: WindowModeOptions) -> Option<resource::DisplayHandle> {
    match mode {
        WindowModeOptions::WindowWindowedModeOptions(_) => None,
        WindowModeOptions::WindowBorderlessModeOptions(options) => options.display,
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(options) => Some(options.display),
    }
}

/// Resolve one mode payload to its optional preferred exclusive mode.
pub(crate) fn mode_display_mode(mode: WindowModeOptions) -> Option<DisplayMode> {
    match mode {
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(options) => options.display_mode,
        _ => None,
    }
}

/// Compare two mode payloads by semantic fields, excluding discriminator string handles.
pub(crate) fn same_window_mode(left: WindowModeOptions, right: WindowModeOptions) -> bool {
    // compare mode payload by semantic fields
    match (left, right) {
        (
            WindowModeOptions::WindowWindowedModeOptions(_),
            WindowModeOptions::WindowWindowedModeOptions(_),
        ) => true,
        (
            WindowModeOptions::WindowBorderlessModeOptions(left),
            WindowModeOptions::WindowBorderlessModeOptions(right),
        ) => left.display == right.display,
        (
            WindowModeOptions::WindowExclusiveFullscreenModeOptions(left),
            WindowModeOptions::WindowExclusiveFullscreenModeOptions(right),
        ) => left.display == right.display && left.display_mode == right.display_mode,
        _ => false,
    }
}

/// Resolve one monitor handle to one display identifier for exclusive-fullscreen updates.
fn mode_target_display(
    mode: WindowModeOptions,
    current_display: Option<resource::DisplayHandle>,
) -> Option<resource::DisplayHandle> {
    if let Some(display) = mode_display(mode) {
        return Some(display);
    }

    if is_exclusive_mode(mode) {
        return current_display;
    }

    current_display
}

/// Restore one captured exclusive-fullscreen display mode snapshot.
pub(crate) fn restore_exclusive_mode(
    context: &BindingCallContext,
    restore: &ExclusiveModeRestore,
    operation: &'static str,
    emit_events: bool,
) -> RuntimeResult<()> {
    monitor::apply_monitor_mode_by_id(&restore.display_id, restore.mode, operation)?;

    if emit_events {
        event::publish_mode_changed_event(context, &restore.display_id, restore.mode);

        if let Some(snapshot) = monitor::monitor_snapshot_by_id(&restore.display_id)? {
            event::publish_descriptor_changed_event(
                context,
                &snapshot.descriptor,
                core::DISPLAY_CHANGED_MASK_BOUNDS
                    | core::DISPLAY_CHANGED_MASK_WORKAREA
                    | core::DISPLAY_CHANGED_MASK_SCALE
                    | core::DISPLAY_CHANGED_MASK_ORIENTATION,
            );
        }
    }

    event::refresh_monitor_topology_cache(context)?;
    Ok(())
}

/// Apply one window-mode transition and associated exclusive-display state changes.
pub(crate) fn apply_mode_options(
    context: &BindingCallContext,
    host_state: &mut Win32WindowHostState,
    mode: WindowModeOptions,
    operation: &'static str,
    emit_monitor_events: bool,
) -> RuntimeResult<()> {
    let target_display = mode_target_display(mode, host_state.display);

    if is_exclusive_mode(mode) {
        let Some(display) = target_display else {
            return Err(core_platform::invalid_argument(
                "mode.display",
                "exclusive fullscreen requires one display target",
            ));
        };

        let display_id = display_resource::resolve_display_id(context, display, operation)?;

        if let Some(restore) = host_state.exclusive_restore.as_ref()
            && restore.display_id != display_id
        {
            let restore = restore.clone();
            restore_exclusive_mode(context, &restore, operation, emit_monitor_events)?;
            host_state.exclusive_restore = None;
        }

        if host_state.exclusive_restore.is_none() {
            let snapshot = monitor::monitor_snapshot_by_id(&display_id)?.ok_or_else(|| {
                core_platform::io_not_found(
                    operation,
                    format!("display id '{display_id}' is no longer available"),
                )
            })?;

            host_state.exclusive_restore = Some(ExclusiveModeRestore {
                display_id: display_id.clone(),
                mode: snapshot.current_mode,
            });
        }

        if let Some(display_mode) = mode_display_mode(mode) {
            monitor::apply_monitor_mode_by_id(&display_id, display_mode, operation)?;

            if emit_monitor_events {
                event::publish_mode_changed_event(context, &display_id, display_mode);

                if let Some(snapshot) = monitor::monitor_snapshot_by_id(&display_id)? {
                    event::publish_descriptor_changed_event(
                        context,
                        &snapshot.descriptor,
                        core::DISPLAY_CHANGED_MASK_BOUNDS
                            | core::DISPLAY_CHANGED_MASK_WORKAREA
                            | core::DISPLAY_CHANGED_MASK_SCALE
                            | core::DISPLAY_CHANGED_MASK_ORIENTATION,
                    );
                }
            }
        }
    } else if let Some(restore) = host_state.exclusive_restore.clone() {
        restore_exclusive_mode(context, &restore, operation, emit_monitor_events)?;
        host_state.exclusive_restore = None;
    }

    host_state.mode = mode;
    host_state.display = target_display;
    apply_window_style(host_state, operation)?;

    if let Some(rectangle) = monitor::mode_target_rect(
        context,
        mode,
        host_state.display,
        host_state.hwnd,
        operation,
    )? {
        apply_window_rect(host_state, rectangle, operation)?;
    }

    event::refresh_monitor_topology_cache(context)?;
    Ok(())
}
