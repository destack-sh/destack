use super::{
    AndroidActivityLifecycle, android_notify_activity_lifecycle,
    android_notify_interruption_changed, android_notify_memory_pressure_changed,
    android_notify_permission_request_in_flight, android_notify_permission_result,
    android_notify_power_mode_changed, android_notify_thermal_state_changed, android_notify_wake,
    android_notify_wall_clock_changed, android_notify_window_available,
    android_notify_window_focus_changed, android_notify_window_resized,
    android_notify_window_terminated,
};
use crate::diagnostic::{RuntimeError, RuntimeResult, RuntimeStatus};
use crate::platform::{NativeStringRef, PlatformError};
use crate::runtime::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};

/// Android lifecycle code for `onCreate`.
const ANDROID_LIFECYCLE_CREATED: u32 = 0;
/// Android lifecycle code for `onStart`.
const ANDROID_LIFECYCLE_STARTED: u32 = 1;
/// Android lifecycle code for `onResume`.
const ANDROID_LIFECYCLE_RESUMED: u32 = 2;
/// Android lifecycle code for `onPause`.
const ANDROID_LIFECYCLE_PAUSED: u32 = 3;
/// Android lifecycle code for `onStop`.
const ANDROID_LIFECYCLE_STOPPED: u32 = 4;
/// Android lifecycle code for `onDestroy`.
const ANDROID_LIFECYCLE_DESTROYED: u32 = 5;
/// Android memory pressure code for normal state.
const ANDROID_MEMORY_PRESSURE_NORMAL: u32 = 0;
/// Android memory pressure code for warning state.
const ANDROID_MEMORY_PRESSURE_WARNING: u32 = 1;
/// Android memory pressure code for critical state.
const ANDROID_MEMORY_PRESSURE_CRITICAL: u32 = 2;
/// Android thermal code for nominal state.
const ANDROID_THERMAL_NOMINAL: u32 = 0;
/// Android thermal code for fair state.
const ANDROID_THERMAL_FAIR: u32 = 1;
/// Android thermal code for serious state.
const ANDROID_THERMAL_SERIOUS: u32 = 2;
/// Android thermal code for critical state.
const ANDROID_THERMAL_CRITICAL: u32 = 3;
/// Android power mode code for normal state.
const ANDROID_POWER_MODE_NORMAL: u32 = 0;
/// Android power mode code for low power state.
const ANDROID_POWER_MODE_LOW_POWER: u32 = 1;

/// Notify the runtime host bridge about one Android activity lifecycle transition.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_notify_activity_lifecycle(
    runtime_id: u64,
    lifecycle_code: u32,
) -> RuntimeStatus {
    let result = decode_android_activity_lifecycle(lifecycle_code)
        .and_then(|lifecycle| android_notify_activity_lifecycle(runtime_id, lifecycle));

    runtime_status(result)
}

/// Notify the runtime host bridge that one Android window became available.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_notify_window_available(
    runtime_id: u64,
) -> RuntimeStatus {
    runtime_status(android_notify_window_available(runtime_id))
}

/// Notify the runtime host bridge that one Android window terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_notify_window_terminated(
    runtime_id: u64,
) -> RuntimeStatus {
    runtime_status(android_notify_window_terminated(runtime_id))
}

/// Notify the runtime host bridge that one Android window resized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_notify_window_resized(
    runtime_id: u64,
    width_px: u32,
    height_px: u32,
) -> RuntimeStatus {
    runtime_status(android_notify_window_resized(
        runtime_id, width_px, height_px,
    ))
}

/// Notify the runtime host bridge that one Android window focus changed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_notify_window_focus_changed(
    runtime_id: u64,
    is_focused: bool,
) -> RuntimeStatus {
    runtime_status(android_notify_window_focus_changed(runtime_id, is_focused))
}

/// Notify the runtime host bridge that one Android permission request changed state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_notify_permission_request_in_flight(
    runtime_id: u64,
    permission: NativeStringRef,
    is_in_flight: bool,
) -> RuntimeStatus {
    let result = decode_permission_name(permission).and_then(|permission| {
        android_notify_permission_request_in_flight(runtime_id, permission.as_str(), is_in_flight)
    });

    runtime_status(result)
}

/// Notify the runtime host bridge with one Android permission result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_notify_permission_result(
    runtime_id: u64,
    permission: NativeStringRef,
    granted: bool,
) -> RuntimeStatus {
    let result = decode_permission_name(permission).and_then(|permission| {
        android_notify_permission_result(runtime_id, permission.as_str(), granted)
    });

    runtime_status(result)
}

/// Notify the runtime host bridge that interruption state changed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeStatus {
    runtime_status(android_notify_interruption_changed(runtime_id, interrupted))
}

