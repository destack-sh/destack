use super::controls::core::*;
use super::core::*;
use crate::platform::diagnostic::io_error_code_from_errno;

/// Collect the candidate Linux camera device nodes.
fn collect_camera_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // device scan
    if let Ok(entries) = std::fs::read_dir("/dev") {
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if !name.starts_with(CAMERA_DEVICE_PREFIX) {
                continue;
            }

            paths.push(path);
        }
    }

    // deterministic order
    paths.sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));

    paths
}

/// Open one camera descriptor.
pub(crate) fn open_camera_descriptor(
    path: &Path,
    flags: i32,
    operation: &'static str,
) -> RuntimeResult<RawFd> {
    // path c string
    let path = CString::new(path.as_os_str().as_encoded_bytes()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "camera path contains one interior nul byte",
        ))
        .boxed()
    })?;

    // host open
    let descriptor = unsafe { libc::open(path.as_ptr(), flags | libc::O_CLOEXEC) };
    if descriptor < 0 {
        return Err(camera_io_error(
            operation,
            "open",
            "failed to open camera endpoint",
        ));
    }

    Ok(descriptor)
}

/// Build one mapped linux camera error.
pub(crate) fn camera_io_error(
    operation: &'static str,
    syscall: &'static str,
    message: &str,
) -> Box<RuntimeError> {
    let errno = core_platform::get_errno();
    let code = io_error_code_from_errno(errno);

    RuntimeError::from(PlatformError::io_with(
        code,
        None,
        Some(errno),
        Some(operation.to_string()),
        Some(syscall.to_string()),
        format!("{syscall} failed: {message}"),
    ))
    .boxed()
}

/// Run one V4L2 ioctl with EINTR retry.
pub(crate) fn xioctl<T>(
    descriptor: RawFd,
    request: libc::c_ulong,
    value: &mut T,
    operation: &'static str,
    syscall: &'static str,
) -> RuntimeResult<()> {
    loop {
        let status =
            unsafe { libc::ioctl(descriptor, request, value as *mut T as *mut libc::c_void) };
        if status == 0 {
            return Ok(());
        }

        let errno = core_platform::get_errno();
        if errno == libc::EINTR {
            continue;
        }

        return Err(camera_io_error(operation, syscall, "camera ioctl failed"));
    }
}

/// Read one V4L2 querycap payload.
fn query_capability(
    descriptor: RawFd,
    operation: &'static str,
) -> RuntimeResult<v4l2::v4l2_capability> {
    // zeroed querycap
    let mut capability = unsafe { std::mem::zeroed::<v4l2::v4l2_capability>() };

    // querycap
    xioctl(
        descriptor,
        VIDIOC_QUERYCAP as libc::c_ulong,
        &mut capability,
        operation,
        "VIDIOC_QUERYCAP",
    )?;

    Ok(capability)
}

/// Return whether one capability payload supports camera capture.
pub(crate) fn effective_capability_bits(capability: &v4l2::v4l2_capability) -> u32 {
    let capabilities = capability.device_caps;
    if capabilities == 0 {
        capability.capabilities
    } else {
        capabilities
    }
}

/// Return whether one capability payload supports one honest camera path here.
fn is_capture_capability(capability: &v4l2::v4l2_capability) -> bool {
    let capabilities = effective_capability_bits(capability);
    supports_single_plane_capture(capabilities)
        && (supports_streaming_capture(capabilities) || supports_read_capture(capabilities))
}

/// Decode one nul-terminated V4L2 byte buffer.
fn v4l2_c_string(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());

    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

