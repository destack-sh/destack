use super::values::*;
use super::*;

/// Grow one header and string buffer after one `bufferTooSmall` result.
fn grow_device_list_buffers(
    headers: &mut Vec<AndroidHostCameraDeviceDescriptorHeader>,
    required_headers: usize,
    string_bytes: &mut Vec<u8>,
    required_string_bytes: usize,
    operation: &'static str,
) -> RuntimeResult<()> {
    let next_header_capacity = headers.len().max(required_headers).saturating_mul(2);
    let next_string_capacity = string_bytes
        .len()
        .max(required_string_bytes)
        .saturating_mul(2);

    if next_header_capacity > MAX_CAMERA_ROW_CAPACITY {
        return Err(invalid_data(
            operation,
            "android host camera device list exceeded the maximum descriptor count",
        ));
    }

    if next_string_capacity > MAX_CAMERA_STRING_CAPACITY {
        return Err(invalid_data(
            operation,
            "android host camera device list exceeded the maximum string payload size",
        ));
    }

    headers.resize(
        next_header_capacity,
        AndroidHostCameraDeviceDescriptorHeader::default(),
    );
    string_bytes.resize(next_string_capacity, 0);

    Ok(())
}

/// Read one vector of Android camera device descriptors.
pub(crate) fn read_camera_device_descriptors(
    binding: &BindingCallContext,
    operation: &'static str,
) -> RuntimeResult<Vec<CameraDeviceDescriptorValue>> {
    let runtime_id = host_runtime_id(binding, operation)?;
    let mut headers =
        vec![AndroidHostCameraDeviceDescriptorHeader::default(); INITIAL_CAMERA_ROW_CAPACITY];
    let mut string_bytes = vec![0u8; INITIAL_CAMERA_STRING_CAPACITY];

    loop {
        let header_capacity = checked_u32_length(headers.len(), operation, "device rows")?;
        let string_capacity = checked_u32_length(string_bytes.len(), operation, "string bytes")?;
        let mut header_count_written = 0u32;
        let mut string_bytes_written = 0u32;

        let status = unsafe {
            destack_host_android_camera_device_list(
                runtime_id,
                NativeSlice {
                    data: headers.as_mut_ptr(),
                    len: header_capacity,
                },
                &mut header_count_written,
                NativeSlice {
                    data: string_bytes.as_mut_ptr(),
                    len: string_capacity,
                },
                &mut string_bytes_written,
            )
        };

        // grow the buffers when the host reports truncation
        if status == HostStatus::BufferTooSmall.code() {
            grow_device_list_buffers(
                &mut headers,
                header_count_written as usize,
                &mut string_bytes,
                string_bytes_written as usize,
                operation,
            )?;
            continue;
        }

        host_status_result(status, operation, "camera device list")?;

        let header_count = header_count_written as usize;
        let string_count = string_bytes_written as usize;
        headers.truncate(header_count);
        string_bytes.truncate(string_count);

        let mut descriptors = Vec::with_capacity(headers.len());

        // decode each device row
        for header in &headers {
            descriptors.push(decode_camera_device_descriptor(
                header,
                &string_bytes,
                operation,
            )?);
        }

        return Ok(descriptors);
    }
}

/// Read one vector of Android camera stream configs.
pub(crate) fn read_camera_stream_configs(
    binding: &BindingCallContext,
    session_id: u64,
    operation: &'static str,
) -> RuntimeResult<Vec<CameraStreamConfigValue>> {
    let runtime_id = host_runtime_id(binding, operation)?;
    let mut headers =
        vec![AndroidHostCameraStreamConfigHeader::default(); INITIAL_CAMERA_ROW_CAPACITY];

    loop {
        let header_capacity = checked_u32_length(headers.len(), operation, "stream configs")?;
        let mut header_count_written = 0u32;

        let status = unsafe {
            destack_host_android_camera_device_stream_config_list(
                runtime_id,
                session_id,
                NativeSlice {
                    data: headers.as_mut_ptr(),
                    len: header_capacity,
                },
                &mut header_count_written,
            )
        };

        // grow the config buffer when the host reports truncation
        if status == HostStatus::BufferTooSmall.code() {
            let next_capacity = headers
                .len()
                .max(header_count_written as usize)
                .saturating_mul(2);
            if next_capacity > MAX_CAMERA_ROW_CAPACITY {
                return Err(invalid_data(
                    operation,
                    "android host camera stream config list exceeded the maximum row count",
                ));
            }

            headers.resize(
                next_capacity,
                AndroidHostCameraStreamConfigHeader::default(),
            );
            continue;
        }

        host_status_result(status, operation, "camera stream config list")?;

        let header_count = header_count_written as usize;
        headers.truncate(header_count);

        let mut configs = Vec::with_capacity(headers.len());

        // decode each stream config row
        for header in &headers {
            configs.push(decode_camera_stream_config(header, operation)?);
        }

        return Ok(configs);
    }
}

/// Read one vector of Android camera stream capabilities.
pub(crate) fn read_camera_stream_capabilities(
    binding: &BindingCallContext,
    session_id: u64,
    operation: &'static str,
) -> RuntimeResult<Vec<CameraStreamCapabilityValue>> {
    let runtime_id = host_runtime_id(binding, operation)?;
    let mut headers =
        vec![AndroidHostCameraStreamCapabilityHeader::default(); INITIAL_CAMERA_ROW_CAPACITY];

    loop {
        let header_capacity = checked_u32_length(headers.len(), operation, "stream capabilities")?;
        let mut header_count_written = 0u32;

        let status = unsafe {
            destack_host_android_camera_device_stream_capability_list(
                runtime_id,
                session_id,
                NativeSlice {
                    data: headers.as_mut_ptr(),
                    len: header_capacity,
                },
                &mut header_count_written,
            )
        };

        // grow the capability buffer when the host reports truncation
        if status == HostStatus::BufferTooSmall.code() {
            let next_capacity = headers
                .len()
                .max(header_count_written as usize)
                .saturating_mul(2);
            if next_capacity > MAX_CAMERA_ROW_CAPACITY {
                return Err(invalid_data(
                    operation,
                    "android host camera stream capability list exceeded the maximum row count",
                ));
            }

            headers.resize(
                next_capacity,
                AndroidHostCameraStreamCapabilityHeader::default(),
            );
            continue;
        }

        host_status_result(status, operation, "camera stream capability list")?;

        let header_count = header_count_written as usize;
        headers.truncate(header_count);

        let mut capabilities = Vec::with_capacity(headers.len());

        // decode each stream capability row
        for header in &headers {
            capabilities.push(decode_camera_stream_capability(header, operation)?);
        }

        return Ok(capabilities);
    }
}

/// Read one current Android camera stream config.
pub(crate) fn current_camera_stream_config(
    binding: &BindingCallContext,
    stream_id: u64,
    operation: &'static str,
) -> RuntimeResult<CameraStreamConfigValue> {
    let runtime_id = host_runtime_id(binding, operation)?;
    let mut header = AndroidHostCameraStreamConfigHeader::default();

    let status =
        unsafe { destack_host_android_camera_stream_config(runtime_id, stream_id, &mut header) };
    host_status_result(status, operation, "camera stream config")?;

    decode_camera_stream_config(&header, operation)
}
