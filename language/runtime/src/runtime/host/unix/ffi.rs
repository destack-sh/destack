use super::UnixApplicationLifecycle;
use crate::diagnostic::{RuntimeError, RuntimeResult, RuntimeStatus};
use crate::platform::{NativeStringRef, PlatformError};
use crate::runtime::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};

/// Unix lifecycle code for app-created initialization.
const UNIX_LIFECYCLE_CREATED: u32 = 0;
/// Unix lifecycle code for app-running foreground state.
const UNIX_LIFECYCLE_RUNNING: u32 = 1;
/// Unix lifecycle code for app-paused state.
const UNIX_LIFECYCLE_PAUSED: u32 = 2;
/// Unix lifecycle code for app-stopping transition.
const UNIX_LIFECYCLE_STOPPED: u32 = 3;
/// Unix lifecycle code for app-destroyed termination.
const UNIX_LIFECYCLE_DESTROYED: u32 = 4;
/// Unix memory pressure code for normal state.
const UNIX_MEMORY_PRESSURE_NORMAL: u32 = 0;
/// Unix memory pressure code for warning state.
const UNIX_MEMORY_PRESSURE_WARNING: u32 = 1;
/// Unix memory pressure code for critical state.
const UNIX_MEMORY_PRESSURE_CRITICAL: u32 = 2;
/// Unix thermal code for nominal state.
const UNIX_THERMAL_NOMINAL: u32 = 0;
/// Unix thermal code for fair state.
const UNIX_THERMAL_FAIR: u32 = 1;
/// Unix thermal code for serious state.
const UNIX_THERMAL_SERIOUS: u32 = 2;
/// Unix thermal code for critical state.
const UNIX_THERMAL_CRITICAL: u32 = 3;
/// Unix power mode code for normal state.
const UNIX_POWER_MODE_NORMAL: u32 = 0;
/// Unix power mode code for low power state.
const UNIX_POWER_MODE_LOW_POWER: u32 = 1;

/// Convert one unix lifecycle code into the runtime lifecycle enum.
pub(crate) fn decode_unix_application_lifecycle(
    lifecycle_code: u32,
) -> RuntimeResult<UnixApplicationLifecycle> {
    let lifecycle = match lifecycle_code {
        UNIX_LIFECYCLE_CREATED => UnixApplicationLifecycle::Created,
        UNIX_LIFECYCLE_RUNNING => UnixApplicationLifecycle::Running,
        UNIX_LIFECYCLE_PAUSED => UnixApplicationLifecycle::Paused,
        UNIX_LIFECYCLE_STOPPED => UnixApplicationLifecycle::Stopped,
        UNIX_LIFECYCLE_DESTROYED => UnixApplicationLifecycle::Destroyed,
        _ => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "lifecycle_code",
                "invalid unix lifecycle code",
            ))
            .boxed());
        }
    };

    Ok(lifecycle)
}

/// Convert one unix memory pressure code into the runtime memory pressure enum.
pub(crate) fn decode_unix_memory_pressure_level(
    level_code: u32,
) -> RuntimeResult<HostMemoryPressureLevel> {
    let level = match level_code {
        UNIX_MEMORY_PRESSURE_NORMAL => HostMemoryPressureLevel::Normal,
        UNIX_MEMORY_PRESSURE_WARNING => HostMemoryPressureLevel::Warning,
        UNIX_MEMORY_PRESSURE_CRITICAL => HostMemoryPressureLevel::Critical,
        _ => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "level_code",
                "invalid unix memory pressure level code",
            ))
            .boxed());
        }
    };

    Ok(level)
}

/// Convert one unix thermal code into the runtime thermal enum.
pub(crate) fn decode_unix_thermal_state(thermal_code: u32) -> RuntimeResult<HostThermalState> {
    let state = match thermal_code {
        UNIX_THERMAL_NOMINAL => HostThermalState::Nominal,
        UNIX_THERMAL_FAIR => HostThermalState::Fair,
        UNIX_THERMAL_SERIOUS => HostThermalState::Serious,
        UNIX_THERMAL_CRITICAL => HostThermalState::Critical,
        _ => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "thermal_code",
                "invalid unix thermal state code",
            ))
            .boxed());
        }
    };

    Ok(state)
}

/// Convert one unix power mode code into the runtime power mode enum.
pub(crate) fn decode_unix_power_mode(power_mode_code: u32) -> RuntimeResult<HostPowerMode> {
    let mode = match power_mode_code {
        UNIX_POWER_MODE_NORMAL => HostPowerMode::Normal,
        UNIX_POWER_MODE_LOW_POWER => HostPowerMode::LowPower,
        _ => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "power_mode_code",
                "invalid unix power mode code",
            ))
            .boxed());
        }
    };

    Ok(mode)
}

