use super::codec::*;

/// Decode one Android host recording capability header into one runtime value.
fn decode_recording_capabilities_header(
    header: AndroidHostCameraRecordingCapabilitiesHeader,
) -> CameraRecordingCapabilitiesValue {
    let mut containers = Vec::new();
    if header.container_flags & CAMERA_RECORDING_CONTAINER_MP4 != 0 {
        containers.push(CameraRecordingContainer::Mp4);
    }

    let mut video_codecs = Vec::new();
    if header.video_codec_flags & CAMERA_RECORDING_VIDEO_CODEC_H264 != 0 {
        video_codecs.push(CameraVideoCodec::H264);
    }

    let mut audio_codecs = Vec::new();
    if header.audio_codec_flags & CAMERA_RECORDING_AUDIO_CODEC_AAC != 0 {
        audio_codecs.push(CameraAudioCodec::Aac);
    }

    CameraRecordingCapabilitiesValue {
        containers,
        video_codecs,
        audio_supported: header.audio_supported != 0,
        audio_codecs: if audio_codecs.is_empty() {
            None
        } else {
            Some(audio_codecs)
        },
        pause_supported: header.pause_supported != 0,
        maximum_video_bit_rate: if header.has_maximum_video_bit_rate != 0 {
            u32::try_from(header.maximum_video_bit_rate).ok()
        } else {
            None
        },
        maximum_audio_bit_rate: if header.has_maximum_audio_bit_rate != 0 {
            u32::try_from(header.maximum_audio_bit_rate).ok()
        } else {
            None
        },
    }
}

/// Encode one runtime recording options value for the Android host bridge.
fn encode_recording_options_header(
    options: &CameraRecordingOptionsValue,
    operation: &'static str,
) -> RuntimeResult<AndroidHostCameraRecordingOptionsHeader> {
    let container = match options.container {
        Some(CameraRecordingContainer::Mp4) => CAMERA_RECORDING_CONTAINER_MP4,
        Some(_) => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "options.container",
                "Android camera recording currently supports only mp4 output",
            ))
            .boxed());
        }
        None => 0,
    };

    let video_codec = match options.video_codec {
        Some(CameraVideoCodec::H264) => CAMERA_RECORDING_VIDEO_CODEC_H264,
        Some(_) => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "options.video_codec",
                "Android camera recording currently supports only h264 video",
            ))
            .boxed());
        }
        None => 0,
    };

    let audio_codec = match options.audio_codec {
        Some(CameraAudioCodec::Aac) => CAMERA_RECORDING_AUDIO_CODEC_AAC,
        Some(_) => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "options.audio_codec",
                "Android camera recording currently supports only aac audio",
            ))
            .boxed());
        }
        None => 0,
    };

    if options.audio_enabled.unwrap_or(false) || options.audio_codec.is_some() {
        return Err(camera_not_supported(operation));
    }

    Ok(AndroidHostCameraRecordingOptionsHeader {
        container,
        video_codec,
        has_audio_enabled: u32::from(options.audio_enabled.is_some()),
        audio_enabled: u32::from(options.audio_enabled.unwrap_or(false)),
        audio_codec,
        video_bit_rate: u64::from(options.video_bit_rate.unwrap_or(0)),
        has_video_bit_rate: u32::from(options.video_bit_rate.is_some()),
        audio_bit_rate: u64::from(options.audio_bit_rate.unwrap_or(0)),
        has_audio_bit_rate: u32::from(options.audio_bit_rate.is_some()),
        key_frame_interval_frames: options.key_frame_interval_frames.unwrap_or(0),
        has_key_frame_interval_frames: u32::from(options.key_frame_interval_frames.is_some()),
        maximum_duration_ns: options.maximum_duration_ns.unwrap_or(0),
        has_maximum_duration_ns: u32::from(options.maximum_duration_ns.is_some()),
        maximum_bytes: options.maximum_bytes.unwrap_or(0),
        has_maximum_bytes: u32::from(options.maximum_bytes.is_some()),
    })
}