/// Build one stable camera id from one canonical path.
fn camera_id_from_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// Map one V4L2 pixel format to one platform pixel format descriptor.
fn camera_pixel_format_descriptor(fourcc: u32) -> Option<CameraPixelFormatDescriptorValue> {
    let descriptor = match fourcc {
        x if x == v4l2_fourcc(b'M', b'J', b'P', b'G') => CameraPixelFormatDescriptorValue {
            format: CameraPixelFormat::Jpeg,
            family: CameraPixelFormatFamily::Encoded,
            compressed: true,
        },
        x if x == v4l2_fourcc(b'Y', b'U', b'1', b'2')
            || x == v4l2_fourcc(b'N', b'V', b'1', b'2') =>
        {
            CameraPixelFormatDescriptorValue {
                format: CameraPixelFormat::Yuv420,
                family: CameraPixelFormatFamily::PlanarYuv,
                compressed: false,
            }
        }
        x if x == v4l2_fourcc(b'R', b'G', b'B', b'4') => CameraPixelFormatDescriptorValue {
            format: CameraPixelFormat::Rgba8,
            family: CameraPixelFormatFamily::PackedRgb,
            compressed: false,
        },
        x if x == v4l2_fourcc(b'B', b'G', b'R', b'4') => CameraPixelFormatDescriptorValue {
            format: CameraPixelFormat::Bgra8,
            family: CameraPixelFormatFamily::PackedRgb,
            compressed: false,
        },
        _ => return None,
    };

    Some(descriptor)
}

/// Query one camera config and capability cache from one opened descriptor.
fn query_camera_configs(
    descriptor: RawFd,
    operation: &'static str,
) -> RuntimeResult<(
    Vec<CameraStreamConfigValue>,
    Vec<CameraStreamCapabilityValue>,
)> {
    let mut configs = Vec::new();
    let mut capabilities = Vec::new();
    let mut seen = BTreeMap::<(u32, u32, u32, u32), usize>::new();
    let capture_type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
    let control_modes = query_camera_control_modes(descriptor)?;

    // format enumeration
    let mut format_index = 0u32;
    loop {
        let mut format = unsafe { std::mem::zeroed::<v4l2::v4l2_fmtdesc>() };
        format.index = format_index;
        format.type_ = capture_type;

        let status = unsafe {
            libc::ioctl(
                descriptor,
                VIDIOC_ENUM_FMT as libc::c_ulong,
                &mut format as *mut _ as *mut libc::c_void,
            )
        };
        if status != 0 {
            let errno = core_platform::get_errno();
            if errno == libc::EINVAL {
                break;
            }

            return Err(camera_io_error(
                operation,
                "VIDIOC_ENUM_FMT",
                "failed to enumerate formats",
            ));
        }

        let Some(pixel_format) = camera_pixel_format_descriptor(format.pixelformat) else {
            format_index += 1;
            continue;
        };

        // frame size enumeration
        let mut size_index = 0u32;
        loop {
            let mut frame_size = unsafe { std::mem::zeroed::<v4l2::v4l2_frmsizeenum>() };
            frame_size.index = size_index;
            frame_size.pixel_format = format.pixelformat;

            let status = unsafe {
                libc::ioctl(
                    descriptor,
                    VIDIOC_ENUM_FRAMESIZES as libc::c_ulong,
                    &mut frame_size as *mut _ as *mut libc::c_void,
                )
            };
            if status != 0 {
                let errno = core_platform::get_errno();
                if errno == libc::EINVAL {
                    break;
                }

                return Err(camera_io_error(
                    operation,
                    "VIDIOC_ENUM_FRAMESIZES",
                    "failed to enumerate frame sizes",
                ));
            }

            if frame_size.type_ != v4l2::v4l2_frmsizetypes_V4L2_FRMSIZE_TYPE_DISCRETE {
                size_index += 1;
                continue;
            }

            let width = unsafe { frame_size.__bindgen_anon_1.discrete.width };
            let height = unsafe { frame_size.__bindgen_anon_1.discrete.height };

            // frame interval enumeration
            let mut interval_index = 0u32;
            let mut found_interval = false;
            loop {
                let mut frame_interval = unsafe { std::mem::zeroed::<v4l2::v4l2_frmivalenum>() };
                frame_interval.index = interval_index;
                frame_interval.pixel_format = format.pixelformat;
                frame_interval.width = width;
                frame_interval.height = height;

                let status = unsafe {
                    libc::ioctl(
                        descriptor,
                        VIDIOC_ENUM_FRAMEINTERVALS as libc::c_ulong,
                        &mut frame_interval as *mut _ as *mut libc::c_void,
                    )
                };
                if status != 0 {
                    let errno = core_platform::get_errno();
                    if errno == libc::EINVAL {
                        break;
                    }

                    return Err(camera_io_error(
                        operation,
                        "VIDIOC_ENUM_FRAMEINTERVALS",
                        "failed to enumerate frame intervals",
                    ));
                }

                if frame_interval.type_ != v4l2::v4l2_frmivaltypes_V4L2_FRMIVAL_TYPE_DISCRETE {
                    interval_index += 1;
                    continue;
                }

                found_interval = true;
                let numerator = unsafe { frame_interval.__bindgen_anon_1.discrete.numerator };
                let denominator = unsafe { frame_interval.__bindgen_anon_1.discrete.denominator };
                if numerator == 0 {
                    interval_index += 1;
                    continue;
                }

                let frame_rate_millihz = ((denominator as u64) * 1_000u64) / numerator as u64;
                let frame_rate_millihz = u32::try_from(frame_rate_millihz).unwrap_or(u32::MAX);

                let key = (
                    width,
                    height,
                    frame_rate_millihz,
                    pixel_format.format as u32,
                );
                if seen.contains_key(&key) {
                    interval_index += 1;
                    continue;
                }

                let config = CameraStreamConfigValue {
                    width,
                    height,
                    frame_rate_milli_hz: frame_rate_millihz,
                    pixel_format: pixel_format.clone(),
                };

                let capability = basic_camera_stream_capability(&config, &control_modes);

                seen.insert(key, configs.len());
                configs.push(config);
                capabilities.push(capability);
                interval_index += 1;
            }

            if !found_interval {
                let key = (width, height, 30_000, pixel_format.format as u32);
                if !seen.contains_key(&key) {
                    let config = CameraStreamConfigValue {
                        width,
                        height,
                        frame_rate_milli_hz: 30_000,
                        pixel_format: pixel_format.clone(),
                    };
                    let capability = basic_camera_stream_capability(&config, &control_modes);
                    seen.insert(key, configs.len());
                    configs.push(config);
                    capabilities.push(capability);
                }
            }

            size_index += 1;
        }

        format_index += 1;
    }

    Ok((configs, capabilities))
}

