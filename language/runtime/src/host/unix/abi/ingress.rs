use crate::diagnostic::{RuntimeResult, RuntimeStatus};
use crate::host::core::error::invalid_argument_value;
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};
use crate::platform::abi::NativeStringRef;

use crate::host::os::unix::ingress::notify::UnixApplicationLifecycle;

/// Unix lifecycle code for app-created initialization.
pub(crate) const UNIX_LIFECYCLE_CREATED: u32 = 0;
/// Unix lifecycle code for app-running foreground state.
pub(crate) const UNIX_LIFECYCLE_RUNNING: u32 = 1;
/// Unix lifecycle code for app-paused state.
pub(crate) const UNIX_LIFECYCLE_PAUSED: u32 = 2;
/// Unix lifecycle code for app-stopping transition.
pub(crate) const UNIX_LIFECYCLE_STOPPED: u32 = 3;
/// Unix lifecycle code for app-destroyed termination.
pub(crate) const UNIX_LIFECYCLE_DESTROYED: u32 = 4;
/// Unix memory pressure code for normal state.
pub(crate) const UNIX_MEMORY_PRESSURE_NORMAL: u32 = 0;
/// Unix memory pressure code for warning state.
pub(crate) const UNIX_MEMORY_PRESSURE_WARNING: u32 = 1;
/// Unix memory pressure code for critical state.
pub(crate) const UNIX_MEMORY_PRESSURE_CRITICAL: u32 = 2;
/// Unix thermal code for nominal state.
pub(crate) const UNIX_THERMAL_NOMINAL: u32 = 0;
/// Unix thermal code for fair state.
pub(crate) const UNIX_THERMAL_FAIR: u32 = 1;
/// Unix thermal code for serious state.
pub(crate) const UNIX_THERMAL_SERIOUS: u32 = 2;
/// Unix thermal code for critical state.
pub(crate) const UNIX_THERMAL_CRITICAL: u32 = 3;
/// Unix power mode code for normal state.
pub(crate) const UNIX_POWER_MODE_NORMAL: u32 = 0;
/// Unix power mode code for low power state.
pub(crate) const UNIX_POWER_MODE_LOW_POWER: u32 = 1;

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
            return Err(invalid_argument_value(
                "lifecycle_code",
                "invalid unix lifecycle code",
            ));
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
            return Err(invalid_argument_value(
                "level_code",
                "invalid unix memory pressure level code",
            ));
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
            return Err(invalid_argument_value(
                "thermal_code",
                "invalid unix thermal state code",
            ));
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
            return Err(invalid_argument_value(
                "power_mode_code",
                "invalid unix power mode code",
            ));
        }
    };

    Ok(mode)
}

/// Decode one permission string reference.
#[cfg_attr(test, allow(dead_code))]
pub(crate) fn decode_unix_permission_name(permission: NativeStringRef) -> RuntimeResult<String> {
    unsafe { permission.as_str() }.map(|permission| permission.to_string())
}

/// Convert one runtime result into one native status value.
#[cfg_attr(test, allow(dead_code))]
pub(crate) fn unix_runtime_status(result: RuntimeResult<()>) -> RuntimeStatus {
    RuntimeStatus::from_result(result, None)
}
