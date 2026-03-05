use wayland_client::WEnum;
use wayland_client::protocol::wl_output;

use crate::platform::display::DisplayOrientation;

use super::DISPLAY_ID_PREFIX;

/// Return the numeric output global name encoded in one display id.
pub(super) fn output_global_name_from_display_id(display_id: &str) -> Option<u32> {
    let global_name = display_id.strip_prefix(DISPLAY_ID_PREFIX)?;
    global_name.parse::<u32>().ok()
}

/// Return one display orientation from one wl_output transform.
pub(super) fn orientation_from_transform(
    transform: WEnum<wl_output::Transform>,
) -> DisplayOrientation {
    match transform {
        WEnum::Value(wl_output::Transform::Normal) => DisplayOrientation::Landscape,
        WEnum::Value(wl_output::Transform::_90) => DisplayOrientation::Portrait,
        WEnum::Value(wl_output::Transform::_180) => DisplayOrientation::LandscapeFlipped,
        WEnum::Value(wl_output::Transform::_270) => DisplayOrientation::PortraitFlipped,
        WEnum::Value(wl_output::Transform::Flipped) => DisplayOrientation::Landscape,
        WEnum::Value(wl_output::Transform::Flipped90) => DisplayOrientation::Portrait,
        WEnum::Value(wl_output::Transform::Flipped180) => DisplayOrientation::LandscapeFlipped,
        WEnum::Value(wl_output::Transform::Flipped270) => DisplayOrientation::PortraitFlipped,
        _ => DisplayOrientation::Landscape,
    }
}

/// Return mode-current and mode-preferred flags from one mode flag payload.
pub(super) fn parse_mode_flags(flags: WEnum<wl_output::Mode>) -> (bool, bool) {
    match flags {
        WEnum::Value(value) => (
            value.contains(wl_output::Mode::Current),
            value.contains(wl_output::Mode::Preferred),
        ),
        WEnum::Unknown(value) => {
            let current = (value & 0x2) != 0;
            let preferred = (value & 0x1) != 0;
            (current, preferred)
        }
    }
}
