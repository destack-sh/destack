use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::diagnostic::RuntimeResult;
#[cfg(any(
    all(target_os = "android", feature = "audio-aaudio"),
    all(target_os = "android", feature = "audio-opensles"),
    all(target_os = "linux", feature = "audio-pipewire"),
    all(target_os = "linux", feature = "audio-pulseaudio"),
))]
use crate::platform::audio::AudioStreamStateKind;
use crate::platform::audio::{
    AudioDeviceDirection, AudioEventKind, AudioShareMode, AudioStreamAvailability,
    AudioStreamConfig, AudioStreamDescriptor, AudioStreamFlags, AudioStreamRequirementFlags,
    AudioStreamState, AudioStreamStatusFlags, AudioStreamTiming, backend as audio_backend,
};
use crate::runtime::{BindingCallContext, ExecutionMode, ExecutionPolicy, start_with_policy};

use super::constants::{
    DEVICE_CAPABILITY_BIT_EXACT_PCM, MIN_STREAM_PERIOD_FRAMES, STREAM_FLAG_NON_INTERLEAVED,
    STREAM_FLAG_REPORT_XRUN, STREAM_REQUIRE_BIT_EXACT_PCM, STREAM_REQUIRE_HARDWARE_TIMESTAMPS,
    STREAM_REQUIRE_NON_INTERLEAVED, STREAM_REQUIRE_PAUSE, STREAM_REQUIRE_SCHEDULED_WRITE,
    STREAM_STATUS_INPUT_OVERFLOW, STREAM_STATUS_OUTPUT_UNDERFLOW, host_monotonic_nanos,
    resolved_max_queued_frames, resolved_worker_poll_period,
};
use super::error::{stream_shutdown_error, stream_state_is_terminal};
use super::event::publish::publish_stream_event_native;
use super::model::{
    AudioStreamHostState, AudioStreamRuntimeCapabilities, AudioStreamStateInner, AudioStreamSync,
    HostDeviceDescriptor, initial_stream_state,
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

/// Wait for one worker-period slice or one stream wakeup.
pub(crate) fn wait_for_worker_period(stream: &AudioStreamHostState, period_duration: Duration) {
    // worker pacing
    let state = stream
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let wait = stream
        .sync
        .wake
        .wait_timeout(state, period_duration)
        .unwrap_or_else(|error| error.into_inner());

    drop(wait.0);
}

/// Wait until one target presentation time or fail when the stream terminates.
pub(crate) fn wait_for_stream_presentation_time(
    ctx: &BindingCallContext,
    stream: &AudioStreamHostState,
    operation: &'static str,
    presentation_time_ns: u64,
) -> RuntimeResult<()> {
    loop {
        // stop waiting once the target presentation time is reached
        let now_ns = ctx.world().mono_nanos();
        if now_ns >= presentation_time_ns {
            return Ok(());
        }

        // fail loudly when the stream can no longer accept writes
        let mut state = stream
            .sync
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if stream_state_is_terminal(&state) {
            return Err(stream_shutdown_error(operation, &state));
        }

        // wait until one stream wake or the target presentation deadline
        let remaining_ns = presentation_time_ns.saturating_sub(now_ns);
        let duration = Duration::from_nanos(remaining_ns.max(1));
        let wait = stream
            .sync
            .wake
            .wait_timeout(state, duration)
            .unwrap_or_else(|error| error.into_inner());

        state = wait.0;
        if stream_state_is_terminal(&state) {
            return Err(stream_shutdown_error(operation, &state));
        }

        drop(state);
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
pub(crate) fn stream_state_snapshot(stream: &AudioStreamHostState) -> AudioStreamState {
    let state = stream
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let buffered_frames = if stream.direction == AudioDeviceDirection::Capture
        || stream.direction == AudioDeviceDirection::Loopback
    {
        stream.buffered_capture_frames(&state)
    } else {
        stream.buffered_playback_frames(&state)
    };

    let latency_ns = (stream.period_frames as u64)
        .saturating_mul(1_000_000_000u64)
        .checked_div(stream.sample_rate.max(1) as u64)
        .unwrap_or(0);

    AudioStreamState {
        state: state.state_kind,
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
    ctx: &BindingCallContext,
    stream: &AudioStreamHostState,
) -> AudioStreamTiming {
    let state = stream
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
        device_clock_ns: if stream.runtime_capabilities.supports_hardware_timestamps
            && device_clock_ns > 0
        {
            Some(device_clock_ns)
        } else {
            None
        },
        monotonic_clock_ns: ctx.world().mono_nanos(),
        drift_ppm: estimate_drift_ppm(&state, stream.sample_rate),
        callback_cpu_load: state.last_callback_cpu_load,
    }
}

/// Build one stream-option mask from stream runtime capabilities.
pub(crate) fn effective_stream_flags(stream: &AudioStreamHostState) -> AudioStreamFlags {
    let mut flags = stream.requested_flags.0
        & super::device::supported_backend_stream_flags(stream.device.backend).0;

    if stream.runtime_capabilities.supports_non_interleaved {
        flags |= STREAM_FLAG_NON_INTERLEAVED.0;
    } else {
        flags &= !STREAM_FLAG_NON_INTERLEAVED.0;
    }

    // xrun reporting is part of the core stream state for all current stream backends
    flags |= STREAM_FLAG_REPORT_XRUN.0;

    AudioStreamFlags(flags)
}

/// Build one stream-requirement mask satisfied by this stream.
pub(crate) fn satisfied_stream_requirements(
    stream: &AudioStreamHostState,
) -> AudioStreamRequirementFlags {
    let mut requirements = 0u32;
    if stream.runtime_capabilities.supports_non_interleaved {
        requirements |= STREAM_REQUIRE_NON_INTERLEAVED.0;
    }
    if stream.runtime_capabilities.supports_write_at {
        requirements |= STREAM_REQUIRE_SCHEDULED_WRITE.0;
    }
    if stream.runtime_capabilities.supports_pause {
        requirements |= STREAM_REQUIRE_PAUSE.0;
    }
    if stream.runtime_capabilities.supports_hardware_timestamps {
        requirements |= STREAM_REQUIRE_HARDWARE_TIMESTAMPS.0;
    }
    if (stream.device.capability_flags.0 & DEVICE_CAPABILITY_BIT_EXACT_PCM.0) != 0 {
        requirements |= STREAM_REQUIRE_BIT_EXACT_PCM.0;
    }

    AudioStreamRequirementFlags(requirements)
}

/// Build one stream availability snapshot.
pub(crate) fn stream_availability_snapshot(
    ctx: &BindingCallContext,
    stream: &AudioStreamHostState,
) -> AudioStreamAvailability {
    let state = stream
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    let readable_frames = stream.buffered_capture_frames(&state);
    let writable_frames = stream
        .playback_capacity_samples()
        .saturating_sub(state.playback_samples.len())
        .checked_div(stream.channels as usize)
        .unwrap_or(0) as u64;

    AudioStreamAvailability {
        readable_frames,
        writable_frames,
        min_transfer_frames: stream.period_frames,
        max_transfer_frames: stream.period_frames,
        timestamp_ns: ctx.world().mono_nanos(),
    }
}

/// Build one stream snapshot payload.
pub(crate) fn stream_descriptor(
    ctx: &BindingCallContext,
    stream: &AudioStreamHostState,
) -> AudioStreamDescriptor {
    let state = stream
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    AudioStreamDescriptor {
        backend: stream.device.backend,
        backend_id: ctx.store_string(audio_backend::backend_name(stream.device.backend)),
        device_id: ctx.store_string(&stream.device.id),
        sample_rate: stream.sample_rate,
        channels: stream.channels,
        channel_layout: stream.requested.channel_layout,
        channel_mask: stream.requested.channel_mask,
        format: stream.requested.format,
        period_frames: stream.period_frames,
        transfer_mode: stream.requested.transfer_mode,
        share_mode: stream.share_mode,
        requested_flags: stream.requested_flags,
        requested_requirements: stream.requested_requirements,
        effective_flags: effective_stream_flags(stream),
        effective_requirements: satisfied_stream_requirements(stream),
        period_jitter_ns: state.last_period_jitter_ns,
        non_interleaved: stream.runtime_capabilities.supports_non_interleaved,
        supports_write_at: stream.runtime_capabilities.supports_write_at,
        supports_pause: stream.runtime_capabilities.supports_pause,
        supports_non_interleaved: stream.runtime_capabilities.supports_non_interleaved,
        supports_volume: stream.runtime_capabilities.supports_volume,
        supports_mute: stream.runtime_capabilities.supports_mute,
        supports_hardware_timestamps: stream.runtime_capabilities.supports_hardware_timestamps,
    }
}

/// Build one synthetic stream worker thread.
pub(crate) fn build_synthetic_stream_worker(stream: Arc<AudioStreamHostState>) -> JoinHandle<()> {
    let worker_result = start_with_policy(
        "destack-audio-synthetic-stream",
        "destack.audio.stream.open",
        ExecutionPolicy::resource(ExecutionMode::Polling),
        move || {
            let period_frames_u32 = stream.period_frames.max(MIN_STREAM_PERIOD_FRAMES);
            let period_frames = period_frames_u32 as usize;
            let period_duration =
                resolved_worker_poll_period(period_frames_u32, stream.sample_rate);

            loop {
                let mut state = stream
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
                    let scalar_period = period_frames.saturating_mul(stream.channels as usize);
                    state.status_flags = AudioStreamStatusFlags(0);

                    if !state.paused
                        && (stream.direction == AudioDeviceDirection::Playback
                            || stream.direction == AudioDeviceDirection::Duplex)
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
                        && (stream.direction == AudioDeviceDirection::Capture
                            || stream.direction == AudioDeviceDirection::Duplex
                            || stream.direction == AudioDeviceDirection::Loopback)
                    {
                        for _ in 0..scalar_period {
                            state.capture_samples.push_back(0.0);
                        }
                        let max_capture = stream.capture_capacity_samples();
                        if state.capture_samples.len() > max_capture {
                            let extra = state.capture_samples.len() - max_capture;
                            for _ in 0..extra {
                                let _ = state.capture_samples.pop_front();
                            }
                            state.xrun_count = state.xrun_count.saturating_add(1);
                            state.input_overflow_count =
                                state.input_overflow_count.saturating_add(1);
                            state.status_flags = AudioStreamStatusFlags(
                                state.status_flags.0 | STREAM_STATUS_INPUT_OVERFLOW.0,
                            );
                        }
                    }

                    let callback_mono_ns = host_monotonic_nanos();
                    record_stream_callback_timing(
                        &mut state,
                        stream.sample_rate,
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
                stream.sync.wake.notify_all();

                if let Some((status_flags, xrun_count_delta)) = xrun_event
                    && let Some(stream_handle) = stream.stream_handle()
                {
                    publish_stream_event_native(
                        stream_handle,
                        &stream,
                        AudioEventKind::StreamXRun,
                        status_flags,
                        xrun_count_delta,
                    );
                }

                wait_for_worker_period(&stream, period_duration);
            }
        },
    );

    worker_result.unwrap_or_else(|error| {
        panic!("failed to start synthetic audio stream worker: {error}");
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
    stream: &AudioStreamHostState,
    message: impl Into<String>,
) {
    let mut state = stream
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    state.running = false;
    state.paused = false;
    state.shutdown = true;
    state.state_kind = AudioStreamStateKind::BackendDisconnected;
    state.status_flags =
        AudioStreamStatusFlags(state.status_flags.0 | STREAM_STATUS_OUTPUT_UNDERFLOW.0);
    state.last_backend_message = Some(message.into());
    let status_flags = state.status_flags;
    drop(state);
    stream.sync.wake.notify_all();

    if let Some(stream_handle) = stream.stream_handle() {
        publish_stream_event_native(
            stream_handle,
            stream,
            AudioEventKind::BackendDisconnected,
            status_flags,
            0,
        );
        publish_stream_event_native(
            stream_handle,
            stream,
            AudioEventKind::StreamStateChanged,
            status_flags,
            0,
        );
    }
}

/// Run one backend stream start hook when available.
pub(crate) fn host_stream_start(stream: &AudioStreamHostState) -> RuntimeResult<()> {
    let host_ops = stream
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
pub(crate) fn host_stream_pause(stream: &AudioStreamHostState, pause: bool) -> RuntimeResult<()> {
    let host_ops = stream
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
pub(crate) fn host_stream_stop(stream: &AudioStreamHostState) -> RuntimeResult<()> {
    let host_ops = stream
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
pub(crate) fn host_stream_flush(stream: &AudioStreamHostState) -> RuntimeResult<()> {
    let host_ops = stream
        .host_ops
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    if let Some(host_ops) = host_ops {
        host_ops.flush()?;
    }

    Ok(())
}

/// Build one stream host state for the null backend.
pub(crate) fn open_null_stream(
    device: &HostDeviceDescriptor,
    opened_direction: AudioDeviceDirection,
    config: AudioStreamConfig,
    share_mode: AudioShareMode,
    requested_flags: AudioStreamFlags,
    requested_requirements: AudioStreamRequirementFlags,
) -> Arc<AudioStreamHostState> {
    let sync = Arc::new(AudioStreamSync {
        state: Mutex::new(initial_stream_state()),
        wake: Condvar::new(),
    });

    let stream = Arc::new(AudioStreamHostState {
        device: device.clone(),
        direction: opened_direction,
        requested: config,
        requested_flags,
        requested_requirements,
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
        runtime_owner: Mutex::new(None),
        worker_thread: Mutex::new(None),
    });

    let worker = build_synthetic_stream_worker(stream.clone());
    *stream
        .worker_thread
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(worker);

    stream
}
