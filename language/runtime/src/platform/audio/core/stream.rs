use super::*;
use crate::platform::audio::host;

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
    state.last_input_adc_ns = input_adc_ns.unwrap_or(callback_mono_ns);
    state.last_output_dac_ns = output_dac_ns.unwrap_or(callback_mono_ns);
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
    context: &BindingCallContext,
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
        state.last_callback_mono_ns
    };

    AudioStreamTiming {
        stream_frames: state.stream_frames,
        stream_time_ns: state.last_callback_mono_ns,
        has_input_adc_time: state.last_input_adc_ns > 0,
        input_adc_time_ns: state.last_input_adc_ns,
        has_output_dac_time: state.last_output_dac_ns > 0,
        output_dac_time_ns: state.last_output_dac_ns,
        callback_time_ns: state.last_callback_mono_ns,
        device_clock_ns,
        monotonic_clock_ns: context.runtime().time.mono_nanos(),
        drift_ppm: estimate_drift_ppm(&state, binding.sample_rate),
        callback_cpu_load: state.last_callback_cpu_load,
    }
}

/// Build one stream availability snapshot.
pub(crate) fn stream_availability_snapshot(
    context: &BindingCallContext,
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
        timestamp_ns: context.runtime().time.mono_nanos(),
    }
}

/// Build one stream snapshot payload.
pub(crate) fn stream_snapshot(
    context: &BindingCallContext,
    binding: &AudioStreamBinding,
) -> AudioStreamSnapshot {
    let state = binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    AudioStreamSnapshot {
        backend: binding.device.backend,
        backend_id: context.store_string(host::backend_name(binding.device.backend)),
        device_id: context.store_string(&binding.device.id),
        sample_rate: binding.sample_rate,
        channels: binding.channels,
        channel_layout: binding.requested.channel_layout,
        channel_mask: binding.requested.channel_mask,
        format: binding.requested.format,
        period_frames: binding.period_frames,
        transfer_mode: binding.requested.transfer_mode,
        share_mode: binding.share_mode,
        period_jitter_ns: state.last_period_jitter_ns,
        non_interleaved: (binding.requested.flags.0 & STREAM_FLAG_NON_INTERLEAVED.0) != 0
            && binding.runtime_capabilities.supports_non_interleaved,
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
        let period_frames = binding.period_frames.max(MIN_STREAM_PERIOD_FRAMES) as usize;
        let period_duration = Duration::from_nanos(
            (period_frames as u64)
                .saturating_mul(1_000_000_000u64)
                .checked_div(binding.sample_rate.max(1) as u64)
                .unwrap_or(1_000_000),
        );

        loop {
            let mut state = binding
                .sync
                .state
                .lock()
                .unwrap_or_else(|error| error.into_inner());

            if state.shutdown {
                break;
            }

            if state.running {
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
            }

            drop(state);
            binding.sync.wake.notify_all();
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
    drop(state);
    binding.sync.wake.notify_all();
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

    let binding = Arc::new(AudioStreamBinding {
        device: device.clone(),
        direction: opened_direction,
        requested: config,
        sample_rate: config.sample_rate,
        channels: config.channels,
        period_frames: config.period_frames.max(MIN_STREAM_PERIOD_FRAMES),
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
        null_worker: Mutex::new(None),
    });

    let worker = build_null_worker(binding.clone());
    *binding
        .null_worker
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(worker);

    binding
}
