#![allow(unsafe_op_in_unsafe_fn)]

use super::core::*;
use super::metadata::*;
use std::panic::AssertUnwindSafe;

/// Return the supported macOS recording capability descriptor.
fn macos_recording_capabilities_value() -> CameraRecordingCapabilitiesValue {
    CameraRecordingCapabilitiesValue {
        containers: vec![CameraRecordingContainer::Mov],
        video_codecs: vec![CameraVideoCodec::H264],
        audio_supported: false,
        audio_codecs: None,
        pause_supported: true,
        maximum_video_bit_rate: None,
        maximum_audio_bit_rate: None,
    }
}

/// Validate one recording request against the current macOS implementation.
fn validate_macos_recording_options(
    options: &CameraRecordingOptionsValue,
    operation: &'static str,
) -> RuntimeResult<()> {
    // container selection
    if let Some(container) = options.container
        && container != CameraRecordingContainer::Mov
    {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.container",
            "macOS camera recording currently supports only mov output",
        ))
        .boxed());
    }

    // codec selection
    if let Some(video_codec) = options.video_codec
        && video_codec != CameraVideoCodec::H264
    {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.video_codec",
            "macOS camera recording currently supports only h264 video",
        ))
        .boxed());
    }

    // unsupported audio
    if options.audio_enabled.unwrap_or(false) || options.audio_codec.is_some() {
        return Err(camera_not_supported(operation));
    }

    // unsupported encoding knobs
    if options.video_bit_rate.is_some()
        || options.audio_bit_rate.is_some()
        || options.key_frame_interval_frames.is_some()
    {
        return Err(camera_not_supported(operation));
    }

    Ok(())
}

