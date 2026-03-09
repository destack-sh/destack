#[cfg(target_os = "macos")]
use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(target_os = "macos")]
use crate::platform::PlatformError;
#[cfg(target_os = "macos")]
use crate::platform::audio as audio_types;
#[cfg(target_os = "macos")]
use crate::platform::audio::core::codec::frame_bytes;

#[cfg(target_os = "macos")]
use super::device::{get_buffer_frame_size_range, get_sample_rate_range, get_stream_channel_count};
#[cfg(target_os = "macos")]
use crate::platform::audio::unix::coreaudio::abi::{AudioDeviceID, AudioStreamBasicDescription};
#[cfg(target_os = "macos")]
use crate::platform::audio::unix::coreaudio::constants::{
    K_AUDIO_FORMAT_FLAG_IS_FLOAT, K_AUDIO_FORMAT_FLAG_IS_PACKED,
    K_AUDIO_FORMAT_FLAG_IS_SIGNED_INTEGER, K_AUDIO_FORMAT_LINEAR_PCM,
    K_AUDIO_OBJECT_PROPERTY_SCOPE_INPUT, K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT,
};

/// Intersect two optional closed integer ranges.
#[cfg(target_os = "macos")]
pub(crate) fn merge_intersected_range(
    left: Option<(u32, u32)>,
    right: Option<(u32, u32)>,
) -> Option<(u32, u32)> {
    match (left, right) {
        (Some(left), Some(right)) => {
            let minimum = left.0.max(right.0);
            let maximum = left.1.min(right.1);
            if minimum > maximum {
                return None;
            }

            Some((minimum, maximum))
        }
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

/// Validate a stream configuration against CoreAudio device limits.
#[cfg(target_os = "macos")]
pub(crate) fn validate_open_stream_config(
    device_id: AudioDeviceID,
    direction: audio_types::AudioDeviceDirection,
    config: audio_types::AudioStreamConfig,
) -> RuntimeResult<()> {
    // resolve directional channel limits for the requested direction
    let playback_channels =
        get_stream_channel_count(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT).unwrap_or(0);
    let capture_channels =
        get_stream_channel_count(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_INPUT).unwrap_or(0);
    let max_channels = match direction {
        audio_types::AudioDeviceDirection::Playback => playback_channels,
        audio_types::AudioDeviceDirection::Capture => capture_channels,
        audio_types::AudioDeviceDirection::Duplex => playback_channels.min(capture_channels),
        audio_types::AudioDeviceDirection::Loopback => playback_channels,
    };

    // reject directions with no usable lane on this device
    if max_channels == 0 {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.audio.stream.open CoreAudio direction",
        ))
        .boxed());
    }

    // validate the requested channel count against direction limits
    if config.channels > max_channels {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "config.channels",
            format!(
                "requested channel count exceeds CoreAudio direction channel limit {max_channels}",
            ),
        ))
        .boxed());
    }

    // resolve the valid sample-rate range for the requested direction
    let sample_rate_range = match direction {
        audio_types::AudioDeviceDirection::Playback => {
            get_sample_rate_range(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT)
        }
        audio_types::AudioDeviceDirection::Capture => {
            get_sample_rate_range(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_INPUT)
        }
        audio_types::AudioDeviceDirection::Duplex => merge_intersected_range(
            get_sample_rate_range(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT),
            get_sample_rate_range(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_INPUT),
        ),
        audio_types::AudioDeviceDirection::Loopback => {
            get_sample_rate_range(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT)
        }
    };

    // reject sample rates outside the backend-reported range
    if let Some((minimum, maximum)) = sample_rate_range
        && (config.sample_rate < minimum || config.sample_rate > maximum)
    {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "config.sampleRate",
            format!("requested sample rate is outside CoreAudio range [{minimum}, {maximum}]"),
        ))
        .boxed());
    }

    // resolve the valid period range for the requested direction
    let period_range = match direction {
        audio_types::AudioDeviceDirection::Playback => {
            get_buffer_frame_size_range(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT)
        }
        audio_types::AudioDeviceDirection::Capture => {
            get_buffer_frame_size_range(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_INPUT)
        }
        audio_types::AudioDeviceDirection::Duplex => merge_intersected_range(
            get_buffer_frame_size_range(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT),
            get_buffer_frame_size_range(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_INPUT),
        ),
        audio_types::AudioDeviceDirection::Loopback => {
            get_buffer_frame_size_range(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT)
        }
    };

    // reject explicit period requests outside the supported range
    if config.period_frames > 0
        && let Some((minimum, maximum)) = period_range
        && (config.period_frames < minimum || config.period_frames > maximum)
    {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "config.periodFrames",
            format!("requested period is outside CoreAudio range [{minimum}, {maximum}]"),
        ))
        .boxed());
    }

    Ok(())
}

/// Return bits per channel for a runtime sample format.
#[cfg(target_os = "macos")]
pub(crate) fn bits_per_channel(format: audio_types::AudioSampleFormat) -> Option<u32> {
    match format {
        audio_types::AudioSampleFormat::U8 => Some(8),
        audio_types::AudioSampleFormat::S16 => Some(16),
        audio_types::AudioSampleFormat::S24 => Some(32),
        audio_types::AudioSampleFormat::S32 => Some(32),
        audio_types::AudioSampleFormat::F32 => Some(32),
        audio_types::AudioSampleFormat::F64 => Some(64),
    }
}

/// Return CoreAudio PCM format flags for a runtime sample format.
#[cfg(target_os = "macos")]
pub(crate) fn format_flags(format: audio_types::AudioSampleFormat) -> Option<u32> {
    let base = K_AUDIO_FORMAT_FLAG_IS_PACKED;
    match format {
        audio_types::AudioSampleFormat::U8 => Some(base),
        audio_types::AudioSampleFormat::S16
        | audio_types::AudioSampleFormat::S24
        | audio_types::AudioSampleFormat::S32 => Some(base | K_AUDIO_FORMAT_FLAG_IS_SIGNED_INTEGER),
        audio_types::AudioSampleFormat::F32 | audio_types::AudioSampleFormat::F64 => {
            Some(base | K_AUDIO_FORMAT_FLAG_IS_FLOAT)
        }
    }
}

/// Build a CoreAudio stream description from a runtime stream config.
#[cfg(target_os = "macos")]
pub(crate) fn stream_description(
    config: audio_types::AudioStreamConfig,
) -> RuntimeResult<AudioStreamBasicDescription> {
    let bits_per_channel = bits_per_channel(config.format).ok_or_else(|| {
        RuntimeError::from(PlatformError::not_supported(
            "destack.audio.stream.open CoreAudio format",
        ))
        .boxed()
    })?;
    let format_flags = format_flags(config.format).ok_or_else(|| {
        RuntimeError::from(PlatformError::not_supported(
            "destack.audio.stream.open CoreAudio format",
        ))
        .boxed()
    })?;
    let bytes_per_frame = frame_bytes(config.format, config.channels)? as u32;

    Ok(AudioStreamBasicDescription {
        sample_rate: config.sample_rate as f64,
        format_id: K_AUDIO_FORMAT_LINEAR_PCM,
        format_flags,
        bytes_per_packet: bytes_per_frame,
        frames_per_packet: 1,
        bytes_per_frame,
        channels_per_frame: config.channels as u32,
        bits_per_channel,
        reserved: 0,
    })
}
