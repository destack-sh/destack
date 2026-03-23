use crate::diagnostic::{RuntimeResult, RuntimeStatus};
use crate::host::core::error::invalid_argument_value;
use crate::host::ios::ingress::{
    ios_notify_interruption_changed, ios_notify_memory_pressure_changed,
    ios_notify_power_mode_changed, ios_notify_thermal_state_changed, ios_notify_wake,
    ios_notify_wall_clock_changed,
};
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};

use super::core::runtime_status;

/// iOS memory pressure code for normal state.
pub(crate) const IOS_MEMORY_PRESSURE_NORMAL: u32 = 0;
/// iOS memory pressure code for warning state.
pub(crate) const IOS_MEMORY_PRESSURE_WARNING: u32 = 1;
/// iOS memory pressure code for critical state.
pub(crate) const IOS_MEMORY_PRESSURE_CRITICAL: u32 = 2;
/// iOS thermal code for nominal state.
pub(crate) const IOS_THERMAL_NOMINAL: u32 = 0;
/// iOS thermal code for fair state.
pub(crate) const IOS_THERMAL_FAIR: u32 = 1;
/// iOS thermal code for serious state.
pub(crate) const IOS_THERMAL_SERIOUS: u32 = 2;
/// iOS thermal code for critical state.
pub(crate) const IOS_THERMAL_CRITICAL: u32 = 3;
/// iOS power mode code for normal state.
pub(crate) const IOS_POWER_MODE_NORMAL: u32 = 0;
/// iOS power mode code for low power state.
pub(crate) const IOS_POWER_MODE_LOW_POWER: u32 = 1;

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeStatus {
    runtime_status(ios_notify_interruption_changed(runtime_id, interrupted))
}

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_notify_memory_pressure_changed(
    runtime_id: u64,
    level_code: u32,
) -> RuntimeStatus {
    let result = decode_ios_memory_pressure_level(level_code)
        .and_then(|level| ios_notify_memory_pressure_changed(runtime_id, level));

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_notify_thermal_state_changed(
    runtime_id: u64,
    thermal_code: u32,
) -> RuntimeStatus {
    let result = decode_ios_thermal_state(thermal_code)
        .and_then(|state| ios_notify_thermal_state_changed(runtime_id, state));

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_notify_power_mode_changed(
    runtime_id: u64,
    power_mode_code: u32,
) -> RuntimeStatus {
    let result = decode_ios_power_mode(power_mode_code)
        .and_then(|mode| ios_notify_power_mode_changed(runtime_id, mode));

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_notify_wall_clock_changed(
    runtime_id: u64,
) -> RuntimeStatus {
    runtime_status(ios_notify_wall_clock_changed(runtime_id))
}

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_notify_wake(runtime_id: u64) -> RuntimeStatus {
    runtime_status(ios_notify_wake(runtime_id))
}

/// Decode one iOS memory pressure level code.
pub(crate) fn decode_ios_memory_pressure_level(
    level_code: u32,
) -> RuntimeResult<HostMemoryPressureLevel> {
    let level = match level_code {
        IOS_MEMORY_PRESSURE_NORMAL => HostMemoryPressureLevel::Normal,
        IOS_MEMORY_PRESSURE_WARNING => HostMemoryPressureLevel::Warning,
        IOS_MEMORY_PRESSURE_CRITICAL => HostMemoryPressureLevel::Critical,
        _ => {
            return Err(invalid_argument_value(
                "level_code",
                "invalid ios memory pressure level code",
            ));
        }
    };

    Ok(level)
}

/// Decode one iOS thermal state code.
pub(crate) fn decode_ios_thermal_state(thermal_code: u32) -> RuntimeResult<HostThermalState> {
    let state = match thermal_code {
        IOS_THERMAL_NOMINAL => HostThermalState::Nominal,
        IOS_THERMAL_FAIR => HostThermalState::Fair,
        IOS_THERMAL_SERIOUS => HostThermalState::Serious,
        IOS_THERMAL_CRITICAL => HostThermalState::Critical,
        _ => {
            return Err(invalid_argument_value(
                "thermal_code",
                "invalid ios thermal state code",
            ));
        }
    };

    Ok(state)
}

/// Decode one iOS power mode code.
pub(crate) fn decode_ios_power_mode(power_mode_code: u32) -> RuntimeResult<HostPowerMode> {
    let mode = match power_mode_code {
        IOS_POWER_MODE_NORMAL => HostPowerMode::Normal,
        IOS_POWER_MODE_LOW_POWER => HostPowerMode::LowPower,
        _ => {
            return Err(invalid_argument_value(
                "power_mode_code",
                "invalid ios power mode code",
            ));
        }
    };

    Ok(mode)
}
