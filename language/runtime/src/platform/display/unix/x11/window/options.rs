use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::display::{WindowChromeKind, WindowOptions, WindowRole};

use super::geometry;

/// Resolve role-specific open defaults for one window open request.
pub(crate) fn resolve_role_open_defaults(
    role: WindowRole,
    chrome: WindowChromeKind,
    decorated: bool,
    taskbar_visible: bool,
    always_on_top: bool,
) -> (WindowChromeKind, bool, bool, bool) {
    // keep explicit caller values for top-level windows
    if role == WindowRole::Toplevel {
        return (chrome, decorated, taskbar_visible, always_on_top);
    }

    // popup role defaults to popup chrome and hidden taskbar presence
    if role == WindowRole::Popup {
        let resolved_chrome = if chrome == WindowChromeKind::Standard {
            WindowChromeKind::Popup
        } else {
            chrome
        };

        return (resolved_chrome, decorated, false, always_on_top);
    }

    // overlay role defaults to popup chrome, undecorated, topmost, and hidden taskbar presence
    (WindowChromeKind::Popup, false, false, true)
}

/// Validate one x11 window-open request.
pub(crate) fn validate_open_options(options: WindowOptions, opacity: f64) -> RuntimeResult<()> {
    // reject invalid logical sizes eagerly
    if options.size_logical.width <= 0.0 || options.size_logical.height <= 0.0 {
        return Err(core_platform::invalid_argument(
            "sizeLogical",
            "window logical width and height must be greater than zero",
        ));
    }

    // reject invalid opacity payloads
    if !(0.0..=1.0).contains(&opacity) {
        return Err(core_platform::invalid_argument(
            "opacity",
            "window opacity must be between 0.0 and 1.0",
        ));
    }

    // popup windows require one owner relationship
    if options.role == WindowRole::Popup
        && options.transient_for.is_none()
        && options.parent.is_none()
    {
        return Err(core_platform::invalid_argument(
            "role",
            "popup windows require transientFor or parent to be set",
        ));
    }

    // modal windows also require one owner relationship
    if options.modal == Some(true) && options.transient_for.is_none() && options.parent.is_none() {
        return Err(core_platform::invalid_argument(
            "modal",
            "modal windows require transientFor or parent to be set",
        ));
    }

    // reject invalid aspect-ratio payloads
    if let Some(aspect_ratio) = options.aspect_ratio
        && (aspect_ratio.numerator == 0 || aspect_ratio.denominator == 0)
    {
        return Err(core_platform::invalid_argument(
            "aspectRatio",
            "aspect ratio numerator and denominator must both be greater than zero",
        ));
    }

    geometry::validate_size_constraints(options.constraints, "destack.display.window.open")
}
