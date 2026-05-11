use std::sync::Arc;
use std::time::Duration;
use std::{ptr, thread};

use super::abi::{
    audio_capture_client_get_buffer, audio_capture_client_get_next_packet_size,
    audio_capture_client_release_buffer, audio_client_get_current_padding,
    audio_render_client_get_buffer, audio_render_client_release_buffer,
};
use super::constants::{MAX_EVENT_WAIT_HANDLES, MIN_WAIT_TIMEOUT_MILLISECONDS};
use super::core::{
    WasapiCaptureClient, WasapiRenderClient, WasapiStreamRuntime, failed, hresult_error,
    initialize_com, qpc_hundred_nanos_to_mono_ns, qpc_now_mono_ns,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::audio::{AudioStreamStateKind, core as audio_core};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::runtime::{ExecutionMode, ExecutionPolicy, start_with_policy};

use crate::platform::audio as audio_types;
use windows_sys::Win32::Foundation::{
    GetLastError, HANDLE, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::Media::Audio::{
    AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY, AUDCLNT_BUFFERFLAGS_SILENT,
    AUDCLNT_BUFFERFLAGS_TIMESTAMP_ERROR, IAudioCaptureClient, IAudioClient, IAudioRenderClient,
};
use windows_sys::Win32::System::Threading::{WaitForMultipleObjects, WaitForSingleObject};

/// Spawn one worker loop that transfers stream data between runtime queues and WASAPI clients.
pub(super) fn spawn_worker(
    binding: Arc<audio_core::AudioStreamHostState>,
    runtime: Arc<WasapiStreamRuntime>,
) -> thread::JoinHandle<()> {
    start_with_policy(
        "destack-audio-wasapi-transfer",
        "destack.audio.stream.open",
        ExecutionPolicy::resource(ExecutionMode::Loop),
        move || {
            // initialize one COM apartment for this worker thread
            if initialize_com().is_err() {
                return;
            }

            loop {
                // stop when runtime stream state has been closed
                let state = binding
                    .sync
                    .state
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                let shutdown = state.shutdown;
                let should_run = state.running && !state.paused;
                drop(state);

                if shutdown {
                    break;
                }

                // process one host transfer cycle while the stream runs
                if should_run {
                    if let Err(error) = wait_for_stream_signal(&binding, &runtime) {
                        let backend_message = error.to_string();
                        let mut state = binding
                            .sync
                            .state
                            .lock()
                            .unwrap_or_else(|error| error.into_inner());
                        state.running = false;
                        state.paused = false;
                        state.state_kind = AudioStreamStateKind::DeviceLost;
                        state.last_backend_message = Some(backend_message);
                        drop(state);

                        binding.sync.wake.notify_all();
                        continue;
                    }

                    let transfer_error = match runtime.direction {
                        audio_types::AudioDeviceDirection::Playback => runtime
                            .render_client
                            .as_ref()
                            .map(|render_client| {
                                process_playback_transfer(&binding, &runtime, render_client)
                            })
                            .transpose(),
                        audio_types::AudioDeviceDirection::Capture
                        | audio_types::AudioDeviceDirection::Loopback => runtime
                            .capture_client
                            .as_ref()
                            .map(|capture_client| {
                                process_capture_transfer(&binding, &runtime, capture_client)
                            })
                            .transpose(),
                        audio_types::AudioDeviceDirection::Duplex => {
                            let playback_result = runtime
                                .render_client
                                .as_ref()
                                .map(|render_client| {
                                    process_playback_transfer(&binding, &runtime, render_client)
                                })
                                .transpose();
                            let capture_result = runtime
                                .capture_client
                                .as_ref()
                                .map(|capture_client| {
                                    process_capture_transfer(&binding, &runtime, capture_client)
                                })
                                .transpose();

                            if let Err(error) = playback_result {
                                Err(error)
                            } else if let Err(error) = capture_result {
                                Err(error)
                            } else {
                                Ok(None)
                            }
                        }
                    };

                    if let Err(error) = transfer_error {
                        let backend_message = error.to_string();
                        let mut state = binding
                            .sync
                            .state
                            .lock()
                            .unwrap_or_else(|error| error.into_inner());
                        state.running = false;
                        state.paused = false;
                        state.state_kind = AudioStreamStateKind::DeviceLost;
                        state.last_backend_message = Some(backend_message);
                    }
                }
                // idle pacing: avoid hot spinning while paused or stopped
                else {
                    audio_core::wait_for_worker_period(&binding, runtime.poll_period);
                }

                // wake waiters after each worker cycle
                binding.sync.wake.notify_all();
            }
        },
    )
    .unwrap_or_else(|error| panic!("failed to spawn required attached runtime: {error}"))
}

/// Wait for one WASAPI event callback signal or poll timeout.
fn wait_for_stream_signal(
    binding: &Arc<audio_core::AudioStreamHostState>,
    runtime: &Arc<WasapiStreamRuntime>,
) -> RuntimeResult<()> {
    let mut handles = [0 as HANDLE; MAX_EVENT_WAIT_HANDLES];
    let mut handle_count = 0usize;

    // include playback event when the runtime exposes one
    if let Some(event) = runtime.playback_event.as_ref() {
        handles[handle_count] = event.raw;
        handle_count = handle_count.saturating_add(1);
    }

    // include capture event when the runtime exposes one
    if let Some(event) = runtime.capture_event.as_ref()
        && handle_count < MAX_EVENT_WAIT_HANDLES
    {
        handles[handle_count] = event.raw;
        handle_count = handle_count.saturating_add(1);
    }

    // fall back to periodic pacing when no event handles are attached
    if handle_count == 0 {
        audio_core::wait_for_worker_period(binding, runtime.poll_period);
        return Ok(());
    }

    let timeout_milliseconds = wait_timeout_milliseconds(runtime.poll_period);
    let wait_status = if handle_count == 1 {
        unsafe { WaitForSingleObject(handles[0], timeout_milliseconds) }
    } else {
        unsafe {
            WaitForMultipleObjects(
                handle_count as u32,
                handles.as_ptr(),
                0,
                timeout_milliseconds,
            )
        }
    };

    if wait_status == WAIT_TIMEOUT {
        return Ok(());
    }

    if wait_status == WAIT_FAILED {
        let win32_error = unsafe { GetLastError() };
        return Err(RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoInvalidData),
            None,
            None,
            Some("destack.audio.stream.transfer.wait".to_string()),
            None,
            format!("WASAPI event wait failed (win32 error {win32_error})"),
        ))
        .boxed());
    }

    if wait_status < WAIT_OBJECT_0.saturating_add(handle_count as u32) {
        return Ok(());
    }

    Err(RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInvalidData),
        None,
        None,
        Some("destack.audio.stream.transfer.wait".to_string()),
        None,
        format!("WASAPI event wait returned unexpected status {wait_status}"),
    ))
    .boxed())
}

