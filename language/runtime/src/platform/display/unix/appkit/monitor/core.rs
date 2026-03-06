use objc2_core_graphics::{CGDirectDisplayID, CGDisplayMode, CGDisplayRotation};

use crate::platform::display::{DisplayMode, DisplayOrientation};

use super::super::core;

/// Return one stable display id for one CoreGraphics display lane.
pub(crate) fn display_id(display: CGDirectDisplayID) -> String {
    format!("{}{display}", core::DISPLAY_ID_PREFIX)
}

/// Parse one CoreGraphics display identifier from one stable display id.
pub(crate) fn display_from_id(value: &str) -> Option<CGDirectDisplayID> {
    let suffix = value.strip_prefix(core::DISPLAY_ID_PREFIX)?;
    suffix.parse::<u32>().ok()
}

/// Resolve one orientation from one display rotation value.
pub(crate) fn orientation_from_rotation(rotation_degrees: f64) -> DisplayOrientation {
    let rounded = rotation_degrees.round() as i32;

    // map quarter-turn rotation to portrait
    if rounded == 90 {
        return DisplayOrientation::Portrait;
    }

    // map half-turn rotation to landscape flipped
    if rounded == 180 {
        return DisplayOrientation::LandscapeFlipped;
    }

    // map three-quarter-turn rotation to portrait flipped
    if rounded == 270 {
        return DisplayOrientation::PortraitFlipped;
    }

    DisplayOrientation::Landscape
}

/// Convert one native display mode into one ABI display mode.
pub(crate) fn display_mode_from_native(mode: &CGDisplayMode) -> DisplayMode {
    let refresh_hz = CGDisplayMode::refresh_rate(Some(mode));
    let refresh_milli_hz = if refresh_hz > 0.0 {
        (refresh_hz * 1000.0).round() as u32
    } else {
        60_000
    };

    DisplayMode {
        width: CGDisplayMode::pixel_width(Some(mode)) as u32,
        height: CGDisplayMode::pixel_height(Some(mode)) as u32,
        refresh_milli_hz,
        format: 0,
        bit_depth: 32,
    }
}

/// Resolve one current display orientation for one CoreGraphics display.
pub(crate) fn display_orientation(display: CGDirectDisplayID) -> DisplayOrientation {
    orientation_from_rotation(CGDisplayRotation(display))
}
