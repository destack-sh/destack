use super::core::*;
use super::metadata::*;
use windows_future::{AsyncActionCompletedHandler, AsyncOperationCompletedHandler, AsyncStatus};

/// Return the supported Windows recording capability descriptor.
fn windows_recording_capabilities_value() -> CameraRecordingCapabilitiesValue {
    CameraRecordingCapabilitiesValue {
        containers: vec![CameraRecordingContainer::Mp4],
        video_codecs: vec![CameraVideoCodec::H264],
        audio_supported: false,
        audio_codecs: None,
        pause_supported: true,
        maximum_video_bit_rate: None,
        maximum_audio_bit_rate: None,
    }
}

/// Validate one recording request against the current Windows implementation.
fn validate_windows_recording_options(
    options: &CameraRecordingOptionsValue,
    operation: &'static str,
) -> RuntimeResult<()> {
    // container selection
    if let Some(container) = options.container {
        if container != CameraRecordingContainer::Mp4 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "options.container",
                "Windows camera recording currently supports only mp4 output",
            ))
            .boxed());
        }
    }

    // codec selection
    if let Some(video_codec) = options.video_codec {
        if video_codec != CameraVideoCodec::H264 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "options.video_codec",
                "Windows camera recording currently supports only h264 video",
            ))
            .boxed());
        }
    }

    // unsupported audio
    if options.audio_enabled.unwrap_or(false) || options.audio_codec.is_some() {
        return Err(camera_not_supported(operation));
    }

    // unsupported encoding knobs
    if options.video_bit_rate.is_some()
        || options.audio_bit_rate.is_some()
        || options.key_frame_interval_frames.is_some()
        || options.maximum_duration_ns.is_some()
        || options.maximum_bytes.is_some()
    {
        return Err(camera_not_supported(operation));
    }

    Ok(())
}

/// Create one writable WinRT storage file for recording output.
fn create_recording_file(
    path: &std::path::Path,
    operation: &'static str,
) -> RuntimeResult<StorageFile> {
    let path_string = path.to_string_lossy().to_string();

    // filesystem creation
    std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| {
            core_platform::io_operation_error(
                operation,
                None,
                format!("failed to create recording file at {path_string}: {error}"),
            )
        })?;

    // winrt file binding
    StorageFile::GetFileFromPathAsync(&HSTRING::from(path_string.clone()))
        .map_err(|error| {
            windows_camera_error(operation, "StorageFile::GetFileFromPathAsync", &error)
        })?
        .get()
        .map_err(|error| windows_camera_error(operation, "IAsyncOperation::get", &error))
}

/// Create one Windows recording encoding profile.
fn create_recording_profile(operation: &'static str) -> RuntimeResult<MediaEncodingProfile> {
    MediaEncodingProfile::CreateMp4(VideoEncodingQuality::Auto)
        .map_err(|error| windows_camera_error(operation, "MediaEncodingProfile::CreateMp4", &error))
}

/// Resolve one active Windows recording object.
fn active_recording(
    resource: &WindowsCameraStreamResource,
    operation: &'static str,
) -> RuntimeResult<LowLagMediaRecording> {
    let recording = {
        let state = resource.recording.lock();
        if !state.runtime.is_active {
            return Err(camera_invalid_state("camera recording is not active"));
        }

        state.recording.clone()
    }
    .ok_or_else(|| camera_invalid_state("camera recording is not active"))?;

    recording
        .resolve()
        .map_err(|error| windows_camera_error(operation, "AgileReference::resolve", &error))
}

/// Build one not-supported error for one Windows camera control.
pub(super) fn unsupported_windows_camera_control(operation: &'static str) -> Box<RuntimeError> {
    camera_not_supported(operation)
}

