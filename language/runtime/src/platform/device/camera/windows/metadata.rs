use std::collections::BTreeMap;

use super::core::*;

/// Build one runtime error for one WinRT camera failure.
pub(super) fn windows_camera_error(
    operation: &'static str,
    action: &str,
    error: &WinError,
) -> Box<RuntimeError> {
    core_platform::winrt_io_error(operation, action, error)
}

/// Convert one camera device id into one public stable identifier.
pub(super) fn stable_camera_id(id: &str) -> String {
    format!("{WINDOWS_CAMERA_ID_PREFIX}:{id}")
}

/// Parse one public stable identifier into the underlying WinRT id.
pub(super) fn parse_camera_id(value: &str) -> RuntimeResult<&str> {
    value
        .strip_prefix(&format!("{WINDOWS_CAMERA_ID_PREFIX}:"))
        .ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "camera identifier is not one WinRT camera id",
            ))
            .boxed()
        })
}

/// Map one Windows enclosure panel into the public facing-mode surface.
pub(super) fn facing_mode_from_panel(panel: Option<Panel>) -> CameraFacingMode {
    match panel {
        Some(Panel::Front) => CameraFacingMode::User,
        Some(Panel::Back) => CameraFacingMode::Environment,
        Some(Panel::Left) => CameraFacingMode::Left,
        Some(Panel::Right) => CameraFacingMode::Right,
        _ => CameraFacingMode::External,
    }
}

/// Return whether one frame source is one usable color stream.
pub(super) fn is_color_camera_source(info: &MediaFrameSourceInfo) -> bool {
    matches!(info.SourceKind(), Ok(MediaFrameSourceKind::Color))
}

/// Decode one media-ratio value into milli-hertz.
pub(super) fn frame_rate_milli_hz(format: &MediaFrameFormat) -> u32 {
    let Ok(ratio) = format.FrameRate() else {
        return 0;
    };
    let numerator = ratio.Numerator().unwrap_or(0);
    let denominator = ratio.Denominator().unwrap_or(1);
    if numerator == 0 || denominator == 0 {
        return 0;
    }

    numerator.saturating_mul(1000) / denominator
}

/// Build one public BGRA8 stream config from one WinRT format.
pub(super) fn config_from_media_format(
    format: &MediaFrameFormat,
) -> RuntimeResult<CameraStreamConfigValue> {
    let video_format = format.VideoFormat().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.device.open",
            "MediaFrameFormat::VideoFormat",
            &error,
        )
    })?;

    Ok(CameraStreamConfigValue {
        width: video_format.Width().map_err(|error| {
            windows_camera_error(
                "destack.device.camera.device.open",
                "VideoMediaFrameFormat::Width",
                &error,
            )
        })?,
        height: video_format.Height().map_err(|error| {
            windows_camera_error(
                "destack.device.camera.device.open",
                "VideoMediaFrameFormat::Height",
                &error,
            )
        })?,
        frame_rate_milli_hz: frame_rate_milli_hz(format),
        pixel_format: bgra_camera_pixel_format_descriptor(),
        color_space: Some(CameraColorSpace::Srgb),
        dynamic_range: Some(CameraDynamicRange::Standard),
    })
}

/// Build one camera descriptor row from one frame-source group.
pub(super) fn descriptor_from_source_group(
    group: &MediaFrameSourceGroup,
    source_info: &MediaFrameSourceInfo,
) -> RuntimeResult<CameraDeviceDescriptorValue> {
    let device_information = source_info.DeviceInformation().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.device.list",
            "MediaFrameSourceInfo::DeviceInformation",
            &error,
        )
    })?;
    let name = device_information.Name().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.device.list",
            "DeviceInformation::Name",
            &error,
        )
    })?;
    let enclosure_location = device_information.EnclosureLocation().ok();
    let panel = enclosure_location
        .as_ref()
        .and_then(|location| location.Panel().ok());

    Ok(CameraDeviceDescriptorValue {
        id: stable_camera_id(
            &group
                .Id()
                .map_err(|error| {
                    windows_camera_error(
                        "destack.device.camera.device.list",
                        "MediaFrameSourceGroup::Id",
                        &error,
                    )
                })?
                .to_string(),
        ),
        group_id: group.Id().ok().map(|value| value.to_string()),
        name: name.to_string(),
        manufacturer: None,
        facing_mode: facing_mode_from_panel(panel),
        depth_capable: false,
    })
}