/// Query one camera descriptor info snapshot from one linux device path.
fn query_camera_info(
    path: &Path,
    operation: &'static str,
) -> RuntimeResult<Option<LinuxCameraDescriptorInfo>> {
    // device open
    let descriptor =
        match open_camera_descriptor(path, libc::O_RDONLY | libc::O_NONBLOCK, operation) {
            Ok(descriptor) => descriptor,
            Err(_) => return Ok(None),
        };

    // capability query
    let capability = match query_capability(descriptor, operation) {
        Ok(capability) => capability,
        Err(_) => {
            unsafe {
                libc::close(descriptor);
            }
            return Ok(None);
        }
    };

    // capture filter
    if !is_capture_capability(&capability) {
        unsafe {
            libc::close(descriptor);
        }
        return Ok(None);
    }

    // config cache
    let (configs, capabilities) = match query_camera_configs(descriptor, operation) {
        Ok(values) => values,
        Err(_) => {
            unsafe {
                libc::close(descriptor);
            }
            return Ok(None);
        }
    };

    unsafe {
        libc::close(descriptor);
    }

    // stable descriptor
    let canonical_path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let info = LinuxCameraDescriptorInfo {
        descriptor: CameraDeviceDescriptorValue {
            id: camera_id_from_path(&canonical_path),
            group_id: None,
            name: v4l2_c_string(&capability.card),
            manufacturer: None,
            facing_mode: CameraFacingMode::Unknown,
            depth_capable: false,
        },
        path: canonical_path,
        configs,
        capabilities,
        capability_bits: effective_capability_bits(&capability),
    };

    Ok(Some(info))
}

/// Collect the current Linux camera descriptor snapshot keyed by stable id.
pub(crate) fn camera_descriptor_snapshot(
    operation: &'static str,
) -> RuntimeResult<BTreeMap<String, CameraDeviceDescriptorValue>> {
    let mut snapshot = BTreeMap::new();

    // descriptor scan
    for path in collect_camera_paths() {
        let Some(info) = query_camera_info(&path, operation)? else {
            continue;
        };

        snapshot.insert(info.descriptor.id.clone(), info.descriptor);
    }

    Ok(snapshot)
}

