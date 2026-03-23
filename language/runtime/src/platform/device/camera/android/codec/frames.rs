use super::values::{decode_color_space, decode_pixel_format, decode_pixel_format_family};
use super::*;

/// Build one frame plane layout list from one frame payload.
fn android_camera_plane_layouts(
    pixel_format: CameraPixelFormatDescriptorValue,
    width: u32,
    bytes_len: usize,
) -> Vec<CameraPlaneLayoutValue> {
    let row_stride_bytes = match pixel_format.format {
        CameraPixelFormat::Bgra8 | CameraPixelFormat::Rgba8 => width.saturating_mul(4),
        CameraPixelFormat::Yuv420 => width,
        CameraPixelFormat::Jpeg => 0,
    };
    let pixel_stride_bytes = match pixel_format.format {
        CameraPixelFormat::Bgra8 | CameraPixelFormat::Rgba8 => 4,
        CameraPixelFormat::Yuv420 => 1,
        CameraPixelFormat::Jpeg => 0,
    };

    vec![CameraPlaneLayoutValue {
        offset_bytes: 0,
        length_bytes: bytes_len as u32,
        row_stride_bytes,
        pixel_stride_bytes,
    }]
}

/// Decode one camera frame payload.
fn decode_camera_frame(
    header: &AndroidHostCameraFrameHeader,
    bytes: Vec<u8>,
    operation: &'static str,
) -> RuntimeResult<CameraFrameValue> {
    let pixel_format = CameraPixelFormatDescriptorValue {
        format: decode_pixel_format(header.pixel_format, operation)?,
        family: decode_pixel_format_family(header.pixel_format_family, operation)?,
        compressed: header.is_compressed != 0,
    };

    Ok(CameraFrameValue {
        timestamp_ns: header.timestamp_ns,
        sequence: header.sequence,
        width: header.width,
        height: header.height,
        pixel_format,
        color_space: decode_color_space(header.color_space, operation)?,
        dynamic_range: CameraDynamicRange::Standard,
        planes: android_camera_plane_layouts(pixel_format, header.width, bytes.len()),
        metadata: CameraFrameMetadataValue {
            exposure_time_ns: None,
            sensor_iso: None,
            white_balance_kelvin: None,
            focus_distance_diopters: None,
            zoom_ratio: None,
        },
        bytes,
    })
}

/// Grow one frame buffer after one `bufferTooSmall` result.
fn grow_frame_buffer(
    bytes: &mut Vec<u8>,
    required_bytes: usize,
    operation: &'static str,
) -> RuntimeResult<()> {
    let next_capacity = bytes.len().max(required_bytes).saturating_mul(2);
    if next_capacity > MAX_CAMERA_FRAME_CAPACITY {
        return Err(invalid_data(
            operation,
            "android host camera frame exceeded the maximum payload size",
        ));
    }

    bytes.resize(next_capacity, 0);

    Ok(())
}

/// Read one camera frame through one Android host callback.
pub(crate) fn read_camera_frame_from_host(
    binding: &BindingCallContext,
    stream_id: u64,
    timeout_ns: Option<u64>,
    operation: &'static str,
) -> RuntimeResult<CameraFrameValue> {
    let runtime_id = host_session_id(binding, operation)?;
    let mut header = AndroidHostCameraFrameHeader::default();
    let mut bytes = vec![0u8; INITIAL_CAMERA_FRAME_CAPACITY];

    loop {
        let byte_capacity = checked_u32_length(bytes.len(), operation, "frame bytes")?;
        let mut bytes_written = 0u32;
        let status = if let Some(timeout_ns) = timeout_ns {
            unsafe {
                destack_host_android_camera_stream_read(
                    runtime_id,
                    stream_id,
                    timeout_ns,
                    &mut header,
                    NativeSlice {
                        data: bytes.as_mut_ptr(),
                        len: byte_capacity,
                    },
                    &mut bytes_written,
                )
            }
        } else {
            unsafe {
                destack_host_android_camera_stream_try_read(
                    runtime_id,
                    stream_id,
                    &mut header,
                    NativeSlice {
                        data: bytes.as_mut_ptr(),
                        len: byte_capacity,
                    },
                    &mut bytes_written,
                )
            }
        };

        // grow the frame buffer when the host reports truncation
        if status == HostStatus::BufferTooSmall.code() {
            let required_bytes = (header.bytes_len as usize).max(bytes_written as usize);
            grow_frame_buffer(&mut bytes, required_bytes, operation)?;
            continue;
        }

        host_status_result(status, operation, "camera frame read")?;

        let byte_count = bytes_written as usize;
        if byte_count > bytes.len() {
            return Err(invalid_data(
                operation,
                "android host camera frame read reported one oversized payload",
            ));
        }

        bytes.truncate(byte_count);
        return decode_camera_frame(&header, bytes, operation);
    }
}

/// Capture one still photo through one Android host callback.
pub(crate) fn read_camera_photo_from_host(
    binding: &BindingCallContext,
    stream_id: u64,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<CameraFrameValue> {
    let runtime_id = host_session_id(binding, operation)?;
    let mut header = AndroidHostCameraFrameHeader::default();
    let mut bytes = vec![0u8; INITIAL_CAMERA_FRAME_CAPACITY];

    loop {
        let byte_capacity = checked_u32_length(bytes.len(), operation, "photo bytes")?;
        let mut bytes_written = 0u32;
        let status = unsafe {
            destack_host_android_camera_stream_take_photo(
                runtime_id,
                stream_id,
                timeout_ns,
                &mut header,
                NativeSlice {
                    data: bytes.as_mut_ptr(),
                    len: byte_capacity,
                },
                &mut bytes_written,
            )
        };

        // grow the photo buffer when the host reports truncation
        if status == HostStatus::BufferTooSmall.code() {
            let required_bytes = (header.bytes_len as usize).max(bytes_written as usize);
            grow_frame_buffer(&mut bytes, required_bytes, operation)?;
            continue;
        }

        host_status_result(status, operation, "camera photo capture")?;

        let byte_count = bytes_written as usize;
        if byte_count > bytes.len() {
            return Err(invalid_data(
                operation,
                "android host camera photo capture reported one oversized payload",
            ));
        }

        bytes.truncate(byte_count);
        return decode_camera_frame(&header, bytes, operation);
    }
}