/// Collect the current Windows camera descriptor snapshot keyed by stable id.
pub(super) fn camera_descriptor_snapshot(
    operation: &'static str,
) -> RuntimeResult<BTreeMap<String, CameraDeviceDescriptorValue>> {
    let groups = MediaFrameSourceGroup::FindAllAsync()
        .map_err(|error| {
            windows_camera_error(operation, "MediaFrameSourceGroup::FindAllAsync", &error)
        })?
        .get()
        .map_err(|error| windows_camera_error(operation, "IAsyncOperation::get", &error))?;
    let mut snapshot = BTreeMap::new();

    // descriptor scan
    for group in &groups {
        let source_infos = group.SourceInfos().map_err(|error| {
            windows_camera_error(operation, "MediaFrameSourceGroup::SourceInfos", &error)
        })?;

        for source_info in &source_infos {
            if !is_color_camera_source(&source_info) {
                continue;
            }

            let descriptor = descriptor_from_source_group(&group, &source_info)?;
            snapshot.insert(descriptor.id.clone(), descriptor);
            break;
        }
    }

    Ok(snapshot)
}

/// Build one descriptor snapshot and stream selections from one initialized capture session.
pub(super) fn descriptor_info_from_capture(
    capture: &MediaCapture,
    group: &MediaFrameSourceGroup,
) -> RuntimeResult<WindowsCameraDescriptorInfo> {
    let source_infos = group.SourceInfos().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.device.open",
            "MediaFrameSourceGroup::SourceInfos",
            &error,
        )
    })?;
    let Some(_source_info) = (&source_infos)
        .into_iter()
        .find(|source_info| is_color_camera_source(source_info))
    else {
        return Err(camera_not_supported("destack.device.camera.device.open"));
    };
    let frame_sources = capture.FrameSources().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.device.open",
            "MediaCapture::FrameSources",
            &error,
        )
    })?;

    let mut configs = Vec::new();
    let mut capabilities = Vec::new();
    let mut selections = Vec::new();
    let controller = capture_video_device_controller(capture, "destack.device.camera.device.open")?;
    let control_modes = camera_control_modes(&controller)?;

    // source configs
    for source_info in &source_infos {
        if !is_color_camera_source(&source_info) {
            continue;
        }

        let source_id = source_info.Id().map_err(|error| {
            windows_camera_error(
                "destack.device.camera.device.open",
                "MediaFrameSourceInfo::Id",
                &error,
            )
        })?;
        let source = frame_sources.Lookup(&source_id).map_err(|error| {
            windows_camera_error(
                "destack.device.camera.device.open",
                "IMapView::Lookup",
                &error,
            )
        })?;
        let supported_formats = source.SupportedFormats().map_err(|error| {
            windows_camera_error(
                "destack.device.camera.device.open",
                "MediaFrameSource::SupportedFormats",
                &error,
            )
        })?;

        for format in &supported_formats {
            let config = config_from_media_format(&format)?;
            let subtype = format.Subtype().map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.device.open",
                    "MediaFrameFormat::Subtype",
                    &error,
                )
            })?;
            if !configs.iter().any(|candidate| candidate == &config) {
                configs.push(config);
                capabilities.push(basic_camera_stream_capability(&config, &control_modes));
            }
            selections.push(WindowsCameraStreamSelection {
                config,
                source_id: source_id.to_string(),
                subtype: subtype.to_string(),
            });
        }
    }

    Ok(WindowsCameraDescriptorInfo {
        capabilities,
        selections,
    })
}

/// Resolve one video-device controller from one initialized capture session.
pub(super) fn capture_video_device_controller(
    capture: &MediaCapture,
    operation: &'static str,
) -> RuntimeResult<VideoDeviceController> {
    capture.VideoDeviceController().map_err(|error| {
        windows_camera_error(operation, "MediaCapture::VideoDeviceController", &error)
    })
}

