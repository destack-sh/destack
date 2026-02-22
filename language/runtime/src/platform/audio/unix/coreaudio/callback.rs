#[cfg(target_os = "macos")]
use std::ffi::c_void;
#[cfg(target_os = "macos")]
use std::ptr;
#[cfg(target_os = "macos")]
use std::sync::Arc;

#[cfg(target_os = "macos")]
use crate::platform::audio::core as audio_core;

#[cfg(target_os = "macos")]
use super::abi::{
    AudioGetCurrentHostTime, AudioQueueBufferRef, AudioQueueEnqueueBuffer, AudioQueueRef,
    AudioTimeStamp, CoreAudioStreamContext,
};
#[cfg(target_os = "macos")]
use super::constants::K_AUDIO_TIME_STAMP_HOST_TIME_VALID;
#[cfg(target_os = "macos")]
use super::core::coreaudio_host_time_to_mono_ns;

#[cfg(target_os = "macos")]
pub(super) fn fill_playback_bytes(
    binding: &Arc<audio_core::AudioStreamBinding>,
    output: &mut [u8],
    callback_timestamp_ns: Option<u64>,
) {
    let scalar_width = audio_core::sample_bytes(binding.requested.format);
    if scalar_width == 0 || output.len() < scalar_width {
        return;
    }

    let scalar_count = output.len() / scalar_width;
    let frame_count = scalar_count / binding.channels.max(1) as usize;

    let mut state = binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    let is_active = state.running && !state.paused && !state.shutdown;
    let volume = if state.muted { 0.0 } else { state.volume };

    let mut wrote = 0usize;
    for chunk in output.chunks_exact_mut(scalar_width).take(scalar_count) {
        let sample = if is_active {
            match state.playback_samples.pop_front() {
                Some(sample) => sample * volume as f32,
                None => {
                    state.xrun_count = state.xrun_count.saturating_add(1);
                    state.output_underflow_count = state.output_underflow_count.saturating_add(1);
                    state.status_flags = audio_core::AudioStreamStatusFlags(
                        state.status_flags.0 | audio_core::STREAM_STATUS_OUTPUT_UNDERFLOW.0,
                    );
                    0.0
                }
            }
        } else {
            0.0
        };
        wrote += audio_core::encode_scalar_sample(binding.requested.format, sample, chunk);
    }

    if wrote < output.len() {
        output[wrote..].fill(0);
    }

    if is_active {
        // use host callback timing when provided, otherwise fall back to runtime monotonic
        let callback_mono_ns =
            callback_timestamp_ns.unwrap_or_else(audio_core::host_monotonic_nanos);
        audio_core::record_stream_callback_timing(
            &mut state,
            binding.sample_rate,
            frame_count as u32,
            callback_mono_ns,
            None,
            Some(callback_mono_ns),
        );
    }

    drop(state);
    binding.sync.wake.notify_all();
}

/// Ingest one input byte buffer into queued capture samples.
#[cfg(target_os = "macos")]
pub(super) fn ingest_capture_bytes(
    binding: &Arc<audio_core::AudioStreamBinding>,
    input: &[u8],
    capture_timestamp_ns: Option<u64>,
) {
    let scalar_width = audio_core::sample_bytes(binding.requested.format);
    if scalar_width == 0 || input.len() < scalar_width {
        return;
    }

    let scalar_count = input.len() / scalar_width;
    let frame_count = scalar_count / binding.channels.max(1) as usize;

    let mut state = binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    let is_active = state.running && !state.paused && !state.shutdown;
    if is_active {
        for sample_bytes in input.chunks_exact(scalar_width).take(scalar_count) {
            let Some(sample) =
                audio_core::decode_scalar_sample(binding.requested.format, sample_bytes)
            else {
                continue;
            };
            state.capture_samples.push_back(sample);
        }

        let max_capture = binding.capture_capacity_samples();
        if state.capture_samples.len() > max_capture {
            let extra = state.capture_samples.len() - max_capture;
            for _ in 0..extra {
                let _ = state.capture_samples.pop_front();
            }
            state.xrun_count = state.xrun_count.saturating_add(1);
            state.input_overflow_count = state.input_overflow_count.saturating_add(1);
            state.status_flags = audio_core::AudioStreamStatusFlags(
                state.status_flags.0 | audio_core::STREAM_STATUS_INPUT_OVERFLOW.0,
            );
        }

        // use host callback timing when provided, otherwise fall back to runtime monotonic
        let callback_mono_ns =
            capture_timestamp_ns.unwrap_or_else(audio_core::host_monotonic_nanos);
        let frames_advanced = if binding.direction == audio_core::AudioDeviceDirection::Capture {
            frame_count as u32
        } else {
            0
        };
        audio_core::record_stream_callback_timing(
            &mut state,
            binding.sample_rate,
            frames_advanced,
            callback_mono_ns,
            Some(callback_mono_ns),
            None,
        );
    }

    drop(state);
    binding.sync.wake.notify_all();
}

