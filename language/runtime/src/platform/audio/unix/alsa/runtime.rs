use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::audio::core as audio_core;
use std::sync::{Arc, Condvar, Mutex, Weak};

use super::abi::AlsaPcm;
use super::constants::{ALSA_FALSE, ALSA_STREAM_CAPTURE, ALSA_STREAM_PLAYBACK, ALSA_TRUE};
use super::core::{
    AlsaDirectionLane, AlsaHostStreamOps, AlsaStreamRuntime, alsa_error, alsa_succeeded,
};
use super::host::{open_configured_pcm, recover_pcm, require_alsa_library};
use super::ids::{parse_stable_id, validate_stable_id_direction};
use super::transfer::spawn_worker;
use crate::platform::audio as audio_types;

/// Open one ALSA stream host state.
pub(super) fn open_stream(
    device_info: &audio_core::HostDeviceDescriptor,
    config: audio_types::AudioStreamConfig,
    share_mode: audio_types::AudioShareMode,
    backend_flags: audio_core::AudioBackendOpenFlags,
    requested_flags: audio_types::AudioStreamFlags,
    requested_requirements: audio_types::AudioStreamRequirementFlags,
) -> RuntimeResult<Arc<audio_core::AudioStreamHostState>> {
    // open one initialized ALSA runtime payload from one stable id
    let runtime = open_runtime(
        &device_info.id,
        device_info.direction,
        config,
        share_mode,
        backend_flags,
        requested_flags,
    )?;

    // install one ALSA host-ops payload for stream control transitions
    let host_ops: Arc<dyn audio_core::AudioHostStreamOps> = Arc::new(AlsaHostStreamOps {
        runtime: runtime.clone(),
    });

    // build one stream host state and spawn one transfer worker
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
            supports_pause: runtime.supports_pause,
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

    // publish one weak host-state handle for worker-side queue access
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

/// Open one ALSA runtime payload from one parsed stable id.
fn open_runtime(
    stable_id: &str,
    direction: audio_types::AudioDeviceDirection,
    config: audio_types::AudioStreamConfig,
    share_mode: audio_types::AudioShareMode,
    backend_flags: audio_core::AudioBackendOpenFlags,
    requested_flags: audio_types::AudioStreamFlags,
) -> RuntimeResult<Arc<AlsaStreamRuntime>> {
    // reject ALSA loopback direction since plain ALSA has no generic host loopback lane
    if direction == audio_types::AudioDeviceDirection::Loopback {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.audio.stream.open ALSA loopback",
        ))
        .boxed());
    }

    let parsed = parse_stable_id(stable_id)?;
    validate_stable_id_direction(&parsed, direction)?;

    let mut playback_lane = None;
    let mut capture_lane = None;

    // open one playback lane when requested by stable-id direction
    if parsed.lane == AlsaDirectionLane::Playback || parsed.lane == AlsaDirectionLane::Duplex {
        playback_lane = Some(open_configured_pcm(
            "destack.audio.stream.open",
            &parsed.playback_name,
            ALSA_STREAM_PLAYBACK,
            config,
            backend_flags,
            requested_flags,
        )?);
    }

    // open one capture lane when requested by stable-id direction
    if parsed.lane == AlsaDirectionLane::Capture || parsed.lane == AlsaDirectionLane::Duplex {
        let capture_name = parsed
            .capture_name
            .as_deref()
            .unwrap_or(parsed.playback_name.as_str());

        capture_lane = Some(open_configured_pcm(
            "destack.audio.stream.open",
            capture_name,
            ALSA_STREAM_CAPTURE,
            config,
            backend_flags,
            requested_flags,
        )?);
    }

    let Some(first_lane) = playback_lane.as_ref().or(capture_lane.as_ref()) else {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.audio.stream.open ALSA lane selection",
        ))
        .boxed());
    };

    // reject exclusive mode when one opened lane does not expose hardware endpoint semantics
    if share_mode == audio_types::AudioShareMode::Exclusive {
        let playback_exclusive = playback_lane
            .as_ref()
            .map(|lane| lane.supports_exclusive)
            .unwrap_or(true);
        let capture_exclusive = capture_lane
            .as_ref()
            .map(|lane| lane.supports_exclusive)
            .unwrap_or(true);

        if !playback_exclusive || !capture_exclusive {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.audio.stream.open ALSA exclusive mode",
            ))
            .boxed());
        }
    }

    let sample_rate = first_lane.sample_rate;
    let channels = first_lane.channels;

    // validate negotiated channel and rate alignment for duplex opens
    if let Some(capture_lane) = capture_lane.as_ref()
        && (capture_lane.sample_rate != sample_rate || capture_lane.channels != channels)
    {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.audio.stream.open ALSA duplex negotiation mismatch",
        ))
        .boxed());
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

    let frame_bytes = audio_core::frame_bytes(config.format, channels)?;
    let poll_period = audio_core::resolved_worker_poll_period(period_frames, sample_rate);

    let supports_pause = playback_lane
        .as_ref()
        .map(|lane| lane.supports_pause)
        .unwrap_or(true)
        && capture_lane
            .as_ref()
            .map(|lane| lane.supports_pause)
            .unwrap_or(true);

    Ok(Arc::new(AlsaStreamRuntime {
        sample_rate,
        channels,
        period_frames,
        frame_bytes,
        format: config.format,
        poll_period,
        supports_pause,
        playback_pcm: playback_lane.map(|lane| lane.pcm),
        capture_pcm: capture_lane.map(|lane| lane.pcm),
        stream_state: Mutex::new(Weak::new()),
    }))
}

