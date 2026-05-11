use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::audio::core as audio_core;
use crate::platform::audio::core::codec::clamp_audio_scalar;
use crate::runtime::{ExecutionMode, ExecutionPolicy, start_with_policy};
use std::ffi::{c_int, c_void};
use std::ptr;
use std::sync::{Arc, Condvar, Mutex, Weak};
use std::time::Duration;

use super::abi::{PipewireSampleSpec, PipewireSimple};
use super::constants::{
    PIPEWIRE_APPLICATION_NAME, PIPEWIRE_DEFAULT_DEVICE_NAME, PIPEWIRE_STREAM_DIRECTION_CAPTURE,
    PIPEWIRE_STREAM_DIRECTION_PLAYBACK, PIPEWIRE_STREAM_NAME,
};
use super::core::{
    PipeWireLibrary, c_string, pipewire_error, pipewire_not_supported, pipewire_sample_format,
    pipewire_succeeded, require_pipewire_library,
};
use super::ids::{parse_pipewire_stable_id, validate_pipewire_stable_id_direction};
use crate::platform::audio as audio_types;

/// One owned PipeWire simple-stream handle.
#[derive(Debug)]
struct PipewireSimpleHandle {
    /// Raw `pa_simple*` pointer.
    raw: *mut PipewireSimple,
    /// Shared PipeWire symbol table.
    library: Arc<PipeWireLibrary>,
}

impl Drop for PipewireSimpleHandle {
    fn drop(&mut self) {
        if self.raw.is_null() {
            return;
        }

        // close one simple stream handle
        unsafe {
            (self.library.api.pa_simple_free)(self.raw);
        }
    }
}

unsafe impl Send for PipewireSimpleHandle {}

/// One PipeWire stream runtime payload.
#[derive(Debug)]
struct PipewireStreamRuntime {
    /// Shared PipeWire symbol table.
    library: Arc<PipeWireLibrary>,
    /// Opened playback lane when present.
    playback: Option<Mutex<PipewireSimpleHandle>>,
    /// Opened capture lane when present.
    capture: Option<Mutex<PipewireSimpleHandle>>,
    /// Negotiated stream sample rate.
    sample_rate: u32,
    /// Negotiated stream channel count.
    channels: u16,
    /// Negotiated stream period size in frames.
    period_frames: u32,
    /// Negotiated stream sample format.
    format: audio_types::AudioSampleFormat,
    /// Stream frame size in bytes.
    frame_bytes: usize,
    /// Worker cycle sleep period.
    poll_period: Duration,
    /// Weak link to one stream host state.
    stream_state: Mutex<Weak<audio_core::AudioStreamHostState>>,
}

/// One host-operations payload for one PipeWire stream host state.
#[derive(Debug)]
struct PipewireHostStreamOps {
    /// Shared PipeWire runtime payload.
    runtime: Arc<PipewireStreamRuntime>,
}

impl audio_core::AudioHostStreamOps for PipewireHostStreamOps {
    fn start(&self) -> RuntimeResult<()> {
        let _ = &self.runtime;
        Ok(())
    }

    fn pause(&self, _pause: bool) -> RuntimeResult<()> {
        // pause and resume are synchronized in the runtime worker loop
        Ok(())
    }

    fn stop(&self) -> RuntimeResult<()> {
        // drain playback on stop to preserve queued output ordering
        drain_runtime_playback(&self.runtime, "destack.audio.stream.stop")?;

        // flush lanes on stop so stale queued data does not leak into restart cycles
        flush_runtime_lanes(&self.runtime, "destack.audio.stream.stop")
    }

    fn flush(&self) -> RuntimeResult<()> {
        flush_runtime_lanes(&self.runtime, "destack.audio.stream.flush")
    }
}

