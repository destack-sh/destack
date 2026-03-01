use super::{
    MacosApplicationLifecycle, macos_notify_application_lifecycle,
    macos_notify_interruption_changed, macos_notify_memory_pressure_changed,
    macos_notify_permission_result, macos_notify_power_mode_changed,
    macos_notify_thermal_state_changed, macos_notify_wake, macos_notify_wall_clock_changed,
    macos_notify_window_available, macos_notify_window_focus_changed, macos_notify_window_resized,
    macos_notify_window_terminated,
};
use crate::diagnostic::{RuntimeError, RuntimeResult, RuntimeStatus};
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};
use crate::platform::{NativeStringRef, PlatformError};

/// macOS lifecycle code for `applicationDidFinishLaunching`.
const MACOS_LIFECYCLE_DID_FINISH_LAUNCHING: u32 = 0;
/// macOS lifecycle code for `applicationDidBecomeActive`.
const MACOS_LIFECYCLE_DID_BECOME_ACTIVE: u32 = 1;
/// macOS lifecycle code for `applicationWillResignActive`.
const MACOS_LIFECYCLE_WILL_RESIGN_ACTIVE: u32 = 2;
/// macOS lifecycle code for `applicationWillTerminate`.
const MACOS_LIFECYCLE_WILL_TERMINATE: u32 = 3;
/// macOS memory pressure code for normal state.
const MACOS_MEMORY_PRESSURE_NORMAL: u32 = 0;
/// macOS memory pressure code for warning state.
const MACOS_MEMORY_PRESSURE_WARNING: u32 = 1;
/// macOS memory pressure code for critical state.
const MACOS_MEMORY_PRESSURE_CRITICAL: u32 = 2;
/// macOS thermal code for nominal state.
const MACOS_THERMAL_NOMINAL: u32 = 0;
/// macOS thermal code for fair state.
const MACOS_THERMAL_FAIR: u32 = 1;
/// macOS thermal code for serious state.
const MACOS_THERMAL_SERIOUS: u32 = 2;
/// macOS thermal code for critical state.
const MACOS_THERMAL_CRITICAL: u32 = 3;
/// macOS power mode code for normal state.
const MACOS_POWER_MODE_NORMAL: u32 = 0;
/// macOS power mode code for low power state.
const MACOS_POWER_MODE_LOW_POWER: u32 = 1;

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

/// Notify the runtime host state that one macOS window became available.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_window_available(
    runtime_id: u64,
    window_id: u64,
) -> RuntimeStatus {
    runtime_status(macos_notify_window_available(runtime_id, window_id))
}

/// Notify the runtime host state that one macOS window terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_window_terminated(
    runtime_id: u64,
    window_id: u64,
) -> RuntimeStatus {
    runtime_status(macos_notify_window_terminated(runtime_id, window_id))
}

/// Notify the runtime host state that one macOS window resized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_window_resized(
    runtime_id: u64,
    window_id: u64,
    width_px: u32,
    height_px: u32,
) -> RuntimeStatus {
    runtime_status(macos_notify_window_resized(
        runtime_id, window_id, width_px, height_px,
    ))
}

/// Notify the runtime host state that one macOS window focus changed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_window_focus_changed(
    runtime_id: u64,
    window_id: u64,
    is_focused: bool,
) -> RuntimeStatus {
    runtime_status(macos_notify_window_focus_changed(
        runtime_id, window_id, is_focused,
    ))
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

/// Convert one macOS lifecycle code into the runtime lifecycle enum.
fn decode_macos_application_lifecycle(
    lifecycle_code: u32,
) -> RuntimeResult<MacosApplicationLifecycle> {
    let lifecycle = match lifecycle_code {
        MACOS_LIFECYCLE_DID_FINISH_LAUNCHING => MacosApplicationLifecycle::DidFinishLaunching,
        MACOS_LIFECYCLE_DID_BECOME_ACTIVE => MacosApplicationLifecycle::DidBecomeActive,
        MACOS_LIFECYCLE_WILL_RESIGN_ACTIVE => MacosApplicationLifecycle::WillResignActive,
        MACOS_LIFECYCLE_WILL_TERMINATE => MacosApplicationLifecycle::WillTerminate,
        _ => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "lifecycle_code",
                "invalid macos lifecycle code",
            ))
            .boxed());
        }
    };

    Ok(lifecycle)
}

/// Convert one macOS memory pressure code into the runtime memory pressure enum.
fn decode_macos_memory_pressure_level(level_code: u32) -> RuntimeResult<HostMemoryPressureLevel> {
    let level = match level_code {
        MACOS_MEMORY_PRESSURE_NORMAL => HostMemoryPressureLevel::Normal,
        MACOS_MEMORY_PRESSURE_WARNING => HostMemoryPressureLevel::Warning,
        MACOS_MEMORY_PRESSURE_CRITICAL => HostMemoryPressureLevel::Critical,
        _ => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "level_code",
                "invalid macos memory pressure level code",
            ))
            .boxed());
        }
    };

    Ok(level)
}

/// Convert one macOS thermal code into the runtime thermal enum.
fn decode_macos_thermal_state(thermal_code: u32) -> RuntimeResult<HostThermalState> {
    let state = match thermal_code {
        MACOS_THERMAL_NOMINAL => HostThermalState::Nominal,
        MACOS_THERMAL_FAIR => HostThermalState::Fair,
        MACOS_THERMAL_SERIOUS => HostThermalState::Serious,
        MACOS_THERMAL_CRITICAL => HostThermalState::Critical,
        _ => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "thermal_code",
                "invalid macos thermal state code",
            ))
            .boxed());
        }
    };

    Ok(state)
}

/// Convert one macOS power mode code into the runtime power mode enum.
fn decode_macos_power_mode(power_mode_code: u32) -> RuntimeResult<HostPowerMode> {
    let mode = match power_mode_code {
        MACOS_POWER_MODE_NORMAL => HostPowerMode::Normal,
        MACOS_POWER_MODE_LOW_POWER => HostPowerMode::LowPower,
        _ => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "power_mode_code",
                "invalid macos power mode code",
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
#[path = "tests/ffi.rs"]
mod tests;
