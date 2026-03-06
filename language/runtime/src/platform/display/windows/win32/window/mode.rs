use crate::diagnostic::RuntimeResult;
use crate::platform::display::{DisplayMode, WindowModeOptions};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::super::constants::*;
use super::super::model::{ExclusiveModeRestore, Win32WindowBinding};
use super::super::{core, event, monitor, resource as display_resource};
use super::{apply_window_rect, apply_window_style};

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
    // resolve this variant
    match mode {
        WindowModeOptions::WindowWindowedModeOptions(_) => None,
        WindowModeOptions::WindowBorderlessModeOptions(options) => options.display,
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(options) => Some(options.display),
    }
}

/// Resolve one mode payload to its optional preferred exclusive mode.
pub(crate) fn mode_display_mode(mode: WindowModeOptions) -> Option<DisplayMode> {
    // resolve this variant
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
    // evaluate this condition
    if let Some(display) = mode_display(mode) {
        return Some(display);
    }

    // evaluate this condition
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

    // evaluate this condition
    if emit_events {
        event::publish_mode_changed_event(context, &restore.display_id, restore.mode);

        // evaluate this condition
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
    binding: &mut Win32WindowBinding,
    mode: WindowModeOptions,
    operation: &'static str,
    emit_monitor_events: bool,
) -> RuntimeResult<()> {
    let target_display = mode_target_display(mode, binding.display);

    // evaluate this condition
    if is_exclusive_mode(mode) {
        let Some(display) = target_display else {
            return Err(core_platform::invalid_argument(
                "mode.display",
                "exclusive fullscreen requires one display target",
            ));
        };

        let display_id = display_resource::resolve_display_id(context, display, operation)?;

        // evaluate this condition
        if let Some(restore) = binding.exclusive_restore.as_ref()
            && restore.display_id != display_id
        {
            let restore = restore.clone();
            restore_exclusive_mode(context, &restore, operation, emit_monitor_events)?;
            binding.exclusive_restore = None;
        }

        // evaluate this condition
        if binding.exclusive_restore.is_none() {
            let snapshot = monitor::monitor_snapshot_by_id(&display_id)?.ok_or_else(|| {
                core_platform::io_not_found(
                    operation,
                    format!("display id '{display_id}' is no longer available"),
                )
            })?;

            binding.exclusive_restore = Some(ExclusiveModeRestore {
                display_id: display_id.clone(),
                mode: snapshot.current_mode,
            });
        }

        // evaluate this condition
        if let Some(display_mode) = mode_display_mode(mode) {
            monitor::apply_monitor_mode_by_id(&display_id, display_mode, operation)?;

            // evaluate this condition
            if emit_monitor_events {
                event::publish_mode_changed_event(context, &display_id, display_mode);

                // evaluate this condition
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
    } else if let Some(restore) = binding.exclusive_restore.clone() {
        restore_exclusive_mode(context, &restore, operation, emit_monitor_events)?;
        binding.exclusive_restore = None;
    }

    binding.mode = mode;
    binding.display = target_display;
    apply_window_style(binding, operation)?;

    // evaluate this condition
    if let Some(rectangle) =
        monitor::mode_target_rect(context, mode, binding.display, binding.hwnd, operation)?
    {
        apply_window_rect(binding, rectangle, operation)?;
    }

    event::refresh_monitor_topology_cache(context)?;
    Ok(())
}