/// Open one PipeWire stream host state.
pub(super) fn open_stream(
    device_info: &audio_core::HostDeviceDescriptor,
    config: audio_types::AudioStreamConfig,
    share_mode: audio_types::AudioShareMode,
    requested_flags: audio_types::AudioStreamFlags,
    requested_requirements: audio_types::AudioStreamRequirementFlags,
) -> RuntimeResult<Arc<audio_core::AudioStreamHostState>> {
    // reject unsupported share-mode requests
    if share_mode != audio_types::AudioShareMode::Shared {
        return Err(pipewire_not_supported(
            "destack.audio.stream.open",
            "PipeWire exclusive mode is not supported",
        ));
    }

    let parsed = parse_pipewire_stable_id(&device_info.id)?;
    validate_pipewire_stable_id_direction(&parsed, device_info.direction)?;

    let library = require_pipewire_library("destack.audio.stream.open")?;
    let sample_format = pipewire_sample_format(config.format).ok_or_else(|| {
        pipewire_not_supported(
            "destack.audio.stream.open",
            format!(
                "sample format {:?} is not supported by PipeWire",
                config.format
            ),
        )
    })?;
    let channels = config.channels.max(1);
    let channels_u8 = u8::try_from(channels).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "config.channels",
            "config.channels exceeds PipeWire channel limits",
        ))
        .boxed()
    })?;

    let sample_spec = PipewireSampleSpec {
        format: sample_format,
        rate: config.sample_rate.max(1),
        channels: channels_u8,
    };

    let needs_playback = matches!(
        device_info.direction,
        audio_types::AudioDeviceDirection::Playback | audio_types::AudioDeviceDirection::Duplex
    );
    let needs_capture = matches!(
        device_info.direction,
        audio_types::AudioDeviceDirection::Capture
            | audio_types::AudioDeviceDirection::Duplex
            | audio_types::AudioDeviceDirection::Loopback
    );

    // open one playback stream lane when one playback direction is requested
    let playback = if needs_playback {
        let device_name = if parsed.playback_name.is_empty() {
            PIPEWIRE_DEFAULT_DEVICE_NAME
        } else {
            parsed.playback_name.as_str()
        };
        Some(open_simple_stream(
            &library,
            device_name,
            PIPEWIRE_STREAM_DIRECTION_PLAYBACK,
            &sample_spec,
            "destack.audio.stream.open",
        )?)
    } else {
        None
    };

    // open one capture stream lane when one capture direction is requested
    let capture = if needs_capture {
        let capture_name = parsed
            .capture_name
            .as_deref()
            .unwrap_or(parsed.playback_name.as_str());
        let capture_name = if capture_name.is_empty() {
            PIPEWIRE_DEFAULT_DEVICE_NAME
        } else {
            capture_name
        };

        Some(open_simple_stream(
            &library,
            capture_name,
            PIPEWIRE_STREAM_DIRECTION_CAPTURE,
            &sample_spec,
            "destack.audio.stream.open",
        )?)
    } else {
        None
    };

    let frame_bytes = audio_core::frame_bytes(config.format, channels)?;
    let period_frames = config
        .period_frames
        .max(audio_core::MIN_STREAM_PERIOD_FRAMES);
    let poll_period = audio_core::resolved_worker_poll_period(period_frames, sample_spec.rate);

    let runtime = Arc::new(PipewireStreamRuntime {
        library,
        playback: playback.map(Mutex::new),
        capture: capture.map(Mutex::new),
        sample_rate: sample_spec.rate,
        channels,
        period_frames,
        format: config.format,
        frame_bytes,
        poll_period,
        stream_state: Mutex::new(Weak::new()),
    });

    let host_ops: Arc<dyn audio_core::AudioHostStreamOps> = Arc::new(PipewireHostStreamOps {
        runtime: runtime.clone(),
    });

    let stream_state = Arc::new(audio_core::AudioStreamHostState {
        device: device_info.clone(),
        direction: device_info.direction,
        requested: config,
        requested_flags,
        requested_requirements,
        sample_rate: runtime.sample_rate,
        channels: runtime.channels,
        period_frames: runtime.period_frames,
        max_queued_frames: audio_core::resolved_max_queued_frames(),
        share_mode,
        runtime_capabilities: audio_core::AudioStreamRuntimeCapabilities {
            supports_write_at: false,
            supports_pause: true,
            supports_non_interleaved: false,
            supports_volume: true,
            supports_mute: true,
            supports_hardware_timestamps: false,
        },
        host_ops: Mutex::new(Some(host_ops)),
        name: Mutex::new(String::new()),
        sync: Arc::new(audio_core::AudioStreamSync {
            state: Mutex::new(audio_core::initial_stream_state()),
            wake: Condvar::new(),
        }),
        stream_handle_raw: std::sync::atomic::AtomicU64::new(0),
        runtime_owner: Mutex::new(None),
        worker_thread: Mutex::new(None),
    });

    *runtime
        .stream_state
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Arc::downgrade(&stream_state);

    let worker = spawn_worker(stream_state.clone(), runtime);
    *stream_state
        .worker_thread
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(worker);

    Ok(stream_state)
}

