use super::core::*;
use super::device::*;
use super::recording::*;

/// Open camera stream.
pub(crate) unsafe fn destack_device_camera_stream_open(
    binding: &BindingCallContext,
    out: *mut resource::CameraStreamHandle,
    device: resource::CameraDeviceHandle,
    config: CameraStreamConfig,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // device lookup
    let device_resource = camera_device_resource::<LinuxCameraDeviceResource>(
        binding,
        device,
        "destack.device.camera.stream.open",
    )?;
    let selected_config = unsafe { CameraStreamConfig::into_value(config)? };

    // config validation
    if !device_resource
        .info
        .configs
        .iter()
        .any(|value| value == &selected_config)
    {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "config",
            "camera stream configuration is not supported by this endpoint",
        ))
        .boxed());
    }

    // device open
    let descriptor = open_camera_descriptor(
        &device_resource.info.path,
        libc::O_RDWR | libc::O_NONBLOCK,
        "destack.device.camera.stream.open",
    )?;

    // format negotiation
    let mut format = unsafe { std::mem::zeroed::<v4l2::v4l2_format>() };
    format.type_ = V4L2_BUF_TYPE_VIDEO_CAPTURE;
    format.fmt.pix.width = selected_config.width;
    format.fmt.pix.height = selected_config.height;
    format.fmt.pix.pixelformat = v4l2_fourcc_from_config(&selected_config)?;
    format.fmt.pix.field = V4L2_FIELD_NONE;
    if let Err(error) = xioctl(
        descriptor,
        VIDIOC_S_FMT as libc::c_ulong,
        &mut format,
        "destack.device.camera.stream.open",
        "VIDIOC_S_FMT",
    ) {
        unsafe {
            libc::close(descriptor);
        }
        return Err(error);
    }

    // frame interval request
    let mut stream_parameter = unsafe { std::mem::zeroed::<v4l2::v4l2_streamparm>() };
    stream_parameter.type_ = V4L2_BUF_TYPE_VIDEO_CAPTURE;
    stream_parameter.parm.capture.timeperframe.numerator = 1000;
    stream_parameter.parm.capture.timeperframe.denominator = selected_config.frame_rate_milli_hz;
    let _ = xioctl(
        descriptor,
        VIDIOC_S_PARM as libc::c_ulong,
        &mut stream_parameter,
        "destack.device.camera.stream.open",
        "VIDIOC_S_PARM",
    );

    // host stream mode
    let mode = match prepare_stream_mode(
        descriptor,
        device_resource.info.capability_bits,
        &selected_config,
        &format,
    ) {
        Ok(mode) => mode,
        Err(error) => {
            unsafe {
                libc::close(descriptor);
            }
            return Err(error);
        }
    };

    // resource store
    let resource = Arc::new(LinuxCameraStreamResource {
        descriptor,
        path: device_resource.info.path.clone(),
        capability_bits: device_resource.info.capability_bits,
        config: selected_config,
        mode: mode.clone(),
        state: Mutex::new(CameraStreamState {
            is_started: false,
            next_sequence: 0,
        }),
        recording: Arc::new(Mutex::new(LinuxCameraRecordingState::default())),
    });
    let entry = ResourceEntry::new(ResourceKind::CameraStream)
        .with_label(CAMERA_STREAM_RESOURCE_LABEL)
        .with_payload(Arc::clone(&resource))
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            LinuxCameraStreamFinalizer {
                descriptor,
                mode,
                recording: Arc::clone(&resource.recording),
            },
        ));
    let handle = binding
        .worker()
        .resources
        .insert(binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::CameraStreamHandle(handle));
    }

    Ok(())
}

/// Close camera stream.
pub(crate) unsafe fn destack_device_camera_stream_close(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    close_camera_stream_resource(binding, handle, "destack.device.camera.stream.close")
}

/// Start camera stream.
pub(crate) unsafe fn destack_device_camera_stream_start(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.start",
    )?;

    let mut state = resource.state.lock();
    if state.is_started {
        return Ok(());
    }

    // host start
    start_host_stream(&resource)?;

    // running flag
    state.is_started = true;

    Ok(())
}