/// Convert one poll duration into one wait timeout in milliseconds.
fn wait_timeout_milliseconds(duration: Duration) -> u32 {
    let duration_milliseconds = duration.as_millis();
    duration_milliseconds
        .min(u32::MAX as u128)
        .max(MIN_WAIT_TIMEOUT_MILLISECONDS as u128) as u32
}

/// Process one WASAPI playback transfer cycle.
fn process_playback_transfer(
    binding: &Arc<audio_core::AudioStreamHostState>,
    runtime: &Arc<WasapiStreamRuntime>,
    render_client: &Arc<WasapiRenderClient>,
) -> RuntimeResult<()> {
    let Some(playback_client) = runtime.playback_client.as_ref() else {
        return Err(RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoInvalidData),
            None,
            None,
            Some("destack.audio.stream.write".to_string()),
            None,
            "missing WASAPI playback client for playback transfer".to_string(),
        ))
        .boxed());
    };

    // read one current padding snapshot to compute writable frames
    let mut padding_frames = 0u32;
    let padding_status = unsafe {
        audio_client_get_current_padding(
            playback_client.raw as IAudioClient,
            &mut padding_frames as *mut u32,
        )
    };
    if failed(padding_status) {
        return Err(hresult_error(
            "destack.audio.stream.write",
            padding_status,
            "failed to query WASAPI padding",
        ));
    }

    let writable_frames = runtime
        .playback_buffer_frames
        .saturating_sub(padding_frames);
    if writable_frames == 0 {
        return Ok(());
    }

    // borrow one writable host buffer and fill it from queued playback samples
    let mut output_pointer = ptr::null_mut();
    let get_status = unsafe {
        audio_render_client_get_buffer(
            render_client.raw as IAudioRenderClient,
            writable_frames,
            &mut output_pointer,
        )
    };
    if failed(get_status) {
        return Err(hresult_error(
            "destack.audio.stream.write",
            get_status,
            "failed to lock WASAPI render buffer",
        ));
    }

    let byte_count = (writable_frames as usize).saturating_mul(runtime.frame_bytes);
    let output = unsafe { std::slice::from_raw_parts_mut(output_pointer, byte_count) };
    write_playback_bytes(binding, runtime, output, writable_frames as usize);

    // commit one filled host buffer back to the endpoint engine
    let release_status = unsafe {
        audio_render_client_release_buffer(
            render_client.raw as IAudioRenderClient,
            writable_frames,
            0,
        )
    };
    if failed(release_status) {
        return Err(hresult_error(
            "destack.audio.stream.write",
            release_status,
            "failed to release WASAPI render buffer",
        ));
    }

    Ok(())
}

