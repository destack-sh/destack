use super::{core as input_core, raw as raw_input};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{
    InputSensorConfig, InputSensorEffectiveConfig, InputSensorInfo, InputSensorKind,
    InputSensorSample, validation as input_validation,
};
use crate::platform::{NativeArray, PlatformError, resource};
use crate::runtime::RuntimeCallContext;

/// Resolve one opened sensor-capable device descriptor.
fn resolve_sensor_device(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<raw_input::RawInputDeviceDescriptor> {
    // resolve one opened raw device descriptor
    let device = input_core::raw_device(context, handle, operation)?;

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
///
/// Apply one enable and sample-rate configuration for one sensor stream on one opened input device.
/// Backends can negotiate one effective sample rate and one effective batching latency.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where sensor stream configuration is unavailable.
/// Uses backend-specific sensor configuration APIs on Unix and Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_sensor_configure(
    context: &RuntimeCallContext,
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
    let device = resolve_sensor_device(context, handle, "destack.input.sensor.configure")?;
    let sensor_kind = resolve_sensor_kind(&device, kind, "destack.input.sensor.configure")?;

    // clamp invalid requested rates into one explicit invalid-argument error
    input_validation::validate_sensor_sample_rate_hz(config.sample_rate_hz)?;

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
        context,
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
///
/// Return sensor capability metadata for one opened sensor-capable device.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where sensor streams are unavailable.
/// Uses backend-specific sensor capability tables from evdev and hidraw class stacks on Unix.
/// Uses HID sensor or controller APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_sensor_list(
    context: &RuntimeCallContext,
    out: *mut NativeArray<InputSensorInfo>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate capability support for this handle
    // resolve one sensor-capable opened device
    let device = resolve_sensor_device(context, handle, "destack.input.sensor.list")?;
    let infos = raw_input::sensor_infos_for_device(&device);
    if infos.is_empty() {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.input.sensor.list")).boxed(),
        );
    }

    // publish one backend-probed sensor capability descriptor for this device
    unsafe {
        *out = context.store_array(infos);
    }

    Ok(())
}

/// Read one sensor sample.
///
/// Read one pending sample from one configured sensor stream.
/// Timeout and blocking behavior follow backend stream semantics.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where sensor streams are unavailable.
/// Uses backend-specific blocking sensor queue reads on Unix and Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_sensor_read(
    context: &RuntimeCallContext,
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
    let device = resolve_sensor_device(context, handle, "destack.input.sensor.read")?;
    let sensor_kind = resolve_sensor_kind(&device, kind, "destack.input.sensor.read")?;

    // require explicit stream enable before blocking sensor reads
    if !input_core::is_sensor_stream_enabled(
        context,
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
    let sample =
        raw_input::read_sensor_sample(&device, sensor_kind, false, "destack.input.sensor.read")?;
    unsafe {
        *out = sample;
    }

    Ok(())
}

/// Poll one sensor sample without blocking.
///
/// Poll one pending sample from one configured sensor stream and return immediately when none is available.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where sensor streams are unavailable.
/// Uses backend-specific nonblocking sensor queue reads on Unix and Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_sensor_try_read(
    context: &RuntimeCallContext,
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
    let device = resolve_sensor_device(context, handle, "destack.input.sensor.tryRead")?;
    let sensor_kind = resolve_sensor_kind(&device, kind, "destack.input.sensor.tryRead")?;

    // require explicit stream enable before nonblocking sensor polls
    if !input_core::is_sensor_stream_enabled(
        context,
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
    let sample =
        raw_input::read_sensor_sample(&device, sensor_kind, true, "destack.input.sensor.tryRead")?;
    unsafe {
        *out = sample;
    }

    Ok(())
}