/// Stop camera stream.
pub(crate) unsafe fn destack_device_camera_stream_stop(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.stop",
    )?;

    let mut state = resource.state.lock();
    if !state.is_started {
        return Ok(());
    }

    // active recording
    if resource.recording.lock().runtime.is_active {
        return Err(camera_invalid_state("camera recording is still active"));
    }

    // host stop
    stop_host_stream(&resource)?;

    // running flag
    state.is_started = false;

    Ok(())
}

/// Read current camera stream configuration.
pub(crate) unsafe fn destack_device_camera_stream_config(
    binding: &BindingCallContext,
    out: *mut CameraStreamConfig,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.config",
    )?;
    unsafe {
        out.write(CameraStreamConfig::from_value(
            binding,
            resource.config.clone(),
        ));
    }

    Ok(())
}

/// Read one still-photo state snapshot from one opened camera stream.
pub(crate) unsafe fn destack_device_camera_stream_photo_state(
    binding: &BindingCallContext,
    out: *mut CameraPhotoState,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // stream config
    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.photoState",
    )?;
    let state = camera_photo_state_from_config(&resource.config);

    unsafe {
        out.write(state);
    }

    Ok(())
}

/// Read one still-photo capability snapshot from one opened camera stream.
pub(crate) unsafe fn destack_device_camera_stream_photo_capabilities(
    binding: &BindingCallContext,
    out: *mut CameraPhotoCapabilities,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // stream config
    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.photoCapabilities",
    )?;
    let capabilities = camera_photo_capabilities_from_config(binding, &resource.config);

    unsafe {
        out.write(capabilities);
    }

    Ok(())
}

/// Read one camera frame from one running stream.
fn read_camera_frame_value(
    resource: &Arc<LinuxCameraStreamResource>,
    timeout_ns: u64,
    nonblocking: bool,
) -> RuntimeResult<CameraFrameValue> {
    // started guard
    if !resource.state.lock().is_started {
        return Err(core_platform::io_would_block(
            "destack.device.camera.stream.read",
            "camera stream is not started",
        ));
    }

    // wait for frame
    if nonblocking {
        let mut pollfd = libc::pollfd {
            fd: resource.descriptor,
            events: CAMERA_POLL_READ_FLAGS,
            revents: 0,
        };
        let status = unsafe { libc::poll(&mut pollfd, 1, 0) };
        if status <= 0 || (pollfd.revents & libc::POLLIN) == 0 {
            return Err(core_platform::io_would_block(
                "destack.device.camera.stream.tryRead",
                "camera frame is not ready",
            ));
        }
    } else if !wait_for_frame(resource.descriptor, timeout_ns)? {
        return Err(core_platform::io_would_block(
            "destack.device.camera.stream.read",
            "camera frame wait timed out",
        ));
    }

    // host read
    let bytes = read_host_frame_bytes(
        resource.descriptor,
        &resource.mode,
        "destack.device.camera.stream.read",
    )?;

    // sequence
    let mut state = resource.state.lock();
    let sequence = state.next_sequence;
    state.next_sequence = sequence.saturating_add(1);

    // frame value
    Ok(CameraFrameValue {
        timestamp_ns: core_platform::monotonic_now_ns(),
        sequence,
        width: resource.config.width,
        height: resource.config.height,
        pixel_format: resource.config.pixel_format.clone(),
        color_space: CameraColorSpace::Unknown,
        planes: frame_plane_layouts(&resource.config, bytes.len()),
        metadata: empty_camera_metadata(),
        bytes,
    })
}

/// Read one camera frame from one running stream.
fn read_camera_frame(
    binding: &BindingCallContext,
    resource: &Arc<LinuxCameraStreamResource>,
    timeout_ns: u64,
    nonblocking: bool,
) -> RuntimeResult<CameraFrame> {
    let frame = read_camera_frame_value(resource, timeout_ns, nonblocking)?;

    Ok(camera_frame_from_value(binding, frame))
}

/// Release one temporary linux host stream mode after one one-shot capture.
fn release_temporary_stream_mode(mode: &LinuxCameraStreamMode) {
    // mapped buffers
    if let LinuxCameraStreamMode::Mmap { buffers, .. } = mode {
        for buffer in buffers {
            unsafe {
                libc::munmap(buffer.address.cast::<libc::c_void>(), buffer.length);
            }
        }
    }
}