/// Build one supported control-mode snapshot from one WinRT controller.
pub(super) fn camera_control_modes(
    controller: &VideoDeviceController,
) -> RuntimeResult<CameraControlModes> {
    let mut modes = default_camera_control_modes();

    // exposure modes
    let exposure = controller.ExposureControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.device.open",
            "VideoDeviceController::ExposureControl",
            &error,
        )
    })?;
    if exposure.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.device.open",
            "ExposureControl::Supported",
            &error,
        )
    })? {
        modes.exposure_modes.clear();
        modes.exposure_modes.push(CameraExposureMode::Manual);

        if exposure.Auto().map_err(|error| {
            windows_camera_error(
                "destack.device.camera.device.open",
                "ExposureControl::Auto",
                &error,
            )
        })? {
            modes.exposure_modes.push(CameraExposureMode::Auto);
        }
    }

    // white balance modes
    let white_balance = controller.WhiteBalanceControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.device.open",
            "VideoDeviceController::WhiteBalanceControl",
            &error,
        )
    })?;
    if white_balance.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.device.open",
            "WhiteBalanceControl::Supported",
            &error,
        )
    })? {
        modes.white_balance_modes.clear();
        modes
            .white_balance_modes
            .push(CameraWhiteBalanceMode::Manual);
        modes.white_balance_modes.push(CameraWhiteBalanceMode::Auto);
    }

    // focus modes
    let focus = controller.FocusControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.device.open",
            "VideoDeviceController::FocusControl",
            &error,
        )
    })?;
    if focus.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.device.open",
            "FocusControl::Supported",
            &error,
        )
    })? {
        modes.focus_modes.clear();
        let supported_modes = focus.SupportedFocusModes().map_err(|error| {
            windows_camera_error(
                "destack.device.camera.device.open",
                "FocusControl::SupportedFocusModes",
                &error,
            )
        })?;
        for mode in &supported_modes {
            match mode {
                WindowsFocusMode::Auto | WindowsFocusMode::Single => {
                    if !modes.focus_modes.contains(&CameraFocusMode::Auto) {
                        modes.focus_modes.push(CameraFocusMode::Auto);
                    }
                }
                WindowsFocusMode::Continuous => {
                    if !modes.focus_modes.contains(&CameraFocusMode::ContinuousAuto) {
                        modes.focus_modes.push(CameraFocusMode::ContinuousAuto);
                    }
                }
                WindowsFocusMode::Manual => {
                    if !modes.focus_modes.contains(&CameraFocusMode::Manual) {
                        modes.focus_modes.push(CameraFocusMode::Manual);
                    }
                }
                _ => {}
            }
        }
        if modes.focus_modes.is_empty() {
            modes.focus_modes.push(CameraFocusMode::Auto);
        }
    }

    // stabilization modes
    let stabilization = controller
        .OpticalImageStabilizationControl()
        .map_err(|error| {
            windows_camera_error(
                "destack.device.camera.device.open",
                "VideoDeviceController::OpticalImageStabilizationControl",
                &error,
            )
        })?;
    if stabilization.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.device.open",
            "OpticalImageStabilizationControl::Supported",
            &error,
        )
    })? {
        modes.stabilization_modes.clear();
        let supported_modes = stabilization.SupportedModes().map_err(|error| {
            windows_camera_error(
                "destack.device.camera.device.open",
                "OpticalImageStabilizationControl::SupportedModes",
                &error,
            )
        })?;
        for mode in &supported_modes {
            match mode {
                OpticalImageStabilizationMode::Off => {
                    if !modes
                        .stabilization_modes
                        .contains(&CameraStabilizationMode::Off)
                    {
                        modes.stabilization_modes.push(CameraStabilizationMode::Off);
                    }
                }
                OpticalImageStabilizationMode::On | OpticalImageStabilizationMode::Auto => {
                    if !modes
                        .stabilization_modes
                        .contains(&CameraStabilizationMode::Standard)
                    {
                        modes
                            .stabilization_modes
                            .push(CameraStabilizationMode::Standard);
                    }
                }
                _ => {}
            }
        }
        if modes.stabilization_modes.is_empty() {
            modes.stabilization_modes.push(CameraStabilizationMode::Off);
        }
    }

    // torch modes
    let torch = controller.TorchControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.device.open",
            "VideoDeviceController::TorchControl",
            &error,
        )
    })?;
    if torch.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.device.open",
            "TorchControl::Supported",
            &error,
        )
    })? {
        modes.torch_modes.clear();
        modes.torch_modes.push(CameraTorchMode::Off);
        modes.torch_modes.push(CameraTorchMode::On);
    }

    Ok(modes)
}

