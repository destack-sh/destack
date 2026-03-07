use crate::platform::display::{DisplayMode, WindowModeOptions};
use crate::platform::resource;

/// Resolve one preferred display handle from one mode payload.
pub(crate) fn mode_display(mode: WindowModeOptions) -> Option<resource::DisplayHandle> {
    match mode {
        WindowModeOptions::WindowWindowedModeOptions(_) => None,
        WindowModeOptions::WindowBorderlessModeOptions(value) => value.display,
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(value) => Some(value.display),
    }
}

/// Resolve one preferred display mode from one mode payload.
pub(crate) fn mode_display_mode(mode: WindowModeOptions) -> Option<DisplayMode> {
    match mode {
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(value) => value.display_mode,
        _ => None,
    }
}

/// Compare two mode payloads by semantic fields.
pub(crate) fn same_window_mode(left: WindowModeOptions, right: WindowModeOptions) -> bool {
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