/// Build one local runtime recording state snapshot.
fn recording_state_snapshot(resource: &AndroidCameraStreamResource) -> CameraRecordingRuntimeState {
    resource.recording.lock().runtime.clone()
}

/// Encode one camera pixel-format enum into the Android host code.
fn encode_pixel_format(
    pixel_format: CameraPixelFormat,
    operation: &'static str,
) -> RuntimeResult<u32> {
    let code = match pixel_format {
        CameraPixelFormat::Bgra8 => 1,
        CameraPixelFormat::Rgba8 => 2,
        CameraPixelFormat::Yuv420 => 3,
        CameraPixelFormat::Jpeg => 4,
    };

    if code == 0 {
        return Err(invalid_data(
            operation,
            "camera stream config returned one unknown pixel format",
        ));
    }

    Ok(code)
}

/// Encode one camera pixel-format-family enum into the Android host code.
fn encode_pixel_format_family(
    family: CameraPixelFormatFamily,
    operation: &'static str,
) -> RuntimeResult<u32> {
    let code = match family {
        CameraPixelFormatFamily::PackedRgb => 1,
        CameraPixelFormatFamily::PlanarYuv => 2,
        CameraPixelFormatFamily::Encoded => 3,
    };

    if code == 0 {
        return Err(invalid_data(
            operation,
            "camera stream config returned one unknown pixel format family",
        ));
    }

    Ok(code)
}

/// Encode one binding camera stream config for the Android host bridge.
fn encode_camera_stream_config_header(
    config: &CameraStreamConfigValue,
    operation: &'static str,
) -> RuntimeResult<AndroidHostCameraStreamConfigHeader> {
    Ok(AndroidHostCameraStreamConfigHeader {
        width: config.width,
        height: config.height,
        frame_rate_milli_hz: config.frame_rate_milli_hz,
        pixel_format: encode_pixel_format(config.pixel_format.format, operation)?,
        pixel_format_family: encode_pixel_format_family(config.pixel_format.family, operation)?,
        is_compressed: u32::from(config.pixel_format.compressed),
    })
}

/// Open one Android camera stream.
pub(crate) unsafe fn destack_device_camera_stream_open(
    binding: &BindingCallContext,
    out: *mut resource::CameraStreamHandle,
    device: resource::CameraDeviceHandle,
    config: CameraStreamConfig,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // validate config
    let device_resource = camera_device_resource::<AndroidCameraDeviceResource>(
        binding,
        device,
        "destack.device.camera.stream.open",
    )?;
    let config = unsafe { CameraStreamConfig::into_value(config)? };
    if !device_resource
        .configs
        .iter()
        .any(|candidate| candidate == &config)
    {
        return Err(core_platform::invalid_argument(
            "config",
            "camera stream configuration is not supported by this endpoint",
        ));
    }

    // host open
    let runtime_id = host_session_id(binding, "destack.device.camera.stream.open")?;
    let host_config =
        encode_camera_stream_config_header(&config, "destack.device.camera.stream.open")?;
    let mut stream_id = 0u64;
    let status = unsafe {
        destack_host_android_camera_stream_open(
            runtime_id,
            device_resource.session_id,
            host_config,
            &mut stream_id,
        )
    };
    host_status_result(
        status,
        "destack.device.camera.stream.open",
        "camera stream open",
    )?;

    // cache capability
    let capability = device_resource
        .capabilities
        .iter()
        .find(|capability| capability.config == config)
        .cloned();

    // resource store
    let resource = Arc::new(AndroidCameraStreamResource {
        stream_id,
        capability,
        recording: Arc::new(Mutex::new(AndroidCameraRecordingState::default())),
    });
    let entry = ResourceEntry::new(ResourceKind::CameraStream)
        .with_label(CAMERA_STREAM_RESOURCE_LABEL)
        .with_payload(resource)
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            AndroidCameraStreamFinalizer {
                runtime_id,
                stream_id,
            },
        ));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::CameraStreamHandle(resource_id));
    }

    Ok(())
}

/// Close one Android camera stream.
pub(crate) unsafe fn destack_device_camera_stream_close(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    close_camera_stream_resource(binding, handle, "destack.device.camera.stream.close")
}

