use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::thread::JoinHandle;

use crate::diagnostic::RuntimeResult;
use crate::platform::audio::{
    AudioDeviceDirection, AudioEventKind, AudioShareMode, AudioStreamAvailability,
    AudioStreamConfig, AudioStreamDescriptor, AudioStreamFlags, AudioStreamRequirementFlags,
    AudioStreamState, AudioStreamStateKind, AudioStreamStatusFlags, AudioStreamTiming, host,
};
use crate::runtime::BindingCallContext;

use super::{
    AudioStreamBinding, AudioStreamRuntimeCapabilities, AudioStreamStateInner, AudioStreamSync,
    DEVICE_CAPABILITY_BIT_EXACT_PCM, HostDeviceDescriptor, MIN_STREAM_PERIOD_FRAMES,
    STREAM_FLAG_EXPLICIT_SAMPLE_FORMAT, STREAM_FLAG_MINIMIZE_LATENCY, STREAM_FLAG_NEVER_DROP_INPUT,
    STREAM_FLAG_NO_AUTO_CONVERT, STREAM_FLAG_NON_INTERLEAVED, STREAM_FLAG_PRIME_OUTPUT_BUFFERS,
    STREAM_FLAG_REPORT_XRUN, STREAM_FLAG_SCHEDULE_REALTIME, STREAM_REQUIRE_BIT_EXACT_PCM,
    STREAM_REQUIRE_HARDWARE_TIMESTAMPS, STREAM_REQUIRE_NON_INTERLEAVED, STREAM_REQUIRE_PAUSE,
    STREAM_REQUIRE_SCHEDULED_WRITE, STREAM_STATUS_INPUT_OVERFLOW, STREAM_STATUS_OUTPUT_UNDERFLOW,
    host_monotonic_nanos, initial_stream_state, publish_stream_event_native,
    resolved_max_queued_frames, resolved_worker_poll_period, stream_handle_for_binding,
};

/// Convert one frame count into nanoseconds for one sample rate.
fn frames_to_nanos(frame_count: u64, sample_rate: u32) -> u64 {
    frame_count
        .saturating_mul(1_000_000_000u64)
        .checked_div(sample_rate.max(1) as u64)
        .unwrap_or(0)
}

/// Estimate drift in ppm from callback history.
fn estimate_drift_ppm(state: &AudioStreamStateInner, sample_rate: u32) -> f64 {
    // require one initialized callback baseline
    if state.first_callback_mono_ns == 0 {
        return 0.0;
    }

    // require one callback interval window
    if state.last_callback_mono_ns <= state.first_callback_mono_ns {
        return 0.0;
    }

    let elapsed_frames = state
        .stream_frames
        .saturating_sub(state.first_callback_stream_frames);
    if elapsed_frames == 0 {
        return 0.0;
    }

    let expected_ns = frames_to_nanos(elapsed_frames, sample_rate);
    if expected_ns == 0 {
        return 0.0;
    }

    let observed_ns = state
        .last_callback_mono_ns
        .saturating_sub(state.first_callback_mono_ns);

    let drift_ratio = (observed_ns as f64 - expected_ns as f64) / expected_ns as f64;
    let drift_ppm = drift_ratio * 1_000_000.0;
    if drift_ppm.is_finite() {
        drift_ppm
    } else {
        0.0
    }
}

/// Record one callback timing sample and update drift and jitter estimates.
pub(crate) fn record_stream_callback_timing(
    state: &mut AudioStreamStateInner,
    sample_rate: u32,
    frame_count: u32,
    callback_mono_ns: u64,
    input_adc_ns: Option<u64>,
    output_dac_ns: Option<u64>,
) {
    let previous_callback_ns = state.last_callback_mono_ns;

    // compute one per-period jitter estimate from callback deltas
    if previous_callback_ns > 0 {
        let observed_period_ns = callback_mono_ns.saturating_sub(previous_callback_ns);
        let expected_period_ns = frames_to_nanos(frame_count as u64, sample_rate);
        state.last_period_jitter_ns = observed_period_ns.abs_diff(expected_period_ns);
    }

    // advance one stream frame counter
    state.stream_frames = state.stream_frames.saturating_add(frame_count as u64);

    // seed one drift-estimation baseline on the first callback sample
    if state.first_callback_mono_ns == 0 {
        state.first_callback_mono_ns = callback_mono_ns;
        state.first_callback_stream_frames = state.stream_frames;
    }

    // publish callback and endpoint timestamps
    state.last_callback_mono_ns = callback_mono_ns;
    state.last_input_adc_ns = input_adc_ns.unwrap_or(0);
    state.last_output_dac_ns = output_dac_ns.unwrap_or(0);
    state.last_callback_cpu_load = 0.0;
}

