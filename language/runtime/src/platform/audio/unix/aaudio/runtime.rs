use std::ffi::{c_int, c_void};
use std::ptr;
use std::sync::{Arc, Condvar, Mutex, Weak};
use std::time::Duration;

use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;
use crate::platform::audio::core::codec::clamp_audio_scalar;
use crate::runtime::{ExecutionMode, ExecutionPolicy, start_with_policy};

use super::abi::{AAudioStream, AAudioStreamBuilder};
use super::constants::{
    AAUDIO_DIRECTION_INPUT, AAUDIO_DIRECTION_OUTPUT, AAUDIO_PERFORMANCE_MODE_LOW_LATENCY,
    AAUDIO_STREAM_NAME,
};
use super::core::{
    AAudioLibrary, aaudio_error, aaudio_not_supported, aaudio_sample_format, aaudio_sharing_mode,
    aaudio_succeeded, io_timeout_nanoseconds, require_aaudio_library, runtime_sample_format,
};
use super::ids::{parse_aaudio_stable_id, validate_aaudio_stable_id_direction};
use crate::platform::audio as audio_types;

/// One owned AAudio stream handle.
#[derive(Debug)]
struct AaudioStreamHandle {
    /// Raw `AAudioStream*` pointer.
    raw: *mut AAudioStream,
    /// Shared AAudio symbol table.
    library: Arc<AAudioLibrary>,
}

impl Drop for AaudioStreamHandle {
    fn drop(&mut self) {
        if self.raw.is_null() {
            return;
        }

        // close one opened AAudio stream lane
        unsafe {
            let _ = (self.library.api.stream_close)(self.raw);
        }
    }
}

unsafe impl Send for AaudioStreamHandle {}

/// One stream lane opened through one AAudio builder.
struct OpenedStreamLane {
    /// Opened stream lane handle.
    stream: AaudioStreamHandle,
    /// Negotiated sample rate in hertz.
    sample_rate: u32,
    /// Negotiated channel count.
    channels: u16,
    /// Negotiated period in frames.
    period_frames: u32,
    /// Negotiated runtime sample format.
    format: audio_types::AudioSampleFormat,
}

/// One AAudio stream runtime payload.
#[derive(Debug)]
struct AaudioStreamRuntime {
    /// Shared AAudio symbol table.
    library: Arc<AAudioLibrary>,
    /// Opened playback lane when present.
    playback: Option<Mutex<AaudioStreamHandle>>,
    /// Opened capture lane when present.
    capture: Option<Mutex<AaudioStreamHandle>>,
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

/// One host-operations payload for one AAudio stream host state.
#[derive(Debug)]
struct AaudioHostStreamOps {
    /// Shared AAudio runtime payload.
    runtime: Arc<AaudioStreamRuntime>,
}

impl audio_core::AudioHostStreamOps for AaudioHostStreamOps {
    fn start(&self) -> RuntimeResult<()> {
        with_runtime_lanes(&self.runtime, "destack.audio.stream.start", |stream| {
            let status = unsafe { (self.runtime.library.api.stream_request_start)(stream) };
            if aaudio_succeeded(status) {
                return Ok(());
            }

            Err(aaudio_error(
                "destack.audio.stream.start",
                Some(status),
                "failed to start AAudio stream lane",
            ))
        })
    }

    fn pause(&self, pause: bool) -> RuntimeResult<()> {
        if pause {
            return with_runtime_lanes(&self.runtime, "destack.audio.stream.pause", |stream| {
                let status = unsafe { (self.runtime.library.api.stream_request_pause)(stream) };
                if aaudio_succeeded(status) {
                    return Ok(());
                }

                Err(aaudio_error(
                    "destack.audio.stream.pause",
                    Some(status),
                    "failed to pause AAudio stream lane",
                ))
            });
        }

        with_runtime_lanes(&self.runtime, "destack.audio.stream.pause", |stream| {
            let status = unsafe { (self.runtime.library.api.stream_request_start)(stream) };
            if aaudio_succeeded(status) {
                return Ok(());
            }

            Err(aaudio_error(
                "destack.audio.stream.pause",
                Some(status),
                "failed to resume AAudio stream lane",
            ))
        })
    }