/// Start one Android camera stream.
pub(crate) unsafe fn destack_device_camera_stream_start(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    // cached resource
    let resource = camera_stream_resource::<AndroidCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.start",
    )?;

    // host start
    let runtime_id = host_session_id(binding, "destack.device.camera.stream.start")?;
    let status =
        unsafe { destack_host_android_camera_stream_start(runtime_id, resource.stream_id) };
    host_status_result(
        status,
        "destack.device.camera.stream.start",
        "camera stream start",
    )
}

/// Stop one Android camera stream.
pub(crate) unsafe fn destack_device_camera_stream_stop(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    // cached resource
    let resource = camera_stream_resource::<AndroidCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.stop",
    )?;

    // host stop
    let runtime_id = host_session_id(binding, "destack.device.camera.stream.stop")?;
    let status = unsafe { destack_host_android_camera_stream_stop(runtime_id, resource.stream_id) };
    host_status_result(
        status,
        "destack.device.camera.stream.stop",
        "camera stream stop",
    )
}

/// Query the current Android camera stream config.
pub(crate) unsafe fn destack_device_camera_stream_config(
    binding: &BindingCallContext,
    out: *mut CameraStreamConfig,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // cached resource
    let resource = camera_stream_resource::<AndroidCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.config",
    )?;

    // host config
    let config = current_camera_stream_config(
        binding,
        resource.stream_id,
        "destack.device.camera.stream.config",
    )?;
    unsafe {
        out.write(CameraStreamConfig::from_value(binding, config));
    }

    Ok(())
}

/// Read one still-photo state snapshot from one opened Android camera stream.
pub(crate) unsafe fn destack_device_camera_stream_photo_state(
    binding: &BindingCallContext,
    out: *mut CameraPhotoState,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // cached resource
    let resource = camera_stream_resource::<AndroidCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.photoState",
    )?;

    // host config
    let config = current_camera_stream_config(
        binding,
        resource.stream_id,
        "destack.device.camera.stream.photoState",
    )?;
    let state = camera_photo_state_from_config(&config);

    unsafe {
        out.write(state);
    }

    Ok(())
}

/// Read one still-photo capability snapshot from one opened Android camera stream.
pub(crate) unsafe fn destack_device_camera_stream_photo_capabilities(
    binding: &BindingCallContext,
    out: *mut CameraPhotoCapabilities,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // cached resource
    let resource = camera_stream_resource::<AndroidCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.photoCapabilities",
    )?;

    // host config
    let config = current_camera_stream_config(
        binding,
        resource.stream_id,
        "destack.device.camera.stream.photoCapabilities",
    )?;
    let capabilities = camera_photo_capabilities_from_config(binding, &config);

    unsafe {
        out.write(capabilities);
    }

    Ok(())
}

/// Read one blocking Android camera frame.
pub(crate) unsafe fn destack_device_camera_stream_read(
    binding: &BindingCallContext,
    out: *mut CameraFrame,
    handle: resource::CameraStreamHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // cached resource
    let resource = camera_stream_resource::<AndroidCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.read",
    )?;

    // host frame
    let frame = read_camera_frame_from_host(
        binding,
        resource.stream_id,
        Some(timeoutns),
        "destack.device.camera.stream.read",
    )?;
    unsafe {
        out.write(camera_frame_from_value(binding, frame));
    }

    Ok(())
}

/// Poll one Android camera frame without blocking.
pub(crate) unsafe fn destack_device_camera_stream_try_read(
    binding: &BindingCallContext,
    out: *mut CameraFrame,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // cached resource
    let resource = camera_stream_resource::<AndroidCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.tryRead",
    )?;

    // host frame
    let frame = read_camera_frame_from_host(
        binding,
        resource.stream_id,
        None,
        "destack.device.camera.stream.tryRead",
    )?;
    unsafe {
        out.write(camera_frame_from_value(binding, frame));
    }

    Ok(())
}