/// Resolve one opened Windows camera stream resource.
pub(super) fn stream_resource(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<WindowsCameraStreamResource>> {
    camera_stream_resource::<WindowsCameraStreamResource>(binding, handle, operation)
}

/// Resolve one WinRT controller from one opened stream.
pub(super) fn video_device_controller(
    resource: &WindowsCameraStreamResource,
    operation: &'static str,
) -> RuntimeResult<VideoDeviceController> {
    let capture = resource
        .capture
        .resolve()
        .map_err(|error| windows_camera_error(operation, "AgileReference::resolve", &error))?;

    capture_video_device_controller(&capture, operation)
}

/// Wait for one async action and map one WinRT failure.
pub(super) fn complete_action(
    action: IAsyncAction,
    operation: &'static str,
    action_name: &str,
) -> RuntimeResult<()> {
    action
        .get()
        .map_err(|error| windows_camera_error(operation, action_name, &error))
}

/// Wait for one frame-reader start result and map one unsuccessful status.
fn start_frame_reader(reader: &MediaFrameReader, operation: &'static str) -> RuntimeResult<()> {
    let status = reader
        .StartAsync()
        .map_err(|error| windows_camera_error(operation, "MediaFrameReader::StartAsync", &error))?
        .get()
        .map_err(|error| windows_camera_error(operation, "IAsyncOperation::get", &error))?;
    if status == MediaFrameReaderStartStatus::Success {
        return Ok(());
    }

    Err(core_platform::io_operation_error(
        operation,
        None,
        format!(
            "camera frame reader failed to start with status {}",
            status.0
        ),
    ))
}

/// Resolve one agile frame reader reference.
fn resolve_reader(
    resource: &WindowsCameraStreamResource,
    operation: &'static str,
) -> RuntimeResult<MediaFrameReader> {
    resource
        .reader
        .resolve()
        .map_err(|error| windows_camera_error(operation, "AgileReference::resolve", &error))
}

/// Read one `MediaDeviceControl` value.
pub(super) fn media_device_control_value(
    control: MediaDeviceControl,
    operation: &'static str,
    action_name: &str,
) -> RuntimeResult<f64> {
    let capabilities = control
        .Capabilities()
        .map_err(|error| windows_camera_error(operation, action_name, &error))?;
    let is_supported = capabilities
        .Supported()
        .map_err(|error| windows_camera_error(operation, action_name, &error))?;
    if !is_supported {
        return Err(unsupported_windows_camera_control(operation));
    }

    let mut value = 0.0f64;
    let has_value = control
        .TryGetValue(&mut value)
        .map_err(|error| windows_camera_error(operation, action_name, &error))?;
    if !has_value {
        return Err(unsupported_windows_camera_control(operation));
    }

    Ok(value)
}

/// Write one `MediaDeviceControl` value.
pub(super) fn set_media_device_control_value(
    control: MediaDeviceControl,
    value: f64,
    operation: &'static str,
    action_name: &str,
) -> RuntimeResult<()> {
    let capabilities = control
        .Capabilities()
        .map_err(|error| windows_camera_error(operation, action_name, &error))?;
    let is_supported = capabilities
        .Supported()
        .map_err(|error| windows_camera_error(operation, action_name, &error))?;
    if !is_supported {
        return Err(unsupported_windows_camera_control(operation));
    }

    let was_set = control
        .TrySetValue(value)
        .map_err(|error| windows_camera_error(operation, action_name, &error))?;
    if !was_set {
        return Err(unsupported_windows_camera_control(operation));
    }

    Ok(())
}

/// Complete one Windows async camera operation within one runtime-aware timeout.
fn complete_async_operation<T>(
    binding: &BindingCallContext,
    async_operation: &windows_future::IAsyncOperation<T>,
    timeout_ns: u64,
    operation: &'static str,
    action_name: &'static str,
    timeout_message: &'static str,
) -> RuntimeResult<T>
where
    T: windows::core::RuntimeType + Clone,
{
    // immediate completion
    let status = async_operation
        .Status()
        .map_err(|error| windows_camera_error(operation, "IAsyncInfo::Status", &error))?;
    if status != AsyncStatus::Started {
        return async_operation
            .GetResults()
            .map_err(|error| windows_camera_error(operation, action_name, &error));
    }

    // completion notification
    let completion = Arc::new((Mutex::new(false), parking_lot::Condvar::new()));
    let completion_signal = Arc::clone(&completion);
    async_operation
        .SetCompleted(&AsyncOperationCompletedHandler::new(
            move |_sender, _status| {
                let (is_completed, finished) = completion_signal.as_ref();
                let mut is_completed = is_completed.lock();
                *is_completed = true;
                finished.notify_all();
                Ok(())
            },
        ))
        .map_err(|error| windows_camera_error(operation, "IAsyncInfo::SetCompleted", &error))?;

    let deadline_ns = binding.mono_nanos().saturating_add(timeout_ns);
    binding.wait_for_binding_result(
        operation,
        timeout_message,
        deadline_ns,
        || {
            let status = async_operation
                .Status()
                .map_err(|error| windows_camera_error(operation, "IAsyncInfo::Status", &error))?;
            if status == AsyncStatus::Started {
                return Ok(None);
            }

            async_operation
                .GetResults()
                .map(Some)
                .map_err(|error| windows_camera_error(operation, action_name, &error))
        },
        |duration| {
            let (is_completed, finished) = completion.as_ref();
            let mut is_completed = is_completed.lock();
            if !*is_completed {
                finished.wait_for(&mut is_completed, duration);
            }
        },
    )
}

/// Complete one Windows async camera action within one runtime-aware timeout.
fn complete_async_action(
    binding: &BindingCallContext,
    async_action: &IAsyncAction,
    timeout_ns: u64,
    operation: &'static str,
    action_name: &'static str,
    timeout_message: &'static str,
) -> RuntimeResult<()> {
    // immediate completion
    let status = async_action
        .Status()
        .map_err(|error| windows_camera_error(operation, "IAsyncInfo::Status", &error))?;
    if status != AsyncStatus::Started {
        return async_action
            .get()
            .map_err(|error| windows_camera_error(operation, action_name, &error));
    }

    // completion notification
    let completion = Arc::new((Mutex::new(false), parking_lot::Condvar::new()));
    let completion_signal = Arc::clone(&completion);
    async_action
        .SetCompleted(&AsyncActionCompletedHandler::new(
            move |_sender, _status| {
                let (is_completed, finished) = completion_signal.as_ref();
                let mut is_completed = is_completed.lock();
                *is_completed = true;
                finished.notify_all();
                Ok(())
            },
        ))
        .map_err(|error| windows_camera_error(operation, "IAsyncInfo::SetCompleted", &error))?;

    let deadline_ns = binding.mono_nanos().saturating_add(timeout_ns);
    binding.wait_for_binding_result(
        operation,
        timeout_message,
        deadline_ns,
        || {
            let status = async_action
                .Status()
                .map_err(|error| windows_camera_error(operation, "IAsyncInfo::Status", &error))?;
            if status == AsyncStatus::Started {
                return Ok(None);
            }

            async_action
                .get()
                .map(Some)
                .map_err(|error| windows_camera_error(operation, action_name, &error))
        },
        |duration| {
            let (is_completed, finished) = completion.as_ref();
            let mut is_completed = is_completed.lock();
            if !*is_completed {
                finished.wait_for(&mut is_completed, duration);
            }
        },
    )
}

/// Return the remaining timeout budget for one Windows camera operation.
fn remaining_timeout_ns(
    binding: &BindingCallContext,
    deadline_ns: u64,
    operation: &'static str,
    timeout_message: &'static str,
) -> RuntimeResult<u64> {
    let now = binding.mono_nanos();
    if now >= deadline_ns {
        return Err(core_platform::io_would_block(operation, timeout_message));
    }

    Ok(deadline_ns.saturating_sub(now))
}

/// Read one `MediaDeviceControl` range.
pub(super) fn media_device_control_range(
    control: MediaDeviceControl,
    operation: &'static str,
    action_name: &str,
) -> RuntimeResult<CameraFloatControlRange> {
    let capabilities = control
        .Capabilities()
        .map_err(|error| windows_camera_error(operation, action_name, &error))?;
    let is_supported = capabilities
        .Supported()
        .map_err(|error| windows_camera_error(operation, action_name, &error))?;
    if !is_supported {
        return Err(unsupported_windows_camera_control(operation));
    }

    Ok(CameraFloatControlRange {
        minimum: capabilities
            .Min()
            .map_err(|error| windows_camera_error(operation, action_name, &error))?,
        maximum: capabilities
            .Max()
            .map_err(|error| windows_camera_error(operation, action_name, &error))?,
        default: capabilities
            .Default()
            .map_err(|error| windows_camera_error(operation, action_name, &error))?,
        step: capabilities
            .Step()
            .map_err(|error| windows_camera_error(operation, action_name, &error))?,
    })
}

/// Convert one WinRT `TimeSpan` into nanoseconds.
pub(super) fn time_span_ns(value: windows::Foundation::TimeSpan) -> u64 {
    value.Duration.max(0) as u64 * 100
}

/// Read one blocking Windows camera frame.
fn read_frame(
    resource: &WindowsCameraStreamResource,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<CameraFrameValue> {
    read_camera_frame_queue(&resource.queue, timeout_ns, operation)
}

/// Read one nonblocking Windows camera frame.
fn try_read_frame(
    resource: &WindowsCameraStreamResource,
    operation: &'static str,
) -> RuntimeResult<CameraFrameValue> {
    try_read_camera_frame_queue(&resource.queue, operation)
}

/// Build one Windows still-photo encoding profile for the selected stream.
fn create_photo_encoding_properties(
    resource: &WindowsCameraStreamResource,
    operation: &'static str,
) -> RuntimeResult<ImageEncodingProperties> {
    let pixel_format = resource.config.pixel_format.format;

    // select the closest still-photo encoding for the current stream shape
    if pixel_format == CameraPixelFormat::Jpeg {
        return ImageEncodingProperties::CreateJpeg().map_err(|error| {
            windows_camera_error(operation, "ImageEncodingProperties::CreateJpeg", &error)
        });
    }

    ImageEncodingProperties::CreateUncompressed(MediaPixelFormat::Bgra8).map_err(|error| {
        windows_camera_error(
            operation,
            "ImageEncodingProperties::CreateUncompressed",
            &error,
        )
    })
}

/// Prepare one low-lag Windows still-photo capture session.
fn prepare_photo_capture(
    binding: &BindingCallContext,
    resource: &WindowsCameraStreamResource,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<LowLagPhotoCapture> {
    let capture = resource
        .capture
        .resolve()
        .map_err(|error| windows_camera_error(operation, "AgileReference::resolve", &error))?;
    let properties = create_photo_encoding_properties(resource, operation)?;

    capture
        .PrepareLowLagPhotoCaptureAsync(&properties)
        .map_err(|error| {
            windows_camera_error(
                operation,
                "MediaCapture::PrepareLowLagPhotoCaptureAsync",
                &error,
            )
        })
        .and_then(|async_operation| {
            complete_async_operation(
                binding,
                &async_operation,
                timeout_ns,
                operation,
                "MediaCapture::PrepareLowLagPhotoCaptureAsync",
                "camera photo capture timed out while preparing still capture",
            )
        })
}

/// Capture one Windows still photo with one dedicated low-lag photo session.
fn capture_photo(
    binding: &BindingCallContext,
    resource: &WindowsCameraStreamResource,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<CameraPhotoValue> {
    let deadline_ns = binding.mono_nanos().saturating_add(timeout_ns);
    let prepare_timeout_ns = remaining_timeout_ns(
        binding,
        deadline_ns,
        operation,
        "camera photo capture timed out while preparing still capture",
    )?;
    let photo_capture = prepare_photo_capture(binding, resource, prepare_timeout_ns, operation)?;
    let capture_result = (|| {
        let capture_timeout_ns = remaining_timeout_ns(
            binding,
            deadline_ns,
            operation,
            "camera photo capture timed out",
        )?;
        let photo = photo_capture
            .CaptureAsync()
            .map_err(|error| {
                windows_camera_error(operation, "LowLagPhotoCapture::CaptureAsync", &error)
            })
            .and_then(|async_operation| {
                complete_async_operation(
                    binding,
                    &async_operation,
                    capture_timeout_ns,
                    operation,
                    "LowLagPhotoCapture::CaptureAsync",
                    "camera photo capture timed out",
                )
            })?;

        photo_value_from_captured_photo(&photo, operation)
    })();
    let finish_result = remaining_timeout_ns(
        binding,
        deadline_ns,
        operation,
        "camera photo capture timed out while finishing still capture",
    )
    .and_then(|finish_timeout_ns| {
        photo_capture
            .FinishAsync()
            .map_err(|error| {
                windows_camera_error(operation, "LowLagPhotoCapture::FinishAsync", &error)
            })
            .and_then(|async_action| {
                complete_async_action(
                    binding,
                    &async_action,
                    finish_timeout_ns,
                    operation,
                    "LowLagPhotoCapture::FinishAsync",
                    "camera photo capture timed out while finishing still capture",
                )
            })
    });

    match (capture_result, finish_result) {
        (Ok(photo), Ok(())) => Ok(photo),
        (Ok(_), Err(error)) => Err(error),
        (Err(error), _) => Err(error),
    }
}

/// Build one white-balance mode from the current WinRT preset.
pub(super) fn white_balance_mode_from_preset(
    preset: ColorTemperaturePreset,
) -> CameraWhiteBalanceMode {
    if preset == ColorTemperaturePreset::Auto {
        return CameraWhiteBalanceMode::Auto;
    }

    CameraWhiteBalanceMode::Manual
}

/// Build one focus mode from the current WinRT focus mode.
pub(super) fn focus_mode_from_windows(mode: WindowsFocusMode) -> CameraFocusMode {
    match mode {
        WindowsFocusMode::Continuous => CameraFocusMode::ContinuousAuto,
        WindowsFocusMode::Manual => CameraFocusMode::Manual,
        _ => CameraFocusMode::Auto,
    }
}

/// Build one stabilization mode from the current WinRT value.
pub(super) fn stabilization_mode_from_windows(
    mode: OpticalImageStabilizationMode,
) -> CameraStabilizationMode {
    match mode {
        OpticalImageStabilizationMode::Off => CameraStabilizationMode::Off,
        _ => CameraStabilizationMode::Standard,
    }
}

/// Open one Windows camera stream.
pub(crate) unsafe fn destack_device_camera_stream_open(
    binding: &BindingCallContext,
    out: *mut resource::CameraStreamHandle,
    device: resource::CameraDeviceHandle,
    config: CameraStreamConfig,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected device and config
    let device_resource = camera_device_resource::<WindowsCameraDeviceResource>(
        binding,
        device,
        "destack.device.camera.stream.open",
    )?;
    let selected_config = unsafe { CameraStreamConfig::into_value(config)? };
    let selection = device_resource
        .info
        .selections
        .iter()
        .find(|selection| selection.config == selected_config)
        .cloned()
        .ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "config",
                "camera stream configuration is not supported by this endpoint",
            ))
            .boxed()
        })?;

    // configure the selected frame source
    let capture = device_resource.capture.resolve().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.open",
            "AgileReference::resolve",
            &error,
        )
    })?;
    let source = capture_source(
        &capture,
        &selection.source_id,
        "destack.device.camera.stream.open",
    )?;
    configure_source_format(&source, &selection, "destack.device.camera.stream.open")?;

    // create the frame reader
    let reader = capture
        .CreateFrameReaderWithSubtypeAsync(&source, &HSTRING::from(WINDOWS_CAMERA_BGRA_SUBTYPE))
        .map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.open",
                "MediaCapture::CreateFrameReaderWithSubtypeAsync",
                &error,
            )
        })?
        .get()
        .map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.open",
                "IAsyncOperation::get",
                &error,
            )
        })?;
    reader
        .SetAcquisitionMode(MediaFrameReaderAcquisitionMode::Realtime)
        .map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.open",
                "MediaFrameReader::SetAcquisitionMode",
                &error,
            )
        })?;

    // wire the frame callback
    let queue = Arc::new(BoundedQueue::new(CAMERA_FRAME_QUEUE_CAPACITY));
    let state = Arc::new(Mutex::new(CameraStreamState {
        is_started: false,
        next_sequence: 0,
    }));
    let callback_queue = queue.clone();
    let callback_state = state.clone();
    let callback_config = selection.config;
    let callback_token = reader
        .FrameArrived(&TypedEventHandler::new(
            move |reader: Ref<'_, MediaFrameReader>, _args| {
                let Some(reader) = reader.as_ref() else {
                    return Ok(());
                };

                let reference = match reader.TryAcquireLatestFrame() {
                    Ok(reference) => reference,
                    Err(_) => return Ok(()),
                };
                let frame = match frame_value_from_reference(
                    &reference,
                    &callback_config,
                    &callback_state,
                    "destack.device.camera.stream.open",
                ) {
                    Ok(frame) => frame,
                    Err(_) => return Ok(()),
                };

                callback_queue.push_drop_oldest(frame);

                Ok(())
            },
        ))
        .map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.open",
                "MediaFrameReader::FrameArrived",
                &error,
            )
        })?;
    let capture_reference = AgileReference::new(&capture).map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.open",
            "AgileReference::new",
            &error,
        )
    })?;
    let reader_reference = AgileReference::new(&reader).map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.open",
            "AgileReference::new",
            &error,
        )
    })?;

    // store the stream resource
    let recording = Arc::new(Mutex::new(WindowsCameraRecordingState {
        runtime: CameraRecordingRuntimeState::default(),
        path: None,
        recording: None,
    }));
    let entry = ResourceEntry::new(ResourceKind::CameraStream)
        .with_label(CAMERA_STREAM_RESOURCE_LABEL)
        .with_payload(Arc::new(WindowsCameraStreamResource {
            capture: capture_reference,
            reader: reader_reference.clone(),
            config: selection.config,
            queue: queue.clone(),
            state: state.clone(),
            recording: recording.clone(),
        }))
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            WindowsCameraStreamFinalizer {
                reader: reader_reference,
                frame_arrived_token: callback_token,
                queue,
                recording,
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

/// Close one Windows camera stream.
pub(crate) unsafe fn destack_device_camera_stream_close(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    close_camera_stream_resource(binding, handle, "destack.device.camera.stream.close")
}

/// Start one Windows camera stream.
pub(crate) unsafe fn destack_device_camera_stream_start(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    // resolve the opened stream
    let resource = stream_resource(binding, handle, "destack.device.camera.stream.start")?;

    // avoid duplicate starts
    let mut state = resource.state.lock();
    if state.is_started {
        return Ok(());
    }

    // start the frame reader
    let reader = resolve_reader(&resource, "destack.device.camera.stream.start")?;
    start_frame_reader(&reader, "destack.device.camera.stream.start")?;
    state.is_started = true;

    Ok(())
}

/// Stop one Windows camera stream.
pub(crate) unsafe fn destack_device_camera_stream_stop(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    // resolve the opened stream
    let resource = stream_resource(binding, handle, "destack.device.camera.stream.stop")?;

    // avoid duplicate stops
    let mut state = resource.state.lock();
    if !state.is_started {
        return Ok(());
    }

    // stop the frame reader
    let reader = resolve_reader(&resource, "destack.device.camera.stream.stop")?;
    complete_action(
        reader.StopAsync().map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.stop",
                "MediaFrameReader::StopAsync",
                &error,
            )
        })?,
        "destack.device.camera.stream.stop",
        "IAsyncAction::get",
    )?;
    state.is_started = false;

    Ok(())
}