/// Build one stream state snapshot.
pub(crate) fn stream_state_snapshot(binding: &AudioStreamBinding) -> AudioStreamState {
    let state = binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let buffered_frames = if binding.direction == AudioDeviceDirection::Capture
        || binding.direction == AudioDeviceDirection::Loopback
    {
        binding.buffered_capture_frames(&state)
    } else {
        binding.buffered_playback_frames(&state)
    };

    let latency_ns = (binding.period_frames as u64)
        .saturating_mul(1_000_000_000u64)
        .checked_div(binding.sample_rate.max(1) as u64)
        .unwrap_or(0);

    AudioStreamState {
        state: state.state,
        running: state.running,
        paused: state.paused,
        buffered_frames,
        input_latency_ns: latency_ns,
        output_latency_ns: latency_ns,
        total_latency_ns: latency_ns.saturating_mul(2),
        status_flags: state.status_flags,
        xrun_count: state.xrun_count,
        input_underflow_count: state.input_underflow_count,
        input_overflow_count: state.input_overflow_count,
        output_underflow_count: state.output_underflow_count,
        output_overflow_count: state.output_overflow_count,
        callback_cpu_load: state.last_callback_cpu_load,
    }
}

/// Build one stream timing snapshot.
pub(crate) fn stream_timing_snapshot(
    binding_2: &BindingCallContext,
    binding: &AudioStreamBinding,
) -> AudioStreamTiming {
    let state = binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // prefer output then input timestamp lanes for device-clock correlation
    let device_clock_ns = if state.last_output_dac_ns > 0 {
        state.last_output_dac_ns
    } else if state.last_input_adc_ns > 0 {
        state.last_input_adc_ns
    } else {
        0
    };

    AudioStreamTiming {
        stream_frames: state.stream_frames,
        stream_time_ns: state.last_callback_mono_ns,
        input_adc_time_ns: if state.last_input_adc_ns > 0 {
            Some(state.last_input_adc_ns)
        } else {
            None
        },
        output_dac_time_ns: if state.last_output_dac_ns > 0 {
            Some(state.last_output_dac_ns)
        } else {
            None
        },
        callback_time_ns: if state.last_callback_mono_ns > 0 {
            Some(state.last_callback_mono_ns)
        } else {
            None
        },
        device_clock_ns: if binding.runtime_capabilities.supports_hardware_timestamps
            && device_clock_ns > 0
        {
            Some(device_clock_ns)
        } else {
            None
        },
        monotonic_clock_ns: binding_2.world().mono_nanos(),
        drift_ppm: estimate_drift_ppm(&state, binding.sample_rate),
        callback_cpu_load: state.last_callback_cpu_load,
    }
}

/// Build one stream-option mask from stream runtime capabilities.
pub(crate) fn effective_stream_flags(binding: &AudioStreamBinding) -> AudioStreamFlags {
    let mut flags = STREAM_FLAG_REPORT_XRUN.0;
    flags |= STREAM_FLAG_MINIMIZE_LATENCY.0;
    flags |= STREAM_FLAG_SCHEDULE_REALTIME.0;
    flags |= STREAM_FLAG_EXPLICIT_SAMPLE_FORMAT.0;
    flags |= STREAM_FLAG_NO_AUTO_CONVERT.0;
    flags |= STREAM_FLAG_NEVER_DROP_INPUT.0;
    flags |= STREAM_FLAG_PRIME_OUTPUT_BUFFERS.0;

    if binding.runtime_capabilities.supports_non_interleaved {
        flags |= STREAM_FLAG_NON_INTERLEAVED.0;
    }

    AudioStreamFlags(flags)
}

/// Build one stream-requirement mask satisfied by this stream.
pub(crate) fn satisfied_stream_requirements(
    binding: &AudioStreamBinding,
) -> AudioStreamRequirementFlags {
    let mut requirements = 0u32;
    if binding.runtime_capabilities.supports_non_interleaved {
        requirements |= STREAM_REQUIRE_NON_INTERLEAVED.0;
    }
    if binding.runtime_capabilities.supports_write_at {
        requirements |= STREAM_REQUIRE_SCHEDULED_WRITE.0;
    }
    if binding.runtime_capabilities.supports_pause {
        requirements |= STREAM_REQUIRE_PAUSE.0;
    }
    if binding.runtime_capabilities.supports_hardware_timestamps {
        requirements |= STREAM_REQUIRE_HARDWARE_TIMESTAMPS.0;
    }
    if (binding.device.capability_flags.0 & DEVICE_CAPABILITY_BIT_EXACT_PCM.0) != 0 {
        requirements |= STREAM_REQUIRE_BIT_EXACT_PCM.0;
    }

    AudioStreamRequirementFlags(requirements)
}