/// Wait for one recording to finish or time out.
fn wait_for_recording_finish(
    recording: &MacosCameraRecordingSharedState,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<Option<String>> {
    let timeout = std::time::Duration::from_nanos(timeout_ns);
    let start = std::time::Instant::now();
    let mut state = recording.state.lock();

    // finish wait
    while state.runtime.is_active {
        let elapsed = start.elapsed();
        let remaining = match timeout.checked_sub(elapsed) {
            Some(remaining) => remaining,
            None => {
                return Err(core_platform::io_would_block(
                    operation,
                    "camera recording stop timed out",
                ));
            }
        };

        if recording
            .finished
            .wait_for(&mut state, remaining)
            .timed_out()
        {
            return Err(core_platform::io_would_block(
                operation,
                "camera recording stop timed out",
            ));
        }
    }

    Ok(state.finish_error.take())
}

/// Wait for one still-photo capture to finish or time out.
fn wait_for_photo_capture(
    photo: &MacosCameraPhotoCaptureSharedState,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<CameraPhotoValue> {
    let timeout = std::time::Duration::from_nanos(timeout_ns);
    let start = std::time::Instant::now();
    let mut state = photo.state.lock();

    // photo wait
    while state.is_pending {
        let elapsed = start.elapsed();
        let remaining = match timeout.checked_sub(elapsed) {
            Some(remaining) => remaining,
            None => {
                state.is_pending = false;
                state.result = None;
                return Err(core_platform::io_would_block(
                    operation,
                    "camera photo capture timed out",
                ));
            }
        };

        if photo.finished.wait_for(&mut state, remaining).timed_out() {
            state.is_pending = false;
            state.result = None;
            return Err(core_platform::io_would_block(
                operation,
                "camera photo capture timed out",
            ));
        }
    }

    match state.result.take() {
        Some(Ok(photo)) => Ok(photo),
        Some(Err(message)) => Err(core_platform::io_operation_error(operation, None, message)),
        None => Err(core_platform::io_operation_error(
            operation,
            None,
            "camera photo capture did not return one result",
        )),
    }
}

/// Catch one Objective-C exception from the recording path and convert it to one runtime error.
fn catch_recording_exception<R>(
    operation: &'static str,
    action_name: &'static str,
    callback: impl FnOnce() -> R,
) -> RuntimeResult<R> {
    exception::catch(AssertUnwindSafe(callback)).map_err(|exception| {
        let message = exception
            .map(|exception| exception.to_string())
            .unwrap_or_else(|| String::from("unknown Objective-C exception"));

        core_platform::io_operation_error(
            operation,
            None,
            format!("{action_name} raised Objective-C exception: {message}"),
        )
    })
}

/// Catch one Objective-C exception from the still-photo path and convert it to one runtime error.
fn catch_photo_exception<R>(
    operation: &'static str,
    action_name: &'static str,
    callback: impl FnOnce() -> R,
) -> RuntimeResult<R> {
    exception::catch(AssertUnwindSafe(callback)).map_err(|exception| {
        let message = exception
            .map(|exception| exception.to_string())
            .unwrap_or_else(|| String::from("unknown Objective-C exception"));

        core_platform::io_operation_error(
            operation,
            None,
            format!("{action_name} raised Objective-C exception: {message}"),
        )
    })
}

/// Open one macOS camera stream.
pub(crate) unsafe fn destack_device_camera_stream_open(
    binding: &BindingCallContext,
    out: *mut resource::CameraStreamHandle,
    device: resource::CameraDeviceHandle,
    config: CameraStreamConfig,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected device and config
    let device_resource = camera_device_resource::<MacosCameraDeviceResource>(
        binding,
        device,
        "destack.device.camera.stream.open",
    )?;
    let selected_config = unsafe { CameraStreamConfig::into_value(config)? };
    if !device_resource
        .info
        .configs
        .iter()
        .any(|candidate| candidate == &selected_config)
    {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "config",
            "camera stream configuration is not supported by this endpoint",
        ))
        .boxed());
    }

    // resolve the live AVFoundation objects
    let descriptor_id = camera_descriptor_id(&device_resource.info.unique_id);
    let device = camera_device_by_id(&descriptor_id, "destack.device.camera.stream.open")?;
    let format = camera_format_for_config(&device, &selected_config).ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "config",
            "camera stream configuration did not match one AVFoundation format",
        ))
        .boxed()
    })?;
    configure_camera_device(
        &device,
        &format,
        &selected_config,
        "destack.device.camera.stream.open",
    )?;

    let input =
        unsafe { AVCaptureDeviceInput::deviceInputWithDevice_error(&device) }.map_err(|error| {
            macos_camera_nserror("destack.device.camera.stream.open", "deviceInput", &error)
        })?;
    let video_output = unsafe { AVCaptureVideoDataOutput::new() };
    let movie_output = unsafe { AVCaptureMovieFileOutput::new() };
    let photo_output = unsafe { AVCapturePhotoOutput::new() };
    let session = unsafe { AVCaptureSession::new() };
    let media_type = unsafe { AVMediaTypeVideo }
        .ok_or_else(|| core_platform::not_supported("destack.device.camera.stream.open"))?;

    // configure one video output path
    let output_settings = camera_output_settings();
    unsafe {
        video_output.setAlwaysDiscardsLateVideoFrames(true);
        video_output.setVideoSettings(Some(&output_settings));
    }

    // frame delegate
    let queue = Arc::new(BoundedQueue::new(CAMERA_FRAME_QUEUE_CAPACITY));
    let state = Arc::new(Mutex::new(CameraStreamState {
        is_started: false,
        next_sequence: 0,
    }));
    let callback_queue = camera_callback_queue();
    let frame_delegate =
        MacosCameraFrameDelegate::new(queue.clone(), state.clone(), selected_config);
    let recording = Arc::new(MacosCameraRecordingSharedState {
        state: Mutex::new(MacosCameraRecordingState {
            runtime: CameraRecordingRuntimeState::default(),
            path: None,
            finish_error: None,
        }),
        finished: Condvar::new(),
    });
    let recording_delegate = MacosCameraRecordingDelegate::new(recording.clone());
    let photo = Arc::new(MacosCameraPhotoCaptureSharedState {
        state: Mutex::new(MacosCameraPhotoCaptureState {
            is_pending: false,
            result: None,
        }),
        finished: Condvar::new(),
    });
    let photo_delegate = MacosCameraPhotoDelegate::new(photo.clone());

    unsafe {
        video_output.setSampleBufferDelegate_queue(
            Some(frame_delegate.as_protocol()),
            Some(&callback_queue),
        );
    }

    // connect the session graph
    unsafe {
        session.beginConfiguration();
        if !session.canAddInput(&input) {
            session.commitConfiguration();
            return Err(macos_camera_error(
                "destack.device.camera.stream.open",
                "AVCaptureSession::canAddInput",
                "camera input is not compatible with this session",
            ));
        }
        if !session.canAddOutput(&video_output) {
            session.commitConfiguration();
            return Err(macos_camera_error(
                "destack.device.camera.stream.open",
                "AVCaptureSession::canAddOutput",
                "camera output is not compatible with this session",
            ));
        }
        if !session.canAddOutput(&movie_output) {
            session.commitConfiguration();
            return Err(macos_camera_error(
                "destack.device.camera.stream.open",
                "AVCaptureSession::canAddOutput",
                "camera movie output is not compatible with this session",
            ));
        }
        if !session.canAddOutput(&photo_output) {
            session.commitConfiguration();
            return Err(macos_camera_error(
                "destack.device.camera.stream.open",
                "AVCaptureSession::canAddOutput",
                "camera photo output is not compatible with this session",
            ));
        }

        session.addInput(&input);
        session.addOutput(&video_output);
        session.addOutput(&movie_output);
        session.addOutput(&photo_output);
        session.commitConfiguration();
    }

    // resolve the live video connection
    let connection =
        unsafe { video_output.connectionWithMediaType(media_type) }.ok_or_else(|| {
            macos_camera_error(
                "destack.device.camera.stream.open",
                "AVCaptureVideoDataOutput::connectionWithMediaType",
                "video output did not expose a video connection",
            )
        })?;

    // store the stream resource
    let device = unsafe { core_platform::DispatchBound::new(device) };
    let session = unsafe { core_platform::DispatchBound::new(session) };
    let connection = unsafe { core_platform::DispatchBound::new(connection) };
    let input = unsafe { core_platform::DispatchBound::new(input) };
    let video_output = unsafe { core_platform::DispatchBound::new(video_output) };
    let movie_output = unsafe { core_platform::DispatchBound::new(movie_output) };
    let photo_output = unsafe { core_platform::DispatchBound::new(photo_output) };
    let frame_delegate = unsafe { core_platform::DispatchBound::new(frame_delegate) };
    let recording_delegate = unsafe { core_platform::DispatchBound::new(recording_delegate) };
    let photo_delegate = unsafe { core_platform::DispatchBound::new(photo_delegate) };
    let resource = Arc::new(MacosCameraStreamResource {
        config: selected_config,
        device: device.clone_on(callback_queue.as_ref()),
        session: session.clone_on(callback_queue.as_ref()),
        connection: connection.clone_on(callback_queue.as_ref()),
        movie_output: movie_output.clone_on(callback_queue.as_ref()),
        photo_output: photo_output.clone_on(callback_queue.as_ref()),
        recording_delegate: recording_delegate.clone_on(callback_queue.as_ref()),
        photo_delegate: photo_delegate.clone_on(callback_queue.as_ref()),
        callback_queue: callback_queue.clone(),
        queue: queue.clone(),
        state: state.clone(),
        recording: recording.clone(),
        photo: photo.clone(),
    });
    let entry = ResourceEntry::new(ResourceKind::CameraStream)
        .with_label(CAMERA_STREAM_RESOURCE_LABEL)
        .with_payload(resource)
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            MacosCameraStreamFinalizer {
                session,
                input,
                video_output,
                movie_output,
                photo_output,
                _frame_delegate: frame_delegate,
                _recording_delegate: recording_delegate,
                _photo_delegate: photo_delegate,
                callback_queue,
                queue,
                recording,
                photo,
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

/// Close one macOS camera stream.
pub(crate) unsafe fn destack_device_camera_stream_close(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    close_camera_stream_resource(binding, handle, "destack.device.camera.stream.close")
}

/// Start one macOS camera stream.
pub(crate) unsafe fn destack_device_camera_stream_start(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    // resolve the selected stream
    let resource = camera_stream_resource::<MacosCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.start",
    )?;

    // avoid duplicate starts
    let mut state = resource.state.lock();
    if state.is_started {
        return Ok(());
    }

    // start the session on the callback queue
    resource
        .session
        .dispatch_on(resource.callback_queue.as_ref(), |session| unsafe {
            if !session.isRunning() {
                session.startRunning();
            }
        });
    state.is_started = true;

    Ok(())
}