/// Process one WASAPI capture transfer cycle.
fn process_capture_transfer(
    binding: &Arc<audio_core::AudioStreamHostState>,
    runtime: &Arc<WasapiStreamRuntime>,
    capture_client: &Arc<WasapiCaptureClient>,
) -> RuntimeResult<()> {
    loop {
        // query one pending capture packet size from the endpoint queue
        let mut packet_frames = 0u32;
        let packet_status = unsafe {
            audio_capture_client_get_next_packet_size(
                capture_client.raw as IAudioCaptureClient,
                &mut packet_frames,
            )
        };
        if failed(packet_status) {
            return Err(hresult_error(
                "destack.audio.stream.read",
                packet_status,
                "failed to query WASAPI capture packet size",
            ));
        }

        if packet_frames == 0 {
            break;
        }

        // lock one capture packet and ingest its payload
        let mut input_pointer = ptr::null_mut();
        let mut read_frames = 0u32;
        let mut packet_flags = 0u32;
        let mut device_position_frames = 0u64;
        let mut qpc_hundred_nanos = 0u64;
        let get_status = unsafe {
            audio_capture_client_get_buffer(
                capture_client.raw as IAudioCaptureClient,
                &mut input_pointer,
                &mut read_frames,
                &mut packet_flags,
                &mut device_position_frames,
                &mut qpc_hundred_nanos,
            )
        };
        if failed(get_status) {
            return Err(hresult_error(
                "destack.audio.stream.read",
                get_status,
                "failed to lock WASAPI capture packet",
            ));
        }

        let has_data_discontinuity =
            (packet_flags & AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY as u32) != 0;
        let has_timestamp_error = (packet_flags & AUDCLNT_BUFFERFLAGS_TIMESTAMP_ERROR as u32) != 0;
        let capture_timestamp_ns = if has_timestamp_error {
            None
        } else {
            qpc_hundred_nanos_to_mono_ns(qpc_hundred_nanos)
        };

        let device_position_frames = if device_position_frames == 0 {
            None
        } else {
            Some(device_position_frames)
        };

        if (packet_flags & AUDCLNT_BUFFERFLAGS_SILENT as u32) != 0 || input_pointer.is_null() {
            push_capture_silent_frames(
                binding,
                runtime,
                read_frames as usize,
                capture_timestamp_ns,
                has_data_discontinuity,
                device_position_frames,
            );
        } else {
            let byte_count = (read_frames as usize).saturating_mul(runtime.frame_bytes);
            let input =
                unsafe { std::slice::from_raw_parts(input_pointer as *const u8, byte_count) };
            push_capture_bytes(
                binding,
                runtime,
                input,
                read_frames as usize,
                capture_timestamp_ns,
                has_data_discontinuity,
                device_position_frames,
            );
        }

        // release one consumed capture packet back to the endpoint queue
        let release_status = unsafe {
            audio_capture_client_release_buffer(
                capture_client.raw as IAudioCaptureClient,
                read_frames,
            )
        };
        if failed(release_status) {
            return Err(hresult_error(
                "destack.audio.stream.read",
                release_status,
                "failed to release WASAPI capture packet",
            ));
        }
    }

    Ok(())
}

/// Fill one WASAPI render packet from queued playback samples.
fn write_playback_bytes(
    binding: &Arc<audio_core::AudioStreamHostState>,
    runtime: &Arc<WasapiStreamRuntime>,
    output: &mut [u8],
    frame_count: usize,
) {
    let scalar_width = audio_core::sample_bytes(runtime.format);
    if scalar_width == 0 || output.len() < scalar_width {
        return;
    }

    // consume queued playback samples and apply stream-level gain and mute
    let scalar_count = frame_count.saturating_mul(runtime.channels as usize);
    let mut state = binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let is_active = state.running && !state.paused && !state.shutdown;
    let volume = if state.muted {
        0.0
    } else {
        state.volume as f32
    };

    let mut written = 0usize;
    for chunk in output.chunks_exact_mut(scalar_width).take(scalar_count) {
        let sample = if is_active {
            match state.playback_samples.pop_front() {
                Some(sample) => sample * volume,
                None => {
                    state.xrun_count = state.xrun_count.saturating_add(1);
                    state.output_underflow_count = state.output_underflow_count.saturating_add(1);
                    state.status_flags = audio_types::AudioStreamStatusFlags(
                        state.status_flags.0 | audio_core::STREAM_STATUS_OUTPUT_UNDERFLOW.0,
                    );
                    0.0
                }
            }
        } else {
            0.0
        };

        written += audio_core::encode_scalar_sample(runtime.format, sample, chunk);
    }

    if written < output.len() {
        output[written..].fill(0);
    }

    // advance stream timing counters after one successful transfer
    if is_active {
        let callback_timestamp_ns = qpc_now_mono_ns();
        audio_core::record_stream_callback_timing(
            &mut state,
            runtime.sample_rate,
            frame_count as u32,
            callback_timestamp_ns,
            None,
            Some(callback_timestamp_ns),
        );
    }
}

