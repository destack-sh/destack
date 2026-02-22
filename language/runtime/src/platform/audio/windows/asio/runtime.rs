use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::abi::{
    AsioBufferInfo, AsioCallbacks, asio_driver_can_sample_rate, asio_driver_create_buffers,
    asio_driver_get_buffer_size, asio_driver_get_channels, asio_driver_get_latencies,
    asio_driver_get_sample_rate, asio_driver_set_sample_rate,
};
use super::constants::{ASE_OK, ASIO_FALSE, ASIO_TRUE};
use super::core::{
    AsioBufferLane, AsioDirectionLane, AsioHostStreamOps, AsioStreamRuntime, asio_error,
    install_active_runtime,
};
use super::host::{
    driver_by_key_name, open_session, query_channel_descriptor, resolve_buffer_size,
};
use super::ids::parse_stable_id;
use super::transfer::{asio_buffer_switch, asio_message, asio_sample_rate_did_change};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::audio::core as audio_core;

/// Open one ASIO stream binding.
pub(super) fn open_stream(
    device_info: &audio_core::HostDeviceDescriptor,
    config: audio_core::AudioStreamConfig,
    share_mode: audio_core::AudioShareMode,
) -> RuntimeResult<Arc<audio_core::AudioStreamBinding>> {
    // open one initialized runtime payload from one ASIO endpoint id
    let runtime = open_runtime(&device_info.id, device_info.direction, config, share_mode)?;

    // build one stream binding with one ASIO host-ops payload
    let host_ops: Arc<dyn audio_core::AudioHostStreamOps> = Arc::new(AsioHostStreamOps {
        runtime: runtime.clone(),
    });
    let binding = Arc::new(audio_core::AudioStreamBinding {
        device: device_info.clone(),
        direction: device_info.direction,
        requested: config,
        sample_rate: runtime.sample_rate,
        channels: runtime.channels,
        period_frames: runtime.period_frames,
        share_mode,
        runtime_capabilities: audio_core::AudioStreamRuntimeCapabilities {
            supports_write_at: false,
            supports_pause: true,
            supports_non_interleaved: true,
            supports_volume: true,
            supports_mute: true,
            supports_hardware_timestamps: false,
        },
        host_ops: audio_core::Mutex::new(Some(host_ops)),
        name: audio_core::Mutex::new(String::new()),
        sync: Arc::new(audio_core::AudioStreamSync {
            state: audio_core::Mutex::new(audio_core::initial_stream_state()),
            wake: audio_core::Condvar::new(),
        }),
        null_worker: audio_core::Mutex::new(None),
    });

    // install one weak binding pointer for callback-side queue access
    let set_binding = runtime.binding.set(Arc::downgrade(&binding));
    if set_binding.is_err() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "ASIO runtime binding was already initialized",
        ))
        .boxed());
    }

    // publish one active ASIO runtime for callback dispatch
    install_active_runtime(&runtime)?;

    Ok(binding)
}

