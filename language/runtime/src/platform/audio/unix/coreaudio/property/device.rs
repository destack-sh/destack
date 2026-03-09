#[cfg(target_os = "macos")]
use std::mem::size_of;

#[cfg(target_os = "macos")]
use super::access::{error, get_data, get_data_size, get_scalar_optional};
#[cfg(target_os = "macos")]
use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "macos")]
use crate::platform::audio::unix::coreaudio::abi::{
    AudioBuffer, AudioBufferList, AudioDeviceID, AudioObjectPropertyScope,
    AudioObjectPropertySelector, AudioValueRange,
};
#[cfg(target_os = "macos")]
use crate::platform::audio::unix::coreaudio::constants::{
    K_AUDIO_DEVICE_PROPERTY_AVAILABLE_NOMINAL_SAMPLE_RATES,
    K_AUDIO_DEVICE_PROPERTY_BUFFER_FRAME_SIZE_RANGE, K_AUDIO_DEVICE_PROPERTY_STREAM_CONFIGURATION,
    K_AUDIO_HARDWARE_PROPERTY_DEVICES, K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
    K_AUDIO_OBJECT_SYSTEM_OBJECT,
};

/// Return the channel count for a CoreAudio stream configuration scope.
#[cfg(target_os = "macos")]
pub(crate) fn get_stream_channel_count(
    device_id: AudioDeviceID,
    scope: AudioObjectPropertyScope,
) -> Option<u16> {
    let size = get_data_size(
        device_id,
        K_AUDIO_DEVICE_PROPERTY_STREAM_CONFIGURATION,
        scope,
    )
    .ok()?;
    if size == 0 {
        return Some(0);
    }

    let mut bytes = vec![0u8; size as usize];
    get_data(
        device_id,
        K_AUDIO_DEVICE_PROPERTY_STREAM_CONFIGURATION,
        scope,
        &mut bytes,
    )
    .ok()?;

    if bytes.len() < size_of::<AudioBufferList>() {
        return None;
    }

    let buffer_list = bytes.as_ptr() as *const AudioBufferList;
    let buffer_count = unsafe { (*buffer_list).number_buffers as usize };
    let first_buffer = unsafe { std::ptr::addr_of!((*buffer_list).buffers) as *const AudioBuffer };
    let base_pointer = buffer_list as usize;
    let first_pointer = first_buffer as usize;
    let offset = first_pointer.checked_sub(base_pointer)?;
    let expected_size = offset.checked_add(buffer_count.checked_mul(size_of::<AudioBuffer>())?)?;
    if expected_size > bytes.len() {
        return None;
    }

    let mut channels = 0u64;
    for index in 0..buffer_count {
        let buffer = unsafe { first_buffer.add(index).read_unaligned() };
        channels = channels.saturating_add(buffer.number_channels as u64);
    }

    Some(channels.min(u16::MAX as u64) as u16)
}

/// Return the sample-rate range reported by CoreAudio for a scope.
#[cfg(target_os = "macos")]
pub(crate) fn get_sample_rate_range(
    device_id: AudioDeviceID,
    scope: AudioObjectPropertyScope,
) -> Option<(u32, u32)> {
    let size = get_data_size(
        device_id,
        K_AUDIO_DEVICE_PROPERTY_AVAILABLE_NOMINAL_SAMPLE_RATES,
        scope,
    )
    .ok()?;
    if size == 0 {
        return None;
    }

    let mut bytes = vec![0u8; size as usize];
    get_data(
        device_id,
        K_AUDIO_DEVICE_PROPERTY_AVAILABLE_NOMINAL_SAMPLE_RATES,
        scope,
        &mut bytes,
    )
    .ok()?;

    let stride = size_of::<AudioValueRange>();
    if stride == 0 || bytes.len() < stride {
        return None;
    }

    let mut min_rate = u32::MAX;
    let mut max_rate = 0u32;
    for index in 0..(bytes.len() / stride) {
        let pointer = unsafe { bytes.as_ptr().add(index * stride) as *const AudioValueRange };
        let range = unsafe { pointer.read_unaligned() };
        let minimum = rate_to_u32(range.minimum)?;
        let maximum = rate_to_u32(range.maximum)?;
        let low = minimum.min(maximum);
        let high = minimum.max(maximum);
        min_rate = min_rate.min(low);
        max_rate = max_rate.max(high);
    }

    if min_rate == u32::MAX || max_rate == 0 {
        return None;
    }

    Some((min_rate, max_rate))
}

/// Return the period-frame range reported by CoreAudio for a scope.
#[cfg(target_os = "macos")]
pub(crate) fn get_buffer_frame_size_range(
    device_id: AudioDeviceID,
    scope: AudioObjectPropertyScope,
) -> Option<(u32, u32)> {
    let range = get_scalar_optional::<AudioValueRange>(
        device_id,
        K_AUDIO_DEVICE_PROPERTY_BUFFER_FRAME_SIZE_RANGE,
        scope,
    )?;
    let minimum = frames_to_u32(range.minimum)?;
    let maximum = frames_to_u32(range.maximum)?;
    Some((minimum.min(maximum), minimum.max(maximum)))
}

/// Convert a finite CoreAudio sample-rate scalar into a u32 value.
#[cfg(target_os = "macos")]
pub(crate) fn rate_to_u32(rate: f64) -> Option<u32> {
    if !rate.is_finite() || rate < 1.0 || rate > u32::MAX as f64 {
        return None;
    }

    Some(rate.round() as u32)
}

/// Convert a finite CoreAudio frame-count scalar into a u32 value.
#[cfg(target_os = "macos")]
pub(crate) fn frames_to_u32(frames: f64) -> Option<u32> {
    if !frames.is_finite() || frames < 1.0 || frames > u32::MAX as f64 {
        return None;
    }

    Some(frames.round() as u32)
}

/// Return the default CoreAudio device identifier for a selector.
#[cfg(target_os = "macos")]
pub(crate) fn default_device_id(selector: AudioObjectPropertySelector) -> Option<AudioDeviceID> {
    get_scalar_optional(
        K_AUDIO_OBJECT_SYSTEM_OBJECT,
        selector,
        K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
    )
}

/// Enumerate CoreAudio device identifiers from the system object.
#[cfg(target_os = "macos")]
pub(crate) fn device_ids() -> RuntimeResult<Vec<AudioDeviceID>> {
    let size = get_data_size(
        K_AUDIO_OBJECT_SYSTEM_OBJECT,
        K_AUDIO_HARDWARE_PROPERTY_DEVICES,
        K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
    )
    .map_err(|status| {
        error(
            "destack.audio.device.list",
            status,
            "failed to query CoreAudio device list size",
        )
    })?;

    if size == 0 {
        return Ok(Vec::new());
    }

    if !(size as usize).is_multiple_of(size_of::<AudioDeviceID>()) {
        return Err(error(
            "destack.audio.device.list",
            -1,
            "CoreAudio device list has invalid element size",
        ));
    }

    let mut bytes = vec![0u8; size as usize];
    get_data(
        K_AUDIO_OBJECT_SYSTEM_OBJECT,
        K_AUDIO_HARDWARE_PROPERTY_DEVICES,
        K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        &mut bytes,
    )
    .map_err(|status| {
        error(
            "destack.audio.device.list",
            status,
            "failed to query CoreAudio device list",
        )
    })?;

    let mut devices = Vec::with_capacity(bytes.len() / size_of::<AudioDeviceID>());
    for chunk in bytes.chunks_exact(size_of::<AudioDeviceID>()) {
        let value = u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        devices.push(value);
    }

    Ok(devices)
}
