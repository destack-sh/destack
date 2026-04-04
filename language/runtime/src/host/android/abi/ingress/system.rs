use crate::diagnostic::{RuntimeResult, RuntimeStatus};
use crate::host::core::error::invalid_argument_value;
use crate::host::os::android::ingress::{
    android_notify_interruption_changed, android_notify_memory_pressure_changed,
    android_notify_power_mode_changed, android_notify_thermal_state_changed, android_notify_wake,
    android_notify_wall_clock_changed,
};
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};

use super::core::runtime_status;

/// Android memory pressure code for normal state.
pub(crate) const ANDROID_MEMORY_PRESSURE_NORMAL: u32 = 0;
/// Android memory pressure code for warning state.
pub(crate) const ANDROID_MEMORY_PRESSURE_WARNING: u32 = 1;
/// Android memory pressure code for critical state.
pub(crate) const ANDROID_MEMORY_PRESSURE_CRITICAL: u32 = 2;
/// Android thermal code for nominal state.
pub(crate) const ANDROID_THERMAL_NOMINAL: u32 = 0;
/// Android thermal code for fair state.
pub(crate) const ANDROID_THERMAL_FAIR: u32 = 1;
/// Android thermal code for serious state.
pub(crate) const ANDROID_THERMAL_SERIOUS: u32 = 2;
/// Android thermal code for critical state.
pub(crate) const ANDROID_THERMAL_CRITICAL: u32 = 3;
/// Android power mode code for normal state.
pub(crate) const ANDROID_POWER_MODE_NORMAL: u32 = 0;
/// Android power mode code for low power state.
pub(crate) const ANDROID_POWER_MODE_LOW_POWER: u32 = 1;

pub unsafe fn destack_host_android_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeStatus {
    runtime_status(android_notify_interruption_changed(runtime_id, interrupted))
}

pub unsafe fn destack_host_android_notify_memory_pressure_changed(
    runtime_id: u64,
    level_code: u32,
) -> RuntimeStatus {
    let result = decode_android_memory_pressure_level(level_code)
        .and_then(|level| android_notify_memory_pressure_changed(runtime_id, level));

    runtime_status(result)
}

pub unsafe fn destack_host_android_notify_thermal_state_changed(
    runtime_id: u64,
    thermal_code: u32,
) -> RuntimeStatus {
    let result = decode_android_thermal_state(thermal_code)
        .and_then(|state| android_notify_thermal_state_changed(runtime_id, state));

    runtime_status(result)
}

pub unsafe fn destack_host_android_notify_power_mode_changed(
    runtime_id: u64,
    power_mode_code: u32,
) -> RuntimeStatus {
    let result = decode_android_power_mode(power_mode_code)
        .and_then(|mode| android_notify_power_mode_changed(runtime_id, mode));

    runtime_status(result)
}

pub unsafe fn destack_host_android_notify_wall_clock_changed(runtime_id: u64) -> RuntimeStatus {
    runtime_status(android_notify_wall_clock_changed(runtime_id))
}

pub unsafe fn destack_host_android_notify_wake(runtime_id: u64) -> RuntimeStatus {
    runtime_status(android_notify_wake(runtime_id))
}

/// Decode one Android memory pressure level code.
pub(crate) fn decode_android_memory_pressure_level(
    level_code: u32,
) -> RuntimeResult<HostMemoryPressureLevel> {
    let level = match level_code {
        ANDROID_MEMORY_PRESSURE_NORMAL => HostMemoryPressureLevel::Normal,
        ANDROID_MEMORY_PRESSURE_WARNING => HostMemoryPressureLevel::Warning,
        ANDROID_MEMORY_PRESSURE_CRITICAL => HostMemoryPressureLevel::Critical,
        _ => {
            return Err(invalid_argument_value(
                "level_code",
                "invalid android memory pressure level code",
            ));
        }
    };

    Ok(level)
}

/// Decode one Android thermal state code.
pub(crate) fn decode_android_thermal_state(thermal_code: u32) -> RuntimeResult<HostThermalState> {
    let state = match thermal_code {
        ANDROID_THERMAL_NOMINAL => HostThermalState::Nominal,
        ANDROID_THERMAL_FAIR => HostThermalState::Fair,
        ANDROID_THERMAL_SERIOUS => HostThermalState::Serious,
        ANDROID_THERMAL_CRITICAL => HostThermalState::Critical,
        _ => {
            return Err(invalid_argument_value(
                "thermal_code",
                "invalid android thermal state code",
            ));
        }
    };

    Ok(state)
}

/// Decode one Android power mode code.
pub(crate) fn decode_android_power_mode(power_mode_code: u32) -> RuntimeResult<HostPowerMode> {
    let mode = match power_mode_code {
        ANDROID_POWER_MODE_NORMAL => HostPowerMode::Normal,
        ANDROID_POWER_MODE_LOW_POWER => HostPowerMode::LowPower,
        _ => {
            return Err(invalid_argument_value(
                "power_mode_code",
                "invalid android power mode code",
            ));
        }
    };

    Ok(mode)
}
