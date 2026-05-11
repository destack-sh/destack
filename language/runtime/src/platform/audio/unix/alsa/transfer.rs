use std::ffi::{c_int, c_void};
use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core::codec::clamp_audio_scalar;
use crate::platform::audio::{AudioStreamStateKind, AudioStreamStatusFlags, core as audio_core};
use crate::runtime::{ExecutionMode, ExecutionPolicy, start_with_policy};

use super::abi::{AlsaPcm, AlsaSignedFrames, AlsaUnsignedFrames};
use super::core::{AlsaLibrary, AlsaStreamRuntime, recover_pcm, wait_for_pcm_ready};
use super::ffi::alsa_library;

/// Spawn one ALSA transfer worker thread.
pub(super) fn spawn_worker(
    binding: Arc<audio_core::AudioStreamHostState>,
    runtime: Arc<AlsaStreamRuntime>,
) -> std::thread::JoinHandle<()> {
    start_with_policy(
        "destack-audio-alsa-transfer",
        "destack.audio.stream.open",
        ExecutionPolicy::resource(ExecutionMode::Loop),
        move || {
            let Some(library) = alsa_library() else {
                return;
            };

            loop {
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

                if !should_run {
                    audio_core::wait_for_worker_period(&binding, runtime.poll_period);
                    continue;
                }

                let transfer_result = process_transfer_cycle(&library, &binding, &runtime);
                if let Err(error) = transfer_result {
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

                binding.sync.wake.notify_all();
            }
        },
    )
    .unwrap_or_else(|error| panic!("failed to spawn required attached runtime: {error}"))
}

/// Process one ALSA transfer cycle.
fn process_transfer_cycle(
    library: &AlsaLibrary,
    binding: &Arc<audio_core::AudioStreamHostState>,
    runtime: &Arc<AlsaStreamRuntime>,
) -> RuntimeResult<()> {
    if let Some(playback_pcm) = runtime.playback_pcm.as_ref() {
        process_playback_transfer(library, binding, runtime, playback_pcm.raw)?;
    }

    if let Some(capture_pcm) = runtime.capture_pcm.as_ref() {
        process_capture_transfer(library, binding, runtime, capture_pcm.raw)?;
    }

    Ok(())
}

/// Process one ALSA playback transfer cycle.
fn process_playback_transfer(
    library: &AlsaLibrary,
    binding: &Arc<audio_core::AudioStreamHostState>,
    runtime: &Arc<AlsaStreamRuntime>,
    pcm: *mut AlsaPcm,
) -> RuntimeResult<()> {
    wait_for_pcm_ready(pcm);

    // query writable frame budget for this cycle
    let available_frames = unsafe { (library.api.snd_pcm_avail_update)(pcm) };
    if available_frames < 0 {
        recover_pcm(
            library,
            pcm,
            available_frames as c_int,
            "destack.audio.stream.write",
        )?;

        let mut state = binding
            .sync
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        state.xrun_count = state.xrun_count.saturating_add(1);
        state.output_underflow_count = state.output_underflow_count.saturating_add(1);
        state.status_flags = AudioStreamStatusFlags(
            state.status_flags.0 | audio_core::STREAM_STATUS_OUTPUT_UNDERFLOW.0,
        );

        return Ok(());
    }

    let frames_to_write = (available_frames as u32).min(runtime.period_frames) as usize;
    if frames_to_write == 0 {
        return Ok(());
    }

    let scalar_frames = frames_to_write.saturating_mul(runtime.channels as usize);
    let mut samples = Vec::with_capacity(scalar_frames);

    {
        let mut state = binding
            .sync
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        // clear stream status bits before producing this transfer packet
        state.status_flags = AudioStreamStatusFlags(0);

        for _ in 0..scalar_frames {
            if let Some(sample) = state.playback_samples.pop_front() {
                samples.push(sample);
            }
        }

        // fill underflow gaps with silence and update counters
        if samples.len() < scalar_frames {
            let missing = scalar_frames - samples.len();
            samples.resize(scalar_frames, 0.0);

            if missing > 0 {
                state.xrun_count = state.xrun_count.saturating_add(1);
                state.output_underflow_count = state.output_underflow_count.saturating_add(1);
                state.status_flags = AudioStreamStatusFlags(
                    state.status_flags.0 | audio_core::STREAM_STATUS_OUTPUT_UNDERFLOW.0,
                );
            }
        }

        // apply one runtime gain and mute transform in-place
        if state.muted {
            samples.fill(0.0);
        } else if (state.volume - 1.0).abs() > f64::EPSILON {
            let gain = state.volume as f32;
            for sample in &mut samples {
                *sample = clamp_audio_scalar(*sample * gain);
            }
        }
    }

    let encoded = audio_core::encode_audio_bytes(&samples, runtime.format);
    let mut written_frames = 0usize;

    // write as many frames as ALSA currently accepts
    while written_frames < frames_to_write {
        let remaining_frames = frames_to_write - written_frames;
        let byte_offset = written_frames.saturating_mul(runtime.frame_bytes);
        let pointer = unsafe { encoded.as_ptr().add(byte_offset) };

        let status = unsafe {
            (library.api.snd_pcm_writei)(
                pcm,
                pointer as *const c_void,
                remaining_frames as AlsaUnsignedFrames,
            )
        };

        if status == -(libc::EAGAIN as AlsaSignedFrames) {
            break;
        }

        if status < 0 {
            recover_pcm(library, pcm, status as c_int, "destack.audio.stream.write")?;

            let mut state = binding
                .sync
                .state
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            state.xrun_count = state.xrun_count.saturating_add(1);
            state.output_underflow_count = state.output_underflow_count.saturating_add(1);
            state.status_flags = AudioStreamStatusFlags(
                state.status_flags.0 | audio_core::STREAM_STATUS_OUTPUT_UNDERFLOW.0,
            );

            break;
        }

        if status == 0 {
            break;
        }

        written_frames = written_frames.saturating_add(status as usize);
    }

    // return unsent samples to the queue front when ALSA accepted a short write
    if written_frames < frames_to_write {
        let unsent_offset = written_frames
            .saturating_mul(runtime.channels as usize)
            .min(samples.len());
        let unsent = &samples[unsent_offset..];

        let mut state = binding
            .sync
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        for sample in unsent.iter().rev() {
            state.playback_samples.push_front(*sample);
        }
    }

    let now = audio_core::host_monotonic_nanos();
    let mut delay_frames = 0 as AlsaSignedFrames;
    let _ = unsafe { (library.api.snd_pcm_delay)(pcm, &mut delay_frames) };

    let mut state = binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    audio_core::record_stream_callback_timing(
        &mut state,
        runtime.sample_rate,
        written_frames as u32,
        now,
        None,
        None,
    );

    Ok(())
}

/// Process one ALSA capture transfer cycle.
fn process_capture_transfer(
    library: &AlsaLibrary,
    binding: &Arc<audio_core::AudioStreamHostState>,
    runtime: &Arc<AlsaStreamRuntime>,
    pcm: *mut AlsaPcm,
) -> RuntimeResult<()> {
    wait_for_pcm_ready(pcm);

    // query readable frame budget for this cycle
    let available_frames = unsafe { (library.api.snd_pcm_avail_update)(pcm) };
    if available_frames < 0 {
        recover_pcm(
            library,
            pcm,
            available_frames as c_int,
            "destack.audio.stream.read",
        )?;

        let mut state = binding
            .sync
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        state.xrun_count = state.xrun_count.saturating_add(1);
        state.input_overflow_count = state.input_overflow_count.saturating_add(1);
        state.status_flags = AudioStreamStatusFlags(
            state.status_flags.0 | audio_core::STREAM_STATUS_INPUT_OVERFLOW.0,
        );

        return Ok(());
    }

    let frames_to_read = (available_frames as u32).min(runtime.period_frames) as usize;
    if frames_to_read == 0 {
        return Ok(());
    }

    let mut bytes = vec![0u8; frames_to_read.saturating_mul(runtime.frame_bytes)];
    let mut read_frames = 0usize;

    // read as many frames as ALSA currently exposes
    while read_frames < frames_to_read {
        let remaining_frames = frames_to_read - read_frames;
        let byte_offset = read_frames.saturating_mul(runtime.frame_bytes);
        let pointer = unsafe { bytes.as_mut_ptr().add(byte_offset) };

        let status = unsafe {
            (library.api.snd_pcm_readi)(
                pcm,
                pointer as *mut c_void,
                remaining_frames as AlsaUnsignedFrames,
            )
        };

        if status == -(libc::EAGAIN as AlsaSignedFrames) {
            break;
        }

        if status < 0 {
            recover_pcm(library, pcm, status as c_int, "destack.audio.stream.read")?;

            let mut state = binding
                .sync
                .state
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            state.xrun_count = state.xrun_count.saturating_add(1);
            state.input_overflow_count = state.input_overflow_count.saturating_add(1);
            state.status_flags = AudioStreamStatusFlags(
                state.status_flags.0 | audio_core::STREAM_STATUS_INPUT_OVERFLOW.0,
            );

            break;
        }

        if status == 0 {
            break;
        }

        read_frames = read_frames.saturating_add(status as usize);
    }

    if read_frames == 0 {
        return Ok(());
    }

    bytes.truncate(read_frames.saturating_mul(runtime.frame_bytes));
    let samples = audio_core::decode_audio_bytes(&bytes, runtime.format)?;
    let now = audio_core::host_monotonic_nanos();

    let mut delay_frames = 0 as AlsaSignedFrames;
    let _ = unsafe { (library.api.snd_pcm_delay)(pcm, &mut delay_frames) };

    let mut state = binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // append captured samples and enforce queue-capacity limits
    for sample in samples {
        state.capture_samples.push_back(sample);
    }

    let capture_capacity = binding.capture_capacity_samples();
    if state.capture_samples.len() > capture_capacity {
        let overflow = state.capture_samples.len() - capture_capacity;
        for _ in 0..overflow {
            let _ = state.capture_samples.pop_front();
        }

        state.xrun_count = state.xrun_count.saturating_add(1);
        state.input_overflow_count = state.input_overflow_count.saturating_add(1);
        state.status_flags = AudioStreamStatusFlags(
            state.status_flags.0 | audio_core::STREAM_STATUS_INPUT_OVERFLOW.0,
        );
    }

    audio_core::record_stream_callback_timing(
        &mut state,
        runtime.sample_rate,
        read_frames as u32,
        now,
        None,
        None,
    );

    Ok(())
}