/// Notify the runtime host bridge that memory pressure changed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_notify_memory_pressure_changed(
    runtime_id: u64,
    level_code: u32,
) -> RuntimeStatus {
    let result = decode_android_memory_pressure_level(level_code)
        .and_then(|level| android_notify_memory_pressure_changed(runtime_id, level));

    runtime_status(result)
}

/// Notify the runtime host bridge that thermal state changed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_notify_thermal_state_changed(
    runtime_id: u64,
    thermal_code: u32,
) -> RuntimeStatus {
    let result = decode_android_thermal_state(thermal_code)
        .and_then(|state| android_notify_thermal_state_changed(runtime_id, state));

    runtime_status(result)
}

/// Notify the runtime host bridge that power mode changed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_notify_power_mode_changed(
    runtime_id: u64,
    power_mode_code: u32,
) -> RuntimeStatus {
    let result = decode_android_power_mode(power_mode_code)
        .and_then(|mode| android_notify_power_mode_changed(runtime_id, mode));

    runtime_status(result)
}

/// Notify the runtime host bridge that wall clock changed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_notify_wall_clock_changed(
    runtime_id: u64,
) -> RuntimeStatus {
    runtime_status(android_notify_wall_clock_changed(runtime_id))
}

/// Wake one blocked host event poll operation for Android.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_notify_wake(
    runtime_id: u64,
) -> RuntimeStatus {
    runtime_status(android_notify_wake(runtime_id))
}

/// Convert one android lifecycle code into the runtime lifecycle enum.
fn decode_android_activity_lifecycle(
    lifecycle_code: u32,
) -> RuntimeResult<AndroidActivityLifecycle> {
    let lifecycle = match lifecycle_code {
        ANDROID_LIFECYCLE_CREATED => AndroidActivityLifecycle::Created,
        ANDROID_LIFECYCLE_STARTED => AndroidActivityLifecycle::Started,
        ANDROID_LIFECYCLE_RESUMED => AndroidActivityLifecycle::Resumed,
        ANDROID_LIFECYCLE_PAUSED => AndroidActivityLifecycle::Paused,
        ANDROID_LIFECYCLE_STOPPED => AndroidActivityLifecycle::Stopped,
        ANDROID_LIFECYCLE_DESTROYED => AndroidActivityLifecycle::Destroyed,
        _ => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "lifecycle_code",
                "invalid android lifecycle code",
            ))
            .boxed());
        }
    };

    Ok(lifecycle)
}

/// Convert one android memory pressure code into the runtime memory pressure enum.
fn decode_android_memory_pressure_level(level_code: u32) -> RuntimeResult<HostMemoryPressureLevel> {
    let level = match level_code {
        ANDROID_MEMORY_PRESSURE_NORMAL => HostMemoryPressureLevel::Normal,
        ANDROID_MEMORY_PRESSURE_WARNING => HostMemoryPressureLevel::Warning,
        ANDROID_MEMORY_PRESSURE_CRITICAL => HostMemoryPressureLevel::Critical,
        _ => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "level_code",
                "invalid android memory pressure level code",
            ))
            .boxed());
        }
    };

    Ok(level)
}

/// Convert one android thermal code into the runtime thermal enum.
fn decode_android_thermal_state(thermal_code: u32) -> RuntimeResult<HostThermalState> {
    let state = match thermal_code {
        ANDROID_THERMAL_NOMINAL => HostThermalState::Nominal,
        ANDROID_THERMAL_FAIR => HostThermalState::Fair,
        ANDROID_THERMAL_SERIOUS => HostThermalState::Serious,
        ANDROID_THERMAL_CRITICAL => HostThermalState::Critical,
        _ => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "thermal_code",
                "invalid android thermal state code",
            ))
            .boxed());
        }
    };

    Ok(state)
}

/// Convert one android power mode code into the runtime power mode enum.
fn decode_android_power_mode(power_mode_code: u32) -> RuntimeResult<HostPowerMode> {
    let mode = match power_mode_code {
        ANDROID_POWER_MODE_NORMAL => HostPowerMode::Normal,
        ANDROID_POWER_MODE_LOW_POWER => HostPowerMode::LowPower,
        _ => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "power_mode_code",
                "invalid android power mode code",
            ))
            .boxed());
        }
    };

    Ok(mode)
}

/// Decode one permission string reference.
fn decode_permission_name(permission: NativeStringRef) -> RuntimeResult<String> {
    unsafe { permission.as_str() }.map(|permission| permission.to_string())
}

/// Convert one runtime result into one native status value.
fn runtime_status(result: RuntimeResult<()>) -> RuntimeStatus {
    RuntimeStatus::from_result(result, None)
}

