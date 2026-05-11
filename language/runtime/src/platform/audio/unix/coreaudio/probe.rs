#[cfg(target_os = "macos")]
use std::collections::HashMap;
#[cfg(target_os = "macos")]
use std::ptr;
#[cfg(target_os = "macos")]
use std::sync::{Arc, Mutex};

#[cfg(target_os = "macos")]
use crate::platform::audio::core::codec::frame_bytes;
use crate::runtime::service::Service;
#[cfg(target_os = "macos")]
use crate::runtime::{ExecutionMode, ExecutionPolicy};

#[cfg(target_os = "macos")]
use super::abi::{
    AudioDeviceID, AudioQueueAllocateBuffer, AudioQueueDispose, AudioQueueEnqueueBuffer,
    AudioQueueNewInput, AudioQueueStart, AudioQueueStop,
};
#[cfg(target_os = "macos")]
use super::callback::loopback_probe_input_callback;
#[cfg(target_os = "macos")]
use super::constants::{
    COREAUDIO_LOOPBACK_PROBE_FRAMES, K_AUDIO_DEVICE_PROPERTY_DEVICE_UID,
    K_AUDIO_DEVICE_PROPERTY_NOMINAL_SAMPLE_RATE, K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
    K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT, K_FALLBACK_SAMPLE_RATE, K_NO_ERR,
};
#[cfg(target_os = "macos")]
use super::format::{channel_layout, channel_mask};
#[cfg(target_os = "macos")]
use super::property::{
    get_cfstring_optional, get_scalar_optional, get_stream_channel_count, rate_to_u32,
    stream_description,
};
#[cfg(target_os = "macos")]
use super::queue::bind_queue_device;
use crate::platform::audio as audio_types;

/// Dispose one temporary CoreAudio loopback probe queue.
#[cfg(target_os = "macos")]
fn dispose_probe_queue(queue: super::abi::AudioQueueRef) {
    unsafe {
        let _ = AudioQueueStop(queue, 1);
        let _ = AudioQueueDispose(queue, 1);
    }
}

/// One process-global CoreAudio loopback probe service.
#[cfg(target_os = "macos")]
struct CoreAudioLoopbackProbeService {
    /// Cached loopback support keyed by stable device uid.
    support_cache: Mutex<HashMap<String, bool>>,
}

#[cfg(target_os = "macos")]
impl CoreAudioLoopbackProbeService {
    /// Build one empty CoreAudio loopback probe service.
    fn new() -> Self {
        Self {
            support_cache: Mutex::new(HashMap::new()),
        }
    }
}

#[cfg(target_os = "macos")]
impl Service for CoreAudioLoopbackProbeService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Inline);
}

/// Return the shared CoreAudio loopback probe service.
#[cfg(target_os = "macos")]
fn coreaudio_loopback_probe_service() -> Arc<CoreAudioLoopbackProbeService> {
    CoreAudioLoopbackProbeService::global(|| Ok(CoreAudioLoopbackProbeService::new()))
        .expect("CoreAudio loopback probe service initialization should not fail")
}

/// Clear cached CoreAudio loopback capability probe results.
#[cfg(target_os = "macos")]
pub(super) fn clear_loopback_support_cache() {
    let service = coreaudio_loopback_probe_service();
    let mut support_cache = service
        .support_cache
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    support_cache.clear();
}

/// Return one stable cache key for a CoreAudio device when available.
#[cfg(target_os = "macos")]
fn loopback_support_cache_key(device_id: AudioDeviceID) -> Option<String> {
    get_cfstring_optional(
        device_id,
        K_AUDIO_DEVICE_PROPERTY_DEVICE_UID,
        K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
    )
}

#[cfg(target_os = "macos")]
fn probe_loopback_support_uncached(device_id: AudioDeviceID) -> bool {
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
        dispose_probe_queue(queue);
        return false;
    }

    // compute and allocate the probe buffer
    let bytes_per_frame = match frame_bytes(config.format, config.channels) {
        Ok(value) => value as u32,
        Err(_) => {
            dispose_probe_queue(queue);
            return false;
        }
    };
    let buffer_bytes = COREAUDIO_LOOPBACK_PROBE_FRAMES.saturating_mul(bytes_per_frame);
    let mut buffer = ptr::null_mut();
    let allocate_status = unsafe { AudioQueueAllocateBuffer(queue, buffer_bytes, &mut buffer) };
    if allocate_status != K_NO_ERR {
        dispose_probe_queue(queue);
        return false;
    }

    // enqueue a buffer and attempt a start/stop cycle
    let buffer_mut = unsafe { &mut *buffer };
    buffer_mut.audio_data_byte_size = buffer_mut.audio_data_bytes_capacity;
    let enqueue_status = unsafe { AudioQueueEnqueueBuffer(queue, buffer, 0, ptr::null()) };
    if enqueue_status != K_NO_ERR {
        dispose_probe_queue(queue);
        return false;
    }

    let start_status = unsafe { AudioQueueStart(queue, ptr::null()) };
    if start_status != K_NO_ERR {
        dispose_probe_queue(queue);
        return false;
    }

    // tear down the probe queue and report support
    dispose_probe_queue(queue);

    true
}

/// Return whether one CoreAudio output device supports loopback capture.
#[cfg(target_os = "macos")]
pub(super) fn probe_loopback_support(device_id: AudioDeviceID) -> bool {
    let Some(cache_key) = loopback_support_cache_key(device_id) else {
        return probe_loopback_support_uncached(device_id);
    };

    // reuse one cached probe result for this stable device uid
    {
        let service = coreaudio_loopback_probe_service();
        let support_cache = service
            .support_cache
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        if let Some(value) = support_cache.get(&cache_key) {
            return *value;
        }
    }

    // otherwise probe once and publish the result for later enumerations
    let support = probe_loopback_support_uncached(device_id);
    let service = coreaudio_loopback_probe_service();
    let mut support_cache = service
        .support_cache
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    support_cache.insert(cache_key, support);

    support
}