/// Build one initialized Windows media-capture session for one source group.
pub(super) fn open_media_capture(
    group: &MediaFrameSourceGroup,
    operation: &'static str,
) -> RuntimeResult<MediaCapture> {
    let settings = MediaCaptureInitializationSettings::new().map_err(|error| {
        windows_camera_error(operation, "MediaCaptureInitializationSettings::new", &error)
    })?;
    settings.SetSourceGroup(group).map_err(|error| {
        windows_camera_error(
            operation,
            "MediaCaptureInitializationSettings::SetSourceGroup",
            &error,
        )
    })?;
    settings
        .SetStreamingCaptureMode(StreamingCaptureMode::Video)
        .map_err(|error| {
            windows_camera_error(
                operation,
                "MediaCaptureInitializationSettings::SetStreamingCaptureMode",
                &error,
            )
        })?;
    settings
        .SetSharingMode(MediaCaptureSharingMode::SharedReadOnly)
        .map_err(|error| {
            windows_camera_error(
                operation,
                "MediaCaptureInitializationSettings::SetSharingMode",
                &error,
            )
        })?;
    settings
        .SetMemoryPreference(MediaCaptureMemoryPreference::Cpu)
        .map_err(|error| {
            windows_camera_error(
                operation,
                "MediaCaptureInitializationSettings::SetMemoryPreference",
                &error,
            )
        })?;

    let capture = MediaCapture::new()
        .map_err(|error| windows_camera_error(operation, "MediaCapture::new", &error))?;
    capture
        .InitializeWithSettingsAsync(&settings)
        .map_err(|error| {
            windows_camera_error(
                operation,
                "MediaCapture::InitializeWithSettingsAsync",
                &error,
            )
        })?
        .get()
        .map_err(|error| windows_camera_error(operation, "IAsyncAction::get", &error))?;

    Ok(capture)
}

/// Resolve one frame source from one initialized capture session.
pub(super) fn capture_source(
    capture: &MediaCapture,
    source_id: &str,
    operation: &'static str,
) -> RuntimeResult<MediaFrameSource> {
    let frame_sources = capture
        .FrameSources()
        .map_err(|error| windows_camera_error(operation, "MediaCapture::FrameSources", &error))?;
    let source_id = HSTRING::from(source_id);

    frame_sources
        .Lookup(&source_id)
        .map_err(|error| windows_camera_error(operation, "IMapView::Lookup", &error))
}

/// Configure one source to one selected WinRT media subtype.
pub(super) fn configure_source_format(
    source: &MediaFrameSource,
    selection: &WindowsCameraStreamSelection,
    operation: &'static str,
) -> RuntimeResult<()> {
    let supported_formats = source.SupportedFormats().map_err(|error| {
        windows_camera_error(operation, "MediaFrameSource::SupportedFormats", &error)
    })?;

    // config match
    for format in &supported_formats {
        let config = config_from_media_format(&format)?;
        let subtype = format.Subtype().map_err(|error| {
            windows_camera_error(operation, "MediaFrameFormat::Subtype", &error)
        })?;
        if config == selection.config && subtype.to_string() == selection.subtype {
            source
                .SetFormatAsync(&format)
                .map_err(|error| {
                    windows_camera_error(operation, "MediaFrameSource::SetFormatAsync", &error)
                })?
                .get()
                .map_err(|error| windows_camera_error(operation, "IAsyncAction::get", &error))?;

            return Ok(());
        }
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "config",
        "camera stream configuration is not supported by this endpoint",
    ))
    .boxed())
}

/// Convert one delivered frame into one public camera frame payload.
pub(super) fn frame_value_from_reference(
    reference: &MediaFrameReference,
    config: &CameraStreamConfigValue,
    state: &Mutex<CameraStreamState>,
    operation: &'static str,
) -> RuntimeResult<CameraFrameValue> {
    let video_frame = reference.VideoMediaFrame().map_err(|error| {
        windows_camera_error(operation, "MediaFrameReference::VideoMediaFrame", &error)
    })?;
    let bitmap = video_frame.SoftwareBitmap().map_err(|error| {
        windows_camera_error(operation, "VideoMediaFrame::SoftwareBitmap", &error)
    })?;
    let bitmap = if bitmap
        .BitmapPixelFormat()
        .unwrap_or(BitmapPixelFormat::Bgra8)
        != BitmapPixelFormat::Bgra8
    {
        SoftwareBitmap::Convert(&bitmap, BitmapPixelFormat::Bgra8)
            .map_err(|error| windows_camera_error(operation, "SoftwareBitmap::Convert", &error))?
    } else {
        bitmap
    };
    let width = bitmap
        .PixelWidth()
        .map_err(|error| windows_camera_error(operation, "SoftwareBitmap::PixelWidth", &error))?
        as u32;
    let height = bitmap
        .PixelHeight()
        .map_err(|error| windows_camera_error(operation, "SoftwareBitmap::PixelHeight", &error))?
        as u32;
    let bytes_len = width.saturating_mul(height).saturating_mul(4);
    let buffer = Buffer::Create(bytes_len)
        .map_err(|error| windows_camera_error(operation, "Buffer::Create", &error))?;
    bitmap
        .CopyToBuffer(&buffer)
        .map_err(|error| windows_camera_error(operation, "SoftwareBitmap::CopyToBuffer", &error))?;
    let reader = DataReader::FromBuffer(&buffer)
        .map_err(|error| windows_camera_error(operation, "DataReader::FromBuffer", &error))?;
    let mut bytes = vec![0u8; bytes_len as usize];
    reader
        .ReadBytes(&mut bytes)
        .map_err(|error| windows_camera_error(operation, "DataReader::ReadBytes", &error))?;
    let timestamp_ns = reference
        .SystemRelativeTime()
        .ok()
        .and_then(|value| value.Value().ok())
        .map(|value| value.Duration.max(0) as u64 * 100)
        .unwrap_or_else(core_platform::monotonic_now_ns);

    Ok(CameraFrameValue {
        timestamp_ns,
        sequence: next_camera_sequence(state),
        width,
        height,
        pixel_format: CameraPixelFormatDescriptorValue {
            format: CameraPixelFormat::Bgra8,
            family: CameraPixelFormatFamily::PackedRgb,
            compressed: false,
        },
        color_space: CameraColorSpace::Srgb,
        dynamic_range: CameraDynamicRange::Standard,
        planes: frame_plane_layouts(config, bytes.len()),
        metadata: empty_camera_metadata(),
        bytes,
    })
}