    fn stop(&self) -> RuntimeResult<()> {
        with_runtime_lanes(&self.runtime, "destack.audio.stream.stop", |stream| {
            let status = unsafe { (self.runtime.library.api.stream_request_stop)(stream) };
            if aaudio_succeeded(status) {
                return Ok(());
            }

            Err(aaudio_error(
                "destack.audio.stream.stop",
                Some(status),
                "failed to stop AAudio stream lane",
            ))
        })
    }

    fn flush(&self) -> RuntimeResult<()> {
        let Some(playback) = self.runtime.playback.as_ref() else {
            return Ok(());
        };

        let playback = playback.lock().unwrap_or_else(|error| error.into_inner());

        // flush one opened playback lane when available
        let status = unsafe { (self.runtime.library.api.stream_request_flush)(playback.raw) };
        if aaudio_succeeded(status) {
            return Ok(());
        }

        Err(aaudio_error(
            "destack.audio.stream.flush",
            Some(status),
            "failed to flush AAudio playback lane",
        ))
    }
}

/// Open one AAudio stream host state.
pub(super) fn open_stream(
    device_info: &audio_core::HostDeviceDescriptor,
    config: audio_types::AudioStreamConfig,
    share_mode: audio_types::AudioShareMode,
    requested_flags: audio_types::AudioStreamFlags,
    requested_requirements: audio_types::AudioStreamRequirementFlags,
) -> RuntimeResult<Arc<audio_core::AudioStreamHostState>> {
    // reject unsupported direction requests
    if device_info.direction == audio_types::AudioDeviceDirection::Loopback {
        return Err(aaudio_not_supported(
            "destack.audio.stream.open",
            "AAudio loopback direction is not implemented",
        ));
    }

    // validate one stable-id contract before opening one runtime
    let parsed = parse_aaudio_stable_id(&device_info.id)?;
    validate_aaudio_stable_id_direction(&parsed, device_info.direction)?;
    let _ = (
        parsed.playback_name.as_str(),
        parsed.capture_name.as_deref(),
        AAUDIO_STREAM_NAME,
    );

    let library = require_aaudio_library("destack.audio.stream.open")?;
    let sample_format = aaudio_sample_format(config.format).ok_or_else(|| {
        aaudio_not_supported(
            "destack.audio.stream.open",
            format!(
                "sample format {:?} is not supported by AAudio",
                config.format
            ),
        )
    })?;
    let sharing_mode = aaudio_sharing_mode(share_mode);

    let needs_playback = matches!(
        device_info.direction,
        audio_types::AudioDeviceDirection::Playback | audio_types::AudioDeviceDirection::Duplex
    );
    let needs_capture = matches!(
        device_info.direction,
        audio_types::AudioDeviceDirection::Capture | audio_types::AudioDeviceDirection::Duplex
    );

    // open one playback stream lane when one playback direction is requested
    let playback_lane = if needs_playback {
        Some(open_stream_lane(
            &library,
            AAUDIO_DIRECTION_OUTPUT,
            sample_format,
            sharing_mode,
            config,
            requested_flags,
            "destack.audio.stream.open",
        )?)
    } else {
        None
    };

    // open one capture stream lane when one capture direction is requested
    let capture_lane = if needs_capture {
        Some(open_stream_lane(
            &library,
            AAUDIO_DIRECTION_INPUT,
            sample_format,
            sharing_mode,
            config,
            requested_flags,
            "destack.audio.stream.open",
        )?)
    } else {
        None
    };

    let (sample_rate, channels, format) = if let Some(playback_lane) = playback_lane.as_ref() {
        (
            playback_lane.sample_rate,
            playback_lane.channels,
            playback_lane.format,
        )
    } else if let Some(capture_lane) = capture_lane.as_ref() {
        (
            capture_lane.sample_rate,
            capture_lane.channels,
            capture_lane.format,
        )
    } else {
        return Err(aaudio_not_supported(
            "destack.audio.stream.open",
            "AAudio stream direction is not supported",
        ));
    };

    // validate negotiated duplex lane alignment
    if let Some(capture_lane) = capture_lane.as_ref()
        && (capture_lane.sample_rate != sample_rate
            || capture_lane.channels != channels
            || capture_lane.format != format)
    {
        return Err(aaudio_not_supported(
            "destack.audio.stream.open",
            "AAudio duplex lanes negotiated mismatched formats",
        ));
    }

    let period_frames = playback_lane
        .as_ref()
        .map(|lane| lane.period_frames)
        .unwrap_or(u32::MAX)
        .min(
            capture_lane
                .as_ref()
                .map(|lane| lane.period_frames)
                .unwrap_or(u32::MAX),
        )
        .max(audio_core::MIN_STREAM_PERIOD_FRAMES);

    let frame_bytes = audio_core::frame_bytes(format, channels)?;
    let poll_period = audio_core::resolved_worker_poll_period(period_frames, sample_rate);

    let runtime = Arc::new(AaudioStreamRuntime {
        library,
        playback: playback_lane.map(|lane| Mutex::new(lane.stream)),
        capture: capture_lane.map(|lane| Mutex::new(lane.stream)),
        sample_rate,
        channels,
        period_frames,
        format,
        frame_bytes,
        poll_period,
        stream_state: Mutex::new(Weak::new()),
    });

    let host_ops: Arc<dyn audio_core::AudioHostStreamOps> = Arc::new(AaudioHostStreamOps {
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

/// Open one configured stream lane for one AAudio direction.
fn open_stream_lane(
    library: &Arc<AAudioLibrary>,
    direction: c_int,
    sample_format: c_int,
    sharing_mode: c_int,
    config: audio_types::AudioStreamConfig,
    requested_flags: audio_types::AudioStreamFlags,
    operation: &'static str,
) -> RuntimeResult<OpenedStreamLane> {
    let mut builder = ptr::null_mut::<AAudioStreamBuilder>();

    // create one stream-builder payload for this lane
    let create_status = unsafe { (library.api.create_stream_builder)(&mut builder) };
    if !aaudio_succeeded(create_status) || builder.is_null() {
        return Err(aaudio_error(
            operation,
            Some(create_status),
            "failed to create AAudio stream builder",
        ));
    }

    let builder_guard = AaudioBuilderGuard {
        builder,
        library: library.clone(),
    };

    // use the tighter builder knobs only for explicit low-latency requests
    let request_minimize_latency =
        (requested_flags.0 & audio_core::STREAM_FLAG_MINIMIZE_LATENCY.0) != 0;

    // configure one stream builder with requested lane properties
    unsafe {
        (library.api.stream_builder_set_direction)(builder_guard.builder, direction);
        (library.api.stream_builder_set_sample_rate)(
            builder_guard.builder,
            config.sample_rate as c_int,
        );
        (library.api.stream_builder_set_channel_count)(
            builder_guard.builder,
            config.channels as c_int,
        );
        (library.api.stream_builder_set_format)(builder_guard.builder, sample_format);
        (library.api.stream_builder_set_sharing_mode)(builder_guard.builder, sharing_mode);
        (library.api.stream_builder_set_buffer_capacity_frames)(
            builder_guard.builder,
            config
                .period_frames
                .saturating_mul(if request_minimize_latency { 2 } else { 4 }) as c_int,
        );
    }

    // request low-latency mode only when the caller asks for it
    if request_minimize_latency {
        unsafe {
            (library.api.stream_builder_set_performance_mode)(
                builder_guard.builder,
                AAUDIO_PERFORMANCE_MODE_LOW_LATENCY,
            );
        }
    }

    let mut stream = ptr::null_mut::<AAudioStream>();

    // open one configured stream lane from this builder
    let open_status =
        unsafe { (library.api.stream_builder_open_stream)(builder_guard.builder, &mut stream) };
    if !aaudio_succeeded(open_status) || stream.is_null() {
        return Err(aaudio_error(
            operation,
            Some(open_status),
            "failed to open AAudio stream lane",
        ));
    }

    // validate one exclusive-mode negotiation outcome when requested
    let opened_sharing_mode = unsafe { (library.api.stream_get_sharing_mode)(stream) };
    if sharing_mode != opened_sharing_mode {
        unsafe {
            let _ = (library.api.stream_close)(stream);
        }

        return Err(aaudio_not_supported(
            operation,
            "AAudio could not satisfy requested sharing mode",
        ));
    }

    let sample_rate = unsafe { (library.api.stream_get_sample_rate)(stream) }.max(1) as u32;
    let channels = unsafe { (library.api.stream_get_channel_count)(stream) }.max(1) as u16;
    let period_frames = unsafe { (library.api.stream_get_frames_per_burst)(stream) }
        .max(audio_core::MIN_STREAM_PERIOD_FRAMES as c_int) as u32;
    let opened_format = unsafe { (library.api.stream_get_format)(stream) };
    let format = runtime_sample_format(opened_format).ok_or_else(|| {
        unsafe {
            let _ = (library.api.stream_close)(stream);
        }

        aaudio_not_supported(
            operation,
            format!("AAudio negotiated unsupported stream format selector {opened_format}"),
        )
    })?;

    Ok(OpenedStreamLane {
        stream: AaudioStreamHandle {
            raw: stream,
            library: library.clone(),
        },
        sample_rate,
        channels,
        period_frames,
        format,
    })
}

/// One RAII wrapper for one opened AAudio stream builder.
struct AaudioBuilderGuard {
    /// Opened builder pointer.
    builder: *mut AAudioStreamBuilder,
    /// Shared AAudio symbol table.
    library: Arc<AAudioLibrary>,
}

impl Drop for AaudioBuilderGuard {
    fn drop(&mut self) {
        if self.builder.is_null() {
            return;
        }

        // release one opened builder payload
        unsafe {
            let _ = (self.library.api.stream_builder_delete)(self.builder);
        }
    }
}

/// Apply one closure to each opened runtime lane.
fn with_runtime_lanes(
    runtime: &AaudioStreamRuntime,
    operation: &'static str,
    mut callback: impl FnMut(*mut AAudioStream) -> RuntimeResult<()>,
) -> RuntimeResult<()> {
    if let Some(playback) = runtime.playback.as_ref() {
        let playback = playback.lock().unwrap_or_else(|error| error.into_inner());
        callback(playback.raw)?;
    }

    if let Some(capture) = runtime.capture.as_ref() {
        let capture = capture.lock().unwrap_or_else(|error| error.into_inner());
        callback(capture.raw)?;
    }

    let _ = operation;
    Ok(())
}

/// Spawn one AAudio transfer worker thread.
fn spawn_worker(
    binding: Arc<audio_core::AudioStreamHostState>,
    runtime: Arc<AaudioStreamRuntime>,
) -> std::thread::JoinHandle<()> {
    start_with_policy(
        "destack-audio-aaudio-transfer",
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
                let mut capture_bytes =
                    vec![
                        0u8;
                        scalar_period.saturating_mul(audio_core::sample_bytes(runtime.format))
                    ];

                // prepare one playback packet for this worker cycle
                let playback_bytes = if runtime.playback.is_some() {
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

                    audio_core::encode_audio_bytes(&playback_packet, runtime.format)
                } else {
                    Vec::new()
                };

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

                // write one playback packet to the AAudio output lane
                if let Some(playback) = runtime.playback.as_ref() {
                    let playback = playback.lock().unwrap_or_else(|error| error.into_inner());
                    let requested_frames = (playback_bytes.len() / runtime.frame_bytes) as c_int;
                    let status = unsafe {
                        (runtime.library.api.stream_write)(
                            playback.raw,
                            playback_bytes.as_ptr().cast::<c_void>(),
                            requested_frames,
                            io_timeout_nanoseconds(runtime.poll_period),
                        )
                    };
                    if status < 0 {
                        mark_backend_disconnected(
                            &binding,
                            format!("AAudio playback write failed with status code {status}"),
                        );
                        break;
                    }
                }

                // read one capture packet from the AAudio input lane
                if let Some(capture) = runtime.capture.as_ref() {
                    let capture = capture.lock().unwrap_or_else(|error| error.into_inner());
                    let requested_frames = (capture_bytes.len() / runtime.frame_bytes) as c_int;
                    let status = unsafe {
                        (runtime.library.api.stream_read)(
                            capture.raw,
                            capture_bytes.as_mut_ptr().cast::<c_void>(),
                            requested_frames,
                            io_timeout_nanoseconds(runtime.poll_period),
                        )
                    };
                    if status < 0 {
                        mark_backend_disconnected(
                            &binding,
                            format!("AAudio capture read failed with status code {status}"),
                        );
                        break;
                    }

                    let read_bytes = (status.max(0) as usize).saturating_mul(runtime.frame_bytes);
                    let capture_packet = match audio_core::decode_audio_bytes(
                        &capture_bytes[..read_bytes.min(capture_bytes.len())],
                        runtime.format,
                    ) {
                        Ok(packet) => packet,
                        Err(error) => {
                            mark_backend_disconnected(
                                &binding,
                                format!("AAudio capture decode failed: {error}"),
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