/// Decode one permission string reference.
pub(crate) fn decode_unix_permission_name(permission: NativeStringRef) -> RuntimeResult<String> {
    unsafe { permission.as_str() }.map(|permission| permission.to_string())
}

/// Convert one runtime result into one native status value.
pub(crate) fn unix_runtime_status(result: RuntimeResult<()>) -> RuntimeStatus {
    RuntimeStatus::from_result(result, None)
}

#[cfg(test)]
mod tests {
    use super::{
        UNIX_LIFECYCLE_CREATED, UNIX_LIFECYCLE_DESTROYED, UNIX_LIFECYCLE_PAUSED,
        UNIX_LIFECYCLE_RUNNING, UNIX_LIFECYCLE_STOPPED, UNIX_MEMORY_PRESSURE_CRITICAL,
        UNIX_MEMORY_PRESSURE_NORMAL, UNIX_MEMORY_PRESSURE_WARNING, UNIX_POWER_MODE_LOW_POWER,
        UNIX_POWER_MODE_NORMAL, UNIX_THERMAL_CRITICAL, UNIX_THERMAL_FAIR, UNIX_THERMAL_NOMINAL,
        UNIX_THERMAL_SERIOUS, decode_unix_application_lifecycle, decode_unix_memory_pressure_level,
        decode_unix_power_mode, decode_unix_thermal_state,
    };
    use crate::runtime::host::unix::UnixApplicationLifecycle;
    use crate::runtime::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};

    #[test]
    fn test_decode_unix_lifecycle_codes() {
        assert_eq!(
            decode_unix_application_lifecycle(UNIX_LIFECYCLE_CREATED).unwrap(),
            UnixApplicationLifecycle::Created
        );
        assert_eq!(
            decode_unix_application_lifecycle(UNIX_LIFECYCLE_RUNNING).unwrap(),
            UnixApplicationLifecycle::Running
        );
        assert_eq!(
            decode_unix_application_lifecycle(UNIX_LIFECYCLE_PAUSED).unwrap(),
            UnixApplicationLifecycle::Paused
        );
        assert_eq!(
            decode_unix_application_lifecycle(UNIX_LIFECYCLE_STOPPED).unwrap(),
            UnixApplicationLifecycle::Stopped
        );
        assert_eq!(
            decode_unix_application_lifecycle(UNIX_LIFECYCLE_DESTROYED).unwrap(),
            UnixApplicationLifecycle::Destroyed
        );
    }

    #[test]
    fn test_decode_unix_lifecycle_rejects_unknown_code() {
        let result = decode_unix_application_lifecycle(u32::MAX);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_unix_memory_pressure_codes() {
        assert_eq!(
            decode_unix_memory_pressure_level(UNIX_MEMORY_PRESSURE_NORMAL).unwrap(),
            HostMemoryPressureLevel::Normal
        );
        assert_eq!(
            decode_unix_memory_pressure_level(UNIX_MEMORY_PRESSURE_WARNING).unwrap(),
            HostMemoryPressureLevel::Warning
        );
        assert_eq!(
            decode_unix_memory_pressure_level(UNIX_MEMORY_PRESSURE_CRITICAL).unwrap(),
            HostMemoryPressureLevel::Critical
        );
        assert!(decode_unix_memory_pressure_level(u32::MAX).is_err());
    }

    #[test]
    fn test_decode_unix_thermal_codes() {
        assert_eq!(
            decode_unix_thermal_state(UNIX_THERMAL_NOMINAL).unwrap(),
            HostThermalState::Nominal
        );
        assert_eq!(
            decode_unix_thermal_state(UNIX_THERMAL_FAIR).unwrap(),
            HostThermalState::Fair
        );
        assert_eq!(
            decode_unix_thermal_state(UNIX_THERMAL_SERIOUS).unwrap(),
            HostThermalState::Serious
        );
        assert_eq!(
            decode_unix_thermal_state(UNIX_THERMAL_CRITICAL).unwrap(),
            HostThermalState::Critical
        );
        assert!(decode_unix_thermal_state(u32::MAX).is_err());
    }

    #[test]
    fn test_decode_unix_power_mode_codes() {
        assert_eq!(
            decode_unix_power_mode(UNIX_POWER_MODE_NORMAL).unwrap(),
            HostPowerMode::Normal
        );
        assert_eq!(
            decode_unix_power_mode(UNIX_POWER_MODE_LOW_POWER).unwrap(),
            HostPowerMode::LowPower
        );
        assert!(decode_unix_power_mode(u32::MAX).is_err());
    }
}