/// Capture one still photo from one running Android camera stream.
pub(crate) unsafe fn destack_device_camera_stream_take_photo(
    binding: &BindingCallContext,
    out: *mut CameraPhoto,
    handle: resource::CameraStreamHandle,
    settings: CameraPhotoSettings,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // cached resource
    let resource = camera_stream_resource::<AndroidCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.takePhoto",
    )?;

    // host config
    let config = current_camera_stream_config(
        binding,
        resource.stream_id,
        "destack.device.camera.stream.takePhoto",
    )?;

    // request validation
    let settings = unsafe { CameraPhotoSettings::into_value(settings)? };
    validate_camera_photo_settings(&settings, &config, "destack.device.camera.stream.takePhoto")?;

    // host frame
    let frame = read_camera_photo_from_host(
        binding,
        resource.stream_id,
        timeoutns,
        "destack.device.camera.stream.takePhoto",
    )?;
    let photo = camera_photo_from_frame_value(binding, frame);

    unsafe {
        out.write(photo);
    }

    Ok(())
}

/// Read one recording-capability snapshot from one opened Android camera stream.
pub(crate) unsafe fn destack_device_camera_stream_recording_capabilities(
    binding: &BindingCallContext,
    out: *mut CameraRecordingCapabilities,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // stream validation
    let resource = camera_stream_resource::<AndroidCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.recordingCapabilities",
    )?;
    let runtime_id = host_session_id(
        binding,
        "destack.device.camera.stream.recordingCapabilities",
    )?;
    let mut header = AndroidHostCameraRecordingCapabilitiesHeader::default();
    let status = unsafe {
        destack_host_android_camera_stream_recording_capabilities(
            runtime_id,
            resource.stream_id,
            &mut header,
        )
    };
    host_status_result(
        status,
        "destack.device.camera.stream.recordingCapabilities",
        "camera stream recording capabilities",
    )?;

    let capabilities = decode_recording_capabilities_header(header);
    let capabilities = camera_recording_capabilities_from_value(binding, &capabilities);

    unsafe {
        out.write(capabilities);
    }

    Ok(())
}

/// Read one recording-state snapshot from one opened Android camera stream.
pub(crate) unsafe fn destack_device_camera_stream_recording_state(
    binding: &BindingCallContext,
    out: *mut CameraRecordingState,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // stream validation
    let resource = camera_stream_resource::<AndroidCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.recordingState",
    )?;
    let state = recording_state_snapshot(&resource);
    let state = camera_recording_state_from_runtime(binding, &state);

    unsafe {
        out.write(state);
    }

    Ok(())
}

/// Start one Android camera recording session.
pub(crate) unsafe fn destack_device_camera_stream_start_recording(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    options: CameraRecordingOptions,
) -> RuntimeResult<()> {
    let resource = camera_stream_resource::<AndroidCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.startRecording",
    )?;

    // duplicate recording
    if resource.recording.lock().runtime.is_active {
        return Err(camera_invalid_state("camera recording is already active"));
    }

    // options validation
    let options = unsafe { CameraRecordingOptions::into_value(options)? };
    let header =
        encode_recording_options_header(&options, "destack.device.camera.stream.startRecording")?;
    let output_path = camera_recording_output_path(
        &options,
        CAMERA_RECORDING_EXTENSION,
        "destack.device.camera.stream.startRecording",
    )?;
    let output_path_string = output_path.to_str().ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "options.output_path",
            "recording output path must be valid utf8 on Android",
        ))
        .boxed()
    })?;
    let runtime_id = host_session_id(binding, "destack.device.camera.stream.startRecording")?;

    // host start
    let status = unsafe {
        destack_host_android_camera_stream_start_recording(
            runtime_id,
            resource.stream_id,
            header,
            NativeStringRef::from(output_path_string),
        )
    };
    host_status_result(
        status,
        "destack.device.camera.stream.startRecording",
        "camera stream recording start",
    )?;

    // local state
    let output_path_value = core_fs::os_path_from_path(binding, &output_path);
    let mut recording = resource.recording.lock();
    recording.runtime.is_active = true;
    recording.runtime.is_paused = false;
    recording.runtime.options = Some(CameraRecordingOptionsValue {
        output_path: Some(unsafe { output_path_value.into_value()? }),
        ..options
    });
    recording.runtime.started_timestamp_ns = Some(core_platform::monotonic_now_ns());
    recording.path = Some(output_path);

    Ok(())
}