/// Build one stream availability snapshot.
pub(crate) fn stream_availability_snapshot(
    binding_2: &BindingCallContext,
    binding: &AudioStreamBinding,
) -> AudioStreamAvailability {
    let state = binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    let readable_frames = binding.buffered_capture_frames(&state);
    let writable_frames = binding
        .playback_capacity_samples()
        .saturating_sub(state.playback_samples.len())
        .checked_div(binding.channels as usize)
        .unwrap_or(0) as u64;

    AudioStreamAvailability {
        readable_frames,
        writable_frames,
        min_transfer_frames: binding.period_frames,
        max_transfer_frames: binding.period_frames,
        timestamp_ns: binding_2.world().mono_nanos(),
    }
}

/// Build one stream snapshot payload.
pub(crate) fn stream_descriptor(
    binding_2: &BindingCallContext,
    binding: &AudioStreamBinding,
) -> AudioStreamDescriptor {
    let state = binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    AudioStreamDescriptor {
        backend: binding.device.backend,
        backend_id: binding_2.store_string(host::backend_name(binding.device.backend)),
        device_id: binding_2.store_string(&binding.device.id),
        sample_rate: binding.sample_rate,
        channels: binding.channels,
        channel_layout: binding.requested.channel_layout,
        channel_mask: binding.requested.channel_mask,
        format: binding.requested.format,
        period_frames: binding.period_frames,
        transfer_mode: binding.requested.transfer_mode,
        share_mode: binding.share_mode,
        requested_flags: AudioStreamFlags(0),
        requested_requirements: AudioStreamRequirementFlags(0),
        effective_flags: effective_stream_flags(binding),
        effective_requirements: satisfied_stream_requirements(binding),
        period_jitter_ns: state.last_period_jitter_ns,
        non_interleaved: binding.runtime_capabilities.supports_non_interleaved,
        supports_write_at: binding.runtime_capabilities.supports_write_at,
        supports_pause: binding.runtime_capabilities.supports_pause,
        supports_non_interleaved: binding.runtime_capabilities.supports_non_interleaved,
        supports_volume: binding.runtime_capabilities.supports_volume,
        supports_mute: binding.runtime_capabilities.supports_mute,
        supports_hardware_timestamps: binding.runtime_capabilities.supports_hardware_timestamps,
    }
}

/// Build one null backend worker thread.
pub(crate) fn build_null_worker(binding: Arc<AudioStreamBinding>) -> JoinHandle<()> {
    thread::spawn(move || {
        let period_frames_u32 = binding.period_frames.max(MIN_STREAM_PERIOD_FRAMES);
        let period_frames = period_frames_u32 as usize;
        let period_duration = resolved_worker_poll_period(period_frames_u32, binding.sample_rate);

        loop {
            let mut state = binding
                .sync
                .state
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let mut xrun_event = None;

            if state.shutdown {
                break;
            }

            if state.running {
                let previous_xrun_count = state.xrun_count;
                let scalar_period = period_frames.saturating_mul(binding.channels as usize);
                state.status_flags = AudioStreamStatusFlags(0);

                if !state.paused
                    && (binding.direction == AudioDeviceDirection::Playback
                        || binding.direction == AudioDeviceDirection::Duplex)
                {
                    for _ in 0..scalar_period {
                        if state.playback_samples.pop_front().is_none() {
                            state.xrun_count = state.xrun_count.saturating_add(1);
                            state.output_underflow_count =
                                state.output_underflow_count.saturating_add(1);
                            state.status_flags = AudioStreamStatusFlags(
                                state.status_flags.0 | STREAM_STATUS_OUTPUT_UNDERFLOW.0,
                            );
                        }
                    }
                }

                if !state.paused
                    && (binding.direction == AudioDeviceDirection::Capture
                        || binding.direction == AudioDeviceDirection::Duplex
                        || binding.direction == AudioDeviceDirection::Loopback)
                {
                    for _ in 0..scalar_period {
                        state.capture_samples.push_back(0.0);
                    }
                    let max_capture = binding.capture_capacity_samples();
                    if state.capture_samples.len() > max_capture {
                        let extra = state.capture_samples.len() - max_capture;
                        for _ in 0..extra {
                            let _ = state.capture_samples.pop_front();
                        }
                        state.xrun_count = state.xrun_count.saturating_add(1);
                        state.input_overflow_count = state.input_overflow_count.saturating_add(1);
                        state.status_flags = AudioStreamStatusFlags(
                            state.status_flags.0 | STREAM_STATUS_INPUT_OVERFLOW.0,
                        );
                    }
                }

                let callback_mono_ns = host_monotonic_nanos();
                record_stream_callback_timing(
                    &mut state,
                    binding.sample_rate,
                    period_frames as u32,
                    callback_mono_ns,
                    Some(callback_mono_ns),
                    Some(callback_mono_ns),
                );

                let xrun_count_delta = state.xrun_count.saturating_sub(previous_xrun_count);
                if xrun_count_delta > 0 {
                    xrun_event = Some((state.status_flags, xrun_count_delta));
                }
            }

            drop(state);
            binding.sync.wake.notify_all();

            if let Some((status_flags, xrun_count_delta)) = xrun_event
                && let Some(stream_handle) = stream_handle_for_binding(&binding)
            {
                publish_stream_event_native(
                    stream_handle,
                    &binding,
                    AudioEventKind::StreamXRun,
                    status_flags,
                    xrun_count_delta,
                );
            }

            thread::sleep(period_duration);
        }
    })
}

