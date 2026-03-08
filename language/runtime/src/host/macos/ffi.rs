use super::{
    MacosApplicationLifecycle, macos_notify_application_lifecycle,
    macos_notify_interruption_changed, macos_notify_memory_pressure_changed,
    macos_notify_permission_result, macos_notify_power_mode_changed,
    macos_notify_thermal_state_changed, macos_notify_wake, macos_notify_wall_clock_changed,
};
use crate::diagnostic::{RuntimeResult, RuntimeStatus};
use crate::host::core::error::invalid_argument_value;
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};
use crate::runtime::NativeStringRef;

/// macOS lifecycle code for `applicationDidFinishLaunching`.
pub(super) const MACOS_LIFECYCLE_DID_FINISH_LAUNCHING: u32 = 0;
/// macOS lifecycle code for `applicationDidBecomeActive`.
pub(super) const MACOS_LIFECYCLE_DID_BECOME_ACTIVE: u32 = 1;
/// macOS lifecycle code for `applicationWillResignActive`.
pub(super) const MACOS_LIFECYCLE_WILL_RESIGN_ACTIVE: u32 = 2;
/// macOS lifecycle code for `applicationWillTerminate`.
pub(super) const MACOS_LIFECYCLE_WILL_TERMINATE: u32 = 3;
/// macOS memory pressure code for normal state.
pub(super) const MACOS_MEMORY_PRESSURE_NORMAL: u32 = 0;
/// macOS memory pressure code for warning state.
pub(super) const MACOS_MEMORY_PRESSURE_WARNING: u32 = 1;
/// macOS memory pressure code for critical state.
pub(super) const MACOS_MEMORY_PRESSURE_CRITICAL: u32 = 2;
/// macOS thermal code for nominal state.
pub(super) const MACOS_THERMAL_NOMINAL: u32 = 0;
/// macOS thermal code for fair state.
pub(super) const MACOS_THERMAL_FAIR: u32 = 1;
/// macOS thermal code for serious state.
pub(super) const MACOS_THERMAL_SERIOUS: u32 = 2;
/// macOS thermal code for critical state.
pub(super) const MACOS_THERMAL_CRITICAL: u32 = 3;
/// macOS power mode code for normal state.
pub(super) const MACOS_POWER_MODE_NORMAL: u32 = 0;
/// macOS power mode code for low power state.
pub(super) const MACOS_POWER_MODE_LOW_POWER: u32 = 1;

/// Notify the runtime host state about one macOS application lifecycle transition.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle_code: u32,
) -> RuntimeStatus {
    let result = decode_macos_application_lifecycle(lifecycle_code)
        .and_then(|lifecycle| macos_notify_application_lifecycle(runtime_id, lifecycle));

    runtime_status(result)
}

/// Notify the runtime host state with one macOS permission result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_permission_result(
    runtime_id: u64,
    permission: NativeStringRef,
    granted: bool,
) -> RuntimeStatus {
    let result = decode_permission_name(permission).and_then(|permission| {
        macos_notify_permission_result(runtime_id, permission.as_str(), granted)
    });

    runtime_status(result)
}

/// Notify the runtime host state that interruption state changed on macOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeStatus {
    runtime_status(macos_notify_interruption_changed(runtime_id, interrupted))
}

/// Notify the runtime host state that memory pressure changed on macOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_memory_pressure_changed(
    runtime_id: u64,
    level_code: u32,
) -> RuntimeStatus {
    let result = decode_macos_memory_pressure_level(level_code)
        .and_then(|level| macos_notify_memory_pressure_changed(runtime_id, level));

    runtime_status(result)
}

/// Notify the runtime host state that thermal state changed on macOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_thermal_state_changed(
    runtime_id: u64,
    thermal_code: u32,
) -> RuntimeStatus {
    let result = decode_macos_thermal_state(thermal_code)
        .and_then(|state| macos_notify_thermal_state_changed(runtime_id, state));

    runtime_status(result)
}