/// Stop one macOS camera stream.
pub(crate) unsafe fn destack_device_camera_stream_stop(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    // resolve the selected stream
    let resource = camera_stream_resource::<MacosCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.stop",
    )?;

    // avoid duplicate stops
    let mut state = resource.state.lock();
    if !state.is_started {
        return Ok(());
    }

    // stop the session on the callback queue
    resource
        .session
        .dispatch_on(resource.callback_queue.as_ref(), |session| unsafe {
            if session.isRunning() {
                session.stopRunning();
            }
        });
    state.is_started = false;

    Ok(())
}

/// Query one macOS camera stream's active configuration.
pub(crate) unsafe fn destack_device_camera_stream_config(
    binding: &BindingCallContext,
    out: *mut CameraStreamConfig,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected stream
    let resource = camera_stream_resource::<MacosCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.config",
    )?;

    // return the selected config snapshot
    unsafe {
        out.write(CameraStreamConfig::from_value(binding, resource.config));
    }

    Ok(())
}

/// Read one still-photo state snapshot from one opened macOS camera stream.
pub(crate) unsafe fn destack_device_camera_stream_photo_state(
    binding: &BindingCallContext,
    out: *mut CameraPhotoState,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected stream
    let resource = camera_stream_resource::<MacosCameraStreamResource>(
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

/// Read one still-photo capability snapshot from one opened macOS camera stream.
pub(crate) unsafe fn destack_device_camera_stream_photo_capabilities(
    binding: &BindingCallContext,
    out: *mut CameraPhotoCapabilities,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected stream
    let resource = camera_stream_resource::<MacosCameraStreamResource>(
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

/// Read one macOS camera frame.
pub(crate) unsafe fn destack_device_camera_stream_read(
    binding: &BindingCallContext,
    out: *mut CameraFrame,
    handle: resource::CameraStreamHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected stream
    let resource = camera_stream_resource::<MacosCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.read",
    )?;
    let frame = read_camera_frame(&resource, timeoutns, "destack.device.camera.stream.read")?;

    unsafe {
        out.write(camera_frame_from_value(binding, frame));
    }

    Ok(())
}

/// Poll one macOS camera frame without blocking.
pub(crate) unsafe fn destack_device_camera_stream_try_read(
    binding: &BindingCallContext,
    out: *mut CameraFrame,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected stream
    let resource = camera_stream_resource::<MacosCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.tryRead",
    )?;
    let frame = try_read_camera_frame(&resource, "destack.device.camera.stream.tryRead")?;

    unsafe {
        out.write(camera_frame_from_value(binding, frame));
    }

    Ok(())
}

/// Capture one still photo from one running macOS camera stream.
pub(crate) unsafe fn destack_device_camera_stream_take_photo(
    binding: &BindingCallContext,
    out: *mut CameraPhoto,
    handle: resource::CameraStreamHandle,
    settings: CameraPhotoSettings,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected stream
    let resource = camera_stream_resource::<MacosCameraStreamResource>(
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

    // started guard
    if !resource.state.lock().is_started {
        return Err(core_platform::io_would_block(
            "destack.device.camera.stream.takePhoto",
            "camera stream is not started",
        ));
    }

    // reset one pending photo capture slot
    {
        let mut state = resource.photo.state.lock();
        if state.is_pending {
            return Err(camera_invalid_state(
                "camera photo capture is already active",
            ));
        }

        state.is_pending = true;
        state.result = None;
    }

    // dispatch one real photo capture through AVCapturePhotoOutput
    resource
        .photo_output
        .dispatch_on(resource.callback_queue.as_ref(), |photo_output| {
            catch_photo_exception(
                "destack.device.camera.stream.takePhoto",
                "AVCapturePhotoOutput::capturePhotoWithSettings:delegate:",
                || unsafe {
                    let output_settings = camera_output_settings();
                    let settings =
                        AVCapturePhotoSettings::photoSettingsWithFormat(Some(&output_settings));
                    photo_output.capturePhotoWithSettings_delegate(
                        &settings,
                        resource.photo_delegate.get_unchecked().as_protocol(),
                    );
                },
            )
        })?;

    let photo = wait_for_photo_capture(
        resource.photo.as_ref(),
        timeoutns,
        "destack.device.camera.stream.takePhoto",
    )?;
    let photo = CameraPhoto::from_value(binding, photo);

    unsafe {
        out.write(photo);
    }

    Ok(())
}

/// Read one recording-capability snapshot from one opened macOS camera stream.
pub(crate) unsafe fn destack_device_camera_stream_recording_capabilities(
    binding: &BindingCallContext,
    out: *mut CameraRecordingCapabilities,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected stream
    let _resource = camera_stream_resource::<MacosCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.recordingCapabilities",
    )?;
    let capabilities = macos_recording_capabilities_value();
    let capabilities = camera_recording_capabilities_from_value(binding, &capabilities);

    unsafe {
        out.write(capabilities);
    }

    Ok(())
}

/// Read one recording-state snapshot from one opened macOS camera stream.
pub(crate) unsafe fn destack_device_camera_stream_recording_state(
    binding: &BindingCallContext,
    out: *mut CameraRecordingState,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected stream
    let resource = camera_stream_resource::<MacosCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.recordingState",
    )?;
    let state = resource.recording.state.lock();
    let state = camera_recording_state_from_runtime(binding, &state.runtime);

    unsafe {
        out.write(state);
    }

    Ok(())
}

/// Start one macOS camera recording.
pub(crate) unsafe fn destack_device_camera_stream_start_recording(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    options: CameraRecordingOptions,
) -> RuntimeResult<()> {
    // resolve the selected stream
    let resource = camera_stream_resource::<MacosCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.startRecording",
    )?;
    let options = unsafe { CameraRecordingOptions::into_value(options)? };

    // running stream
    if !resource.state.lock().is_started {
        return Err(camera_invalid_state("camera stream is not started"));
    }

    // request validation
    validate_macos_recording_options(&options, "destack.device.camera.stream.startRecording")?;

    // duplicate recording
    if resource.recording.state.lock().runtime.is_active {
        return Err(camera_invalid_state("camera recording is already active"));
    }

    // output configuration
    let output_path = camera_recording_output_path(
        &options,
        "mov",
        "destack.device.camera.stream.startRecording",
    )?;
    let output_url = NSURL::from_file_path(&output_path).ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "options.output_path",
            "recording output path is not one valid file path",
        ))
        .boxed()
    })?;
    // movie limits
    let maximum_duration = options.maximum_duration_ns.map(|duration_ns| {
        let duration = duration_ns.min(i64::MAX as u64) as i64;

        CMTime {
            value: duration,
            timescale: CAMERA_FRAME_TIME_SCALE,
            flags: objc2_core_media::CMTimeFlags(1),
            epoch: 0,
        }
    });
    let maximum_bytes = options
        .maximum_bytes
        .map(|value| value.min(i64::MAX as u64) as i64);
    let recording_delegate = resource
        .recording_delegate
        .clone_on(resource.callback_queue.as_ref());

    // recording start
    resource
        .movie_output
        .dispatch_on(resource.callback_queue.as_ref(), |movie_output| {
            catch_recording_exception(
                "destack.device.camera.stream.startRecording",
                "AVCaptureMovieFileOutput::startRecording",
                || unsafe {
                    let recording_delegate = recording_delegate.get_unchecked();

                    if let Some(maximum_duration) = maximum_duration {
                        movie_output.setMaxRecordedDuration(maximum_duration);
                    } else {
                        movie_output.setMaxRecordedDuration(kCMTimeInvalid);
                    }
                    movie_output.setMaxRecordedFileSize(maximum_bytes.unwrap_or(0));
                    movie_output.startRecordingToOutputFileURL_recordingDelegate(
                        &output_url,
                        recording_delegate.as_protocol(),
                    );
                },
            )
        })?;

    // state update
    let mut state = resource.recording.state.lock();
    state.runtime.is_active = true;
    state.runtime.is_paused = false;
    state.runtime.options = Some(options);
    state.runtime.started_timestamp_ns = Some(core_platform::monotonic_now_ns());
    state.path = Some(output_path);
    state.finish_error = None;

    Ok(())
}

