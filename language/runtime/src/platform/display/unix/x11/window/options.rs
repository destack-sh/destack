use crate::platform::display::{WindowChromeKind, WindowRole};

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
