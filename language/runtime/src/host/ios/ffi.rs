use super::{
    IosApplicationLifecycle, ios_notify_application_lifecycle, ios_notify_interruption_changed,
    ios_notify_memory_pressure_changed, ios_notify_permission_result,
    ios_notify_power_mode_changed, ios_notify_thermal_state_changed, ios_notify_wake,
    ios_notify_wall_clock_changed, ios_notify_window_available, ios_notify_window_focus_changed,
    ios_notify_window_resized, ios_notify_window_terminated,
};
use crate::diagnostic::{RuntimeResult, RuntimeStatus};
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState, core as core_host};
use crate::runtime::NativeStringRef;

/// iOS lifecycle code for `applicationDidFinishLaunching`.
const IOS_LIFECYCLE_DID_FINISH_LAUNCHING: u32 = 0;
/// iOS lifecycle code for `applicationDidBecomeActive`.
const IOS_LIFECYCLE_DID_BECOME_ACTIVE: u32 = 1;
/// iOS lifecycle code for `applicationWillResignActive`.
const IOS_LIFECYCLE_WILL_RESIGN_ACTIVE: u32 = 2;
/// iOS lifecycle code for `applicationDidEnterBackground`.
const IOS_LIFECYCLE_DID_ENTER_BACKGROUND: u32 = 3;
/// iOS lifecycle code for `applicationWillEnterForeground`.
const IOS_LIFECYCLE_WILL_ENTER_FOREGROUND: u32 = 4;
/// iOS lifecycle code for `applicationWillTerminate`.
const IOS_LIFECYCLE_WILL_TERMINATE: u32 = 5;
/// iOS memory pressure code for normal state.
const IOS_MEMORY_PRESSURE_NORMAL: u32 = 0;
/// iOS memory pressure code for warning state.
const IOS_MEMORY_PRESSURE_WARNING: u32 = 1;
/// iOS memory pressure code for critical state.
const IOS_MEMORY_PRESSURE_CRITICAL: u32 = 2;
/// iOS thermal code for nominal state.
const IOS_THERMAL_NOMINAL: u32 = 0;
/// iOS thermal code for fair state.
const IOS_THERMAL_FAIR: u32 = 1;
/// iOS thermal code for serious state.
const IOS_THERMAL_SERIOUS: u32 = 2;
/// iOS thermal code for critical state.
const IOS_THERMAL_CRITICAL: u32 = 3;
/// iOS power mode code for normal state.
const IOS_POWER_MODE_NORMAL: u32 = 0;
/// iOS power mode code for low power state.
const IOS_POWER_MODE_LOW_POWER: u32 = 1;

/// Notify the runtime host state about one iOS application lifecycle transition.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle_code: u32,
) -> RuntimeStatus {
    let result = decode_ios_application_lifecycle(lifecycle_code)
        .and_then(|lifecycle| ios_notify_application_lifecycle(runtime_id, lifecycle));

    runtime_status(result)
}

/// Notify the runtime host state that one iOS window became available.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_window_available(
    runtime_id: u64,
    window_id: u64,
) -> RuntimeStatus {
    runtime_status(ios_notify_window_available(runtime_id, window_id))
}

/// Notify the runtime host state that one iOS window terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_window_terminated(
    runtime_id: u64,
    window_id: u64,
) -> RuntimeStatus {
    runtime_status(ios_notify_window_terminated(runtime_id, window_id))
}

/// Notify the runtime host state that one iOS window resized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_window_resized(
    runtime_id: u64,
    window_id: u64,
    width_px: u32,
    height_px: u32,
) -> RuntimeStatus {
    runtime_status(ios_notify_window_resized(
        runtime_id, window_id, width_px, height_px,
    ))
}

/// Notify the runtime host state that one iOS window focus changed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_window_focus_changed(
    runtime_id: u64,
    window_id: u64,
    is_focused: bool,
) -> RuntimeStatus {
    runtime_status(ios_notify_window_focus_changed(
        runtime_id, window_id, is_focused,
    ))
}

/// Notify the runtime host state with one iOS permission result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_permission_result(
    runtime_id: u64,
    permission: NativeStringRef,
    granted: bool,
) -> RuntimeStatus {
    let result = decode_permission_name(permission).and_then(|permission| {
        ios_notify_permission_result(runtime_id, permission.as_str(), granted)
    });

    runtime_status(result)
}

/// Notify the runtime host state that interruption state changed on iOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeStatus {
    runtime_status(ios_notify_interruption_changed(runtime_id, interrupted))
}