/// Lookup one camera descriptor info by stable id.
fn camera_info_by_id(id: &str) -> RuntimeResult<LinuxCameraDescriptorInfo> {
    for path in collect_camera_paths() {
        let Some(info) = query_camera_info(&path, "destack.device.camera.device.open")? else {
            continue;
        };
        if info.descriptor.id == id {
            return Ok(info);
        }
    }

    Err(core_platform::io_not_found(
        "destack.device.camera.device.open",
        "camera endpoint not found",
    ))
}

/// List the Linux camera devices exposed to the runtime.
pub(crate) unsafe fn destack_device_camera_device_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CameraDeviceDescriptor>,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // descriptor scan
    let descriptors = camera_descriptor_snapshot("destack.device.camera.device.list")?;
    let descriptors = descriptors
        .into_values()
        .map(|descriptor| camera_descriptor_from_value(binding, descriptor))
        .collect::<Vec<_>>();

    // binding store
    let descriptors = binding.store_slice(descriptors);
    unsafe {
        out.write(descriptors);
    }

    Ok(())
}

/// Open camera endpoint.
pub(crate) unsafe fn destack_device_camera_device_open(
    binding: &BindingCallContext,
    out: *mut resource::CameraDeviceHandle,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // id decode
    let id = unsafe { id.as_str()? };
    let info = camera_info_by_id(id)?;

    // resource store
    let entry = ResourceEntry::new(ResourceKind::CameraDevice)
        .with_label(CAMERA_DEVICE_RESOURCE_LABEL)
        .with_payload(Arc::new(LinuxCameraDeviceResource { info }))
        .with_finalizer(
            binding
                .worker()
                .platform_state
                .device
                .retain_runtime_activity(),
        );
    let handle = binding
        .worker()
        .resources
        .insert(&binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::CameraDeviceHandle(handle));
    }

    Ok(())
}

/// Close camera endpoint.
pub(crate) unsafe fn destack_device_camera_device_close(
    binding: &BindingCallContext,
    handle: resource::CameraDeviceHandle,
) -> RuntimeResult<()> {
    close_camera_device_resource(binding, handle, "destack.device.camera.device.close")
}

/// List supported stream capabilities for one opened camera endpoint.
pub(crate) unsafe fn destack_device_camera_device_stream_capability_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CameraStreamCapability>,
    handle: resource::CameraDeviceHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resource lookup
    let resource = camera_device_resource::<LinuxCameraDeviceResource>(
        binding,
        handle,
        "destack.device.camera.device.streamCapabilityList",
    )?;

    // binding store
    let capabilities = store_camera_capabilities(binding, &resource.info.capabilities);
    unsafe {
        out.write(capabilities);
    }

    Ok(())
}

/// Map one selected camera config into one V4L2 pixel format code.
pub(crate) fn v4l2_fourcc_from_config(config: &CameraStreamConfigValue) -> RuntimeResult<u32> {
    let fourcc = match config.pixel_format.format {
        CameraPixelFormat::Jpeg => v4l2_fourcc(b'M', b'J', b'P', b'G'),
        CameraPixelFormat::Yuv420 => v4l2_fourcc(b'Y', b'U', b'1', b'2'),
        CameraPixelFormat::Rgba8 => v4l2_fourcc(b'R', b'G', b'B', b'4'),
        CameraPixelFormat::Bgra8 => v4l2_fourcc(b'B', b'G', b'R', b'4'),
    };

    Ok(fourcc)
}

/// Estimate one conservative frame allocation size from one config.
fn frame_bytes_hint(config: &CameraStreamConfigValue) -> usize {
    match config.pixel_format.format {
        CameraPixelFormat::Jpeg => 1_048_576usize,
        CameraPixelFormat::Yuv420 => {
            (config.width as usize * config.height as usize * 3usize) / 2usize
        }
        CameraPixelFormat::Rgba8 | CameraPixelFormat::Bgra8 => {
            config.width as usize * config.height as usize * 4usize
        }
    }
}

/// Return the active V4L2 pixel format payload.
fn active_pix_format(format: &v4l2::v4l2_format) -> v4l2::v4l2_pix_format {
    unsafe { format.fmt.pix }
}