/// Read the active Windows camera stream config.
pub(crate) unsafe fn destack_device_camera_stream_config(
    binding: &BindingCallContext,
    out: *mut CameraStreamConfig,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the opened stream
    let resource = stream_resource(binding, handle, "destack.device.camera.stream.config")?;

    // return the selected config snapshot
    unsafe {
        out.write(CameraStreamConfig::from_value(binding, resource.config));
    }

    Ok(())
}

/// Read one still-photo state snapshot from one opened Windows camera stream.
pub(crate) unsafe fn destack_device_camera_stream_photo_state(
    binding: &BindingCallContext,
    out: *mut CameraPhotoState,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the opened stream
    let resource = stream_resource(binding, handle, "destack.device.camera.stream.photoState")?;
    let state = camera_photo_state_from_config(&resource.config);

    unsafe {
        out.write(state);
    }

    Ok(())
}

/// Read one still-photo capability snapshot from one opened Windows camera stream.
pub(crate) unsafe fn destack_device_camera_stream_photo_capabilities(
    binding: &BindingCallContext,
    out: *mut CameraPhotoCapabilities,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the opened stream
    let resource = stream_resource(
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

/// Read one Windows camera frame with one timeout.
pub(crate) unsafe fn destack_device_camera_stream_read(
    binding: &BindingCallContext,
    out: *mut CameraFrame,
    handle: resource::CameraStreamHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the opened stream
    let resource = stream_resource(binding, handle, "destack.device.camera.stream.read")?;
    let frame = read_frame(&resource, timeoutns, "destack.device.camera.stream.read")?;

    unsafe {
        out.write(camera_frame_from_value(binding, frame));
    }

    Ok(())
}

/// Poll one Windows camera frame without blocking.
pub(crate) unsafe fn destack_device_camera_stream_try_read(
    binding: &BindingCallContext,
    out: *mut CameraFrame,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the opened stream
    let resource = stream_resource(binding, handle, "destack.device.camera.stream.tryRead")?;
    let frame = try_read_frame(&resource, "destack.device.camera.stream.tryRead")?;

    unsafe {
        out.write(camera_frame_from_value(binding, frame));
    }

    Ok(())
}

/// Capture one still photo from one running Windows camera stream.
pub(crate) unsafe fn destack_device_camera_stream_take_photo(
    binding: &BindingCallContext,
    out: *mut CameraPhoto,
    handle: resource::CameraStreamHandle,
    settings: CameraPhotoSettings,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the opened stream
    let resource = stream_resource(binding, handle, "destack.device.camera.stream.takePhoto")?;

    // request validation
    let settings = unsafe { CameraPhotoSettings::into_value(settings)? };
    validate_camera_photo_settings(
        &settings,
        &resource.config,
        "destack.device.camera.stream.takePhoto",
    )?;

    // still capture
    let photo = capture_photo(
        binding,
        &resource,
        timeoutns,
        "destack.device.camera.stream.takePhoto",
    )?;
    let photo = CameraPhoto::from_value(binding, photo);

    unsafe {
        out.write(photo);
    }

    Ok(())
}

/// Read one recording-capability snapshot from one opened Windows camera stream.
pub(crate) unsafe fn destack_device_camera_stream_recording_capabilities(
    binding: &BindingCallContext,
    out: *mut CameraRecordingCapabilities,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the opened stream
    let _resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.recordingCapabilities",
    )?;
    let capabilities = windows_recording_capabilities_value();
    let capabilities = camera_recording_capabilities_from_value(binding, &capabilities);

    unsafe {
        out.write(capabilities);
    }

    Ok(())
}

/// Read one recording-state snapshot from one opened Windows camera stream.
pub(crate) unsafe fn destack_device_camera_stream_recording_state(
    binding: &BindingCallContext,
    out: *mut CameraRecordingState,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the opened stream
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.recordingState",
    )?;
    let state = resource.recording.lock();
    let state = camera_recording_state_from_runtime(binding, &state.runtime);

    unsafe {
        out.write(state);
    }

    Ok(())
}