/// Pause one active macOS camera recording.
pub(crate) unsafe fn destack_device_camera_stream_pause_recording(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    // resolve the selected stream
    let resource = camera_stream_resource::<MacosCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.pauseRecording",
    )?;

    // active recording
    {
        let state = resource.recording.state.lock();
        if !state.runtime.is_active {
            return Err(camera_invalid_state("camera recording is not active"));
        }
    }

    // pause request
    resource
        .movie_output
        .dispatch_on(resource.callback_queue.as_ref(), |movie_output| {
            catch_recording_exception(
                "destack.device.camera.stream.pauseRecording",
                "AVCaptureMovieFileOutput::pauseRecording",
                || unsafe {
                    movie_output.pauseRecording();
                },
            )
        })?;

    // state update
    let mut state = resource.recording.state.lock();
    state.runtime.is_paused = true;

    Ok(())
}

/// Resume one paused macOS camera recording.
pub(crate) unsafe fn destack_device_camera_stream_resume_recording(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    // resolve the selected stream
    let resource = camera_stream_resource::<MacosCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.resumeRecording",
    )?;

    // active recording
    {
        let state = resource.recording.state.lock();
        if !state.runtime.is_active {
            return Err(camera_invalid_state("camera recording is not active"));
        }
    }

    // resume request
    resource
        .movie_output
        .dispatch_on(resource.callback_queue.as_ref(), |movie_output| {
            catch_recording_exception(
                "destack.device.camera.stream.resumeRecording",
                "AVCaptureMovieFileOutput::resumeRecording",
                || unsafe {
                    movie_output.resumeRecording();
                },
            )
        })?;

    // state update
    let mut state = resource.recording.state.lock();
    state.runtime.is_paused = false;

    Ok(())
}

