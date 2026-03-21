use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;
use crate::platform::audio::core::constants::MIN_STREAM_PERIOD_FRAMES;
use crate::platform::core as core_platform;

use super::constants::{
    DEFAULT_MAX_PERIOD_FRAMES, DEFAULT_MAX_SAMPLE_RATE, DEFAULT_MIN_SAMPLE_RATE,
    DEFAULT_PREFERRED_PERIOD_FRAMES, WASAPI_MAX_PROBED_CHANNELS,
};
use super::core::{
    EndpointFlow, WasapiEndpointProfile, collection_count, collection_item,
    create_device_enumerator, default_endpoint_id, endpoint_display_name, endpoint_id,
    enum_audio_endpoints, initialize_com, probe_endpoint_profile,
};
use super::ids::{duplex_stable_id, endpoint_stable_id};

use crate::platform::audio as audio_types;
use windows_sys::Win32::Media::Audio::{
    IMMDevice, IMMDeviceCollection, IMMDeviceEnumerator, eCapture, eRender,
};

/// Enumerate WASAPI endpoints and normalize them into runtime descriptors.
pub(super) fn enumerate_devices() -> RuntimeResult<Vec<audio_core::HostDeviceDescriptor>> {
    let _com = initialize_com()?;

    // create one endpoint enumerator once for this snapshot
    let enumerator = create_device_enumerator()?;

    // resolve default route ids once for consistent marking
    let default_render_endpoint_id =
        default_endpoint_id(enumerator.raw() as IMMDeviceEnumerator, eRender);
    let default_capture_endpoint_id =
        default_endpoint_id(enumerator.raw() as IMMDeviceEnumerator, eCapture);

    // enumerate active render endpoints
    let mut devices = collect_endpoints(
        enumerator.raw() as IMMDeviceEnumerator,
        EndpointFlow::Render,
        default_render_endpoint_id.as_deref(),
    )?;

    // enumerate active capture endpoints
    let capture_devices = collect_endpoints(
        enumerator.raw() as IMMDeviceEnumerator,
        EndpointFlow::Capture,
        default_capture_endpoint_id.as_deref(),
    )?;
    devices.extend(capture_devices);

    // publish one explicit duplex descriptor when default lanes exist
    if let (Some(default_render_endpoint_id), Some(default_capture_endpoint_id)) = (
        default_render_endpoint_id.as_deref(),
        default_capture_endpoint_id.as_deref(),
    ) {
        let default_render_stable_id =
            endpoint_stable_id(EndpointFlow::Render, default_render_endpoint_id);
        let default_capture_stable_id =
            endpoint_stable_id(EndpointFlow::Capture, default_capture_endpoint_id);
        let playback_descriptor = devices
            .iter()
            .find(|device| device.id == default_render_stable_id);
        let capture_descriptor = devices
            .iter()
            .find(|device| device.id == default_capture_stable_id);

        devices.push(duplex_descriptor_from_endpoint_ids(
            default_render_endpoint_id,
            default_capture_endpoint_id,
            playback_descriptor,
            capture_descriptor,
        ));
    }

    if devices.is_empty() {
        return Err(core_platform::io_not_found(
            "destack.audio.device.list",
            "WASAPI reported no active endpoints",
        ));
    }

    Ok(devices)
}

/// Collect and normalize descriptors for one endpoint flow lane.
fn collect_endpoints(
    enumerator: IMMDeviceEnumerator,
    flow: EndpointFlow,
    default_endpoint_id: Option<&str>,
) -> RuntimeResult<Vec<audio_core::HostDeviceDescriptor>> {
    // enumerate active endpoints for the selected data-flow lane
    let collection = enum_audio_endpoints(enumerator, flow.data_flow())?;
    let count = collection_count(collection.raw() as IMMDeviceCollection)?;

    let mut devices = Vec::with_capacity((count as usize).saturating_mul(2));
    for index in 0..count {
        // resolve one endpoint by index and read endpoint metadata
        let endpoint = collection_item(collection.raw() as IMMDeviceCollection, index)?;
        let endpoint = endpoint.raw() as IMMDevice;
        let endpoint_id = endpoint_id(endpoint)?;
        let endpoint_name = endpoint_display_name(endpoint, &endpoint_id);
        let profile = probe_endpoint_profile(endpoint);
        let is_default = default_endpoint_id == Some(endpoint_id.as_str());

        // expand one endpoint into one or more runtime descriptors
        match flow {
            EndpointFlow::Render => {
                devices.push(playback_descriptor_from_endpoint_id(
                    &endpoint_id,
                    &endpoint_name,
                    &profile,
                    is_default,
                ));
                devices.push(loopback_descriptor_from_endpoint_id(
                    &endpoint_id,
                    &endpoint_name,
                    &profile,
                    is_default,
                ));
            }
            EndpointFlow::Capture => {
                devices.push(capture_descriptor_from_endpoint_id(
                    &endpoint_id,
                    &endpoint_name,
                    &profile,
                    is_default,
                ));
            }
            EndpointFlow::Loopback => {}
        }
    }

    Ok(devices)
}