/// Convert one WinRT software bitmap into one public still-photo payload.
pub(super) fn photo_value_from_software_bitmap(
    bitmap: &SoftwareBitmap,
    operation: &'static str,
) -> RuntimeResult<CameraPhotoValue> {
    let bitmap = if bitmap
        .BitmapPixelFormat()
        .unwrap_or(BitmapPixelFormat::Bgra8)
        != BitmapPixelFormat::Bgra8
    {
        SoftwareBitmap::Convert(bitmap, BitmapPixelFormat::Bgra8)
            .map_err(|error| windows_camera_error(operation, "SoftwareBitmap::Convert", &error))?
    } else {
        bitmap.clone()
    };
    let width = bitmap
        .PixelWidth()
        .map_err(|error| windows_camera_error(operation, "SoftwareBitmap::PixelWidth", &error))?
        as u32;
    let height = bitmap
        .PixelHeight()
        .map_err(|error| windows_camera_error(operation, "SoftwareBitmap::PixelHeight", &error))?
        as u32;
    let bytes_len = width.saturating_mul(height).saturating_mul(4);
    let buffer = Buffer::Create(bytes_len)
        .map_err(|error| windows_camera_error(operation, "Buffer::Create", &error))?;
    bitmap
        .CopyToBuffer(&buffer)
        .map_err(|error| windows_camera_error(operation, "SoftwareBitmap::CopyToBuffer", &error))?;
    let reader = DataReader::FromBuffer(&buffer)
        .map_err(|error| windows_camera_error(operation, "DataReader::FromBuffer", &error))?;
    let mut bytes = vec![0u8; bytes_len as usize];
    reader
        .ReadBytes(&mut bytes)
        .map_err(|error| windows_camera_error(operation, "DataReader::ReadBytes", &error))?;

    Ok(CameraPhotoValue {
        timestamp_ns: core_platform::monotonic_now_ns(),
        width,
        height,
        pixel_format: CameraPixelFormatDescriptorValue {
            format: CameraPixelFormat::Bgra8,
            family: CameraPixelFormatFamily::PackedRgb,
            compressed: false,
        },
        color_space: CameraColorSpace::Srgb,
        dynamic_range: CameraDynamicRange::Standard,
        planes: vec![CameraPlaneLayoutValue {
            offset_bytes: 0,
            length_bytes: bytes.len() as u32,
            row_stride_bytes: width.saturating_mul(4),
            pixel_stride_bytes: 4,
        }],
        metadata: empty_camera_metadata(),
        bytes,
    })
}

/// Convert one captured WinRT photo into one public still-photo payload.
pub(super) fn photo_value_from_captured_photo(
    photo: &CapturedPhoto,
    operation: &'static str,
) -> RuntimeResult<CameraPhotoValue> {
    let frame = photo
        .Frame()
        .map_err(|error| windows_camera_error(operation, "CapturedPhoto::Frame", &error))?;
    let bitmap = frame.SoftwareBitmap().map_err(|error| {
        windows_camera_error(operation, "CapturedFrame::SoftwareBitmap", &error)
    })?;
    let result = photo_value_from_software_bitmap(&bitmap, operation);

    let _ = frame.Close();

    result
}