/// Build one MMAP-ready buffer descriptor.
pub(crate) fn mmap_buffer_descriptor(index: u32, capture_type: u32) -> v4l2::v4l2_buffer {
    let mut buffer = unsafe { std::mem::zeroed::<v4l2::v4l2_buffer>() };
    buffer.index = index;
    buffer.type_ = capture_type;
    buffer.memory = V4L2_MEMORY_MMAP;
    buffer
}

/// Prepare one linux camera stream mode from the negotiated descriptor state.
pub(crate) fn prepare_stream_mode(
    descriptor: RawFd,
    capability_bits: u32,
    config: &CameraStreamConfigValue,
    format: &v4l2::v4l2_format,
) -> RuntimeResult<LinuxCameraStreamMode> {
    // prefer MMAP streaming when the driver exposes it
    if supports_streaming_capture(capability_bits) {
        let capture_type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
        let mut request_buffers = unsafe { std::mem::zeroed::<v4l2::v4l2_requestbuffers>() };
        request_buffers.count = CAMERA_STREAM_BUFFER_COUNT;
        request_buffers.type_ = capture_type;
        request_buffers.memory = V4L2_MEMORY_MMAP;

        xioctl(
            descriptor,
            VIDIOC_REQBUFS as libc::c_ulong,
            &mut request_buffers,
            "destack.device.camera.stream.open",
            "VIDIOC_REQBUFS",
        )?;

        if request_buffers.count > 0 {
            let mut buffers =
                Vec::<LinuxCameraMappedBuffer>::with_capacity(request_buffers.count as usize);

            for index in 0..request_buffers.count {
                let mut buffer = mmap_buffer_descriptor(index, capture_type);

                xioctl(
                    descriptor,
                    VIDIOC_QUERYBUF as libc::c_ulong,
                    &mut buffer,
                    "destack.device.camera.stream.open",
                    "VIDIOC_QUERYBUF",
                )?;

                let length = buffer.length as usize;
                let offset = unsafe { buffer.m.offset } as libc::off_t;
                let address = unsafe {
                    libc::mmap(
                        std::ptr::null_mut(),
                        length,
                        libc::PROT_READ | libc::PROT_WRITE,
                        libc::MAP_SHARED,
                        descriptor,
                        offset,
                    )
                };

                if address == libc::MAP_FAILED {
                    for mapped_buffer in &buffers {
                        unsafe {
                            libc::munmap(
                                mapped_buffer.address.cast::<libc::c_void>(),
                                mapped_buffer.length,
                            );
                        }
                    }

                    return Err(camera_io_error(
                        "destack.device.camera.stream.open",
                        "mmap",
                        "failed to map one camera buffer",
                    ));
                }

                buffers.push(LinuxCameraMappedBuffer {
                    address: address.cast::<u8>(),
                    length,
                });
            }

            return Ok(LinuxCameraStreamMode::Mmap {
                capture_type,
                buffers,
            });
        }
    }

    // otherwise fall back to the read interface when the driver exposes it
    if supports_read_capture(capability_bits) {
        let format = active_pix_format(format);
        let frame_bytes_hint = if format.sizeimage > 0 {
            format.sizeimage as usize
        } else {
            frame_bytes_hint(config)
        };

        return Ok(LinuxCameraStreamMode::ReadWrite { frame_bytes_hint });
    }

    Err(camera_not_supported("destack.device.camera.stream.open"))
}

/// Queue all MMAP buffers for one capture stream.
fn queue_mmap_buffers(
    descriptor: RawFd,
    capture_type: u32,
    buffers: &[LinuxCameraMappedBuffer],
) -> RuntimeResult<()> {
    // queue every mapped buffer before stream-on
    for (index, _) in buffers.iter().enumerate() {
        let mut buffer = mmap_buffer_descriptor(index as u32, capture_type);

        xioctl(
            descriptor,
            VIDIOC_QBUF as libc::c_ulong,
            &mut buffer,
            "destack.device.camera.stream.start",
            "VIDIOC_QBUF",
        )?;
    }

    Ok(())
}

/// Start one host camera stream if needed.
pub(crate) fn start_host_stream(resource: &LinuxCameraStreamResource) -> RuntimeResult<()> {
    if let LinuxCameraStreamMode::Mmap {
        capture_type,
        buffers,
    } = &resource.mode
    {
        queue_mmap_buffers(resource.descriptor, *capture_type, buffers)?;

        let mut capture_type = *capture_type;
        xioctl(
            resource.descriptor,
            VIDIOC_STREAMON as libc::c_ulong,
            &mut capture_type,
            "destack.device.camera.stream.start",
            "VIDIOC_STREAMON",
        )?;
    }

    Ok(())
}