/// Start one Windows camera recording.
pub(crate) unsafe fn destack_device_camera_stream_start_recording(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    options: CameraRecordingOptions,
) -> RuntimeResult<()> {
    // resolve the opened stream
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.startRecording",
    )?;
    let options = unsafe { CameraRecordingOptions::into_value(options)? };

    // stream running
    if !resource.state.lock().is_started {
        return Err(camera_invalid_state("camera stream is not started"));
    }

    // request validation
    validate_windows_recording_options(&options, "destack.device.camera.stream.startRecording")?;

    // duplicate recording
    if resource.recording.lock().runtime.is_active {
        return Err(camera_invalid_state("camera recording is already active"));
    }

    // output sink
    let output_path = camera_recording_output_path(
        &options,
        "mp4",
        "destack.device.camera.stream.startRecording",
    )?;
    let output_file =
        create_recording_file(&output_path, "destack.device.camera.stream.startRecording")?;

    // recording object
    let capture = resource.capture.resolve().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.startRecording",
            "AgileReference::resolve",
            &error,
        )
    })?;
    let profile = create_recording_profile("destack.device.camera.stream.startRecording")?;
    let recording = capture
        .PrepareLowLagRecordToStorageFileAsync(&profile, &output_file)
        .map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.startRecording",
                "MediaCapture::PrepareLowLagRecordToStorageFileAsync",
                &error,
            )
        })?
        .get()
        .map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.startRecording",
                "IAsyncOperation::get",
                &error,
            )
        })?;
    complete_action(
        recording.StartAsync().map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.startRecording",
                "LowLagMediaRecording::StartAsync",
                &error,
            )
        })?,
        "destack.device.camera.stream.startRecording",
        "IAsyncAction::get",
    )?;
    let recording = AgileReference::new(&recording).map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.startRecording",
            "AgileReference::new",
            &error,
        )
    })?;

    // state update
    let mut recording_state = resource.recording.lock();
    recording_state.runtime.is_active = true;
    recording_state.runtime.is_paused = false;
    recording_state.runtime.options = Some(options);
    recording_state.runtime.started_timestamp_ns = Some(core_platform::monotonic_now_ns());
    recording_state.path = Some(output_path);
    recording_state.recording = Some(recording);

    Ok(())
}