/// Build one capability flag mask for one endpoint profile and direction shape.
fn capability_flags(
    profile: &WasapiEndpointProfile,
    supports_loopback: bool,
    supports_duplex: bool,
) -> audio_types::AudioDeviceCapabilityFlags {
    let mut flags = audio_core::DEVICE_CAPABILITY_SHARED_MODE.0
        | audio_core::DEVICE_CAPABILITY_STREAM_VOLUME.0
        | audio_core::DEVICE_CAPABILITY_STREAM_MUTE.0
        | audio_core::DEVICE_CAPABILITY_REROUTE_EVENTS.0
        | audio_core::DEVICE_CAPABILITY_BACKEND_DISCONNECT_EVENTS.0;

    if profile.supports_exclusive_mode {
        flags |= audio_core::DEVICE_CAPABILITY_EXCLUSIVE_MODE.0;
    }

    if supports_loopback {
        flags |= audio_core::DEVICE_CAPABILITY_LOOPBACK.0;
    }

    if supports_duplex {
        flags |= audio_core::DEVICE_CAPABILITY_FULL_DUPLEX.0;
    }

    audio_types::AudioDeviceCapabilityFlags(flags)
}

/// Build one share-mode mask for one endpoint profile.
fn share_mode_mask(profile: &WasapiEndpointProfile, shared_only: bool) -> u32 {
    let mut mask = audio_core::SHARE_MODE_SHARED_BIT;
    if !shared_only && profile.supports_exclusive_mode {
        mask |= audio_core::SHARE_MODE_EXCLUSIVE_BIT;
    }

    mask
}

/// Build one WASAPI playback descriptor from one endpoint id.
fn playback_descriptor_from_endpoint_id(
    endpoint_id: &str,
    endpoint_name: &str,
    profile: &WasapiEndpointProfile,
    is_default: bool,
) -> audio_core::HostDeviceDescriptor {
    audio_core::HostDeviceDescriptor {
        id: endpoint_stable_id(EndpointFlow::Render, endpoint_id),
        group_id: format!("wasapi-group:{endpoint_id}"),
        name: endpoint_name.to_string(),
        transport: profile.transport.to_string(),
        backend: audio_types::AudioBackend::Wasapi,
        direction: audio_types::AudioDeviceDirection::Playback,
        supported_directions: audio_core::DIRECTION_MASK_PLAYBACK,
        connected: true,
        is_raw: false,
        is_default_playback: is_default,
        is_default_capture: false,
        is_default_loopback: false,
        capability_flags: audio_types::AudioDeviceCapabilityFlags(
            capability_flags(profile, false, false).0
                | audio_core::DEVICE_CAPABILITY_DEVICE_CLOCK.0
                | audio_core::DEVICE_CAPABILITY_SCHEDULED_WRITE.0,
        ),
        preferred_sample_rate: profile.preferred_sample_rate,
        min_sample_rate: profile.min_sample_rate,
        max_sample_rate: profile.max_sample_rate,
        preferred_period_frames: profile.preferred_period_frames,
        min_channels: profile.min_channels,
        max_channels: profile.max_channels,
        preferred_layout: profile.preferred_layout,
        preferred_channel_mask: profile.preferred_channel_mask,
        supported_channel_mask: profile.supported_channel_mask,
        min_period_frames: profile.min_period_frames,
        max_period_frames: profile.max_period_frames,
        format_mask: profile.format_mask,
        share_mode_mask: share_mode_mask(profile, false),
        is_null: false,
    }
}

/// Build one WASAPI capture descriptor from one endpoint id.
fn capture_descriptor_from_endpoint_id(
    endpoint_id: &str,
    endpoint_name: &str,
    profile: &WasapiEndpointProfile,
    is_default: bool,
) -> audio_core::HostDeviceDescriptor {
    audio_core::HostDeviceDescriptor {
        id: endpoint_stable_id(EndpointFlow::Capture, endpoint_id),
        group_id: format!("wasapi-group:{endpoint_id}"),
        name: endpoint_name.to_string(),
        transport: profile.transport.to_string(),
        backend: audio_types::AudioBackend::Wasapi,
        direction: audio_types::AudioDeviceDirection::Capture,
        supported_directions: audio_core::DIRECTION_MASK_CAPTURE,
        connected: true,
        is_raw: false,
        is_default_playback: false,
        is_default_capture: is_default,
        is_default_loopback: false,
        capability_flags: audio_types::AudioDeviceCapabilityFlags(
            capability_flags(profile, false, false).0
                | audio_core::DEVICE_CAPABILITY_DEVICE_CLOCK.0,
        ),
        preferred_sample_rate: profile.preferred_sample_rate,
        min_sample_rate: profile.min_sample_rate,
        max_sample_rate: profile.max_sample_rate,
        preferred_period_frames: profile.preferred_period_frames,
        min_channels: profile.min_channels,
        max_channels: profile.max_channels,
        preferred_layout: profile.preferred_layout,
        preferred_channel_mask: profile.preferred_channel_mask,
        supported_channel_mask: profile.supported_channel_mask,
        min_period_frames: profile.min_period_frames,
        max_period_frames: profile.max_period_frames,
        format_mask: profile.format_mask,
        share_mode_mask: share_mode_mask(profile, false),
        is_null: false,
    }
}