/// Apply one closure to each opened runtime lane.
fn with_runtime_lanes(
    runtime: &AlsaStreamRuntime,
    mut callback: impl FnMut(*mut AlsaPcm) -> RuntimeResult<()>,
) -> RuntimeResult<()> {
    if let Some(playback_pcm) = runtime.playback_pcm.as_ref() {
        callback(playback_pcm.raw)?;
    }

    if let Some(capture_pcm) = runtime.capture_pcm.as_ref() {
        callback(capture_pcm.raw)?;
    }

    Ok(())
}

impl audio_core::AudioHostStreamOps for AlsaHostStreamOps {
    fn start(&self) -> RuntimeResult<()> {
        let library = require_alsa_library("destack.audio.stream.start")?;

        // prepare each lane before issuing one start transition
        with_runtime_lanes(&self.runtime, |pcm| {
            let status = unsafe { (library.api.snd_pcm_prepare)(pcm) };
            if alsa_succeeded(status) {
                return Ok(());
            }

            Err(alsa_error(
                "destack.audio.stream.start",
                status,
                "failed to prepare ALSA stream lane",
            ))
        })?;

        // start capture before playback to reduce startup skew in duplex mode
        if let Some(capture_pcm) = self.runtime.capture_pcm.as_ref() {
            let status = unsafe { (library.api.snd_pcm_start)(capture_pcm.raw) };
            if !alsa_succeeded(status) {
                recover_pcm(
                    &library,
                    capture_pcm.raw,
                    status,
                    "destack.audio.stream.start",
                )?;

                let status = unsafe { (library.api.snd_pcm_start)(capture_pcm.raw) };
                if !alsa_succeeded(status) {
                    return Err(alsa_error(
                        "destack.audio.stream.start",
                        status,
                        "failed to start ALSA capture lane",
                    ));
                }
            }
        }

        if let Some(playback_pcm) = self.runtime.playback_pcm.as_ref() {
            let status = unsafe { (library.api.snd_pcm_start)(playback_pcm.raw) };
            if !alsa_succeeded(status) {
                recover_pcm(
                    &library,
                    playback_pcm.raw,
                    status,
                    "destack.audio.stream.start",
                )?;

                let status = unsafe { (library.api.snd_pcm_start)(playback_pcm.raw) };
                if !alsa_succeeded(status) {
                    return Err(alsa_error(
                        "destack.audio.stream.start",
                        status,
                        "failed to start ALSA playback lane",
                    ));
                }
            }
        }

        Ok(())
    }

    fn pause(&self, pause: bool) -> RuntimeResult<()> {
        let library = require_alsa_library("destack.audio.stream.pause")?;

        if self.runtime.supports_pause {
            return with_runtime_lanes(&self.runtime, |pcm| {
                let status = unsafe {
                    (library.api.snd_pcm_pause)(pcm, if pause { ALSA_TRUE } else { ALSA_FALSE })
                };
                if alsa_succeeded(status) {
                    return Ok(());
                }

                Err(alsa_error(
                    "destack.audio.stream.pause",
                    status,
                    "failed to toggle ALSA pause state",
                ))
            });
        }

        // emulate pause with drop and restart on devices without native pause support
        if pause {
            return with_runtime_lanes(&self.runtime, |pcm| {
                let status = unsafe { (library.api.snd_pcm_drop)(pcm) };
                if alsa_succeeded(status) {
                    return Ok(());
                }

                Err(alsa_error(
                    "destack.audio.stream.pause",
                    status,
                    "failed to drop ALSA stream lane for pause emulation",
                ))
            });
        }

        with_runtime_lanes(&self.runtime, |pcm| {
            let status = unsafe { (library.api.snd_pcm_prepare)(pcm) };
            if !alsa_succeeded(status) {
                return Err(alsa_error(
                    "destack.audio.stream.pause",
                    status,
                    "failed to prepare ALSA stream lane during resume",
                ));
            }

            let status = unsafe { (library.api.snd_pcm_start)(pcm) };
            if alsa_succeeded(status) {
                return Ok(());
            }

            Err(alsa_error(
                "destack.audio.stream.pause",
                status,
                "failed to restart ALSA stream lane during resume",
            ))
        })
    }

    fn stop(&self) -> RuntimeResult<()> {
        let library = require_alsa_library("destack.audio.stream.stop")?;

        with_runtime_lanes(&self.runtime, |pcm| {
            let status = unsafe { (library.api.snd_pcm_drop)(pcm) };
            if alsa_succeeded(status) {
                return Ok(());
            }

            Err(alsa_error(
                "destack.audio.stream.stop",
                status,
                "failed to stop ALSA stream lane",
            ))
        })
    }

    fn flush(&self) -> RuntimeResult<()> {
        let library = require_alsa_library("destack.audio.stream.flush")?;

        with_runtime_lanes(&self.runtime, |pcm| {
            let drop_status = unsafe { (library.api.snd_pcm_drop)(pcm) };
            if !alsa_succeeded(drop_status) {
                return Err(alsa_error(
                    "destack.audio.stream.flush",
                    drop_status,
                    "failed to drop ALSA stream lane before flush",
                ));
            }

            let reset_status = unsafe { (library.api.snd_pcm_reset)(pcm) };
            if !alsa_succeeded(reset_status) {
                return Err(alsa_error(
                    "destack.audio.stream.flush",
                    reset_status,
                    "failed to reset ALSA stream lane",
                ));
            }

            let prepare_status = unsafe { (library.api.snd_pcm_prepare)(pcm) };
            if alsa_succeeded(prepare_status) {
                return Ok(());
            }

            Err(alsa_error(
                "destack.audio.stream.flush",
                prepare_status,
                "failed to prepare ALSA stream lane after flush",
            ))
        })
    }
}