/// Capture one still-photo frame from one dedicated linux host stream.
fn capture_photo_frame_value(
    resource: &Arc<LinuxCameraStreamResource>,
    timeout_ns: u64,
) -> RuntimeResult<CameraFrameValue> {
    // temporary open
    let descriptor = open_camera_descriptor(
        &resource.path,
        libc::O_RDWR | libc::O_NONBLOCK,
        "destack.device.camera.stream.takePhoto",
    )?;

    // always tear down the temporary descriptor before returning
    let result = (|| {
        // format negotiation
        let mut format = unsafe { std::mem::zeroed::<v4l2::v4l2_format>() };
        format.type_ = V4L2_BUF_TYPE_VIDEO_CAPTURE;
        format.fmt.pix.width = resource.config.width;
        format.fmt.pix.height = resource.config.height;
        format.fmt.pix.pixelformat = v4l2_fourcc_from_config(&resource.config)?;
        format.fmt.pix.field = V4L2_FIELD_NONE;
        xioctl(
            descriptor,
            VIDIOC_S_FMT as libc::c_ulong,
            &mut format,
            "destack.device.camera.stream.takePhoto",
            "VIDIOC_S_FMT",
        )?;

        // frame interval request
        let mut stream_parameter = unsafe { std::mem::zeroed::<v4l2::v4l2_streamparm>() };
        stream_parameter.type_ = V4L2_BUF_TYPE_VIDEO_CAPTURE;
        stream_parameter.parm.capture.timeperframe.numerator = 1000;
        stream_parameter.parm.capture.timeperframe.denominator =
            resource.config.frame_rate_milli_hz;
        let _ = xioctl(
            descriptor,
            VIDIOC_S_PARM as libc::c_ulong,
            &mut stream_parameter,
            "destack.device.camera.stream.takePhoto",
            "VIDIOC_S_PARM",
        );

        // temporary stream mode
        let mode = prepare_stream_mode(
            descriptor,
            resource.capability_bits,
            &resource.config,
            &format,
        )?;
        let temporary_stream = LinuxCameraStreamResource {
            descriptor,
            path: resource.path.clone(),
            capability_bits: resource.capability_bits,
            config: resource.config.clone(),
            mode,
            state: Mutex::new(CameraStreamState {
                is_started: true,
                next_sequence: 0,
            }),
            recording: Arc::new(Mutex::new(LinuxCameraRecordingState::default())),
        };

        let mode = temporary_stream.mode.clone();
        let mut is_started = false;

        // capture one still frame
        let result = (|| {
            start_host_stream(&temporary_stream)?;
            is_started = true;
            let bytes = if wait_for_frame(descriptor, timeout_ns)? {
                read_host_frame_bytes(
                    descriptor,
                    &temporary_stream.mode,
                    "destack.device.camera.stream.takePhoto",
                )?
            } else {
                return Err(core_platform::io_would_block(
                    "destack.device.camera.stream.takePhoto",
                    "camera photo capture timed out",
                ));
            };

            stop_host_stream(&temporary_stream)?;
            is_started = false;

            Ok(bytes)
        })();

        // teardown stop
        let stop_result = if is_started {
            stop_host_stream(&temporary_stream)
        } else {
            Ok(())
        };

        release_temporary_stream_mode(&mode);

        let bytes = match (result, stop_result) {
            (Ok(bytes), Ok(())) => bytes,
            (Ok(_), Err(error)) => return Err(error),
            (Err(error), _) => return Err(error),
        };
        let config = temporary_stream.config.clone();

        Ok(CameraFrameValue {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            width: config.width,
            height: config.height,
            pixel_format: config.pixel_format,
            color_space: CameraColorSpace::Unknown,
            dynamic_range: CameraDynamicRange::Standard,
            planes: frame_plane_layouts(&config, bytes.len()),
            metadata: empty_camera_metadata(),
            bytes,
        })
    })();

    // temporary teardown
    unsafe {
        libc::close(descriptor);
    }

    result
}

/// Read camera frame.
pub(crate) unsafe fn destack_device_camera_stream_read(
    binding: &BindingCallContext,
    out: *mut CameraFrame,
    handle: resource::CameraStreamHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.read",
    )?;
    let frame = read_camera_frame(binding, &resource, timeoutns, false)?;
    unsafe {
        out.write(frame);
    }

    Ok(())
}

/// Poll camera frame without blocking.
pub(crate) unsafe fn destack_device_camera_stream_try_read(
    binding: &BindingCallContext,
    out: *mut CameraFrame,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.tryRead",
    )?;
    let frame = read_camera_frame(binding, &resource, 0, true)?;
    unsafe {
        out.write(frame);
    }

    Ok(())
}