/// Notify the runtime host state that memory pressure changed on iOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_memory_pressure_changed(
    runtime_id: u64,
    level_code: u32,
) -> RuntimeStatus {
    let result = decode_ios_memory_pressure_level(level_code)
        .and_then(|level| ios_notify_memory_pressure_changed(runtime_id, level));

    runtime_status(result)
}

/// Notify the runtime host state that thermal state changed on iOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_thermal_state_changed(
    runtime_id: u64,
    thermal_code: u32,
) -> RuntimeStatus {
    let result = decode_ios_thermal_state(thermal_code)
        .and_then(|state| ios_notify_thermal_state_changed(runtime_id, state));

    runtime_status(result)
}

/// Notify the runtime host state that power mode changed on iOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_power_mode_changed(
    runtime_id: u64,
    power_mode_code: u32,
) -> RuntimeStatus {
    let result = decode_ios_power_mode(power_mode_code)
        .and_then(|mode| ios_notify_power_mode_changed(runtime_id, mode));

    runtime_status(result)
}

/// Notify the runtime host state that wall clock changed on iOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_wall_clock_changed(
    runtime_id: u64,
) -> RuntimeStatus {
    runtime_status(ios_notify_wall_clock_changed(runtime_id))
}

/// Wake one blocked host event poll operation for iOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_wake(runtime_id: u64) -> RuntimeStatus {
    runtime_status(ios_notify_wake(runtime_id))
}

/// Convert one iOS lifecycle code into the runtime lifecycle enum.
fn decode_ios_application_lifecycle(lifecycle_code: u32) -> RuntimeResult<IosApplicationLifecycle> {
    let lifecycle = match lifecycle_code {
        IOS_LIFECYCLE_DID_FINISH_LAUNCHING => IosApplicationLifecycle::DidFinishLaunching,
        IOS_LIFECYCLE_DID_BECOME_ACTIVE => IosApplicationLifecycle::DidBecomeActive,
        IOS_LIFECYCLE_WILL_RESIGN_ACTIVE => IosApplicationLifecycle::WillResignActive,
        IOS_LIFECYCLE_DID_ENTER_BACKGROUND => IosApplicationLifecycle::DidEnterBackground,
        IOS_LIFECYCLE_WILL_ENTER_FOREGROUND => IosApplicationLifecycle::WillEnterForeground,
        IOS_LIFECYCLE_WILL_TERMINATE => IosApplicationLifecycle::WillTerminate,
        _ => {
            return Err(core_host::invalid_argument_value(
                "lifecycle_code",
                "invalid ios lifecycle code",
            ));
        }
    };

    Ok(lifecycle)
}

/// Convert one iOS memory pressure code into the runtime memory pressure enum.
fn decode_ios_memory_pressure_level(level_code: u32) -> RuntimeResult<HostMemoryPressureLevel> {
    let level = match level_code {
        IOS_MEMORY_PRESSURE_NORMAL => HostMemoryPressureLevel::Normal,
        IOS_MEMORY_PRESSURE_WARNING => HostMemoryPressureLevel::Warning,
        IOS_MEMORY_PRESSURE_CRITICAL => HostMemoryPressureLevel::Critical,
        _ => {
            return Err(core_host::invalid_argument_value(
                "level_code",
                "invalid ios memory pressure level code",
            ));
        }
    };

    Ok(level)
}

/// Convert one iOS thermal code into the runtime thermal enum.
fn decode_ios_thermal_state(thermal_code: u32) -> RuntimeResult<HostThermalState> {
    let state = match thermal_code {
        IOS_THERMAL_NOMINAL => HostThermalState::Nominal,
        IOS_THERMAL_FAIR => HostThermalState::Fair,
        IOS_THERMAL_SERIOUS => HostThermalState::Serious,
        IOS_THERMAL_CRITICAL => HostThermalState::Critical,
        _ => {
            return Err(core_host::invalid_argument_value(
                "thermal_code",
                "invalid ios thermal state code",
            ));
        }
    };

    Ok(state)
}

/// Convert one iOS power mode code into the runtime power mode enum.
fn decode_ios_power_mode(power_mode_code: u32) -> RuntimeResult<HostPowerMode> {
    let mode = match power_mode_code {
        IOS_POWER_MODE_NORMAL => HostPowerMode::Normal,
        IOS_POWER_MODE_LOW_POWER => HostPowerMode::LowPower,
        _ => {
            return Err(core_host::invalid_argument_value(
                "power_mode_code",
                "invalid ios power mode code",
            ));
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