/// Return whether one stream binding has entered shutdown state.
#[cfg(target_os = "macos")]
pub(super) fn stream_shutdown(binding: &Arc<audio_core::AudioStreamBinding>) -> bool {
    let state = binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    state.shutdown
}

/// Rebuild one temporary callback context reference from one raw callback pointer.
#[cfg(target_os = "macos")]
pub(super) unsafe fn callback_context(
    in_user_data: *mut c_void,
) -> Option<Arc<CoreAudioStreamContext>> {
    if in_user_data.is_null() {
        return None;
    }

    let context_ptr = in_user_data as *const CoreAudioStreamContext;
    // keep one strong reference while callback runs
    unsafe {
        Arc::increment_strong_count(context_ptr);
    }

    // reconstruct one temporary arc for this callback invocation
    let context = unsafe { Arc::from_raw(context_ptr) };
    Some(context)
}

/// Handle one CoreAudio output queue callback.
#[cfg(target_os = "macos")]
pub(super) unsafe extern "C" fn output_callback(
    in_user_data: *mut c_void,
    in_aq: AudioQueueRef,
    in_buffer: AudioQueueBufferRef,
) {
    if in_buffer.is_null() {
        return;
    }

    let Some(context) = (unsafe { callback_context(in_user_data) }) else {
        return;
    };
    let buffer = unsafe { &mut *in_buffer };

    if !buffer.audio_data.is_null() && buffer.audio_data_bytes_capacity > 0 {
        // capture one host-time-aligned callback timestamp for playback correlation
        let callback_host_time = unsafe { AudioGetCurrentHostTime() };
        let callback_mono_ns = coreaudio_host_time_to_mono_ns(callback_host_time);

        let output = unsafe {
            std::slice::from_raw_parts_mut(
                buffer.audio_data as *mut u8,
                buffer.audio_data_bytes_capacity as usize,
            )
        };
        fill_playback_bytes(&context.binding, output, Some(callback_mono_ns));
        buffer.audio_data_byte_size = buffer.audio_data_bytes_capacity;
    } else {
        buffer.audio_data_byte_size = 0;
    }

    if stream_shutdown(&context.binding) {
        return;
    }

    let _ = unsafe { AudioQueueEnqueueBuffer(in_aq, in_buffer, 0, ptr::null()) };
}

/// Handle one CoreAudio input queue callback.
#[cfg(target_os = "macos")]
pub(super) unsafe extern "C" fn input_callback(
    in_user_data: *mut c_void,
    in_aq: AudioQueueRef,
    in_buffer: AudioQueueBufferRef,
    in_start_time: *const AudioTimeStamp,
    _in_number_packet_descriptions: u32,
    _in_packet_descriptions: *const c_void,
) {
    if in_buffer.is_null() {
        return;
    }

    let Some(context) = (unsafe { callback_context(in_user_data) }) else {
        return;
    };
    let buffer = unsafe { &mut *in_buffer };

    if !buffer.audio_data.is_null() && buffer.audio_data_byte_size > 0 {
        // resolve one capture-side callback timestamp in runtime monotonic space
        let capture_timestamp_ns = if !in_start_time.is_null()
            && (unsafe { (*in_start_time).flags } & K_AUDIO_TIME_STAMP_HOST_TIME_VALID) != 0
        {
            let host_time = unsafe { (*in_start_time).host_time };
            Some(coreaudio_host_time_to_mono_ns(host_time))
        } else {
            let host_time = unsafe { AudioGetCurrentHostTime() };
            Some(coreaudio_host_time_to_mono_ns(host_time))
        };

        let input = unsafe {
            std::slice::from_raw_parts(
                buffer.audio_data as *const u8,
                buffer.audio_data_byte_size as usize,
            )
        };
        ingest_capture_bytes(&context.binding, input, capture_timestamp_ns);
    }

    if stream_shutdown(&context.binding) {
        return;
    }

    // re-arm input buffer for the next capture callback
    buffer.audio_data_byte_size = buffer.audio_data_bytes_capacity;

    let _ = unsafe { AudioQueueEnqueueBuffer(in_aq, in_buffer, 0, ptr::null()) };
}

/// Handle one no-op callback for CoreAudio loopback probing.
#[cfg(target_os = "macos")]
pub(super) unsafe extern "C" fn loopback_probe_input_callback(
    _in_user_data: *mut c_void,
    _in_aq: AudioQueueRef,
    _in_buffer: AudioQueueBufferRef,
    _in_start_time: *const AudioTimeStamp,
    _in_number_packet_descriptions: u32,
    _in_packet_descriptions: *const c_void,
) {
}
