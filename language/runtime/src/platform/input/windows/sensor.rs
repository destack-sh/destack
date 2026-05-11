use super::{core as input_core, raw as raw_input};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{
    InputSensorConfig, InputSensorDescriptor, InputSensorEffectiveConfig, InputSensorKind,
    InputSensorSample, validation as input_validation,
};
use crate::platform::{NativeArray, PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Resolve one opened sensor-capable device descriptor.
fn resolve_sensor_device(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<raw_input::RawInputDeviceDescriptor> {
    // resolve one opened raw device descriptor
    let device = input_core::raw_device(binding, handle, operation)?;

    // reject non-sensor devices for sensor operations
    if !device.supports_sensors {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(device)
}

/// Resolve one requested sensor kind against one raw-input descriptor.
fn resolve_sensor_kind(
    device: &raw_input::RawInputDeviceDescriptor,
    requested: InputSensorKind,
    operation: &'static str,
) -> RuntimeResult<InputSensorKind> {
    let supported = raw_input::sensor_kinds_for_device(device);
    if supported.is_empty() {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    if !supported.contains(&requested) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "kind",
            "sensor kind is not supported by this device",
        ))
        .boxed());
    }

    Ok(requested)
}

/// Configure one sensor stream.
pub(crate) unsafe fn destack_input_sensor_configure(
    binding: &BindingCallContext,
    out: *mut InputSensorEffectiveConfig,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
    config: InputSensorConfig,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve one sensor-capable opened device
    let device = resolve_sensor_device(binding, handle, "destack.input.sensor.configure")?;
    let sensor_kind = resolve_sensor_kind(&device, kind, "destack.input.sensor.configure")?;

    // validate one coherent sensor-stream configuration
    input_validation::validate_sensor_config(config)?;

    // project one stable effective configuration for this stream lane
    let effective = InputSensorEffectiveConfig {
        enabled: config.enabled,
        sample_rate_hz: if config.enabled {
            config.sample_rate_hz
        } else {
            0.0
        },
        batch_latency_ms: if config.enabled {
            config.batch_latency_ms
        } else {
            0
        },
        flags: if config.enabled { config.flags } else { 0 },
    };

    // persist effective stream configuration for follow-up reads
    input_core::set_sensor_stream_config(
        binding,
        handle,
        sensor_kind,
        effective,
        "destack.input.sensor.configure",
    )?;

    // return one backend-projected effective stream configuration
    unsafe {
        *out = effective;
    }

    Ok(())
}

/// List supported sensors for one opened input device.
pub(crate) unsafe fn destack_input_sensor_list(
    binding: &BindingCallContext,
    out: *mut NativeArray<InputSensorDescriptor>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate capability support for this handle
    // resolve one sensor-capable opened device
    let device = resolve_sensor_device(binding, handle, "destack.input.sensor.list")?;
    let infos = raw_input::sensor_infos_for_device(&device);
    if infos.is_empty() {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.input.sensor.list")).boxed(),
        );
    }

    // publish one backend-probed sensor capability descriptor for this device
    unsafe {
        *out = binding.store_array(infos);
    }

    Ok(())
}

/// Read one sensor sample.
pub(crate) unsafe fn destack_input_sensor_read(
    binding: &BindingCallContext,
    out: *mut InputSensorSample,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate capability support for this handle
    // resolve one sensor-capable opened device and requested sensor lane
    let device = resolve_sensor_device(binding, handle, "destack.input.sensor.read")?;
    let sensor_kind = resolve_sensor_kind(&device, kind, "destack.input.sensor.read")?;

    // require explicit stream enable before blocking sensor reads
    if !input_core::is_sensor_stream_enabled(
        binding,
        handle,
        sensor_kind,
        "destack.input.sensor.read",
    )? {
        return Err(RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoWouldBlock),
            None,
            None,
            Some("destack.input.sensor.read".to_string()),
            None,
            "sensor stream is not enabled".to_string(),
        ))
        .boxed());
    }

    // read one sample from the raw-hid sensor queue
    let sample = raw_input::read_sensor_sample(
        binding,
        &device,
        sensor_kind,
        false,
        "destack.input.sensor.read",
    )?;
    unsafe {
        *out = sample;
    }

    Ok(())
}

/// Poll one sensor sample without blocking.
pub(crate) unsafe fn destack_input_sensor_try_read(
    binding: &BindingCallContext,
    out: *mut InputSensorSample,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate capability support for this handle
    // resolve one sensor-capable opened device and requested sensor lane
    let device = resolve_sensor_device(binding, handle, "destack.input.sensor.tryRead")?;
    let sensor_kind = resolve_sensor_kind(&device, kind, "destack.input.sensor.tryRead")?;

    // require explicit stream enable before nonblocking sensor polls
    if !input_core::is_sensor_stream_enabled(
        binding,
        handle,
        sensor_kind,
        "destack.input.sensor.tryRead",
    )? {
        return Err(RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoWouldBlock),
            None,
            None,
            Some("destack.input.sensor.tryRead".to_string()),
            None,
            "sensor stream is not enabled".to_string(),
        ))
        .boxed());
    }

    // poll one sample from the raw-hid sensor queue without blocking
    let sample = raw_input::read_sensor_sample(
        binding,
        &device,
        sensor_kind,
        true,
        "destack.input.sensor.tryRead",
    )?;
    unsafe {
        *out = sample;
    }

    Ok(())
}
