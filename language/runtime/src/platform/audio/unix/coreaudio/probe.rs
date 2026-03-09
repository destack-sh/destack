#[cfg(target_os = "macos")]
use std::ptr;

#[cfg(target_os = "macos")]
use crate::platform::audio::core::codec::frame_bytes;

#[cfg(target_os = "macos")]
use super::abi::{
    AudioDeviceID, AudioQueueAllocateBuffer, AudioQueueDispose, AudioQueueEnqueueBuffer,
    AudioQueueNewInput, AudioQueueStart, AudioQueueStop,
};
#[cfg(target_os = "macos")]
use super::callback::loopback_probe_input_callback;
#[cfg(target_os = "macos")]
use super::constants::{
    COREAUDIO_LOOPBACK_PROBE_FRAMES, K_AUDIO_DEVICE_PROPERTY_NOMINAL_SAMPLE_RATE,
    K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT, K_FALLBACK_SAMPLE_RATE, K_NO_ERR,
};
#[cfg(target_os = "macos")]
use super::format::{channel_layout, channel_mask};
#[cfg(target_os = "macos")]
use super::property::{
    get_scalar_optional, get_stream_channel_count, rate_to_u32, stream_description,
};
#[cfg(target_os = "macos")]
use super::queue::bind_queue_device;
use crate::platform::audio as audio_types;

#[cfg(target_os = "macos")]
pub(super) fn probe_loopback_support(device_id: AudioDeviceID) -> bool {
    // skip probing for devices with no output channels
    let output_channels =
        get_stream_channel_count(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT).unwrap_or(0);
    if output_channels == 0 {
        return false;
    }

    // build a conservative probe configuration for output loopback capture
    let channels = output_channels.clamp(1, 2);
    let sample_rate = get_scalar_optional::<f64>(
        device_id,
        K_AUDIO_DEVICE_PROPERTY_NOMINAL_SAMPLE_RATE,
        K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT,
    )
    .and_then(rate_to_u32)
    .unwrap_or(K_FALLBACK_SAMPLE_RATE);
    let config = audio_types::AudioStreamConfig {
        sample_rate,
        channels,
        channel_layout: channel_layout(channels),
        channel_mask: channel_mask(channels),
        format: audio_types::AudioSampleFormat::F32,
        period_frames: COREAUDIO_LOOPBACK_PROBE_FRAMES,
        transfer_mode: audio_types::AudioStreamTransferMode::Push,
    };

    // reject probing when we cannot derive a valid stream description
    let description = match stream_description(config) {
        Ok(value) => value,
        Err(_) => {
            return false;
        }
    };

    // create a temporary input queue bound to the target output device
    let mut queue = ptr::null_mut();
    let create_status = unsafe {
        AudioQueueNewInput(
            &description,
            Some(loopback_probe_input_callback),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            &mut queue,
        )
    };
    if create_status != K_NO_ERR {
        return false;
    }

    // bind the temporary queue to the requested device
    if bind_queue_device(queue, device_id).is_err() {
        unsafe {
            let _ = AudioQueueDispose(queue, 1);
        }
        return false;
    }

    // compute and allocate the probe buffer
    let bytes_per_frame = match frame_bytes(config.format, config.channels) {
        Ok(value) => value as u32,
        Err(_) => {
            unsafe {
                let _ = AudioQueueDispose(queue, 1);
            }
            return false;
        }
    };
    let buffer_bytes = COREAUDIO_LOOPBACK_PROBE_FRAMES.saturating_mul(bytes_per_frame);
    let mut buffer = ptr::null_mut();
    let allocate_status = unsafe { AudioQueueAllocateBuffer(queue, buffer_bytes, &mut buffer) };
    if allocate_status != K_NO_ERR {
        unsafe {
            let _ = AudioQueueDispose(queue, 1);
        }
        return false;
    }

    // enqueue a buffer and attempt a start/stop cycle
    let buffer_mut = unsafe { &mut *buffer };
    buffer_mut.audio_data_byte_size = buffer_mut.audio_data_bytes_capacity;
    let enqueue_status = unsafe { AudioQueueEnqueueBuffer(queue, buffer, 0, ptr::null()) };
    if enqueue_status != K_NO_ERR {
        unsafe {
            let _ = AudioQueueDispose(queue, 1);
        }
        return false;
    }

    let start_status = unsafe { AudioQueueStart(queue, ptr::null()) };
    if start_status != K_NO_ERR {
        unsafe {
            let _ = AudioQueueDispose(queue, 1);
        }
        return false;
    }

    // tear down the probe queue and report support
    unsafe {
        let _ = AudioQueueStop(queue, 1);
        let _ = AudioQueueDispose(queue, 1);
    }

    true
}