/// Pause one active Android camera recording session.
pub(crate) unsafe fn destack_device_camera_stream_pause_recording(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    let resource = camera_stream_resource::<AndroidCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.pauseRecording",
    )?;

    // active recording
    if !resource.recording.lock().runtime.is_active {
        return Err(camera_invalid_state("camera recording is not active"));
    }

    let runtime_id = host_session_id(binding, "destack.device.camera.stream.pauseRecording")?;

    // host pause
    let status = unsafe {
        destack_host_android_camera_stream_pause_recording(runtime_id, resource.stream_id)
    };
    host_status_result(
        status,
        "destack.device.camera.stream.pauseRecording",
        "camera stream recording pause",
    )?;

    // local state
    resource.recording.lock().runtime.is_paused = true;

    Ok(())
}

/// Resume one paused Android camera recording session.
pub(crate) unsafe fn destack_device_camera_stream_resume_recording(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    let resource = camera_stream_resource::<AndroidCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.resumeRecording",
    )?;

    // active recording
    if !resource.recording.lock().runtime.is_active {
        return Err(camera_invalid_state("camera recording is not active"));
    }

    let runtime_id = host_session_id(binding, "destack.device.camera.stream.resumeRecording")?;

    // host resume
    let status = unsafe {
        destack_host_android_camera_stream_resume_recording(runtime_id, resource.stream_id)
    };
    host_status_result(
        status,
        "destack.device.camera.stream.resumeRecording",
        "camera stream recording resume",
    )?;

    // local state
    resource.recording.lock().runtime.is_paused = false;

    Ok(())
}

/// Stop one active Android camera recording session.
pub(crate) unsafe fn destack_device_camera_stream_stop_recording(
    binding: &BindingCallContext,
    out: *mut CameraRecording,
    handle: resource::CameraStreamHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<AndroidCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.stopRecording",
    )?;

    // active recording snapshot
    let (runtime, output_path) = {
        let recording = resource.recording.lock();
        if !recording.runtime.is_active {
            return Err(camera_invalid_state("camera recording is not active"));
        }

        let output_path = recording
            .path
            .clone()
            .ok_or_else(|| camera_invalid_state("camera recording path is missing"))?;

        (recording.runtime.clone(), output_path)
    };

    let config = current_camera_stream_config(
        binding,
        resource.stream_id,
        "destack.device.camera.stream.stopRecording",
    )?;

    let runtime_id = host_session_id(binding, "destack.device.camera.stream.stopRecording")?;

    // host stop
    let status = unsafe {
        destack_host_android_camera_stream_stop_recording(runtime_id, resource.stream_id, timeoutns)
    };
    host_status_result(
        status,
        "destack.device.camera.stream.stopRecording",
        "camera stream recording stop",
    )?;

    // finalized result
    let metadata = std::fs::metadata(&output_path).ok();
    let duration_ns = runtime
        .started_timestamp_ns
        .map(|started| core_platform::monotonic_now_ns().saturating_sub(started));
    let size_bytes = metadata.map(|metadata| metadata.len());
    let recording = camera_recording_from_path(
        binding,
        &output_path,
        runtime
            .options
            .as_ref()
            .and_then(|options| options.container)
            .unwrap_or(CameraRecordingContainer::Mp4),
        runtime
            .options
            .as_ref()
            .and_then(|options| options.video_codec)
            .or(Some(CameraVideoCodec::H264)),
        runtime
            .options
            .as_ref()
            .and_then(|options| options.audio_codec),
        &config,
        duration_ns,
        size_bytes,
    );

    // reset local state
    let mut resource_recording = resource.recording.lock();
    resource_recording.runtime = CameraRecordingRuntimeState::default();
    resource_recording.path = None;

    unsafe {
        out.write(recording);
    }

    Ok(())
}
