#[cfg(target_os = "macos")]
use crate::platform::audio::core as audio_core;

#[cfg(target_os = "macos")]
use super::abi::{
    AudioObjectPropertyAddress, AudioObjectPropertyScope, AudioObjectPropertySelector,
};
#[cfg(target_os = "macos")]
use super::constants::{
    K_AUDIO_DEVICE_TRANSPORT_TYPE_AGGREGATE, K_AUDIO_DEVICE_TRANSPORT_TYPE_AIRPLAY,
    K_AUDIO_DEVICE_TRANSPORT_TYPE_AVB, K_AUDIO_DEVICE_TRANSPORT_TYPE_BLUETOOTH,
    K_AUDIO_DEVICE_TRANSPORT_TYPE_BLUETOOTH_LE, K_AUDIO_DEVICE_TRANSPORT_TYPE_BUILT_IN,
    K_AUDIO_DEVICE_TRANSPORT_TYPE_CONTINUITY_CAPTURE_WIRED,
    K_AUDIO_DEVICE_TRANSPORT_TYPE_CONTINUITY_CAPTURE_WIRELESS,
    K_AUDIO_DEVICE_TRANSPORT_TYPE_DISPLAY_PORT, K_AUDIO_DEVICE_TRANSPORT_TYPE_FIREWIRE,
    K_AUDIO_DEVICE_TRANSPORT_TYPE_HDMI, K_AUDIO_DEVICE_TRANSPORT_TYPE_PCI,
    K_AUDIO_DEVICE_TRANSPORT_TYPE_THUNDERBOLT, K_AUDIO_DEVICE_TRANSPORT_TYPE_USB,
    K_AUDIO_DEVICE_TRANSPORT_TYPE_VIRTUAL, K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
    K_AUDIO_OBJECT_PROPERTY_SCOPE_INPUT, K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT,
};

#[cfg(target_os = "macos")]
pub(super) const fn fourcc(code: [u8; 4]) -> u32 {
    ((code[0] as u32) << 24) | ((code[1] as u32) << 16) | ((code[2] as u32) << 8) | code[3] as u32
}

/// Build one CoreAudio property address for one selector and scope.
#[cfg(target_os = "macos")]
pub(super) fn property_address(
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress {
        selector,
        scope,
        element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
    }
}

/// Return one normalized transport name for one CoreAudio transport type.
#[cfg(target_os = "macos")]
pub(super) fn transport_name(transport_type: u32) -> &'static str {
    match transport_type {
        K_AUDIO_DEVICE_TRANSPORT_TYPE_BUILT_IN => "builtin",
        K_AUDIO_DEVICE_TRANSPORT_TYPE_AGGREGATE => "aggregate",
        K_AUDIO_DEVICE_TRANSPORT_TYPE_VIRTUAL => "virtual",
        K_AUDIO_DEVICE_TRANSPORT_TYPE_PCI => "pci",
        K_AUDIO_DEVICE_TRANSPORT_TYPE_USB => "usb",
        K_AUDIO_DEVICE_TRANSPORT_TYPE_FIREWIRE => "firewire",
        K_AUDIO_DEVICE_TRANSPORT_TYPE_BLUETOOTH => "bluetooth",
        K_AUDIO_DEVICE_TRANSPORT_TYPE_BLUETOOTH_LE => "bluetoothle",
        K_AUDIO_DEVICE_TRANSPORT_TYPE_HDMI => "hdmi",
        K_AUDIO_DEVICE_TRANSPORT_TYPE_DISPLAY_PORT => "displayport",
        K_AUDIO_DEVICE_TRANSPORT_TYPE_AIRPLAY => "airplay",
        K_AUDIO_DEVICE_TRANSPORT_TYPE_AVB => "avb",
        K_AUDIO_DEVICE_TRANSPORT_TYPE_THUNDERBOLT => "thunderbolt",
        K_AUDIO_DEVICE_TRANSPORT_TYPE_CONTINUITY_CAPTURE_WIRED => "continuitycapturewired",
        K_AUDIO_DEVICE_TRANSPORT_TYPE_CONTINUITY_CAPTURE_WIRELESS => "continuitycapturewireless",
        _ => "coreaudio",
    }
}

/// Return one canonical channel layout for one channel count.
#[cfg(target_os = "macos")]
pub(super) fn channel_layout(channels: u16) -> audio_core::AudioChannelLayout {
    match channels {
        1 => audio_core::AudioChannelLayout::Mono,
        2 => audio_core::AudioChannelLayout::Stereo,
        4 => audio_core::AudioChannelLayout::Quad,
        5 => audio_core::AudioChannelLayout::Surround41,
        6 => audio_core::AudioChannelLayout::Surround51,
        7 => audio_core::AudioChannelLayout::Surround61,
        8 => audio_core::AudioChannelLayout::Surround71,
        _ => audio_core::AudioChannelLayout::Unknown,
    }
}

/// Return one fallback packed channel mask for one channel count.
#[cfg(target_os = "macos")]
pub(super) fn channel_mask(channels: u16) -> u64 {
    if channels == 0 {
        return 0;
    }

    if channels >= 64 {
        return u64::MAX;
    }

    (1u64 << channels) - 1
}

/// Return one CoreAudio scope for one runtime audio direction.
#[cfg(target_os = "macos")]
pub(super) fn scope_for_direction(
    direction: audio_core::AudioDeviceDirection,
) -> AudioObjectPropertyScope {
    match direction {
        audio_core::AudioDeviceDirection::Capture => K_AUDIO_OBJECT_PROPERTY_SCOPE_INPUT,
        audio_core::AudioDeviceDirection::Loopback => K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT,
        audio_core::AudioDeviceDirection::Playback | audio_core::AudioDeviceDirection::Duplex => {
            K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT
        }
    }
}

/// Derive one runtime direction from playback and capture channel counts.
#[cfg(target_os = "macos")]
pub(super) fn direction_from_channels(
    playback_channels: u16,
    capture_channels: u16,
) -> Option<audio_core::AudioDeviceDirection> {
    if playback_channels > 0 && capture_channels > 0 {
        return Some(audio_core::AudioDeviceDirection::Duplex);
    }

    if playback_channels > 0 {
        return Some(audio_core::AudioDeviceDirection::Playback);
    }

    if capture_channels > 0 {
        return Some(audio_core::AudioDeviceDirection::Capture);
    }

    None
}
