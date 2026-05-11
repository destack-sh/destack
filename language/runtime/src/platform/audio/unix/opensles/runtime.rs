use std::ffi::c_void;
use std::ptr;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;
use crate::platform::audio::core::codec::clamp_audio_scalar;
use crate::runtime::{ExecutionMode, ExecutionPolicy, start_with_policy};

use super::abi::{
    SL_IID_ANDROIDSIMPLEBUFFERQUEUE, SL_IID_PLAY, SL_IID_RECORD, SLAndroidSimpleBufferQueueItf,
    SLAndroidSimpleBufferQueueItf_, SLAndroidSimpleBufferQueueState, SLDataFormat_PCM,
    SLDataLocator_AndroidSimpleBufferQueue, SLDataLocator_IODevice, SLDataLocator_OutputMix,
    SLDataSink, SLDataSource, SLObjectItf, SLPlayItf, SLRecordItf, SLboolean, SLuint32,
};
use super::constants::{
    OPENSLES_MAX_CHANNELS, OPENSLES_MAX_PERIOD_FRAMES, OPENSLES_MAX_SAMPLE_RATE,
    OPENSLES_MIN_PERIOD_FRAMES, OPENSLES_MIN_SAMPLE_RATE, OPENSLES_QUEUE_DEPTH,
    OPENSLES_STREAM_NAME, SL_BOOLEAN_TRUE, SL_DATALOCATOR_ANDROIDSIMPLEBUFFERQUEUE,
    SL_DATALOCATOR_IODEVICE, SL_DATALOCATOR_OUTPUTMIX, SL_DEFAULTDEVICEID_AUDIOINPUT,
    SL_IODEVICE_AUDIOINPUT, SL_PLAYSTATE_PAUSED, SL_PLAYSTATE_PLAYING, SL_PLAYSTATE_STOPPED,
    SL_RECORDSTATE_PAUSED, SL_RECORDSTATE_RECORDING, SL_RECORDSTATE_STOPPED,
};
use super::core::{
    OpenSlEngine, SlObjectHandle, create_engine, get_object_interface, interface_id,
    opensles_error, opensles_not_supported, opensles_sample_format, opensles_succeeded, pcm_format,
    realize_object,
};
use super::ids::{parse_opensles_stable_id, validate_opensles_stable_id_direction};
use crate::platform::audio as audio_types;

/// One playback queue operation tag.
const PLAYBACK_QUEUE_OPERATION: &str = "destack.audio.stream.playbackQueue";
/// One capture queue operation tag.
const CAPTURE_QUEUE_OPERATION: &str = "destack.audio.stream.captureQueue";

/// One playback lane opened through OpenSL ES.
#[derive(Debug)]
struct OpenslesPlaybackLane {
    /// Shared engine owner for this lane.
    _engine: Arc<OpenSlEngine>,
    /// Owned player object handle.
    _player_object: SlObjectHandle,
    /// Owned play interface pointer.
    play_interface: SLPlayItf,
    /// Owned queue interface pointer.
    queue_interface: SLAndroidSimpleBufferQueueItf,
    /// Queue buffers and recycling state.
    queue: Mutex<QueueBuffers>,
}

unsafe impl Send for OpenslesPlaybackLane {}
unsafe impl Sync for OpenslesPlaybackLane {}

/// One capture lane opened through OpenSL ES.
#[derive(Debug)]
struct OpenslesCaptureLane {
    /// Shared engine owner for this lane.
    _engine: Arc<OpenSlEngine>,
    /// Owned recorder object handle.
    _recorder_object: SlObjectHandle,
    /// Owned record interface pointer.
    record_interface: SLRecordItf,
    /// Owned queue interface pointer.
    queue_interface: SLAndroidSimpleBufferQueueItf,
    /// Queue buffers and recycling state.
    queue: Mutex<QueueBuffers>,
}

unsafe impl Send for OpenslesCaptureLane {}
unsafe impl Sync for OpenslesCaptureLane {}

/// One queue-buffer ring with in-flight counters.
#[derive(Debug)]
struct QueueBuffers {
    /// Queue packet storage.
    packets: Vec<Vec<u8>>,
    /// Number of packets currently queued in OpenSL ES.
    queued_count: usize,
    /// Next packet index that OpenSL ES completed.
    next_completed_index: usize,
}

/// One stream lane open result.
struct OpenedLane<T> {
    /// Opened lane payload.
    lane: Arc<T>,
    /// Negotiated sample rate in hertz.
    sample_rate: u32,
    /// Negotiated channel count.
    channels: u16,
    /// Negotiated period in frames.
    period_frames: u32,
    /// Negotiated runtime sample format.
    format: audio_types::AudioSampleFormat,
}