/// Build one WASAPI loopback descriptor from one render endpoint id.
fn loopback_descriptor_from_endpoint_id(
    endpoint_id: &str,
    endpoint_name: &str,
    profile: &WasapiEndpointProfile,
    is_default: bool,
) -> audio_core::HostDeviceDescriptor {
    audio_core::HostDeviceDescriptor {
        id: endpoint_stable_id(EndpointFlow::Loopback, endpoint_id),
        group_id: format!("wasapi-group:{endpoint_id}"),
        name: format!("{endpoint_name} (loopback)"),
        transport: profile.transport.to_string(),
        backend: audio_types::AudioBackend::Wasapi,
        direction: audio_types::AudioDeviceDirection::Loopback,
        supported_directions: audio_core::DIRECTION_MASK_LOOPBACK,
        connected: true,
        is_raw: false,
        is_default_playback: false,
        is_default_capture: false,
        is_default_loopback: is_default,
        capability_flags: audio_types::AudioDeviceCapabilityFlags(
            capability_flags(profile, true, false).0 | audio_core::DEVICE_CAPABILITY_DEVICE_CLOCK.0,
        ),
        preferred_sample_rate: profile.preferred_sample_rate,
        min_sample_rate: profile.min_sample_rate,
        max_sample_rate: profile.max_sample_rate,
        preferred_period_frames: profile.preferred_period_frames,
        min_channels: profile.min_channels,
        max_channels: profile.max_channels,
        preferred_layout: profile.preferred_layout,
        preferred_channel_mask: profile.preferred_channel_mask,
        supported_channel_mask: profile.supported_channel_mask,
        min_period_frames: profile.min_period_frames,
        max_period_frames: profile.max_period_frames,
        format_mask: profile.format_mask,
        share_mode_mask: share_mode_mask(profile, true),
        is_null: false,
    }
}