/// Notify the runtime host state that power mode changed on macOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_power_mode_changed(
    runtime_id: u64,
    power_mode_code: u32,
) -> RuntimeStatus {
    let result = decode_macos_power_mode(power_mode_code)
        .and_then(|mode| macos_notify_power_mode_changed(runtime_id, mode));

    runtime_status(result)
}

/// Notify the runtime host state that wall clock changed on macOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_wall_clock_changed(
    runtime_id: u64,
) -> RuntimeStatus {
    runtime_status(macos_notify_wall_clock_changed(runtime_id))
}

/// Wake one blocked host event poll operation for macOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_wake(runtime_id: u64) -> RuntimeStatus {
    runtime_status(macos_notify_wake(runtime_id))
}

// lifecycle decode
pub(super) fn decode_macos_application_lifecycle(
    lifecycle_code: u32,
) -> RuntimeResult<MacosApplicationLifecycle> {
    let lifecycle = match lifecycle_code {
        MACOS_LIFECYCLE_DID_FINISH_LAUNCHING => MacosApplicationLifecycle::DidFinishLaunching,
        MACOS_LIFECYCLE_DID_BECOME_ACTIVE => MacosApplicationLifecycle::DidBecomeActive,
        MACOS_LIFECYCLE_WILL_RESIGN_ACTIVE => MacosApplicationLifecycle::WillResignActive,
        MACOS_LIFECYCLE_WILL_TERMINATE => MacosApplicationLifecycle::WillTerminate,
        _ => {
            return Err(invalid_argument_value(
                "lifecycle_code",
                "invalid macos lifecycle code",
            ));
        }
    };

    Ok(lifecycle)
}

// memory pressure decode
pub(super) fn decode_macos_memory_pressure_level(
    level_code: u32,
) -> RuntimeResult<HostMemoryPressureLevel> {
    let level = match level_code {
        MACOS_MEMORY_PRESSURE_NORMAL => HostMemoryPressureLevel::Normal,
        MACOS_MEMORY_PRESSURE_WARNING => HostMemoryPressureLevel::Warning,
        MACOS_MEMORY_PRESSURE_CRITICAL => HostMemoryPressureLevel::Critical,
        _ => {
            return Err(invalid_argument_value(
                "level_code",
                "invalid macos memory pressure level code",
            ));
        }
    };

    Ok(level)
}

// thermal decode
pub(super) fn decode_macos_thermal_state(thermal_code: u32) -> RuntimeResult<HostThermalState> {
    let state = match thermal_code {
        MACOS_THERMAL_NOMINAL => HostThermalState::Nominal,
        MACOS_THERMAL_FAIR => HostThermalState::Fair,
        MACOS_THERMAL_SERIOUS => HostThermalState::Serious,
        MACOS_THERMAL_CRITICAL => HostThermalState::Critical,
        _ => {
            return Err(invalid_argument_value(
                "thermal_code",
                "invalid macos thermal state code",
            ));
        }
    };

    Ok(state)
}

// power mode decode
pub(super) fn decode_macos_power_mode(power_mode_code: u32) -> RuntimeResult<HostPowerMode> {
    let mode = match power_mode_code {
        MACOS_POWER_MODE_NORMAL => HostPowerMode::Normal,
        MACOS_POWER_MODE_LOW_POWER => HostPowerMode::LowPower,
        _ => {
            return Err(invalid_argument_value(
                "power_mode_code",
                "invalid macos power mode code",
            ));
        }
    };

    Ok(mode)
}

// permission decode
fn decode_permission_name(permission: NativeStringRef) -> RuntimeResult<String> {
    unsafe { permission.as_str() }.map(|permission| permission.to_string())
}

// ffi result conversion
fn runtime_status(result: RuntimeResult<()>) -> RuntimeStatus {
    RuntimeStatus::from_result(result, None)
}
