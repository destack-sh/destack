use std::sync::{Condvar, Mutex};

use super::abi::{
    audio_client_get_buffer_size, audio_client_initialize, audio_client_set_event_handle,
};
use super::core::{
    EndpointFlow, OpenedWasapiEndpoint, SelectedEndpoint, WasapiAudioClient, WasapiCaptureClient,
    WasapiHostStreamOps, WasapiRenderClient, WasapiStreamRuntime,
};
use super::host::{
    activate_audio_client, build_wave_format, create_device_enumerator, create_stream_event_handle,
    failed, get_capture_client, get_endpoint_by_id, get_render_client, hresult_error,
    initialize_com,
};
use super::ids::{parse_duplex_stable_id, parse_endpoint_stable_id};
use super::transfer::spawn_worker;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::core as audio_core;
use crate::platform::audio::core::constants::MIN_STREAM_PERIOD_FRAMES;
use crate::platform::{PlatformError, audio as audio_types};
use std::sync::Arc;
use windows_sys::Win32::Media::Audio::{
    AUDCLNT_SHAREMODE_EXCLUSIVE, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
    AUDCLNT_STREAMFLAGS_LOOPBACK, IAudioClient, IMMDevice, IMMDeviceEnumerator,
};

/// Open a WASAPI audio stream.
pub(super) fn open_stream(
    device_info: &audio_core::HostDeviceDescriptor,
    config: audio_types::AudioStreamConfig,
    share_mode: audio_types::AudioShareMode,
    requested_flags: audio_types::AudioStreamFlags,
    requested_requirements: audio_types::AudioStreamRequirementFlags,
) -> RuntimeResult<Arc<audio_core::AudioStreamHostState>> {
    // open one initialized runtime payload from one host endpoint
    let runtime = open_runtime(&device_info.id, config, device_info.direction, share_mode)?;

    // install one host-ops controller over the opened audio client
    let mut host_clients = Vec::with_capacity(2);
    if let Some(playback_client) = runtime.playback_client.as_ref() {
        host_clients.push(playback_client.clone());
    }
    if let Some(capture_client) = runtime.capture_client_owner.as_ref() {
        host_clients.push(capture_client.clone());
    }
    let host_ops: Arc<dyn audio_core::AudioHostStreamOps> = Arc::new(WasapiHostStreamOps {
        clients: host_clients,
    });

    // build one stream host state with one spawned worker thread
    let supports_write_at = matches!(
        device_info.direction,
        audio_types::AudioDeviceDirection::Playback | audio_types::AudioDeviceDirection::Duplex
    );
    let stream_state = Arc::new(audio_core::AudioStreamHostState {
        device: device_info.clone(),
        direction: device_info.direction,
        requested: config,
        requested_flags,
        requested_requirements,
        sample_rate: config.sample_rate,
        channels: config.channels,
        period_frames: runtime.period_frames,
        max_queued_frames: audio_core::resolved_max_queued_frames(),
        share_mode,
        runtime_capabilities: audio_core::AudioStreamRuntimeCapabilities {
            supports_write_at,
            supports_pause: true,
            supports_non_interleaved: false,
            supports_volume: true,
            supports_mute: true,
            supports_hardware_timestamps: true,
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

    let worker = spawn_worker(stream_state.clone(), runtime);
    *stream_state
        .worker_thread
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(worker);

    Ok(stream_state)
}

/// Open one initialized WASAPI runtime payload.
fn open_runtime(
    stable_id: &str,
    config: audio_types::AudioStreamConfig,
    direction: audio_types::AudioDeviceDirection,
    share_mode: audio_types::AudioShareMode,
) -> RuntimeResult<Arc<WasapiStreamRuntime>> {
    let _com = initialize_com()?;

    // require shared mode for loopback streams
    if direction == audio_types::AudioDeviceDirection::Loopback
        && share_mode == audio_types::AudioShareMode::Exclusive
    {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.audio.stream.open WASAPI loopback exclusive mode",
        ))
        .boxed());
    }

    // open one endpoint payload for one non-duplex lane
    let opened = if direction != audio_types::AudioDeviceDirection::Duplex {
        let selected = parse_endpoint_stable_id(stable_id)?;
        validate_selected_endpoint(&selected, direction)?;
        let opened = open_endpoint_runtime(&selected.endpoint_id, direction, config, share_mode)?;

        let period_frames = config
            .period_frames
            .max(MIN_STREAM_PERIOD_FRAMES)
            .min(opened.buffer_frames.max(MIN_STREAM_PERIOD_FRAMES));
        let poll_period =
            audio_core::resolved_worker_poll_period(period_frames, config.sample_rate);

        Arc::new(WasapiStreamRuntime {
            direction,
            format: config.format,
            sample_rate: config.sample_rate,
            channels: config.channels,
            period_frames,
            frame_bytes: audio_core::frame_bytes(config.format, config.channels)?,
            poll_period,
            playback_buffer_frames: opened.buffer_frames.max(1),
            playback_client: if direction == audio_types::AudioDeviceDirection::Playback {
                Some(opened.client.clone())
            } else {
                None
            },
            capture_client_owner: if direction == audio_types::AudioDeviceDirection::Capture
                || direction == audio_types::AudioDeviceDirection::Loopback
            {
                Some(opened.client.clone())
            } else {
                None
            },
            render_client: opened.render_client,
            capture_client: opened.capture_client,
            playback_event: if direction == audio_types::AudioDeviceDirection::Playback {
                Some(opened.event_handle.clone())
            } else {
                None
            },
            capture_event: if direction == audio_types::AudioDeviceDirection::Capture
                || direction == audio_types::AudioDeviceDirection::Loopback
            {
                Some(opened.event_handle.clone())
            } else {
                None
            },
        })
    } else {
        // parse one explicit duplex pair and open both endpoint lanes
        let (render_endpoint_id, capture_endpoint_id) = parse_duplex_stable_id(stable_id)?;
        let render_opened = open_endpoint_runtime(
            &render_endpoint_id,
            audio_types::AudioDeviceDirection::Playback,
            config,
            share_mode,
        )?;
        let capture_opened = open_endpoint_runtime(
            &capture_endpoint_id,
            audio_types::AudioDeviceDirection::Capture,
            config,
            share_mode,
        )?;

        let smallest_buffer = render_opened
            .buffer_frames
            .min(capture_opened.buffer_frames)
            .max(MIN_STREAM_PERIOD_FRAMES);
        let period_frames = config
            .period_frames
            .max(MIN_STREAM_PERIOD_FRAMES)
            .min(smallest_buffer);
        let poll_period =
            audio_core::resolved_worker_poll_period(period_frames, config.sample_rate);

        Arc::new(WasapiStreamRuntime {
            direction,
            format: config.format,
            sample_rate: config.sample_rate,
            channels: config.channels,
            period_frames,
            frame_bytes: audio_core::frame_bytes(config.format, config.channels)?,
            poll_period,
            playback_buffer_frames: render_opened.buffer_frames.max(1),
            playback_client: Some(render_opened.client),
            capture_client_owner: Some(capture_opened.client),
            render_client: render_opened.render_client,
            capture_client: capture_opened.capture_client,
            playback_event: Some(render_opened.event_handle),
            capture_event: Some(capture_opened.event_handle),
        })
    };

    Ok(opened)
}