/// Build one WASAPI duplex descriptor from one explicit render and capture endpoint pair.
fn duplex_descriptor_from_endpoint_ids(
    render_endpoint_id: &str,
    capture_endpoint_id: &str,
    playback_descriptor: Option<&audio_core::HostDeviceDescriptor>,
    capture_descriptor: Option<&audio_core::HostDeviceDescriptor>,
) -> audio_core::HostDeviceDescriptor {
    let preferred_sample_rate = playback_descriptor
        .map(|descriptor| descriptor.preferred_sample_rate)
        .or_else(|| capture_descriptor.map(|descriptor| descriptor.preferred_sample_rate))
        .unwrap_or(48_000);

    let minimum_sample_rate = playback_descriptor
        .map(|descriptor| descriptor.min_sample_rate)
        .zip(capture_descriptor.map(|descriptor| descriptor.min_sample_rate))
        .map(|(playback, capture): (u32, u32)| playback.max(capture))
        .unwrap_or(DEFAULT_MIN_SAMPLE_RATE);
    let maximum_sample_rate = playback_descriptor
        .map(|descriptor| descriptor.max_sample_rate)
        .zip(capture_descriptor.map(|descriptor| descriptor.max_sample_rate))
        .map(|(playback, capture): (u32, u32)| playback.min(capture))
        .unwrap_or(DEFAULT_MAX_SAMPLE_RATE)
        .max(minimum_sample_rate);

    let minimum_channels = playback_descriptor
        .map(|descriptor| descriptor.min_channels)
        .zip(capture_descriptor.map(|descriptor| descriptor.min_channels))
        .map(|(playback, capture): (u16, u16)| playback.max(capture))
        .unwrap_or(1);
    let maximum_channels = playback_descriptor
        .map(|descriptor| descriptor.max_channels)
        .zip(capture_descriptor.map(|descriptor| descriptor.max_channels))
        .map(|(playback, capture): (u16, u16)| playback.min(capture))
        .unwrap_or(WASAPI_MAX_PROBED_CHANNELS)
        .max(minimum_channels);

    let preferred_period_frames = playback_descriptor
        .map(|descriptor| descriptor.preferred_period_frames)
        .zip(capture_descriptor.map(|descriptor| descriptor.preferred_period_frames))
        .map(|(playback, capture): (u32, u32)| playback.max(capture))
        .unwrap_or(DEFAULT_PREFERRED_PERIOD_FRAMES);
    let minimum_period_frames = playback_descriptor
        .map(|descriptor| descriptor.min_period_frames)
        .zip(capture_descriptor.map(|descriptor| descriptor.min_period_frames))
        .map(|(playback, capture): (u32, u32)| playback.max(capture))
        .unwrap_or(MIN_STREAM_PERIOD_FRAMES);
    let maximum_period_frames = playback_descriptor
        .map(|descriptor| descriptor.max_period_frames)
        .zip(capture_descriptor.map(|descriptor| descriptor.max_period_frames))
        .map(|(playback, capture): (u32, u32)| playback.min(capture))
        .unwrap_or(DEFAULT_MAX_PERIOD_FRAMES)
        .max(minimum_period_frames);

    let preferred_layout = playback_descriptor
        .map(|descriptor| descriptor.preferred_layout)
        .unwrap_or(audio_types::AudioChannelLayout::Stereo);
    let preferred_channel_mask = playback_descriptor
        .map(|descriptor| descriptor.preferred_channel_mask)
        .zip(capture_descriptor.map(|descriptor| descriptor.preferred_channel_mask))
        .map(|(playback, capture)| playback & capture)
        .unwrap_or(0b11);
    let supported_channel_mask = playback_descriptor
        .map(|descriptor| descriptor.supported_channel_mask)
        .zip(capture_descriptor.map(|descriptor| descriptor.supported_channel_mask))
        .map(|(playback, capture)| playback & capture)
        .unwrap_or(0xff)
        | preferred_channel_mask;

    let format_mask = playback_descriptor
        .map(|descriptor| descriptor.format_mask)
        .zip(capture_descriptor.map(|descriptor| descriptor.format_mask))
        .map(|(playback, capture)| playback & capture)
        .unwrap_or(audio_core::all_sample_format_mask());
    let share_mode_mask = playback_descriptor
        .map(|descriptor| descriptor.share_mode_mask)
        .zip(capture_descriptor.map(|descriptor| descriptor.share_mode_mask))
        .map(|(playback, capture)| playback & capture)
        .unwrap_or(audio_core::SHARE_MODE_SHARED_BIT)
        | audio_core::SHARE_MODE_SHARED_BIT;

    let capability_flags = audio_core::DEVICE_CAPABILITY_SHARED_MODE.0
        | audio_core::DEVICE_CAPABILITY_FULL_DUPLEX.0
        | audio_core::DEVICE_CAPABILITY_DEVICE_CLOCK.0
        | audio_core::DEVICE_CAPABILITY_SCHEDULED_WRITE.0
        | audio_core::DEVICE_CAPABILITY_STREAM_VOLUME.0
        | audio_core::DEVICE_CAPABILITY_STREAM_MUTE.0
        | audio_core::DEVICE_CAPABILITY_REROUTE_EVENTS.0
        | audio_core::DEVICE_CAPABILITY_BACKEND_DISCONNECT_EVENTS.0
        | if (share_mode_mask & audio_core::SHARE_MODE_EXCLUSIVE_BIT) != 0 {
            audio_core::DEVICE_CAPABILITY_EXCLUSIVE_MODE.0
        } else {
            0
        };

    audio_core::HostDeviceDescriptor {
        id: duplex_stable_id(render_endpoint_id, capture_endpoint_id),
        group_id: format!("wasapi-group:duplex:{render_endpoint_id}:{capture_endpoint_id}"),
        name: "WASAPI duplex pair".to_string(),
        transport: "wasapi".to_string(),
        backend: audio_types::AudioBackend::Wasapi,
        direction: audio_types::AudioDeviceDirection::Duplex,
        supported_directions: audio_core::DIRECTION_MASK_DUPLEX,
        connected: true,
        is_raw: false,
        is_default_playback: true,
        is_default_capture: true,
        is_default_loopback: false,
        capability_flags: audio_types::AudioDeviceCapabilityFlags(capability_flags),
        preferred_sample_rate,
        min_sample_rate: minimum_sample_rate,
        max_sample_rate: maximum_sample_rate,
        preferred_period_frames,
        min_channels: minimum_channels,
        max_channels: maximum_channels,
        preferred_layout,
        preferred_channel_mask,
        supported_channel_mask,
        min_period_frames: minimum_period_frames,
        max_period_frames: maximum_period_frames,
        format_mask,
        share_mode_mask,
        is_null: false,
    }
}