/// Pause one active Windows camera recording.
pub(crate) unsafe fn destack_device_camera_stream_pause_recording(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    // resolve the opened stream
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.pauseRecording",
    )?;
    let recording = active_recording(&resource, "destack.device.camera.stream.pauseRecording")?;

    // pause recording
    complete_action(
        recording
            .PauseAsync(MediaCapturePauseBehavior::RetainHardwareResources)
            .map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.pauseRecording",
                    "LowLagMediaRecording::PauseAsync",
                    &error,
                )
            })?,
        "destack.device.camera.stream.pauseRecording",
        "IAsyncAction::get",
    )?;

    // state update
    let mut state = resource.recording.lock();
    state.runtime.is_paused = true;

    Ok(())
}

/// Resume one paused Windows camera recording.
pub(crate) unsafe fn destack_device_camera_stream_resume_recording(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    // resolve the opened stream
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.resumeRecording",
    )?;
    let recording = active_recording(&resource, "destack.device.camera.stream.resumeRecording")?;

    // resume recording
    complete_action(
        recording.ResumeAsync().map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.resumeRecording",
                "LowLagMediaRecording::ResumeAsync",
                &error,
            )
        })?,
        "destack.device.camera.stream.resumeRecording",
        "IAsyncAction::get",
    )?;

    // state update
    let mut state = resource.recording.lock();
    state.runtime.is_paused = false;

    Ok(())
}

