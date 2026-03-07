use crate::diagnostic::RuntimeResult;
use crate::platform::display::{WindowModeOptions, WindowOptions, WindowRole, WindowVisibility};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::{mode_display, mode_display_mode};
use crate::platform::display::unix::wayland::monitor;

/// Resolve role-specific defaults for one wayland window open request.
pub(crate) fn resolve_role_open_defaults(
    role: WindowRole,
    taskbar_visible: bool,
    always_on_top: bool,
) -> (bool, bool) {
    // keep explicit caller values for top-level windows
    if role == WindowRole::Toplevel {
        return (taskbar_visible, always_on_top);
    }

    // popup and overlay roles are transient or layered surfaces by construction
    if role == WindowRole::Popup {
        return (false, false);
    }

    (false, true)
}

/// Validate unsupported open-option lanes for this wayland backend.
pub(crate) fn validate_unsupported_open_options(options: WindowOptions) -> RuntimeResult<()> {
    // reject hidden open visibility: xdg-shell has no portable hidden lane
    if options.visibility == WindowVisibility::Hidden {
        return Err(core_platform::not_supported("destack.display.window.open"));
    }

    // reject explicit desktop placement requests: wayland toplevel placement is compositor-owned
    if options.position.is_some() {
        return Err(core_platform::not_supported("destack.display.window.open"));
    }

    // reject explicit always-on-top requests for non-overlay roles
    if options.always_on_top && options.role != WindowRole::Overlay {
        return Err(core_platform::not_supported("destack.display.window.open"));
    }

    // reject explicit taskbar policy requests for top-level roles
    if !options.taskbar_visible && options.role == WindowRole::Toplevel {
        return Err(core_platform::not_supported("destack.display.window.open"));
    }

    // reject initial aspect-ratio locks: no standard xdg-shell request lane exists
    if let Some(aspect_ratio) = options.aspect_ratio {
        if aspect_ratio.numerator == 0 || aspect_ratio.denominator == 0 {
            return Err(core_platform::invalid_argument(
                "options.aspectRatio",
                "aspect ratio numerator and denominator must be greater than zero",
            ));
        }

        return Err(core_platform::not_supported("destack.display.window.open"));
    }

    Ok(())
}

/// Resolve one scale-factor value from one optional display selection.
pub(crate) fn resolve_scale_factor_milli(
    context: &BindingCallContext,
    display: Option<resource::DisplayHandle>,
    operation: &'static str,
) -> RuntimeResult<u32> {
    // return display-specific scale when display handle is configured
    if let Some(display) = display {
        let snapshot = monitor::snapshot_by_display_handle(context, display, operation)?;
        return Ok(snapshot.descriptor.scale_factor_milli.max(1));
    }

    // otherwise use primary-monitor scale when available
    let snapshots = monitor::enumerate_monitor_snapshots(context)?;
    if let Some(snapshot) = snapshots
        .iter()
        .find(|snapshot| snapshot.descriptor.primary)
    {
        return Ok(snapshot.descriptor.scale_factor_milli.max(1));
    }

    Ok(1000)
}

/// Resolve one initial display selection from window options.
pub(crate) fn resolve_initial_display(
    options: WindowOptions,
) -> RuntimeResult<Option<resource::DisplayHandle>> {
    let mode_display = mode_display(options.mode);

    // reject conflicting explicit display selections
    if let (Some(display), Some(mode_display)) = (options.display, mode_display)
        && display != mode_display
    {
        return Err(core_platform::invalid_argument(
            "options",
            "options.display must match options.mode display when both are set",
        ));
    }

    Ok(mode_display.or(options.display))
}

/// Validate one initial mode payload.
pub(crate) fn validate_initial_mode(
    context: &BindingCallContext,
    mode: WindowModeOptions,
    display: Option<resource::DisplayHandle>,
    operation: &'static str,
) -> RuntimeResult<()> {
    // reject exclusive fullscreen on wayland: this backend only supports compositor fullscreen
    if matches!(
        mode,
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(_)
    ) {
        return Err(core_platform::not_supported(operation));
    }

    // resolve display handle when configured
    let Some(display) = display else {
        return Ok(());
    };
    let snapshot = monitor::snapshot_by_display_handle(context, display, operation)?;

    // validate optional exclusive mode payload against host-reported modes
    let Some(display_mode) = mode_display_mode(mode) else {
        return Ok(());
    };

    if !snapshot.modes.contains(&display_mode) {
        return Err(core_platform::invalid_argument(
            "mode",
            "display mode is not supported by the target display",
        ));
    }

    Ok(())
}
