use super::{
    WindowsApplicationLifecycle, windows_notify_application_lifecycle,
    windows_notify_interruption_changed, windows_notify_memory_pressure_changed,
    windows_notify_permission_result, windows_notify_power_mode_changed,
    windows_notify_thermal_state_changed, windows_notify_wake, windows_notify_wall_clock_changed,
    windows_notify_window_available, windows_notify_window_focus_changed,
    windows_notify_window_resized, windows_notify_window_terminated,
};
use crate::diagnostic::{RuntimeError, RuntimeResult, RuntimeStatus};
use crate::platform::{NativeStringRef, PlatformError};
use crate::runtime::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};

/// Windows lifecycle code for app-created initialization.
const WINDOWS_LIFECYCLE_CREATED: u32 = 0;
/// Windows lifecycle code for app-activated foreground state.
const WINDOWS_LIFECYCLE_ACTIVATED: u32 = 1;
/// Windows lifecycle code for app-resumed state.
const WINDOWS_LIFECYCLE_RESUMED: u32 = 2;
/// Windows lifecycle code for app-suspended state.
const WINDOWS_LIFECYCLE_SUSPENDED: u32 = 3;
/// Windows lifecycle code for app-stopping transition.
const WINDOWS_LIFECYCLE_STOPPING: u32 = 4;
/// Windows lifecycle code for app-destroyed termination.
const WINDOWS_LIFECYCLE_DESTROYED: u32 = 5;
/// Windows memory pressure code for normal state.
const WINDOWS_MEMORY_PRESSURE_NORMAL: u32 = 0;
/// Windows memory pressure code for warning state.
const WINDOWS_MEMORY_PRESSURE_WARNING: u32 = 1;
/// Windows memory pressure code for critical state.
const WINDOWS_MEMORY_PRESSURE_CRITICAL: u32 = 2;
/// Windows thermal code for nominal state.
const WINDOWS_THERMAL_NOMINAL: u32 = 0;
/// Windows thermal code for fair state.
const WINDOWS_THERMAL_FAIR: u32 = 1;
/// Windows thermal code for serious state.
const WINDOWS_THERMAL_SERIOUS: u32 = 2;
/// Windows thermal code for critical state.
const WINDOWS_THERMAL_CRITICAL: u32 = 3;
/// Windows power mode code for normal state.
const WINDOWS_POWER_MODE_NORMAL: u32 = 0;
/// Windows power mode code for low power state.
const WINDOWS_POWER_MODE_LOW_POWER: u32 = 1;

/// Notify the runtime host bridge about one Windows application lifecycle transition.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_windows_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle_code: u32,
) -> RuntimeStatus {
    let result = decode_windows_application_lifecycle(lifecycle_code)
        .and_then(|lifecycle| windows_notify_application_lifecycle(runtime_id, lifecycle));

    runtime_status(result)
}

/// Notify the runtime host bridge that one Windows window became available.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_windows_notify_window_available(
    runtime_id: u64,
) -> RuntimeStatus {
    runtime_status(windows_notify_window_available(runtime_id))
}

/// Notify the runtime host bridge that one Windows window terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_windows_notify_window_terminated(
    runtime_id: u64,
) -> RuntimeStatus {
    runtime_status(windows_notify_window_terminated(runtime_id))
}

/// Notify the runtime host bridge that one Windows window resized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_windows_notify_window_resized(
    runtime_id: u64,
    width_px: u32,
    height_px: u32,
) -> RuntimeStatus {
    runtime_status(windows_notify_window_resized(
        runtime_id, width_px, height_px,
    ))
}

/// Notify the runtime host bridge that one Windows window focus changed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_windows_notify_window_focus_changed(
    runtime_id: u64,
    is_focused: bool,
) -> RuntimeStatus {
    runtime_status(windows_notify_window_focus_changed(runtime_id, is_focused))
}

/// Notify the runtime host bridge with one Windows permission result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_windows_notify_permission_result(
    runtime_id: u64,
    permission: NativeStringRef,
    granted: bool,
) -> RuntimeStatus {
    let result = decode_permission_name(permission).and_then(|permission| {
        windows_notify_permission_result(runtime_id, permission.as_str(), granted)
    });

    runtime_status(result)
}

/// Notify the runtime host bridge that interruption state changed on Windows.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_windows_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeStatus {
    runtime_status(windows_notify_interruption_changed(runtime_id, interrupted))
}

/// Notify the runtime host bridge that memory pressure changed on Windows.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_windows_notify_memory_pressure_changed(
    runtime_id: u64,
    level_code: u32,
) -> RuntimeStatus {
    let result = decode_windows_memory_pressure_level(level_code)
        .and_then(|level| windows_notify_memory_pressure_changed(runtime_id, level));

    runtime_status(result)
}