/// Mark one stream as backend-disconnected and wake blocked callers.
#[cfg(any(
    all(target_os = "android", feature = "audio-aaudio"),
    all(target_os = "android", feature = "audio-opensles"),
    all(target_os = "linux", feature = "audio-pipewire"),
    all(target_os = "linux", feature = "audio-pulseaudio"),
))]
pub(crate) fn mark_stream_backend_disconnected(
    binding: &AudioStreamBinding,
    message: impl Into<String>,
) {
    let mut state = binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    state.running = false;
    state.paused = false;
    state.shutdown = true;
    state.state = AudioStreamStateKind::BackendDisconnected;
    state.status_flags =
        AudioStreamStatusFlags(state.status_flags.0 | STREAM_STATUS_OUTPUT_UNDERFLOW.0);
    state.last_backend_message = Some(message.into());
    let status_flags = state.status_flags;
    drop(state);
    binding.sync.wake.notify_all();

    if let Some(stream_handle) = stream_handle_for_binding(binding) {
        publish_stream_event_native(
            stream_handle,
            binding,
            AudioEventKind::BackendDisconnected,
            status_flags,
            0,
        );
        publish_stream_event_native(
            stream_handle,
            binding,
            AudioEventKind::StreamStateChanged,
            status_flags,
            0,
        );
    }
}

/// Run one backend stream start hook when available.
pub(crate) fn host_stream_start(binding: &AudioStreamBinding) -> RuntimeResult<()> {
    let host_ops = binding
        .host_ops
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    if let Some(host_ops) = host_ops {
        host_ops.start()?;
    }

    Ok(())
}

/// Run one backend stream pause hook when available.
pub(crate) fn host_stream_pause(binding: &AudioStreamBinding, pause: bool) -> RuntimeResult<()> {
    let host_ops = binding
        .host_ops
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    if let Some(host_ops) = host_ops {
        host_ops.pause(pause)?;
    }

    Ok(())
}

/// Run one backend stream stop hook when available.
pub(crate) fn host_stream_stop(binding: &AudioStreamBinding) -> RuntimeResult<()> {
    let host_ops = binding
        .host_ops
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    if let Some(host_ops) = host_ops {
        host_ops.stop()?;
    }

    Ok(())
}

/// Run one backend stream flush hook when available.
pub(crate) fn host_stream_flush(binding: &AudioStreamBinding) -> RuntimeResult<()> {
    let host_ops = binding
        .host_ops
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    if let Some(host_ops) = host_ops {
        host_ops.flush()?;
    }

    Ok(())
}

/// Build one stream binding for the null backend.
pub(crate) fn open_null_stream(
    device: &HostDeviceDescriptor,
    opened_direction: AudioDeviceDirection,
    config: AudioStreamConfig,
    share_mode: AudioShareMode,
) -> Arc<AudioStreamBinding> {
    let sync = Arc::new(AudioStreamSync {
        state: Mutex::new(initial_stream_state()),
        wake: Condvar::new(),
    });

    let stream_binding = Arc::new(AudioStreamBinding {
        device: device.clone(),
        direction: opened_direction,
        requested: config,
        sample_rate: config.sample_rate,
        channels: config.channels,
        period_frames: config.period_frames.max(MIN_STREAM_PERIOD_FRAMES),
        max_queued_frames: resolved_max_queued_frames(),
        share_mode,
        runtime_capabilities: AudioStreamRuntimeCapabilities {
            supports_write_at: opened_direction != AudioDeviceDirection::Capture
                && opened_direction != AudioDeviceDirection::Loopback,
            supports_pause: true,
            supports_non_interleaved: false,
            supports_volume: true,
            supports_mute: true,
            supports_hardware_timestamps: true,
        },
        host_ops: Mutex::new(None),
        name: Mutex::new(String::new()),
        sync: sync.clone(),
        stream_handle_raw: std::sync::atomic::AtomicU64::new(0),
        event_runtime_state: Mutex::new(None),
        null_worker: Mutex::new(None),
    });

    let worker = build_null_worker(stream_binding.clone());
    *stream_binding
        .null_worker
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(worker);

    stream_binding
}