#[cfg(test)]
mod tests {
    use super::{
        ANDROID_LIFECYCLE_CREATED, ANDROID_LIFECYCLE_DESTROYED, ANDROID_LIFECYCLE_PAUSED,
        ANDROID_LIFECYCLE_RESUMED, ANDROID_LIFECYCLE_STARTED, ANDROID_LIFECYCLE_STOPPED,
        ANDROID_MEMORY_PRESSURE_CRITICAL, ANDROID_MEMORY_PRESSURE_NORMAL,
        ANDROID_MEMORY_PRESSURE_WARNING, ANDROID_POWER_MODE_LOW_POWER, ANDROID_POWER_MODE_NORMAL,
        ANDROID_THERMAL_CRITICAL, ANDROID_THERMAL_FAIR, ANDROID_THERMAL_NOMINAL,
        ANDROID_THERMAL_SERIOUS, decode_android_activity_lifecycle,
        decode_android_memory_pressure_level, decode_android_power_mode,
        decode_android_thermal_state,
    };
    use crate::runtime::host::{
        AndroidActivityLifecycle, HostMemoryPressureLevel, HostPowerMode, HostThermalState,
    };

    #[test]
    fn test_decode_lifecycle_created() {
        let lifecycle = decode_android_activity_lifecycle(ANDROID_LIFECYCLE_CREATED).unwrap();
        assert_eq!(lifecycle, AndroidActivityLifecycle::Created);
    }

    #[test]
    fn test_decode_lifecycle_resumed() {
        let lifecycle = decode_android_activity_lifecycle(ANDROID_LIFECYCLE_RESUMED).unwrap();
        assert_eq!(lifecycle, AndroidActivityLifecycle::Resumed);
    }

    #[test]
    fn test_decode_lifecycle_started() {
        let lifecycle = decode_android_activity_lifecycle(ANDROID_LIFECYCLE_STARTED).unwrap();
        assert_eq!(lifecycle, AndroidActivityLifecycle::Started);
    }

    #[test]
    fn test_decode_lifecycle_paused() {
        let lifecycle = decode_android_activity_lifecycle(ANDROID_LIFECYCLE_PAUSED).unwrap();
        assert_eq!(lifecycle, AndroidActivityLifecycle::Paused);
    }

    #[test]
    fn test_decode_lifecycle_stopped() {
        let lifecycle = decode_android_activity_lifecycle(ANDROID_LIFECYCLE_STOPPED).unwrap();
        assert_eq!(lifecycle, AndroidActivityLifecycle::Stopped);
    }

    #[test]
    fn test_decode_lifecycle_destroyed() {
        let lifecycle = decode_android_activity_lifecycle(ANDROID_LIFECYCLE_DESTROYED).unwrap();
        assert_eq!(lifecycle, AndroidActivityLifecycle::Destroyed);
    }

    #[test]
    fn test_decode_lifecycle_rejects_unknown_code() {
        let result = decode_android_activity_lifecycle(u32::MAX);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_memory_pressure_codes() {
        assert_eq!(
            decode_android_memory_pressure_level(ANDROID_MEMORY_PRESSURE_NORMAL).unwrap(),
            HostMemoryPressureLevel::Normal
        );
        assert_eq!(
            decode_android_memory_pressure_level(ANDROID_MEMORY_PRESSURE_WARNING).unwrap(),
            HostMemoryPressureLevel::Warning
        );
        assert_eq!(
            decode_android_memory_pressure_level(ANDROID_MEMORY_PRESSURE_CRITICAL).unwrap(),
            HostMemoryPressureLevel::Critical
        );
        assert!(decode_android_memory_pressure_level(u32::MAX).is_err());
    }

    #[test]
    fn test_decode_thermal_codes() {
        assert_eq!(
            decode_android_thermal_state(ANDROID_THERMAL_NOMINAL).unwrap(),
            HostThermalState::Nominal
        );
        assert_eq!(
            decode_android_thermal_state(ANDROID_THERMAL_FAIR).unwrap(),
            HostThermalState::Fair
        );
        assert_eq!(
            decode_android_thermal_state(ANDROID_THERMAL_SERIOUS).unwrap(),
            HostThermalState::Serious
        );
        assert_eq!(
            decode_android_thermal_state(ANDROID_THERMAL_CRITICAL).unwrap(),
            HostThermalState::Critical
        );
        assert!(decode_android_thermal_state(u32::MAX).is_err());
    }

    #[test]
    fn test_decode_power_mode_codes() {
        assert_eq!(
            decode_android_power_mode(ANDROID_POWER_MODE_NORMAL).unwrap(),
            HostPowerMode::Normal
        );
        assert_eq!(
            decode_android_power_mode(ANDROID_POWER_MODE_LOW_POWER).unwrap(),
            HostPowerMode::LowPower
        );
        assert!(decode_android_power_mode(u32::MAX).is_err());
    }
}
