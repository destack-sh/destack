use super::*;

/// Read one `u64` Android camera control value.
pub(crate) fn camera_get_u64(
    binding: &BindingCallContext,
    stream_id: u64,
    selector: u32,
    operation: &'static str,
) -> RuntimeResult<u64> {
    let runtime_id = host_runtime_id(binding, operation)?;
    let mut value = 0u64;

    let status = unsafe {
        destack_host_android_camera_stream_get_u64(runtime_id, stream_id, selector, &mut value)
    };
    host_status_result(status, operation, "camera control read")?;

    Ok(value)
}

/// Read one `u32` Android camera control value.
pub(crate) fn camera_get_u32(
    binding: &BindingCallContext,
    stream_id: u64,
    selector: u32,
    operation: &'static str,
) -> RuntimeResult<u32> {
    let runtime_id = host_runtime_id(binding, operation)?;
    let mut value = 0u32;

    let status = unsafe {
        destack_host_android_camera_stream_get_u32(runtime_id, stream_id, selector, &mut value)
    };
    host_status_result(status, operation, "camera control read")?;

    Ok(value)
}

/// Read one `f64` Android camera control value.
pub(crate) fn camera_get_f64(
    binding: &BindingCallContext,
    stream_id: u64,
    selector: u32,
    operation: &'static str,
) -> RuntimeResult<f64> {
    let runtime_id = host_runtime_id(binding, operation)?;
    let mut value = 0.0f64;

    let status = unsafe {
        destack_host_android_camera_stream_get_f64(runtime_id, stream_id, selector, &mut value)
    };
    host_status_result(status, operation, "camera control read")?;

    Ok(value)
}

/// Write one `u64` Android camera control value.
pub(crate) fn camera_set_u64(
    binding: &BindingCallContext,
    stream_id: u64,
    selector: u32,
    value: u64,
    operation: &'static str,
) -> RuntimeResult<()> {
    let runtime_id = host_runtime_id(binding, operation)?;
    let status = unsafe {
        destack_host_android_camera_stream_set_u64(runtime_id, stream_id, selector, value)
    };

    host_status_result(status, operation, "camera control write")
}

/// Write one `u32` Android camera control value.
pub(crate) fn camera_set_u32(
    binding: &BindingCallContext,
    stream_id: u64,
    selector: u32,
    value: u32,
    operation: &'static str,
) -> RuntimeResult<()> {
    let runtime_id = host_runtime_id(binding, operation)?;
    let status = unsafe {
        destack_host_android_camera_stream_set_u32(runtime_id, stream_id, selector, value)
    };

    host_status_result(status, operation, "camera control write")
}

/// Write one `f64` Android camera control value.
pub(crate) fn camera_set_f64(
    binding: &BindingCallContext,
    stream_id: u64,
    selector: u32,
    value: f64,
    operation: &'static str,
) -> RuntimeResult<()> {
    let runtime_id = host_runtime_id(binding, operation)?;
    let status = unsafe {
        destack_host_android_camera_stream_set_f64(runtime_id, stream_id, selector, value)
    };

    host_status_result(status, operation, "camera control write")
}

/// Read one `f64` Android camera control range.
pub(crate) fn camera_range_f64(
    binding: &BindingCallContext,
    stream_id: u64,
    selector: u32,
    operation: &'static str,
) -> RuntimeResult<(f64, f64, f64)> {
    let runtime_id = host_runtime_id(binding, operation)?;
    let mut minimum = 0.0f64;
    let mut maximum = 0.0f64;
    let mut step = 0.0f64;

    let status = unsafe {
        destack_host_android_camera_stream_get_range_f64(
            runtime_id,
            stream_id,
            selector,
            &mut minimum,
            &mut maximum,
            &mut step,
        )
    };
    host_status_result(status, operation, "camera control range")?;

    Ok((minimum, maximum, step))
}

/// Read one `u64` Android camera control range.
pub(crate) fn camera_range_u64(
    binding: &BindingCallContext,
    stream_id: u64,
    selector: u32,
    operation: &'static str,
) -> RuntimeResult<(u64, u64, u64)> {
    let runtime_id = host_runtime_id(binding, operation)?;
    let mut minimum = 0u64;
    let mut maximum = 0u64;
    let mut step = 0u64;

    let status = unsafe {
        destack_host_android_camera_stream_get_range_u64(
            runtime_id,
            stream_id,
            selector,
            &mut minimum,
            &mut maximum,
            &mut step,
        )
    };
    host_status_result(status, operation, "camera control range")?;

    Ok((minimum, maximum, step))
}

/// Read one `u32` Android camera control range.
pub(crate) fn camera_range_u32(
    binding: &BindingCallContext,
    stream_id: u64,
    selector: u32,
    operation: &'static str,
) -> RuntimeResult<(u32, u32, u32)> {
    let runtime_id = host_runtime_id(binding, operation)?;
    let mut minimum = 0u32;
    let mut maximum = 0u32;
    let mut step = 0u32;

    let status = unsafe {
        destack_host_android_camera_stream_get_range_u32(
            runtime_id,
            stream_id,
            selector,
            &mut minimum,
            &mut maximum,
            &mut step,
        )
    };
    host_status_result(status, operation, "camera control range")?;

    Ok((minimum, maximum, step))
}