/// Stop one active Windows camera recording and return its finalized descriptor.
pub(crate) unsafe fn destack_device_camera_stream_stop_recording(
    binding: &BindingCallContext,
    out: *mut CameraRecording,
    handle: resource::CameraStreamHandle,
    _timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the opened stream
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.stopRecording",
    )?;

    // active recording snapshot
    let (recording, output_path) = {
        let state = resource.recording.lock();
        if !state.runtime.is_active {
            return Err(camera_invalid_state("camera recording is not active"));
        }

        let recording = state
            .recording
            .clone()
            .ok_or_else(|| camera_invalid_state("camera recording is not active"))?;
        let output_path = state
            .path
            .clone()
            .ok_or_else(|| camera_invalid_state("camera recording path is missing"))?;

        (recording, output_path)
    };
    let recording = recording.resolve().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.stopRecording",
            "AgileReference::resolve",
            &error,
        )
    })?;

    // stop recording
    let stop_result = recording
        .StopWithResultAsync()
        .map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.stopRecording",
                "LowLagMediaRecording::StopWithResultAsync",
                &error,
            )
        })?
        .get()
        .map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.stopRecording",
                "IAsyncOperation::get",
                &error,
            )
        })?;
    let duration_ns = stop_result
        .RecordDuration()
        .map(time_span_ns)
        .map(Some)
        .map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.stopRecording",
                "MediaCaptureStopResult::RecordDuration",
                &error,
            )
        })?;
    let _ = stop_result.Close();
    let size_bytes = std::fs::metadata(&output_path)
        .ok()
        .map(|metadata| metadata.len());

    // state reset
    {
        let mut state = resource.recording.lock();
        state.runtime = CameraRecordingRuntimeState::default();
        state.path = None;
        state.recording = None;
    }

    // result payload
    let recording = camera_recording_from_path(
        binding,
        &output_path,
        CameraRecordingContainer::Mp4,
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