/// Push one silent capture packet into the runtime queue.
fn push_capture_silent_frames(
    binding: &Arc<audio_core::AudioStreamHostState>,
    runtime: &Arc<WasapiStreamRuntime>,
    frame_count: usize,
    capture_timestamp_ns: Option<u64>,
    has_data_discontinuity: bool,
    device_position_frames: Option<u64>,
) {
    let sample_count = frame_count.saturating_mul(runtime.channels as usize);

    let mut state = binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let is_active = state.running && !state.paused && !state.shutdown;

    if !is_active {
        return;
    }

    for _ in 0..sample_count {
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
        state.status_flags = audio_types::AudioStreamStatusFlags(
            state.status_flags.0 | audio_core::STREAM_STATUS_INPUT_OVERFLOW.0,
        );
    }

    if has_data_discontinuity {
        state.xrun_count = state.xrun_count.saturating_add(1);
        state.input_overflow_count = state.input_overflow_count.saturating_add(1);
        state.status_flags = audio_types::AudioStreamStatusFlags(
            state.status_flags.0 | audio_core::STREAM_STATUS_INPUT_OVERFLOW.0,
        );
    }

    let callback_timestamp_ns =
        capture_timestamp_ns.unwrap_or_else(audio_core::host_monotonic_nanos);
    let frames_advanced = if let Some(device_position_frames) = device_position_frames {
        state.stream_frames = state.stream_frames.max(device_position_frames);
        0
    } else {
        frame_count as u32
    };

    audio_core::record_stream_callback_timing(
        &mut state,
        runtime.sample_rate,
        frames_advanced,
        callback_timestamp_ns,
        Some(callback_timestamp_ns),
        None,
    );
}

/// Decode one capture packet and push it into the runtime queue.
fn push_capture_bytes(
    binding: &Arc<audio_core::AudioStreamHostState>,
    runtime: &Arc<WasapiStreamRuntime>,
    input: &[u8],
    frame_count: usize,
    capture_timestamp_ns: Option<u64>,
    has_data_discontinuity: bool,
    device_position_frames: Option<u64>,
) {
    let scalar_width = audio_core::sample_bytes(runtime.format);
    if scalar_width == 0 || input.len() < scalar_width {
        return;
    }

    // decode scalar lanes and append them to the capture queue
    let sample_count = frame_count.saturating_mul(runtime.channels as usize);
    let mut state = binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let is_active = state.running && !state.paused && !state.shutdown;

    if !is_active {
        return;
    }

    for sample_bytes in input.chunks_exact(scalar_width).take(sample_count) {
        if let Some(sample) = audio_core::decode_scalar_sample(runtime.format, sample_bytes) {
            state.capture_samples.push_back(sample);
        }
    }

    let max_capture = binding.capture_capacity_samples();
    if state.capture_samples.len() > max_capture {
        let extra = state.capture_samples.len() - max_capture;
        for _ in 0..extra {
            let _ = state.capture_samples.pop_front();
        }
        state.xrun_count = state.xrun_count.saturating_add(1);
        state.input_overflow_count = state.input_overflow_count.saturating_add(1);
        state.status_flags = audio_types::AudioStreamStatusFlags(
            state.status_flags.0 | audio_core::STREAM_STATUS_INPUT_OVERFLOW.0,
        );
    }

    if has_data_discontinuity {
        state.xrun_count = state.xrun_count.saturating_add(1);
        state.input_overflow_count = state.input_overflow_count.saturating_add(1);
        state.status_flags = audio_types::AudioStreamStatusFlags(
            state.status_flags.0 | audio_core::STREAM_STATUS_INPUT_OVERFLOW.0,
        );
    }

    let callback_timestamp_ns =
        capture_timestamp_ns.unwrap_or_else(audio_core::host_monotonic_nanos);
    let frames_advanced = if let Some(device_position_frames) = device_position_frames {
        state.stream_frames = state.stream_frames.max(device_position_frames);
        0
    } else {
        frame_count as u32
    };

    audio_core::record_stream_callback_timing(
        &mut state,
        runtime.sample_rate,
        frames_advanced,
        callback_timestamp_ns,
        Some(callback_timestamp_ns),
        None,
    );
}