/// Stop one host camera stream if needed.
pub(crate) fn stop_host_stream(resource: &LinuxCameraStreamResource) -> RuntimeResult<()> {
    if let LinuxCameraStreamMode::Mmap { capture_type, .. } = &resource.mode {
        let mut capture_type = *capture_type;
        xioctl(
            resource.descriptor,
            VIDIOC_STREAMOFF as libc::c_ulong,
            &mut capture_type,
            "destack.device.camera.stream.stop",
            "VIDIOC_STREAMOFF",
        )?;
    }

    Ok(())
}

/// Wait for one readable camera frame.
pub(crate) fn wait_for_frame(descriptor: RawFd, timeout_ns: u64) -> RuntimeResult<bool> {
    let deadline = if timeout_ns == u64::MAX {
        None
    } else {
        Some(Instant::now() + std::time::Duration::from_nanos(timeout_ns))
    };

    loop {
        let timeout_millis = match deadline {
            Some(deadline) => {
                let now = Instant::now();
                if now >= deadline {
                    return Ok(false);
                }
                let remaining = deadline.saturating_duration_since(now).as_millis();
                remaining.min(MAX_POLL_TIMEOUT_MILLIS as u128) as i32
            }
            None => -1,
        };

        let mut poll_descriptor = libc::pollfd {
            fd: descriptor,
            events: CAMERA_POLL_READ_FLAGS,
            revents: 0,
        };

        let status = unsafe { libc::poll(&mut poll_descriptor, 1, timeout_millis) };
        if status > 0 {
            return Ok((poll_descriptor.revents & libc::POLLIN) != 0);
        }
        if status == 0 {
            return Ok(false);
        }

        let errno = core_platform::get_errno();
        if errno == libc::EINTR {
            continue;
        }

        return Err(camera_io_error(
            "destack.device.camera.stream.read",
            "poll",
            "failed while waiting for one camera frame",
        ));
    }
}

/// Read one host frame byte payload from one opened linux stream.
pub(crate) fn read_host_frame_bytes(
    descriptor: RawFd,
    mode: &LinuxCameraStreamMode,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    match mode {
        LinuxCameraStreamMode::ReadWrite { frame_bytes_hint } => {
            let mut bytes = vec![0u8; *frame_bytes_hint];
            let read_count = unsafe {
                libc::read(
                    descriptor,
                    bytes.as_mut_ptr().cast::<libc::c_void>(),
                    bytes.len(),
                )
            };
            if read_count < 0 {
                let errno = core_platform::get_errno();
                if errno == libc::EAGAIN || errno == libc::EWOULDBLOCK {
                    return Err(core_platform::io_would_block(
                        operation,
                        "camera frame is not ready",
                    ));
                }

                return Err(camera_io_error(
                    operation,
                    "read",
                    "failed to read one camera frame",
                ));
            }

            bytes.truncate(read_count as usize);

            Ok(bytes)
        }
        LinuxCameraStreamMode::Mmap {
            capture_type,
            buffers,
        } => {
            let mut buffer = mmap_buffer_descriptor(0, *capture_type);
            xioctl(
                descriptor,
                VIDIOC_DQBUF as libc::c_ulong,
                &mut buffer,
                operation,
                "VIDIOC_DQBUF",
            )?;

            let buffer_index = buffer.index as usize;
            let mapped_buffer = buffers.get(buffer_index).ok_or_else(|| {
                RuntimeError::from(PlatformError::invalid_data(
                    "camera driver returned one invalid buffer index",
                ))
                .boxed()
            })?;

            let bytes_used = buffer.bytesused as usize;
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    mapped_buffer.address,
                    bytes_used.min(mapped_buffer.length),
                )
            }
            .to_vec();

            xioctl(
                descriptor,
                VIDIOC_QBUF as libc::c_ulong,
                &mut buffer,
                operation,
                "VIDIOC_QBUF",
            )?;

            Ok(bytes)
        }
    }
}