/// Capture one still photo from one running camera stream.
pub(crate) unsafe fn destack_device_camera_stream_take_photo(
    binding: &BindingCallContext,
    out: *mut CameraPhoto,
    handle: resource::CameraStreamHandle,
    settings: CameraPhotoSettings,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // stream state
    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.takePhoto",
    )?;

    // request validation
    let settings = unsafe { CameraPhotoSettings::into_value(settings)? };
    validate_camera_photo_settings(
        &settings,
        &resource.config,
        "destack.device.camera.stream.takePhoto",
    )?;

    // dedicated still capture
    let frame = capture_photo_frame_value(&resource, timeoutns)?;
    let photo = camera_photo_from_frame_value(binding, frame);

    unsafe {
        out.write(photo);
    }

    Ok(())
}

/// Read one recording-capability snapshot from one opened camera stream.
pub(crate) unsafe fn destack_device_camera_stream_recording_capabilities(
    binding: &BindingCallContext,
    out: *mut CameraRecordingCapabilities,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // stream validation
    let _resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.recordingCapabilities",
    )?;
    let capabilities = linux_recording_capabilities_value();
    let capabilities = camera_recording_capabilities_from_value(binding, &capabilities);

    unsafe {
        out.write(capabilities);
    }

    Ok(())
}

/// Read one recording-state snapshot from one opened camera stream.
pub(crate) unsafe fn destack_device_camera_stream_recording_state(
    binding: &BindingCallContext,
    out: *mut CameraRecordingState,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // stream validation
    let _resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.recordingState",
    )?;
    let state = _resource.recording.lock().runtime.clone();
    let state = camera_recording_state_from_runtime(binding, &state);

    unsafe {
        out.write(state);
    }

    Ok(())
}

/// Start one Linux camera recording.
pub(crate) unsafe fn destack_device_camera_stream_start_recording(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    options: CameraRecordingOptions,
) -> RuntimeResult<()> {
    // stream lookup
    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.startRecording",
    )?;
    let options = unsafe { CameraRecordingOptions::into_value(options)? };

    // active stream
    if !resource.state.lock().is_started {
        return Err(camera_invalid_state("camera stream is not started"));
    }

    // duplicate recording
    if resource.recording.lock().runtime.is_active {
        return Err(camera_invalid_state("camera recording is already active"));
    }

    // request validation
    validate_linux_recording_options(&options, "destack.device.camera.stream.startRecording")?;

    start_linux_recording_worker(&resource, &options)
}

/// Pause one active Linux camera recording.
pub(crate) unsafe fn destack_device_camera_stream_pause_recording(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    // stream validation
    let _resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.pauseRecording",
    )?;

    Err(shared_camera_not_supported(
        "destack.device.camera.stream.pauseRecording",
    ))
}

/// Resume one paused Linux camera recording.
pub(crate) unsafe fn destack_device_camera_stream_resume_recording(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    // stream validation
    let _resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.resumeRecording",
    )?;

    Err(shared_camera_not_supported(
        "destack.device.camera.stream.resumeRecording",
    ))
}

/// Stop one active Linux camera recording.
pub(crate) unsafe fn destack_device_camera_stream_stop_recording(
    binding: &BindingCallContext,
    out: *mut CameraRecording,
    handle: resource::CameraStreamHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // stream lookup
    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.stopRecording",
    )?;

    // worker stop
    let finish = stop_active_recording_worker(&resource.recording, Some(timeoutns))?;
    let finish = finish.ok_or_else(|| camera_invalid_state("camera recording is not active"))?;

    // output path
    let mut recording_state = resource.recording.lock();
    let output_path = recording_state
        .path
        .clone()
        .ok_or_else(|| camera_invalid_state("camera recording path is missing"))?;

    // output descriptor
    let recording = camera_recording_from_path(
        binding,
        &output_path,
        CameraRecordingContainer::Mp4,
        Some(CameraVideoCodec::H264),
        None,
        &resource.config,
        finish.duration_ns,
        finish.size_bytes,
    );

    // reset state
    recording_state.runtime = CameraRecordingRuntimeState::default();
    recording_state.path = None;

    unsafe {
        out.write(recording);
    }

    Ok(())
}