/// Notify the runtime host bridge that thermal state changed on Windows.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_windows_notify_thermal_state_changed(
    runtime_id: u64,
    thermal_code: u32,
) -> RuntimeStatus {
    let result = decode_windows_thermal_state(thermal_code)
        .and_then(|state| windows_notify_thermal_state_changed(runtime_id, state));

    runtime_status(result)
}

/// Notify the runtime host bridge that power mode changed on Windows.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_windows_notify_power_mode_changed(
    runtime_id: u64,
    power_mode_code: u32,
) -> RuntimeStatus {
    let result = decode_windows_power_mode(power_mode_code)
        .and_then(|mode| windows_notify_power_mode_changed(runtime_id, mode));

    runtime_status(result)
}

/// Notify the runtime host bridge that wall clock changed on Windows.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_windows_notify_wall_clock_changed(
    runtime_id: u64,
) -> RuntimeStatus {
    runtime_status(windows_notify_wall_clock_changed(runtime_id))
}

/// Wake one blocked host event poll operation for Windows.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_windows_notify_wake(
    runtime_id: u64,
) -> RuntimeStatus {
    runtime_status(windows_notify_wake(runtime_id))
}

/// Convert one Windows lifecycle code into the runtime lifecycle enum.
fn decode_windows_application_lifecycle(
    lifecycle_code: u32,
) -> RuntimeResult<WindowsApplicationLifecycle> {
    let lifecycle = match lifecycle_code {
        WINDOWS_LIFECYCLE_CREATED => WindowsApplicationLifecycle::Created,
        WINDOWS_LIFECYCLE_ACTIVATED => WindowsApplicationLifecycle::Activated,
        WINDOWS_LIFECYCLE_RESUMED => WindowsApplicationLifecycle::Resumed,
        WINDOWS_LIFECYCLE_SUSPENDED => WindowsApplicationLifecycle::Suspended,
        WINDOWS_LIFECYCLE_STOPPING => WindowsApplicationLifecycle::Stopping,
        WINDOWS_LIFECYCLE_DESTROYED => WindowsApplicationLifecycle::Destroyed,
        _ => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "lifecycle_code",
                "invalid windows lifecycle code",
            ))
            .boxed());
        }
    };

    Ok(lifecycle)
}

/// Convert one Windows memory pressure code into the runtime memory pressure enum.
fn decode_windows_memory_pressure_level(level_code: u32) -> RuntimeResult<HostMemoryPressureLevel> {
    let level = match level_code {
        WINDOWS_MEMORY_PRESSURE_NORMAL => HostMemoryPressureLevel::Normal,
        WINDOWS_MEMORY_PRESSURE_WARNING => HostMemoryPressureLevel::Warning,
        WINDOWS_MEMORY_PRESSURE_CRITICAL => HostMemoryPressureLevel::Critical,
        _ => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "level_code",
                "invalid windows memory pressure level code",
            ))
            .boxed());
        }
    };

    Ok(level)
}

/// Convert one Windows thermal code into the runtime thermal enum.
fn decode_windows_thermal_state(thermal_code: u32) -> RuntimeResult<HostThermalState> {
    let state = match thermal_code {
        WINDOWS_THERMAL_NOMINAL => HostThermalState::Nominal,
        WINDOWS_THERMAL_FAIR => HostThermalState::Fair,
        WINDOWS_THERMAL_SERIOUS => HostThermalState::Serious,
        WINDOWS_THERMAL_CRITICAL => HostThermalState::Critical,
        _ => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "thermal_code",
                "invalid windows thermal state code",
            ))
            .boxed());
        }
    };

    Ok(state)
}