/// Open one ASIO runtime payload for one parsed stable id.
fn open_runtime(
    stable_id: &str,
    direction: audio_core::AudioDeviceDirection,
    config: audio_core::AudioStreamConfig,
    share_mode: audio_core::AudioShareMode,
) -> RuntimeResult<Arc<AsioStreamRuntime>> {
    // require ASIO exclusive mode to avoid shared-mode surprises
    if share_mode != audio_core::AudioShareMode::Exclusive {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.audio.stream.open ASIO requires exclusive mode",
        ))
        .boxed());
    }

    // parse one stable id lane and validate stream direction
    let parsed = parse_stable_id(stable_id)?;
    validate_lane(parsed.lane, direction)?;

    // resolve one driver row and open one initialized session
    let row = driver_by_key_name(&parsed.key_name)?;
    let (com, session) = open_session(&row)?;
    let _com = com;

    let mut input_channels_available = 0i32;
    let mut output_channels_available = 0i32;
    let channels_status = unsafe {
        asio_driver_get_channels(
            session.driver.raw,
            &mut input_channels_available,
            &mut output_channels_available,
        )
    };
    if channels_status != ASE_OK {
        return Err(asio_error(
            "destack.audio.stream.open",
            channels_status,
            &session.driver_name,
            "failed to query ASIO channel counts",
        ));
    }

    let input_channels_available = input_channels_available.max(0) as u16;
    let output_channels_available = output_channels_available.max(0) as u16;

    // resolve one requested lane channel budget from direction and config
    let (output_channels, input_channels) = match direction {
        audio_core::AudioDeviceDirection::Playback => (config.channels, 0),
        audio_core::AudioDeviceDirection::Capture | audio_core::AudioDeviceDirection::Loopback => {
            (0, config.channels)
        }
        audio_core::AudioDeviceDirection::Duplex => (config.channels, config.channels),
    };

    // validate output channel count against driver limits
    if output_channels > output_channels_available {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "config.channels",
            "requested playback channel count exceeds ASIO output channel capacity",
        ))
        .boxed());
    }

    // validate input channel count against driver limits
    if input_channels > input_channels_available {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "config.channels",
            "requested capture channel count exceeds ASIO input channel capacity",
        ))
        .boxed());
    }

    // validate sample-rate support and configure driver rate
    let rate_support_status =
        unsafe { asio_driver_can_sample_rate(session.driver.raw, config.sample_rate as f64) };
    if rate_support_status != ASE_OK {
        return Err(asio_error(
            "destack.audio.stream.open",
            rate_support_status,
            &session.driver_name,
            format!(
                "ASIO driver does not support sample rate {}",
                config.sample_rate
            ),
        ));
    }

    let mut current_sample_rate = 0.0f64;
    let current_rate_status =
        unsafe { asio_driver_get_sample_rate(session.driver.raw, &mut current_sample_rate) };
    if current_rate_status == ASE_OK
        && current_sample_rate.is_finite()
        && (current_sample_rate.round() as u32) != config.sample_rate
    {
        let set_rate_status =
            unsafe { asio_driver_set_sample_rate(session.driver.raw, config.sample_rate as f64) };
        if set_rate_status != ASE_OK {
            return Err(asio_error(
                "destack.audio.stream.open",
                set_rate_status,
                &session.driver_name,
                "failed to set requested ASIO sample rate",
            ));
        }
    }

    // resolve one negotiated sample rate for runtime snapshots
    let mut effective_sample_rate = 0.0f64;
    let effective_rate_status =
        unsafe { asio_driver_get_sample_rate(session.driver.raw, &mut effective_sample_rate) };
    let effective_sample_rate =
        if effective_rate_status == ASE_OK && effective_sample_rate.is_finite() {
            effective_sample_rate.round().clamp(1.0, u32::MAX as f64) as u32
        } else {
            config.sample_rate
        };

    let mut min_size = 0i32;
    let mut max_size = 0i32;
    let mut preferred_size = 0i32;
    let mut granularity = 0i32;

    // query one ASIO period contract tuple
    let buffer_size_status = unsafe {
        asio_driver_get_buffer_size(
            session.driver.raw,
            &mut min_size,
            &mut max_size,
            &mut preferred_size,
            &mut granularity,
        )
    };
    if buffer_size_status != ASE_OK {
        return Err(asio_error(
            "destack.audio.stream.open",
            buffer_size_status,
            &session.driver_name,
            "failed to query ASIO buffer size contract",
        ));
    }

    let min_size = min_size.max(audio_core::MIN_STREAM_PERIOD_FRAMES as i32) as u32;
    let max_size = max_size.max(min_size as i32) as u32;
    let preferred_size = preferred_size.max(min_size as i32).min(max_size as i32) as u32;

    let period_frames = resolve_buffer_size(
        config.period_frames,
        min_size,
        max_size,
        preferred_size,
        granularity,
    );

    // probe one representative output encoding when playback lanes exist
    let output_encoding = if output_channels > 0 {
        Some(query_channel_descriptor(&session, false, 0)?.encoding)
    } else {
        None
    };

    // probe one representative input encoding when capture lanes exist
    let input_encoding = if input_channels > 0 {
        Some(query_channel_descriptor(&session, true, 0)?.encoding)
    } else {
        None
    };

    let callbacks = AsioCallbacks {
        buffer_switch: Some(asio_buffer_switch),
        sample_rate_did_change: Some(asio_sample_rate_did_change),
        asio_message: Some(asio_message),
        buffer_switch_time_info: None,
    };

    // build one non-interleaved ASIO buffer lane table
    let mut buffer_infos =
        Vec::with_capacity((output_channels as usize) + (input_channels as usize));
    for channel_index in 0..output_channels {
        buffer_infos.push(AsioBufferInfo {
            is_input: ASIO_FALSE,
            channel_num: channel_index as i32,
            buffers: [std::ptr::null_mut(), std::ptr::null_mut()],
        });
    }
    for channel_index in 0..input_channels {
        buffer_infos.push(AsioBufferInfo {
            is_input: ASIO_TRUE,
            channel_num: channel_index as i32,
            buffers: [std::ptr::null_mut(), std::ptr::null_mut()],
        });
    }

    // create one callback buffer set and retry preferred size on strict drivers
    let mut create_status = unsafe {
        asio_driver_create_buffers(
            session.driver.raw,
            buffer_infos.as_mut_ptr(),
            buffer_infos.len() as i32,
            period_frames as i32,
            &callbacks,
        )
    };
    let period_frames = if create_status != ASE_OK && preferred_size != period_frames {
        let retry_status = unsafe {
            asio_driver_create_buffers(
                session.driver.raw,
                buffer_infos.as_mut_ptr(),
                buffer_infos.len() as i32,
                preferred_size as i32,
                &callbacks,
            )
        };
        create_status = retry_status;
        preferred_size
    } else {
        period_frames
    };

    if create_status != ASE_OK {
        return Err(asio_error(
            "destack.audio.stream.open",
            create_status,
            &session.driver_name,
            "failed to create ASIO callback buffers",
        ));
    }

    session.buffers_created.store(true, Ordering::Release);

    let mut output_lanes = Vec::new();
    let mut input_lanes = Vec::new();

    // capture one lane pointer table for callback transfer
    for info in &buffer_infos {
        let lane = AsioBufferLane {
            is_input: info.is_input == ASIO_TRUE,
            buffer_a: info.buffers[0] as *mut u8,
            buffer_b: info.buffers[1] as *mut u8,
        };
        if lane.is_input {
            input_lanes.push(lane);
        } else {
            output_lanes.push(lane);
        }
    }

    // sample the initial latency tuple to warm driver side timing paths
    let mut input_latency = 0i32;
    let mut output_latency = 0i32;
    let _ = unsafe {
        asio_driver_get_latencies(session.driver.raw, &mut input_latency, &mut output_latency)
    };

    Ok(Arc::new(AsioStreamRuntime {
        sample_rate: effective_sample_rate,
        period_frames,
        channels: config.channels,
        output_encoding,
        input_encoding,
        output_lanes,
        input_lanes,
        session,
        binding: std::sync::OnceLock::new(),
    }))
}

/// Validate one parsed stable-id lane against one requested stream direction.
fn validate_lane(
    lane: AsioDirectionLane,
    direction: audio_core::AudioDeviceDirection,
) -> RuntimeResult<()> {
    match (lane, direction) {
        (AsioDirectionLane::Playback, audio_core::AudioDeviceDirection::Playback)
        | (AsioDirectionLane::Capture, audio_core::AudioDeviceDirection::Capture)
        | (AsioDirectionLane::Duplex, audio_core::AudioDeviceDirection::Duplex) => Ok(()),
        _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "asio stable id lane does not match requested stream direction",
        ))
        .boxed()),
    }
}
