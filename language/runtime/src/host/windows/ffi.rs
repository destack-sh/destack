use super::{
    WindowsApplicationLifecycle, windows_notify_application_lifecycle,
    windows_notify_interruption_changed, windows_notify_memory_pressure_changed,
    windows_notify_permission_result, windows_notify_power_mode_changed,
    windows_notify_thermal_state_changed, windows_notify_wake, windows_notify_wall_clock_changed,
};
use crate::diagnostic::{RuntimeResult, RuntimeStatus};
use crate::host::core::error::invalid_argument_value;
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};
use crate::runtime::NativeStringRef;

pub(super) const WINDOWS_LIFECYCLE_CREATED: u32 = 0;
pub(super) const WINDOWS_LIFECYCLE_ACTIVATED: u32 = 1;
pub(super) const WINDOWS_LIFECYCLE_RESUMED: u32 = 2;
pub(super) const WINDOWS_LIFECYCLE_SUSPENDED: u32 = 3;
pub(super) const WINDOWS_LIFECYCLE_STOPPING: u32 = 4;
pub(super) const WINDOWS_LIFECYCLE_DESTROYED: u32 = 5;
pub(super) const WINDOWS_MEMORY_PRESSURE_NORMAL: u32 = 0;
pub(super) const WINDOWS_MEMORY_PRESSURE_WARNING: u32 = 1;
pub(super) const WINDOWS_MEMORY_PRESSURE_CRITICAL: u32 = 2;
pub(super) const WINDOWS_THERMAL_NOMINAL: u32 = 0;
pub(super) const WINDOWS_THERMAL_FAIR: u32 = 1;
pub(super) const WINDOWS_THERMAL_SERIOUS: u32 = 2;
pub(super) const WINDOWS_THERMAL_CRITICAL: u32 = 3;
pub(super) const WINDOWS_POWER_MODE_NORMAL: u32 = 0;
pub(super) const WINDOWS_POWER_MODE_LOW_POWER: u32 = 1;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_windows_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle_code: u32,
) -> RuntimeStatus {
    let result = decode_windows_application_lifecycle(lifecycle_code)
        .and_then(|lifecycle| windows_notify_application_lifecycle(runtime_id, lifecycle));

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_windows_notify_permission_result(
    runtime_id: u64,
    permission: NativeStringRef,
    granted: bool,
) -> RuntimeStatus {
    let result = decode_permission_name(permission).and_then(|permission| {
        windows_notify_permission_result(runtime_id, permission.as_str(), granted)
    });

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_windows_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeStatus {
    runtime_status(windows_notify_interruption_changed(runtime_id, interrupted))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_windows_notify_memory_pressure_changed(
    runtime_id: u64,
    level_code: u32,
) -> RuntimeStatus {
    let result = decode_windows_memory_pressure_level(level_code)
        .and_then(|level| windows_notify_memory_pressure_changed(runtime_id, level));

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_windows_notify_thermal_state_changed(
    runtime_id: u64,
    thermal_code: u32,
) -> RuntimeStatus {
    let result = decode_windows_thermal_state(thermal_code)
        .and_then(|state| windows_notify_thermal_state_changed(runtime_id, state));

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_windows_notify_power_mode_changed(
    runtime_id: u64,
    power_mode_code: u32,
) -> RuntimeStatus {
    let result = decode_windows_power_mode(power_mode_code)
        .and_then(|mode| windows_notify_power_mode_changed(runtime_id, mode));

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_windows_notify_wall_clock_changed(
    runtime_id: u64,
) -> RuntimeStatus {
    runtime_status(windows_notify_wall_clock_changed(runtime_id))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_windows_notify_wake(runtime_id: u64) -> RuntimeStatus {
    runtime_status(windows_notify_wake(runtime_id))
}

pub(super) fn decode_windows_application_lifecycle(
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
            return Err(invalid_argument_value(
                "lifecycle_code",
                "invalid windows lifecycle code",
            ));
        }
    };

    Ok(lifecycle)
}

pub(super) fn decode_windows_memory_pressure_level(
    level_code: u32,
) -> RuntimeResult<HostMemoryPressureLevel> {
    let level = match level_code {
        WINDOWS_MEMORY_PRESSURE_NORMAL => HostMemoryPressureLevel::Normal,
        WINDOWS_MEMORY_PRESSURE_WARNING => HostMemoryPressureLevel::Warning,
        WINDOWS_MEMORY_PRESSURE_CRITICAL => HostMemoryPressureLevel::Critical,
        _ => {
            return Err(invalid_argument_value(
                "level_code",
                "invalid windows memory pressure level code",
            ));
        }
    };

    Ok(level)
}

pub(super) fn decode_windows_thermal_state(thermal_code: u32) -> RuntimeResult<HostThermalState> {
    let state = match thermal_code {
        WINDOWS_THERMAL_NOMINAL => HostThermalState::Nominal,
        WINDOWS_THERMAL_FAIR => HostThermalState::Fair,
        WINDOWS_THERMAL_SERIOUS => HostThermalState::Serious,
        WINDOWS_THERMAL_CRITICAL => HostThermalState::Critical,
        _ => {
            return Err(invalid_argument_value(
                "thermal_code",
                "invalid windows thermal state code",
            ));
        }
    };

    Ok(state)
}

pub(super) fn decode_windows_power_mode(power_mode_code: u32) -> RuntimeResult<HostPowerMode> {
    let mode = match power_mode_code {
        WINDOWS_POWER_MODE_NORMAL => HostPowerMode::Normal,
        WINDOWS_POWER_MODE_LOW_POWER => HostPowerMode::LowPower,
        _ => {
            return Err(invalid_argument_value(
                "power_mode_code",
                "invalid windows power mode code",
            ));
        }
    };

    Ok(mode)
}

fn decode_permission_name(permission: NativeStringRef) -> RuntimeResult<String> {
    unsafe { permission.as_str() }.map(|permission| permission.to_string())
}

fn runtime_status(result: RuntimeResult<()>) -> RuntimeStatus {
    RuntimeStatus::from_result(result, None)
}