/// Convert one Windows power mode code into the runtime power mode enum.
fn decode_windows_power_mode(power_mode_code: u32) -> RuntimeResult<HostPowerMode> {
    let mode = match power_mode_code {
        WINDOWS_POWER_MODE_NORMAL => HostPowerMode::Normal,
        WINDOWS_POWER_MODE_LOW_POWER => HostPowerMode::LowPower,
        _ => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "power_mode_code",
                "invalid windows power mode code",
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
        WINDOWS_LIFECYCLE_ACTIVATED, WINDOWS_LIFECYCLE_CREATED, WINDOWS_LIFECYCLE_DESTROYED,
        WINDOWS_LIFECYCLE_RESUMED, WINDOWS_LIFECYCLE_STOPPING, WINDOWS_LIFECYCLE_SUSPENDED,
        WINDOWS_MEMORY_PRESSURE_CRITICAL, WINDOWS_MEMORY_PRESSURE_NORMAL,
        WINDOWS_MEMORY_PRESSURE_WARNING, WINDOWS_POWER_MODE_LOW_POWER, WINDOWS_POWER_MODE_NORMAL,
        WINDOWS_THERMAL_CRITICAL, WINDOWS_THERMAL_FAIR, WINDOWS_THERMAL_NOMINAL,
        WINDOWS_THERMAL_SERIOUS, decode_windows_application_lifecycle,
        decode_windows_memory_pressure_level, decode_windows_power_mode,
        decode_windows_thermal_state,
    };
    use crate::runtime::host::{
        HostMemoryPressureLevel, HostPowerMode, HostThermalState, WindowsApplicationLifecycle,
    };

    #[test]
    fn test_decode_windows_lifecycle_created() {
        let lifecycle = decode_windows_application_lifecycle(WINDOWS_LIFECYCLE_CREATED).unwrap();
        assert_eq!(lifecycle, WindowsApplicationLifecycle::Created);
    }

    #[test]
    fn test_decode_windows_lifecycle_activated() {
        let lifecycle = decode_windows_application_lifecycle(WINDOWS_LIFECYCLE_ACTIVATED).unwrap();
        assert_eq!(lifecycle, WindowsApplicationLifecycle::Activated);
    }

    #[test]
    fn test_decode_windows_lifecycle_resumed() {
        let lifecycle = decode_windows_application_lifecycle(WINDOWS_LIFECYCLE_RESUMED).unwrap();
        assert_eq!(lifecycle, WindowsApplicationLifecycle::Resumed);
    }

    #[test]
    fn test_decode_windows_lifecycle_suspended() {
        let lifecycle = decode_windows_application_lifecycle(WINDOWS_LIFECYCLE_SUSPENDED).unwrap();
        assert_eq!(lifecycle, WindowsApplicationLifecycle::Suspended);
    }

    #[test]
    fn test_decode_windows_lifecycle_stopping() {
        let lifecycle = decode_windows_application_lifecycle(WINDOWS_LIFECYCLE_STOPPING).unwrap();
        assert_eq!(lifecycle, WindowsApplicationLifecycle::Stopping);
    }

    #[test]
    fn test_decode_windows_lifecycle_destroyed() {
        let lifecycle = decode_windows_application_lifecycle(WINDOWS_LIFECYCLE_DESTROYED).unwrap();
        assert_eq!(lifecycle, WindowsApplicationLifecycle::Destroyed);
    }

    #[test]
    fn test_decode_windows_lifecycle_rejects_unknown_code() {
        let result = decode_windows_application_lifecycle(u32::MAX);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_windows_memory_pressure_codes() {
        assert_eq!(
            decode_windows_memory_pressure_level(WINDOWS_MEMORY_PRESSURE_NORMAL).unwrap(),
            HostMemoryPressureLevel::Normal
        );
        assert_eq!(
            decode_windows_memory_pressure_level(WINDOWS_MEMORY_PRESSURE_WARNING).unwrap(),
            HostMemoryPressureLevel::Warning
        );
        assert_eq!(
            decode_windows_memory_pressure_level(WINDOWS_MEMORY_PRESSURE_CRITICAL).unwrap(),
            HostMemoryPressureLevel::Critical
        );
        assert!(decode_windows_memory_pressure_level(u32::MAX).is_err());
    }

    #[test]
    fn test_decode_windows_thermal_codes() {
        assert_eq!(
            decode_windows_thermal_state(WINDOWS_THERMAL_NOMINAL).unwrap(),
            HostThermalState::Nominal
        );
        assert_eq!(
            decode_windows_thermal_state(WINDOWS_THERMAL_FAIR).unwrap(),
            HostThermalState::Fair
        );
        assert_eq!(
            decode_windows_thermal_state(WINDOWS_THERMAL_SERIOUS).unwrap(),
            HostThermalState::Serious
        );
        assert_eq!(
            decode_windows_thermal_state(WINDOWS_THERMAL_CRITICAL).unwrap(),
            HostThermalState::Critical
        );
        assert!(decode_windows_thermal_state(u32::MAX).is_err());
    }

    #[test]
    fn test_decode_windows_power_mode_codes() {
        assert_eq!(
            decode_windows_power_mode(WINDOWS_POWER_MODE_NORMAL).unwrap(),
            HostPowerMode::Normal
        );
        assert_eq!(
            decode_windows_power_mode(WINDOWS_POWER_MODE_LOW_POWER).unwrap(),
            HostPowerMode::LowPower
        );
        assert!(decode_windows_power_mode(u32::MAX).is_err());
    }
}
