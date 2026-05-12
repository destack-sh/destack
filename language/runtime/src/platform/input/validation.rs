use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
#[cfg(unix)]
use crate::platform::input::InputWindowTarget;
use crate::platform::input::{InputHapticEffectParameters, InputSensorConfig};

/// Maximum event count accepted by one input batch read call.
pub(crate) const MAX_READ_BATCH_EVENTS: u32 = 4_096;
/// Maximum payload size accepted for one raw-hid read call.
pub(crate) const MAX_RAW_HID_BYTES: u32 = 65_536;

/// Validate one haptics parameter payload.
pub(crate) fn validate_haptics_params(params: InputHapticEffectParameters) -> RuntimeResult<()> {
    // validate finite and normalized rumble magnitudes
    for (field, value) in [
        ("params.strongMagnitude", params.strong_magnitude),
        ("params.weakMagnitude", params.weak_magnitude),
        ("params.leftTrigger", params.left_trigger),
        ("params.rightTrigger", params.right_trigger),
    ] {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                field,
                "haptics magnitudes must be finite and within [0, 1]",
            ))
            .boxed());
        }
    }

    Ok(())
}

/// Validate one max-bytes argument for raw-hid reads.
pub(crate) fn validate_raw_hid_max_bytes(maxbytes: u32) -> RuntimeResult<()> {
    // require one nonzero read budget
    if maxbytes == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "maxbytes",
            "maxbytes must be greater than zero",
        ))
        .boxed());
    }

    // bound raw-hid allocations and copy budget
    if maxbytes > MAX_RAW_HID_BYTES {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "maxbytes",
            "maxbytes exceeds supported raw-hid payload limit",
        ))
        .boxed());
    }

    Ok(())
}

/// Validate one byte-slice payload used by one input binding.
pub(crate) fn validate_non_empty_bytes(field: &'static str, payload: &[u8]) -> RuntimeResult<()> {
    // require payload bytes for report writes
    if payload.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "payload must contain at least one byte",
        ))
        .boxed());
    }

    Ok(())
}

/// Validate one requested sensor sample rate value.
pub(crate) fn validate_sensor_sample_rate_hz(sample_rate_hz: f64) -> RuntimeResult<()> {
    // require finite, non-negative sample rates
    if !sample_rate_hz.is_finite() || sample_rate_hz < 0.0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "config.sampleRateHz",
            "config.sampleRateHz must be finite and non-negative",
        ))
        .boxed());
    }

    Ok(())
}

/// Validate one requested sensor stream configuration.
pub(crate) fn validate_sensor_config(config: InputSensorConfig) -> RuntimeResult<()> {
    // validate sample-rate shape first
    validate_sensor_sample_rate_hz(config.sample_rate_hz)?;

    // require one positive sample rate for enabled streams
    if config.enabled && config.sample_rate_hz <= 0.0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "config.sampleRateHz",
            "config.sampleRateHz must be greater than zero when the sensor stream is enabled",
        ))
        .boxed());
    }

    Ok(())
}

/// Validate one pointer coordinate pair for warp operations.
pub(crate) fn validate_pointer_coordinates(x: f64, y: f64) -> RuntimeResult<()> {
    // require finite target coordinates
    if !x.is_finite() || !y.is_finite() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "x",
            "x and y must be finite",
        ))
        .boxed());
    }

    Ok(())
}

/// Return whether one window target is explicitly scoped.
#[cfg(unix)]
pub(crate) fn has_explicit_window_target(target: InputWindowTarget) -> bool {
    target.window.is_some_and(|window| window.0.local_id != 0)
}

/// Validate one global-only target requirement.
#[cfg(unix)]
pub(crate) fn validate_global_window_target(
    target: InputWindowTarget,
    operation: &'static str,
) -> RuntimeResult<()> {
    // reject explicit window-scoped targets on global-only backends
    if has_explicit_window_target(target) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(())
}

/// Validate one max-events argument for batch reads and return one usize capacity.
pub(crate) fn validate_read_batch_maxevents(maxevents: u32) -> RuntimeResult<usize> {
    // require one nonzero batch size
    if maxevents == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "maxevents",
            "maxevents must be greater than zero",
        ))
        .boxed());
    }

    // bound one call-local batch allocation
    if maxevents > MAX_READ_BATCH_EVENTS {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "maxevents",
            "maxevents exceeds supported input batch limit",
        ))
        .boxed());
    }

    Ok(maxevents as usize)
}