/// Open one PipeWire simple stream handle.
fn open_simple_stream(
    library: &Arc<PipeWireLibrary>,
    device_name: &str,
    direction: c_int,
    sample_spec: &PipewireSampleSpec,
    operation: &'static str,
) -> RuntimeResult<PipewireSimpleHandle> {
    let application_name = c_string(PIPEWIRE_APPLICATION_NAME, "applicationName")?;
    let stream_name = c_string(PIPEWIRE_STREAM_NAME, "streamName")?;
    let device_name = c_string(device_name, "deviceName")?;
    let mut error = 0;

    // open one pulse simple stream lane
    let stream = unsafe {
        (library.api.pa_simple_new)(
            ptr::null(),
            application_name.as_ptr(),
            direction,
            device_name.as_ptr(),
            stream_name.as_ptr(),
            sample_spec,
            ptr::null(),
            ptr::null(),
            &mut error,
        )
    };
    if stream.is_null() {
        return Err(pipewire_error(
            operation,
            Some(error),
            "failed to open PipeWire stream lane",
        ));
    }

    Ok(PipewireSimpleHandle {
        raw: stream,
        library: library.clone(),
    })
}

/// Flush all opened runtime lanes.
fn flush_runtime_lanes(
    runtime: &PipewireStreamRuntime,
    operation: &'static str,
) -> RuntimeResult<()> {
    if let Some(playback) = runtime.playback.as_ref() {
        let playback = playback.lock().unwrap_or_else(|error| error.into_inner());
        let mut error = 0;

        // flush one playback lane
        let status = unsafe { (runtime.library.api.pa_simple_flush)(playback.raw, &mut error) };
        if !pipewire_succeeded(status) {
            return Err(pipewire_error(
                operation,
                Some(error),
                "failed to flush PipeWire playback lane",
            ));
        }
    }

    if let Some(capture) = runtime.capture.as_ref() {
        let capture = capture.lock().unwrap_or_else(|error| error.into_inner());
        let mut error = 0;

        // flush one capture lane
        let status = unsafe { (runtime.library.api.pa_simple_flush)(capture.raw, &mut error) };
        if !pipewire_succeeded(status) {
            return Err(pipewire_error(
                operation,
                Some(error),
                "failed to flush PipeWire capture lane",
            ));
        }
    }

    Ok(())
}

/// Drain one opened playback lane.
fn drain_runtime_playback(
    runtime: &PipewireStreamRuntime,
    operation: &'static str,
) -> RuntimeResult<()> {
    let Some(playback) = runtime.playback.as_ref() else {
        return Ok(());
    };

    let playback = playback.lock().unwrap_or_else(|error| error.into_inner());
    let mut error = 0;

    // drain one playback lane before stop transitions
    let status = unsafe { (runtime.library.api.pa_simple_drain)(playback.raw, &mut error) };
    if pipewire_succeeded(status) {
        return Ok(());
    }

    Err(pipewire_error(
        operation,
        Some(error),
        "failed to drain PipeWire playback lane",
    ))
}