/// One OpenSL ES stream runtime payload.
#[derive(Debug)]
struct OpenslesStreamRuntime {
    /// Opened playback lane when present.
    playback: Option<Arc<OpenslesPlaybackLane>>,
    /// Opened capture lane when present.
    capture: Option<Arc<OpenslesCaptureLane>>,
    /// Negotiated stream sample rate.
    sample_rate: u32,
    /// Negotiated stream channel count.
    channels: u16,
    /// Negotiated stream period size in frames.
    period_frames: u32,
    /// Negotiated stream sample format.
    format: audio_types::AudioSampleFormat,
    /// Worker cycle sleep period.
    poll_period: Duration,
}

/// One host-operations payload for one OpenSL ES stream host state.
#[derive(Debug)]
struct OpenslesHostStreamOps {
    /// Shared OpenSL ES runtime payload.
    runtime: Arc<OpenslesStreamRuntime>,
}

impl audio_core::AudioHostStreamOps for OpenslesHostStreamOps {
    fn start(&self) -> RuntimeResult<()> {
        if let Some(playback) = self.runtime.playback.as_ref() {
            set_play_state(
                playback.play_interface,
                SL_PLAYSTATE_PLAYING,
                "destack.audio.stream.start",
                "failed to start OpenSL ES playback lane",
            )?;
        }

        if let Some(capture) = self.runtime.capture.as_ref() {
            set_record_state(
                capture.record_interface,
                SL_RECORDSTATE_RECORDING,
                "destack.audio.stream.start",
                "failed to start OpenSL ES capture lane",
            )?;
        }

        Ok(())
    }

    fn pause(&self, pause: bool) -> RuntimeResult<()> {
        let play_state = if pause {
            SL_PLAYSTATE_PAUSED
        } else {
            SL_PLAYSTATE_PLAYING
        };
        let record_state = if pause {
            SL_RECORDSTATE_PAUSED
        } else {
            SL_RECORDSTATE_RECORDING
        };

        if let Some(playback) = self.runtime.playback.as_ref() {
            set_play_state(
                playback.play_interface,
                play_state,
                "destack.audio.stream.pause",
                "failed to change OpenSL ES playback state",
            )?;
        }

        if let Some(capture) = self.runtime.capture.as_ref() {
            set_record_state(
                capture.record_interface,
                record_state,
                "destack.audio.stream.pause",
                "failed to change OpenSL ES capture state",
            )?;
        }

        Ok(())
    }

    fn stop(&self) -> RuntimeResult<()> {
        if let Some(playback) = self.runtime.playback.as_ref() {
            set_play_state(
                playback.play_interface,
                SL_PLAYSTATE_STOPPED,
                "destack.audio.stream.stop",
                "failed to stop OpenSL ES playback lane",
            )?;
        }

        if let Some(capture) = self.runtime.capture.as_ref() {
            set_record_state(
                capture.record_interface,
                SL_RECORDSTATE_STOPPED,
                "destack.audio.stream.stop",
                "failed to stop OpenSL ES capture lane",
            )?;
        }

        Ok(())
    }

    fn flush(&self) -> RuntimeResult<()> {
        if let Some(playback) = self.runtime.playback.as_ref() {
            clear_playback_queue(playback, "destack.audio.stream.flush")?;
            prime_playback_queue(playback, "destack.audio.stream.flush")?;
        }

        if let Some(capture) = self.runtime.capture.as_ref() {
            clear_capture_queue(capture, "destack.audio.stream.flush")?;
            prime_capture_queue(capture, "destack.audio.stream.flush")?;
        }

        Ok(())
    }
}