/// Stop one active macOS camera recording and return its finalized descriptor.
pub(crate) unsafe fn destack_device_camera_stream_stop_recording(
    binding: &BindingCallContext,
    out: *mut CameraRecording,
    handle: resource::CameraStreamHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected stream
    let resource = camera_stream_resource::<MacosCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.stopRecording",
    )?;

    // recording snapshot
    let (output_path, started_timestamp_ns): (PathBuf, Option<u64>) = {
        let state = resource.recording.state.lock();
        if !state.runtime.is_active {
            return Err(camera_invalid_state("camera recording is not active"));
        }

        let output_path = state
            .path
            .clone()
            .ok_or_else(|| camera_invalid_state("camera recording path is missing"))?;

        (output_path, state.runtime.started_timestamp_ns)
    };

    // stop request
    resource
        .movie_output
        .dispatch_on(resource.callback_queue.as_ref(), |movie_output| {
            catch_recording_exception(
                "destack.device.camera.stream.stopRecording",
                "AVCaptureMovieFileOutput::stopRecording",
                || unsafe {
                    if movie_output.isRecording() {
                        movie_output.stopRecording();
                    }
                },
            )
        })?;
    let finish_error = wait_for_recording_finish(
        resource.recording.as_ref(),
        timeoutns,
        "destack.device.camera.stream.stopRecording",
    )?;

    // finish failure
    if let Some(error) = finish_error {
        return Err(core_platform::io_operation_error(
            "destack.device.camera.stream.stopRecording",
            None,
            format!("camera recording failed: {error}"),
        ));
    }

    // result payload
    let duration_ns: Option<u64> = started_timestamp_ns.map(|started_timestamp_ns| {
        core_platform::monotonic_now_ns().saturating_sub(started_timestamp_ns)
    });
    let size_bytes = std::fs::metadata(&output_path)
        .ok()
        .map(|metadata| metadata.len());
    let recording = camera_recording_from_path(
        binding,
        &output_path,
        CameraRecordingContainer::Mov,
        Some(CameraVideoCodec::H264),
        None,
        &resource.config,
        duration_ns,
        size_bytes,
    );

    unsafe {
        out.write(recording);
    }

    Ok(())
}