/// Spawn one PipeWire transfer worker thread.
fn spawn_worker(
    binding: Arc<audio_core::AudioStreamHostState>,
    runtime: Arc<PipewireStreamRuntime>,
) -> std::thread::JoinHandle<()> {
    start_with_policy(
        "destack-audio-pipewire-transfer",
        "destack.audio.stream.open",
        ExecutionPolicy::resource(ExecutionMode::Loop),
        move || {
            loop {
                let mut state = binding
                    .sync
                    .state
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());

                // stop the worker once stream shutdown is requested
                if state.shutdown {
                    break;
                }

                // sleep while the stream is not actively running
                if !state.running || state.paused {
                    drop(state);
                    audio_core::wait_for_worker_period(&binding, runtime.poll_period);
                    continue;
                }

                state.status_flags = audio_types::AudioStreamStatusFlags(0);
                let scalar_period = runtime
                    .period_frames
                    .saturating_mul(runtime.channels as u32)
                    as usize;
                let mut playback_bytes = Vec::new();
                let capture_bytes_len =
                    (runtime.period_frames as usize).saturating_mul(runtime.frame_bytes);
                let mut capture_bytes = vec![0u8; capture_bytes_len];

                // prepare one playback packet for this worker cycle
                if runtime.playback.is_some() {
                    let mut playback_packet = Vec::with_capacity(scalar_period);
                    let mut underflow = false;
                    for _ in 0..scalar_period {
                        if let Some(sample) = state.playback_samples.pop_front() {
                            playback_packet.push(sample);
                        } else {
                            playback_packet.push(0.0);
                            underflow = true;
                        }
                    }

                    if state.muted {
                        playback_packet.fill(0.0);
                    } else if (state.volume - 1.0).abs() > f64::EPSILON {
                        let gain = state.volume as f32;
                        for sample in &mut playback_packet {
                            *sample = clamp_audio_scalar(*sample * gain);
                        }
                    }

                    if underflow {
                        state.xrun_count = state.xrun_count.saturating_add(1);
                        state.output_underflow_count =
                            state.output_underflow_count.saturating_add(1);
                        state.status_flags = audio_types::AudioStreamStatusFlags(
                            state.status_flags.0 | audio_core::STREAM_STATUS_OUTPUT_UNDERFLOW.0,
                        );
                    }

                    playback_bytes =
                        audio_core::encode_audio_bytes(&playback_packet, runtime.format);
                }

                // publish one stream timing sample for this worker cycle
                let callback_mono_ns = audio_core::host_monotonic_nanos();
                audio_core::record_stream_callback_timing(
                    &mut state,
                    runtime.sample_rate,
                    runtime.period_frames,
                    callback_mono_ns,
                    None,
                    None,
                );
                drop(state);

                // write one playback packet to the PipeWire server
                if let Some(playback) = runtime.playback.as_ref() {
                    let playback = playback.lock().unwrap_or_else(|error| error.into_inner());
                    let mut error = 0;
                    let status = unsafe {
                        (runtime.library.api.pa_simple_write)(
                            playback.raw,
                            playback_bytes.as_ptr().cast::<c_void>(),
                            playback_bytes.len(),
                            &mut error,
                        )
                    };
                    if !pipewire_succeeded(status) {
                        mark_backend_disconnected(
                            &binding,
                            format!("PipeWire playback write failed with error code {error}"),
                        );
                        break;
                    }
                }

                // read one capture packet from the PipeWire server
                if let Some(capture) = runtime.capture.as_ref() {
                    let capture = capture.lock().unwrap_or_else(|error| error.into_inner());
                    let mut error = 0;
                    let status = unsafe {
                        (runtime.library.api.pa_simple_read)(
                            capture.raw,
                            capture_bytes.as_mut_ptr().cast::<c_void>(),
                            capture_bytes.len(),
                            &mut error,
                        )
                    };
                    if !pipewire_succeeded(status) {
                        mark_backend_disconnected(
                            &binding,
                            format!("PipeWire capture read failed with error code {error}"),
                        );
                        break;
                    }

                    let capture_packet =
                        match audio_core::decode_audio_bytes(&capture_bytes, runtime.format) {
                            Ok(packet) => packet,
                            Err(error) => {
                                mark_backend_disconnected(
                                    &binding,
                                    format!("PipeWire capture decode failed: {error}"),
                                );
                                break;
                            }
                        };

                    let mut state = binding
                        .sync
                        .state
                        .lock()
                        .unwrap_or_else(|error| error.into_inner());

                    // append one capture packet to the shared capture queue
                    for sample in capture_packet {
                        state.capture_samples.push_back(sample);
                    }

                    // enforce one bounded capture queue and report overflow
                    let capture_capacity = binding.capture_capacity_samples();
                    if state.capture_samples.len() > capture_capacity {
                        let overflow = state.capture_samples.len() - capture_capacity;
                        for _ in 0..overflow {
                            let _ = state.capture_samples.pop_front();
                        }

                        state.xrun_count = state.xrun_count.saturating_add(1);
                        state.input_overflow_count = state.input_overflow_count.saturating_add(1);
                        state.status_flags = audio_types::AudioStreamStatusFlags(
                            state.status_flags.0 | audio_core::STREAM_STATUS_INPUT_OVERFLOW.0,
                        );
                    }
                }

                binding.sync.wake.notify_all();
            }
        },
    )
    .unwrap_or_else(|error| panic!("failed to spawn required attached runtime: {error}"))
}

/// Mark one stream as backend-disconnected and wake blocked callers.
fn mark_backend_disconnected(binding: &audio_core::AudioStreamHostState, message: String) {
    audio_core::mark_stream_backend_disconnected(binding, message);
}