/// Validate one parsed endpoint id against one requested stream direction.
fn validate_selected_endpoint(
    selected: &SelectedEndpoint,
    direction: audio_types::AudioDeviceDirection,
) -> RuntimeResult<()> {
    match (selected.flow, direction) {
        (EndpointFlow::Render, audio_types::AudioDeviceDirection::Playback)
        | (EndpointFlow::Capture, audio_types::AudioDeviceDirection::Capture)
        | (EndpointFlow::Loopback, audio_types::AudioDeviceDirection::Loopback) => Ok(()),
        (EndpointFlow::Render, _) | (EndpointFlow::Capture, _) | (EndpointFlow::Loopback, _) => {
            Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "wasapi endpoint lane does not match requested stream direction",
            ))
            .boxed())
        }
    }
}

/// Open one endpoint runtime payload for one concrete direction lane.
fn open_endpoint_runtime(
    endpoint_id: &str,
    direction: audio_types::AudioDeviceDirection,
    config: audio_types::AudioStreamConfig,
    share_mode: audio_types::AudioShareMode,
) -> RuntimeResult<OpenedWasapiEndpoint> {
    let enumerator = create_device_enumerator()?;
    let endpoint = get_endpoint_by_id(enumerator.raw() as IMMDeviceEnumerator, endpoint_id)?;
    let audio_client = activate_audio_client(endpoint.raw() as IMMDevice)?;

    let format = build_wave_format(config)?;
    let native_share_mode = match share_mode {
        audio_types::AudioShareMode::Shared => AUDCLNT_SHAREMODE_SHARED,
        audio_types::AudioShareMode::Exclusive => AUDCLNT_SHAREMODE_EXCLUSIVE,
    };

    let mut stream_flags = AUDCLNT_STREAMFLAGS_EVENTCALLBACK;
    if direction == audio_types::AudioDeviceDirection::Loopback {
        stream_flags |= AUDCLNT_STREAMFLAGS_LOOPBACK;
    }

    let period_hns = ((config.period_frames as u64)
        .saturating_mul(10_000_000u64)
        .checked_div(config.sample_rate.max(1) as u64)
        .unwrap_or(0)
        .max(1)) as i64;
    let periodicity_hns = if share_mode == audio_types::AudioShareMode::Exclusive {
        period_hns
    } else {
        0
    };

    let initialize_status = unsafe {
        audio_client_initialize(
            audio_client.raw() as IAudioClient,
            native_share_mode,
            stream_flags,
            period_hns,
            periodicity_hns,
            &format,
        )
    };
    if failed(initialize_status) {
        return Err(hresult_error(
            "destack.audio.stream.open",
            initialize_status,
            "failed to initialize WASAPI audio client",
        ));
    }

    let event_handle = create_stream_event_handle()?;
    let event_status = unsafe {
        audio_client_set_event_handle(audio_client.raw() as IAudioClient, event_handle.raw)
    };
    if failed(event_status) {
        return Err(hresult_error(
            "destack.audio.stream.open",
            event_status,
            "failed to set WASAPI event callback handle",
        ));
    }

    let mut buffer_frames = 0u32;
    let buffer_status = unsafe {
        audio_client_get_buffer_size(audio_client.raw() as IAudioClient, &mut buffer_frames)
    };
    if failed(buffer_status) {
        return Err(hresult_error(
            "destack.audio.stream.open",
            buffer_status,
            "failed to read WASAPI buffer size",
        ));
    }

    let render_client = if direction == audio_types::AudioDeviceDirection::Playback {
        Some(Arc::new(WasapiRenderClient {
            raw: get_render_client(audio_client.raw() as IAudioClient)?.into_raw(),
        }))
    } else {
        None
    };
    let capture_client = if direction == audio_types::AudioDeviceDirection::Capture
        || direction == audio_types::AudioDeviceDirection::Loopback
    {
        Some(Arc::new(WasapiCaptureClient {
            raw: get_capture_client(audio_client.raw() as IAudioClient)?.into_raw(),
        }))
    } else {
        None
    };
    let client = Arc::new(WasapiAudioClient {
        raw: audio_client.into_raw(),
    });

    Ok(OpenedWasapiEndpoint {
        client,
        render_client,
        capture_client,
        buffer_frames: buffer_frames.max(1),
        event_handle,
    })
}