/// Open one OpenSL ES stream host state.
pub(super) fn open_stream(
    device_info: &audio_core::HostDeviceDescriptor,
    config: audio_types::AudioStreamConfig,
    share_mode: audio_types::AudioShareMode,
    requested_flags: audio_types::AudioStreamFlags,
    requested_requirements: audio_types::AudioStreamRequirementFlags,
) -> RuntimeResult<Arc<audio_core::AudioStreamHostState>> {
    // reject unsupported direction requests
    if device_info.direction == audio_types::AudioDeviceDirection::Loopback {
        return Err(opensles_not_supported(
            "destack.audio.stream.open",
            "OpenSL ES loopback direction is not implemented",
        ));
    }

    // reject unsupported share modes
    if share_mode == audio_types::AudioShareMode::Exclusive {
        return Err(opensles_not_supported(
            "destack.audio.stream.open",
            "OpenSL ES does not support exclusive mode",
        ));
    }

    // validate one stable-id contract before opening one runtime
    let parsed = parse_opensles_stable_id(&device_info.id)?;
    validate_opensles_stable_id_direction(&parsed, device_info.direction)?;
    let _ = (
        parsed.playback_name.as_str(),
        parsed.capture_name.as_deref(),
        OPENSLES_STREAM_NAME,
    );

    // validate one requested stream configuration for OpenSL ES limits
    validate_stream_config(config, "destack.audio.stream.open")?;
    let format = opensles_sample_format(config.format).ok_or_else(|| {
        opensles_not_supported(
            "destack.audio.stream.open",
            format!(
                "sample format {:?} is not supported by OpenSL ES",
                config.format
            ),
        )
    })?;

    let needs_playback = matches!(
        device_info.direction,
        audio_types::AudioDeviceDirection::Playback | audio_types::AudioDeviceDirection::Duplex
    );
    let needs_capture = matches!(
        device_info.direction,
        audio_types::AudioDeviceDirection::Capture | audio_types::AudioDeviceDirection::Duplex
    );

    let engine = Arc::new(create_engine("destack.audio.stream.open")?);

    // open one playback stream lane when one playback direction is requested
    let playback_lane = if needs_playback {
        Some(open_playback_lane(engine.clone(), config, format)?)
    } else {
        None
    };

    // open one capture stream lane when one capture direction is requested
    let capture_lane = if needs_capture {
        Some(open_capture_lane(engine, config, format)?)
    } else {
        None
    };

    let (primary_sample_rate, primary_channels, primary_format) =
        if let Some(playback_lane) = playback_lane.as_ref() {
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
            return Err(opensles_not_supported(
                "destack.audio.stream.open",
                "OpenSL ES stream direction is not supported",
            ));
        };

    // validate negotiated duplex lane alignment
    if let Some(capture_lane) = capture_lane.as_ref()
        && (capture_lane.sample_rate != primary_sample_rate
            || capture_lane.channels != primary_channels
            || capture_lane.format != primary_format)
    {
        return Err(opensles_not_supported(
            "destack.audio.stream.open",
            "OpenSL ES duplex lanes negotiated mismatched formats",
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

    let poll_period = audio_core::resolved_worker_poll_period(period_frames, primary_sample_rate);

    let runtime = Arc::new(OpenslesStreamRuntime {
        playback: playback_lane.map(|lane| lane.lane),
        capture: capture_lane.map(|lane| lane.lane),
        sample_rate: primary_sample_rate,
        channels: primary_channels,
        period_frames,
        format: primary_format,
        poll_period,
    });

    let host_ops: Arc<dyn audio_core::AudioHostStreamOps> = Arc::new(OpenslesHostStreamOps {
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

    let worker = spawn_worker(stream_state.clone(), runtime);
    *stream_state
        .worker_thread
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(worker);

    Ok(stream_state)
}

/// Validate one stream configuration against OpenSL ES backend limits.
fn validate_stream_config(
    config: audio_types::AudioStreamConfig,
    operation: &'static str,
) -> RuntimeResult<()> {
    if config.channels == 0 || config.channels > OPENSLES_MAX_CHANNELS {
        return Err(opensles_not_supported(
            operation,
            format!(
                "OpenSL ES channel count {} is outside supported range 1..={OPENSLES_MAX_CHANNELS}",
                config.channels
            ),
        ));
    }

    if config.sample_rate < OPENSLES_MIN_SAMPLE_RATE
        || config.sample_rate > OPENSLES_MAX_SAMPLE_RATE
    {
        return Err(opensles_not_supported(
            operation,
            format!(
                "OpenSL ES sample rate {} is outside supported range {OPENSLES_MIN_SAMPLE_RATE}..={OPENSLES_MAX_SAMPLE_RATE}",
                config.sample_rate
            ),
        ));
    }

    if config.period_frames < OPENSLES_MIN_PERIOD_FRAMES
        || config.period_frames > OPENSLES_MAX_PERIOD_FRAMES
    {
        return Err(opensles_not_supported(
            operation,
            format!(
                "OpenSL ES period frames {} is outside supported range {OPENSLES_MIN_PERIOD_FRAMES}..={OPENSLES_MAX_PERIOD_FRAMES}",
                config.period_frames
            ),
        ));
    }

    Ok(())
}

/// Open one playback lane with one stream configuration.
fn open_playback_lane(
    engine: Arc<OpenSlEngine>,
    config: audio_types::AudioStreamConfig,
    format: audio_types::AudioSampleFormat,
) -> RuntimeResult<OpenedLane<OpenslesPlaybackLane>> {
    let mut queue_locator = SLDataLocator_AndroidSimpleBufferQueue {
        locatorType: SL_DATALOCATOR_ANDROIDSIMPLEBUFFERQUEUE,
        numBuffers: OPENSLES_QUEUE_DEPTH as SLuint32,
    };
    let mut pcm = pcm_format(config.channels, config.sample_rate)?;
    let mut source = SLDataSource {
        pLocator: (&mut queue_locator as *mut SLDataLocator_AndroidSimpleBufferQueue)
            .cast::<c_void>(),
        pFormat: (&mut pcm as *mut SLDataFormat_PCM).cast::<c_void>(),
    };
    let mut sink_locator = SLDataLocator_OutputMix {
        locatorType: SL_DATALOCATOR_OUTPUTMIX,
        outputMix: engine.output_mix_object.raw,
    };
    let mut sink = SLDataSink {
        pLocator: (&mut sink_locator as *mut SLDataLocator_OutputMix).cast::<c_void>(),
        pFormat: ptr::null_mut(),
    };

    let queue_id = interface_id(
        "destack.audio.stream.open",
        "SL_IID_ANDROIDSIMPLEBUFFERQUEUE",
        unsafe { SL_IID_ANDROIDSIMPLEBUFFERQUEUE },
    )?;
    let play_id = interface_id("destack.audio.stream.open", "SL_IID_PLAY", unsafe {
        SL_IID_PLAY
    })?;
    let interface_ids = [queue_id, play_id];
    let interface_required = [SL_BOOLEAN_TRUE as SLboolean, SL_BOOLEAN_TRUE as SLboolean];

    let engine_table = unsafe { *engine.engine_interface };
    if engine_table.is_null() {
        return Err(opensles_error(
            "destack.audio.stream.open",
            None,
            "failed to load OpenSL ES engine interface table",
        ));
    }

    let Some(create_audio_player) = (unsafe { (*engine_table).CreateAudioPlayer }) else {
        return Err(opensles_not_supported(
            "destack.audio.stream.open",
            "OpenSL ES audio player creation is unavailable",
        ));
    };

    let mut player_object: SLObjectItf = ptr::null();
    let create_status = unsafe {
        create_audio_player(
            engine.engine_interface,
            &mut player_object,
            &mut source,
            &mut sink,
            interface_ids.len() as SLuint32,
            interface_ids.as_ptr(),
            interface_required.as_ptr(),
        )
    };
    if !opensles_succeeded(create_status) || player_object.is_null() {
        return Err(opensles_error(
            "destack.audio.stream.open",
            Some(create_status),
            "failed to create OpenSL ES audio player object",
        ));
    }

    let player_object = SlObjectHandle { raw: player_object };
    realize_object(
        player_object.raw,
        "destack.audio.stream.open",
        "audio player object",
    )?;

    let play_interface = get_object_interface(
        player_object.raw,
        play_id,
        "destack.audio.stream.open",
        "play interface",
    )? as SLPlayItf;
    if play_interface.is_null() {
        return Err(opensles_error(
            "destack.audio.stream.open",
            None,
            "failed to resolve OpenSL ES play interface pointer",
        ));
    }

    let queue_interface = get_object_interface(
        player_object.raw,
        queue_id,
        "destack.audio.stream.open",
        "android simple-buffer-queue interface",
    )? as SLAndroidSimpleBufferQueueItf;
    if queue_interface.is_null() {
        return Err(opensles_error(
            "destack.audio.stream.open",
            None,
            "failed to resolve OpenSL ES playback queue interface pointer",
        ));
    }

    let frame_bytes = audio_core::frame_bytes(format, config.channels)?;
    let packet_len = frame_bytes.saturating_mul(config.period_frames as usize);
    let queue = QueueBuffers {
        packets: vec![vec![0u8; packet_len]; OPENSLES_QUEUE_DEPTH],
        queued_count: 0,
        next_completed_index: 0,
    };

    let lane = Arc::new(OpenslesPlaybackLane {
        _engine: engine,
        _player_object: player_object,
        play_interface,
        queue_interface,
        queue: Mutex::new(queue),
    });

    prime_playback_queue(&lane, "destack.audio.stream.open")?;
    set_play_state(
        lane.play_interface,
        SL_PLAYSTATE_STOPPED,
        "destack.audio.stream.open",
        "failed to initialize OpenSL ES playback lane state",
    )?;

    Ok(OpenedLane {
        lane,
        sample_rate: config.sample_rate,
        channels: config.channels,
        period_frames: config.period_frames,
        format,
    })
}

/// Open one capture lane with one stream configuration.
fn open_capture_lane(
    engine: Arc<OpenSlEngine>,
    config: audio_types::AudioStreamConfig,
    format: audio_types::AudioSampleFormat,
) -> RuntimeResult<OpenedLane<OpenslesCaptureLane>> {
    let mut source_locator = SLDataLocator_IODevice {
        locatorType: SL_DATALOCATOR_IODEVICE,
        deviceType: SL_IODEVICE_AUDIOINPUT,
        deviceID: SL_DEFAULTDEVICEID_AUDIOINPUT,
        device: ptr::null(),
    };
    let mut source = SLDataSource {
        pLocator: (&mut source_locator as *mut SLDataLocator_IODevice).cast::<c_void>(),
        pFormat: ptr::null_mut(),
    };
    let mut queue_locator = SLDataLocator_AndroidSimpleBufferQueue {
        locatorType: SL_DATALOCATOR_ANDROIDSIMPLEBUFFERQUEUE,
        numBuffers: OPENSLES_QUEUE_DEPTH as SLuint32,
    };
    let mut pcm = pcm_format(config.channels, config.sample_rate)?;
    let mut sink = SLDataSink {
        pLocator: (&mut queue_locator as *mut SLDataLocator_AndroidSimpleBufferQueue)
            .cast::<c_void>(),
        pFormat: (&mut pcm as *mut SLDataFormat_PCM).cast::<c_void>(),
    };

    let queue_id = interface_id(
        "destack.audio.stream.open",
        "SL_IID_ANDROIDSIMPLEBUFFERQUEUE",
        unsafe { SL_IID_ANDROIDSIMPLEBUFFERQUEUE },
    )?;
    let record_id = interface_id("destack.audio.stream.open", "SL_IID_RECORD", unsafe {
        SL_IID_RECORD
    })?;
    let interface_ids = [queue_id, record_id];
    let interface_required = [SL_BOOLEAN_TRUE as SLboolean, SL_BOOLEAN_TRUE as SLboolean];

    let engine_table = unsafe { *engine.engine_interface };
    if engine_table.is_null() {
        return Err(opensles_error(
            "destack.audio.stream.open",
            None,
            "failed to load OpenSL ES engine interface table",
        ));
    }

    let Some(create_audio_recorder) = (unsafe { (*engine_table).CreateAudioRecorder }) else {
        return Err(opensles_not_supported(
            "destack.audio.stream.open",
            "OpenSL ES audio recorder creation is unavailable",
        ));
    };

    let mut recorder_object: SLObjectItf = ptr::null();
    let create_status = unsafe {
        create_audio_recorder(
            engine.engine_interface,
            &mut recorder_object,
            &mut source,
            &mut sink,
            interface_ids.len() as SLuint32,
            interface_ids.as_ptr(),
            interface_required.as_ptr(),
        )
    };
    if !opensles_succeeded(create_status) || recorder_object.is_null() {
        return Err(opensles_error(
            "destack.audio.stream.open",
            Some(create_status),
            "failed to create OpenSL ES audio recorder object",
        ));
    }

    let recorder_object = SlObjectHandle {
        raw: recorder_object,
    };
    realize_object(
        recorder_object.raw,
        "destack.audio.stream.open",
        "audio recorder object",
    )?;

    let record_interface = get_object_interface(
        recorder_object.raw,
        record_id,
        "destack.audio.stream.open",
        "record interface",
    )? as SLRecordItf;
    if record_interface.is_null() {
        return Err(opensles_error(
            "destack.audio.stream.open",
            None,
            "failed to resolve OpenSL ES record interface pointer",
        ));
    }

    let queue_interface = get_object_interface(
        recorder_object.raw,
        queue_id,
        "destack.audio.stream.open",
        "android simple-buffer-queue interface",
    )? as SLAndroidSimpleBufferQueueItf;
    if queue_interface.is_null() {
        return Err(opensles_error(
            "destack.audio.stream.open",
            None,
            "failed to resolve OpenSL ES capture queue interface pointer",
        ));
    }

    let frame_bytes = audio_core::frame_bytes(format, config.channels)?;
    let packet_len = frame_bytes.saturating_mul(config.period_frames as usize);
    let queue = QueueBuffers {
        packets: vec![vec![0u8; packet_len]; OPENSLES_QUEUE_DEPTH],
        queued_count: 0,
        next_completed_index: 0,
    };

    let lane = Arc::new(OpenslesCaptureLane {
        _engine: engine,
        _recorder_object: recorder_object,
        record_interface,
        queue_interface,
        queue: Mutex::new(queue),
    });

    prime_capture_queue(&lane, "destack.audio.stream.open")?;
    set_record_state(
        lane.record_interface,
        SL_RECORDSTATE_STOPPED,
        "destack.audio.stream.open",
        "failed to initialize OpenSL ES capture lane state",
    )?;

    Ok(OpenedLane {
        lane,
        sample_rate: config.sample_rate,
        channels: config.channels,
        period_frames: config.period_frames,
        format,
    })
}

/// Spawn one OpenSL ES transfer worker thread.
fn spawn_worker(
    binding: Arc<audio_core::AudioStreamHostState>,
    runtime: Arc<OpenslesStreamRuntime>,
) -> std::thread::JoinHandle<()> {
    start_with_policy(
        "destack-audio-opensles-transfer",
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

                let mut disconnected_message = None::<String>;

                // refill completed playback queue packets
                if let Some(playback) = runtime.playback.as_ref() {
                    let completed_indices = match completed_queue_indices(
                        &playback.queue,
                        playback.queue_interface,
                        PLAYBACK_QUEUE_OPERATION,
                    ) {
                        Ok(indices) => indices,
                        Err(error) => {
                            disconnected_message = Some(error.to_string());
                            Vec::new()
                        }
                    };

                    for index in completed_indices {
                        let packet = build_playback_packet(&binding, &runtime);

                        if let Err(error) = enqueue_packet(
                            &playback.queue,
                            playback.queue_interface,
                            index,
                            &packet,
                            PLAYBACK_QUEUE_OPERATION,
                        ) {
                            disconnected_message = Some(error.to_string());
                            break;
                        }
                    }
                }

                // read completed capture queue packets
                if disconnected_message.is_none()
                    && let Some(capture) = runtime.capture.as_ref()
                {
                    let completed_indices = match completed_queue_indices(
                        &capture.queue,
                        capture.queue_interface,
                        CAPTURE_QUEUE_OPERATION,
                    ) {
                        Ok(indices) => indices,
                        Err(error) => {
                            disconnected_message = Some(error.to_string());
                            Vec::new()
                        }
                    };

                    for index in completed_indices {
                        let packet = {
                            let queue = capture
                                .queue
                                .lock()
                                .unwrap_or_else(|error| error.into_inner());
                            match queue.packets.get(index) {
                                Some(packet) => packet.clone(),
                                None => {
                                    disconnected_message = Some(format!(
                                        "OpenSL ES capture queue packet slot index {index} is out of bounds"
                                    ));
                                    break;
                                }
                            }
                        };

                        let decoded = match audio_core::decode_audio_bytes(&packet, runtime.format)
                        {
                            Ok(decoded) => decoded,
                            Err(error) => {
                                disconnected_message =
                                    Some(format!("OpenSL ES capture decode failed: {error}"));
                                break;
                            }
                        };

                        push_capture_packet(&binding, decoded);

                        if let Err(error) = enqueue_zero_packet(
                            &capture.queue,
                            capture.queue_interface,
                            index,
                            CAPTURE_QUEUE_OPERATION,
                        ) {
                            disconnected_message = Some(error.to_string());
                            break;
                        }
                    }
                }

                if let Some(message) = disconnected_message {
                    mark_backend_disconnected(&binding, message);
                    break;
                }

                binding.sync.wake.notify_all();
                audio_core::wait_for_worker_period(&binding, runtime.poll_period);
            }
        },
    )
    .unwrap_or_else(|error| panic!("failed to spawn required attached runtime: {error}"))
}

/// Build one playback packet from one stream state queue.
fn build_playback_packet(
    binding: &audio_core::AudioStreamHostState,
    runtime: &OpenslesStreamRuntime,
) -> Vec<u8> {
    let scalar_period = runtime
        .period_frames
        .saturating_mul(runtime.channels as u32) as usize;

    let mut state = binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

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
        state.output_underflow_count = state.output_underflow_count.saturating_add(1);
        state.status_flags = audio_types::AudioStreamStatusFlags(
            state.status_flags.0 | audio_core::STREAM_STATUS_OUTPUT_UNDERFLOW.0,
        );
    }

    audio_core::encode_audio_bytes(&playback_packet, runtime.format)
}

/// Push one capture packet into one shared stream queue.
fn push_capture_packet(binding: &audio_core::AudioStreamHostState, packet: Vec<f32>) {
    let mut state = binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    for sample in packet {
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
        state.status_flags = audio_types::AudioStreamStatusFlags(
            state.status_flags.0 | audio_core::STREAM_STATUS_INPUT_OVERFLOW.0,
        );
    }
}

/// Mark one stream as backend-disconnected and wake blocked callers.
fn mark_backend_disconnected(binding: &audio_core::AudioStreamHostState, message: String) {
    audio_core::mark_stream_backend_disconnected(binding, message);
}

/// Set one playback state through one OpenSL ES play interface.
fn set_play_state(
    play_interface: SLPlayItf,
    play_state: SLuint32,
    operation: &'static str,
    message: &'static str,
) -> RuntimeResult<()> {
    if play_interface.is_null() {
        return Err(opensles_error(
            operation,
            None,
            "OpenSL ES play interface is null",
        ));
    }

    let table = unsafe { *play_interface };
    if table.is_null() {
        return Err(opensles_error(
            operation,
            None,
            "OpenSL ES play interface table is null",
        ));
    }

    let Some(set_play_state) = (unsafe { (*table).SetPlayState }) else {
        return Err(opensles_not_supported(
            operation,
            "OpenSL ES SetPlayState is unavailable",
        ));
    };

    let status = unsafe { set_play_state(play_interface, play_state) };
    if opensles_succeeded(status) {
        return Ok(());
    }

    Err(opensles_error(operation, Some(status), message))
}

/// Set one capture state through one OpenSL ES record interface.
fn set_record_state(
    record_interface: SLRecordItf,
    record_state: SLuint32,
    operation: &'static str,
    message: &'static str,
) -> RuntimeResult<()> {
    if record_interface.is_null() {
        return Err(opensles_error(
            operation,
            None,
            "OpenSL ES record interface is null",
        ));
    }

    let table = unsafe { *record_interface };
    if table.is_null() {
        return Err(opensles_error(
            operation,
            None,
            "OpenSL ES record interface table is null",
        ));
    }

    let Some(set_record_state) = (unsafe { (*table).SetRecordState }) else {
        return Err(opensles_not_supported(
            operation,
            "OpenSL ES SetRecordState is unavailable",
        ));
    };

    let status = unsafe { set_record_state(record_interface, record_state) };
    if opensles_succeeded(status) {
        return Ok(());
    }

    Err(opensles_error(operation, Some(status), message))
}

/// Prime one playback queue with silent packets.
fn prime_playback_queue(
    playback: &OpenslesPlaybackLane,
    operation: &'static str,
) -> RuntimeResult<()> {
    let queue_len = {
        let queue = playback
            .queue
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        queue.packets.len()
    };

    for index in 0..queue_len {
        enqueue_zero_packet(&playback.queue, playback.queue_interface, index, operation)?;
    }

    Ok(())
}

/// Prime one capture queue with empty packets.
fn prime_capture_queue(
    capture: &OpenslesCaptureLane,
    operation: &'static str,
) -> RuntimeResult<()> {
    let queue_len = {
        let queue = capture
            .queue
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        queue.packets.len()
    };

    for index in 0..queue_len {
        enqueue_zero_packet(&capture.queue, capture.queue_interface, index, operation)?;
    }

    Ok(())
}

/// Clear one playback queue and reset local queue tracking.
fn clear_playback_queue(
    playback: &OpenslesPlaybackLane,
    operation: &'static str,
) -> RuntimeResult<()> {
    clear_queue_and_reset(&playback.queue, playback.queue_interface, operation)
}

/// Clear one capture queue and reset local queue tracking.
fn clear_capture_queue(
    capture: &OpenslesCaptureLane,
    operation: &'static str,
) -> RuntimeResult<()> {
    clear_queue_and_reset(&capture.queue, capture.queue_interface, operation)
}

/// Clear one OpenSL ES queue and reset one queue tracker.
fn clear_queue_and_reset(
    queue_mutex: &Mutex<QueueBuffers>,
    queue_interface: SLAndroidSimpleBufferQueueItf,
    operation: &'static str,
) -> RuntimeResult<()> {
    let mut queue = queue_mutex
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    queue_clear(queue_interface, operation)?;
    queue.queued_count = 0;
    queue.next_completed_index = 0;

    Ok(())
}

/// Return queue slots completed since the previous poll.
fn completed_queue_indices(
    queue_mutex: &Mutex<QueueBuffers>,
    queue_interface: SLAndroidSimpleBufferQueueItf,
    operation: &'static str,
) -> RuntimeResult<Vec<usize>> {
    let mut queue = queue_mutex
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let open_sl_queued = queue_count(queue_interface, operation)?.min(queue.packets.len());
    let completed = queue.queued_count.saturating_sub(open_sl_queued);
    queue.queued_count = open_sl_queued;

    let mut indices = Vec::with_capacity(completed);
    for _ in 0..completed {
        let index = queue.next_completed_index;
        queue.next_completed_index = if queue.packets.is_empty() {
            0
        } else {
            (queue.next_completed_index + 1) % queue.packets.len()
        };
        indices.push(index);
    }

    Ok(indices)
}

/// Enqueue one packet payload in one queue slot.
fn enqueue_packet(
    queue_mutex: &Mutex<QueueBuffers>,
    queue_interface: SLAndroidSimpleBufferQueueItf,
    index: usize,
    packet: &[u8],
    operation: &'static str,
) -> RuntimeResult<()> {
    let mut queue = queue_mutex
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let packet_slot = queue.packets.get_mut(index).ok_or_else(|| {
        opensles_error(
            operation,
            None,
            format!("OpenSL ES queue packet slot index {index} is out of bounds"),
        )
    })?;

    if packet.len() != packet_slot.len() {
        return Err(opensles_error(
            operation,
            None,
            format!(
                "OpenSL ES queue packet length mismatch: expected {}, got {}",
                packet_slot.len(),
                packet.len()
            ),
        ));
    }

    packet_slot.copy_from_slice(packet);
    queue_enqueue(queue_interface, packet_slot, operation)?;
    queue.queued_count = queue.queued_count.saturating_add(1);

    Ok(())
}

/// Enqueue one silent packet in one queue slot.
fn enqueue_zero_packet(
    queue_mutex: &Mutex<QueueBuffers>,
    queue_interface: SLAndroidSimpleBufferQueueItf,
    index: usize,
    operation: &'static str,
) -> RuntimeResult<()> {
    let mut queue = queue_mutex
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let packet_slot = queue.packets.get_mut(index).ok_or_else(|| {
        opensles_error(
            operation,
            None,
            format!("OpenSL ES queue packet slot index {index} is out of bounds"),
        )
    })?;

    packet_slot.fill(0);
    queue_enqueue(queue_interface, packet_slot, operation)?;
    queue.queued_count = queue.queued_count.saturating_add(1);

    Ok(())
}

/// Read the queue packet count for one OpenSL ES queue interface.
fn queue_count(
    queue_interface: SLAndroidSimpleBufferQueueItf,
    operation: &'static str,
) -> RuntimeResult<usize> {
    let table = queue_table(queue_interface, operation)?;
    let Some(get_state) = (unsafe { (*table).GetState }) else {
        return Err(opensles_not_supported(
            operation,
            "OpenSL ES queue GetState is unavailable",
        ));
    };

    let mut state = SLAndroidSimpleBufferQueueState { count: 0, index: 0 };
    let status = unsafe { get_state(queue_interface, &mut state) };
    if !opensles_succeeded(status) {
        return Err(opensles_error(
            operation,
            Some(status),
            "failed to read OpenSL ES queue state",
        ));
    }

    Ok(state.count as usize)
}

/// Clear one OpenSL ES queue interface.
fn queue_clear(
    queue_interface: SLAndroidSimpleBufferQueueItf,
    operation: &'static str,
) -> RuntimeResult<()> {
    let table = queue_table(queue_interface, operation)?;
    let Some(clear) = (unsafe { (*table).Clear }) else {
        return Err(opensles_not_supported(
            operation,
            "OpenSL ES queue Clear is unavailable",
        ));
    };

    let status = unsafe { clear(queue_interface) };
    if opensles_succeeded(status) {
        return Ok(());
    }

    Err(opensles_error(
        operation,
        Some(status),
        "failed to clear OpenSL ES queue",
    ))
}

/// Enqueue one packet payload for one OpenSL ES queue interface.
fn queue_enqueue(
    queue_interface: SLAndroidSimpleBufferQueueItf,
    packet: &[u8],
    operation: &'static str,
) -> RuntimeResult<()> {
    let table = queue_table(queue_interface, operation)?;
    let Some(enqueue) = (unsafe { (*table).Enqueue }) else {
        return Err(opensles_not_supported(
            operation,
            "OpenSL ES queue Enqueue is unavailable",
        ));
    };

    if packet.len() > u32::MAX as usize {
        return Err(opensles_error(
            operation,
            None,
            format!(
                "OpenSL ES queue packet length {} exceeds u32::MAX bytes",
                packet.len()
            ),
        ));
    }

    let packet_len = packet.len() as SLuint32;
    let status = unsafe {
        enqueue(
            queue_interface,
            packet.as_ptr().cast::<c_void>(),
            packet_len,
        )
    };
    if opensles_succeeded(status) {
        return Ok(());
    }

    Err(opensles_error(
        operation,
        Some(status),
        "failed to enqueue OpenSL ES queue packet",
    ))
}

/// Resolve one queue vtable pointer from one queue interface pointer.
fn queue_table(
    queue_interface: SLAndroidSimpleBufferQueueItf,
    operation: &'static str,
) -> RuntimeResult<*const SLAndroidSimpleBufferQueueItf_> {
    if queue_interface.is_null() {
        return Err(opensles_error(
            operation,
            None,
            "OpenSL ES queue interface pointer is null",
        ));
    }

    let table = unsafe { *queue_interface };
    if table.is_null() {
        return Err(opensles_error(
            operation,
            None,
            "OpenSL ES queue interface table is null",
        ));
    }

    Ok(table)
}
