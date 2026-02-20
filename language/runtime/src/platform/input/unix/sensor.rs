use super::core as input_core;
#[cfg(target_os = "linux")]
use super::linux as input_linux;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{
    InputSensorConfig, InputSensorDescriptor, InputSensorEffectiveConfig, InputSensorKind,
    InputSensorSample, validation as input_validation,
};
use crate::platform::{NativeArray, PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Resolve one opened sensor-capable Unix binding.
fn resolve_sensor_binding(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<input_core::UnixInputBinding> {
    // validate one opened unix input binding
    let binding = input_core::resolve_unix_input_binding(context, handle, operation)?;

    // require platform-backed descriptor lanes for sensor streams
    if binding.backend != input_core::UnixInputBackend::Platform || binding.descriptor.is_none() {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(binding)
}

/// Return sensor capability metadata for one opened Unix binding.
fn sensor_infos_for_binding(
    binding: &input_core::UnixInputBinding,
) -> RuntimeResult<Vec<InputSensorDescriptor>> {
    #[cfg(target_os = "linux")]
    {
        let Some(descriptor) = binding.descriptor else {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.input.sensor.list",
            ))
            .boxed());
        };

        let infos = input_linux::linux_sensor_infos(descriptor, binding.device_kind);
        return Ok(infos);
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = binding;
        Ok(Vec::new())
    }
}

/// Validate one requested sensor lane for one opened Unix binding.
fn validate_sensor_kind(
    binding: &input_core::UnixInputBinding,
    kind: InputSensorKind,
) -> RuntimeResult<()> {
    // validate requested kind against backend capability metadata
    let infos = sensor_infos_for_binding(binding)?;
    if infos.iter().all(|info| info.kind != kind) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "kind",
            "sensor kind is not supported by this device",
        ))
        .boxed());
    }

    Ok(())
}

/// Build one effective sensor stream configuration from one requested config.
fn effective_sensor_config(config: InputSensorConfig) -> InputSensorEffectiveConfig {
    InputSensorEffectiveConfig {
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
    }
}

/// Return io-would-block when one sensor stream is disabled.
fn disabled_sensor_stream(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoWouldBlock),
        None,
        None,
        Some(operation.to_string()),
        None,
        "sensor stream is not enabled".to_string(),
    ))
    .boxed()
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
    context: &BindingCallContext,
    out: *mut InputSensorEffectiveConfig,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
    config: InputSensorConfig,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve one opened sensor-capable binding
    let binding = resolve_sensor_binding(context, handle, "destack.input.sensor.configure")?;

    // validate stream configuration and requested lane
    input_validation::validate_sensor_sample_rate_hz(config.sample_rate_hz)?;
    validate_sensor_kind(&binding, kind)?;

    // persist one effective configuration for this sensor lane
    let effective = effective_sensor_config(config);
    input_core::set_sensor_stream_config(
        context,
        handle,
        kind,
        effective,
        "destack.input.sensor.configure",
    )?;

    // write effective output payload
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
    context: &BindingCallContext,
    out: *mut NativeArray<InputSensorDescriptor>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve one opened sensor-capable binding
    let binding = resolve_sensor_binding(context, handle, "destack.input.sensor.list")?;

    // query backend-reported sensor capability metadata
    let infos = sensor_infos_for_binding(&binding)?;
    if infos.is_empty() {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.input.sensor.list")).boxed(),
        );
    }

    // write capability output payload
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
    context: &BindingCallContext,
    out: *mut InputSensorSample,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve one opened sensor-capable binding
    let binding = resolve_sensor_binding(context, handle, "destack.input.sensor.read")?;
    validate_sensor_kind(&binding, kind)?;

    // require explicit stream enable before blocking reads
    if !input_core::is_sensor_stream_enabled(context, handle, kind, "destack.input.sensor.read")? {
        return Err(disabled_sensor_stream("destack.input.sensor.read"));
    }

    // route sensor reads by backend support
    #[cfg(target_os = "linux")]
    {
        let Some(descriptor) = binding.descriptor else {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.input.sensor.read",
            ))
            .boxed());
        };

        let sample = input_linux::read_linux_sensor_sample(
            descriptor,
            binding.device_kind,
            kind,
            false,
            "destack.input.sensor.read",
        )?;
        unsafe {
            *out = sample;
        }

        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = binding;
        Err(RuntimeError::from(PlatformError::not_supported("destack.input.sensor.read")).boxed())
    }
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
    context: &BindingCallContext,
    out: *mut InputSensorSample,
    handle: resource::InputDeviceHandle,
    kind: InputSensorKind,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve one opened sensor-capable binding
    let binding = resolve_sensor_binding(context, handle, "destack.input.sensor.tryRead")?;
    validate_sensor_kind(&binding, kind)?;

    // require explicit stream enable before nonblocking reads
    if !input_core::is_sensor_stream_enabled(context, handle, kind, "destack.input.sensor.tryRead")?
    {
        return Err(disabled_sensor_stream("destack.input.sensor.tryRead"));
    }

    // route sensor reads by backend support
    #[cfg(target_os = "linux")]
    {
        let Some(descriptor) = binding.descriptor else {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.input.sensor.tryRead",
            ))
            .boxed());
        };

        let sample = input_linux::read_linux_sensor_sample(
            descriptor,
            binding.device_kind,
            kind,
            true,
            "destack.input.sensor.tryRead",
        )?;
        unsafe {
            *out = sample;
        }

        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = binding;
        Err(
            RuntimeError::from(PlatformError::not_supported("destack.input.sensor.tryRead"))
                .boxed(),
        )
    }
}
