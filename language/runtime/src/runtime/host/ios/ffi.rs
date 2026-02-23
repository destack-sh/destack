use super::{
    IosApplicationLifecycle, ios_notify_application_lifecycle, ios_notify_interruption_changed,
    ios_notify_memory_pressure_changed, ios_notify_permission_result,
    ios_notify_power_mode_changed, ios_notify_thermal_state_changed, ios_notify_wake,
    ios_notify_wall_clock_changed, ios_notify_window_available, ios_notify_window_focus_changed,
    ios_notify_window_resized, ios_notify_window_terminated,
};
use crate::diagnostic::{RuntimeError, RuntimeResult, RuntimeStatus};
use crate::platform::{NativeStringRef, PlatformError};
use crate::runtime::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};

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

/// Notify the runtime host bridge about one iOS application lifecycle transition.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_ios_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle_code: u32,
) -> RuntimeStatus {
    let result = decode_ios_application_lifecycle(lifecycle_code)
        .and_then(|lifecycle| ios_notify_application_lifecycle(runtime_id, lifecycle));

    runtime_status(result)
}

/// Notify the runtime host bridge that one iOS window became available.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_ios_notify_window_available(
    runtime_id: u64,
) -> RuntimeStatus {
    runtime_status(ios_notify_window_available(runtime_id))
}

/// Notify the runtime host bridge that one iOS window terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_ios_notify_window_terminated(
    runtime_id: u64,
) -> RuntimeStatus {
    runtime_status(ios_notify_window_terminated(runtime_id))
}

/// Notify the runtime host bridge that one iOS window resized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_ios_notify_window_resized(
    runtime_id: u64,
    width_px: u32,
    height_px: u32,
) -> RuntimeStatus {
    runtime_status(ios_notify_window_resized(runtime_id, width_px, height_px))
}

/// Notify the runtime host bridge that one iOS window focus changed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_ios_notify_window_focus_changed(
    runtime_id: u64,
    is_focused: bool,
) -> RuntimeStatus {
    runtime_status(ios_notify_window_focus_changed(runtime_id, is_focused))
}

/// Notify the runtime host bridge with one iOS permission result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_ios_notify_permission_result(
    runtime_id: u64,
    permission: NativeStringRef,
    granted: bool,
) -> RuntimeStatus {
    let result = decode_permission_name(permission).and_then(|permission| {
        ios_notify_permission_result(runtime_id, permission.as_str(), granted)
    });

    runtime_status(result)
}

/// Notify the runtime host bridge that interruption state changed on iOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_ios_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeStatus {
    runtime_status(ios_notify_interruption_changed(runtime_id, interrupted))
}

/// Notify the runtime host bridge that memory pressure changed on iOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_ios_notify_memory_pressure_changed(
    runtime_id: u64,
    level_code: u32,
) -> RuntimeStatus {
    let result = decode_ios_memory_pressure_level(level_code)
        .and_then(|level| ios_notify_memory_pressure_changed(runtime_id, level));

    runtime_status(result)
}

/// Notify the runtime host bridge that thermal state changed on iOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_ios_notify_thermal_state_changed(
    runtime_id: u64,
    thermal_code: u32,
) -> RuntimeStatus {
    let result = decode_ios_thermal_state(thermal_code)
        .and_then(|state| ios_notify_thermal_state_changed(runtime_id, state));

    runtime_status(result)
}

/// Notify the runtime host bridge that power mode changed on iOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_ios_notify_power_mode_changed(
    runtime_id: u64,
    power_mode_code: u32,
) -> RuntimeStatus {
    let result = decode_ios_power_mode(power_mode_code)
        .and_then(|mode| ios_notify_power_mode_changed(runtime_id, mode));

    runtime_status(result)
}

/// Notify the runtime host bridge that wall clock changed on iOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_ios_notify_wall_clock_changed(
    runtime_id: u64,
) -> RuntimeStatus {
    runtime_status(ios_notify_wall_clock_changed(runtime_id))
}

/// Wake one blocked host event poll operation for iOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_ios_notify_wake(runtime_id: u64) -> RuntimeStatus {
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
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "lifecycle_code",
                "invalid ios lifecycle code",
            ))
            .boxed());
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
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "level_code",
                "invalid ios memory pressure level code",
            ))
            .boxed());
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
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "thermal_code",
                "invalid ios thermal state code",
            ))
            .boxed());
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
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "power_mode_code",
                "invalid ios power mode code",
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
        IOS_LIFECYCLE_DID_BECOME_ACTIVE, IOS_LIFECYCLE_DID_ENTER_BACKGROUND,
        IOS_LIFECYCLE_DID_FINISH_LAUNCHING, IOS_LIFECYCLE_WILL_ENTER_FOREGROUND,
        IOS_LIFECYCLE_WILL_RESIGN_ACTIVE, IOS_LIFECYCLE_WILL_TERMINATE,
        IOS_MEMORY_PRESSURE_CRITICAL, IOS_MEMORY_PRESSURE_NORMAL, IOS_MEMORY_PRESSURE_WARNING,
        IOS_POWER_MODE_LOW_POWER, IOS_POWER_MODE_NORMAL, IOS_THERMAL_CRITICAL, IOS_THERMAL_FAIR,
        IOS_THERMAL_NOMINAL, IOS_THERMAL_SERIOUS, decode_ios_application_lifecycle,
        decode_ios_memory_pressure_level, decode_ios_power_mode, decode_ios_thermal_state,
    };
    use crate::runtime::host::{
        HostMemoryPressureLevel, HostPowerMode, HostThermalState, IosApplicationLifecycle,
    };

    #[test]
    fn test_decode_lifecycle_did_finish_launching() {
        let lifecycle =
            decode_ios_application_lifecycle(IOS_LIFECYCLE_DID_FINISH_LAUNCHING).unwrap();
        assert_eq!(lifecycle, IosApplicationLifecycle::DidFinishLaunching);
    }

    #[test]
    fn test_decode_lifecycle_did_become_active() {
        let lifecycle = decode_ios_application_lifecycle(IOS_LIFECYCLE_DID_BECOME_ACTIVE).unwrap();
        assert_eq!(lifecycle, IosApplicationLifecycle::DidBecomeActive);
    }

    #[test]
    fn test_decode_lifecycle_will_resign_active() {
        let lifecycle = decode_ios_application_lifecycle(IOS_LIFECYCLE_WILL_RESIGN_ACTIVE).unwrap();
        assert_eq!(lifecycle, IosApplicationLifecycle::WillResignActive);
    }

    #[test]
    fn test_decode_lifecycle_did_enter_background() {
        let lifecycle =
            decode_ios_application_lifecycle(IOS_LIFECYCLE_DID_ENTER_BACKGROUND).unwrap();
        assert_eq!(lifecycle, IosApplicationLifecycle::DidEnterBackground);
    }

    #[test]
    fn test_decode_lifecycle_will_enter_foreground() {
        let lifecycle =
            decode_ios_application_lifecycle(IOS_LIFECYCLE_WILL_ENTER_FOREGROUND).unwrap();
        assert_eq!(lifecycle, IosApplicationLifecycle::WillEnterForeground);
    }

    #[test]
    fn test_decode_lifecycle_will_terminate() {
        let lifecycle = decode_ios_application_lifecycle(IOS_LIFECYCLE_WILL_TERMINATE).unwrap();
        assert_eq!(lifecycle, IosApplicationLifecycle::WillTerminate);
    }

    #[test]
    fn test_decode_lifecycle_rejects_unknown_code() {
        let result = decode_ios_application_lifecycle(u32::MAX);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_memory_pressure_codes() {
        assert_eq!(
            decode_ios_memory_pressure_level(IOS_MEMORY_PRESSURE_NORMAL).unwrap(),
            HostMemoryPressureLevel::Normal
        );
        assert_eq!(
            decode_ios_memory_pressure_level(IOS_MEMORY_PRESSURE_WARNING).unwrap(),
            HostMemoryPressureLevel::Warning
        );
        assert_eq!(
            decode_ios_memory_pressure_level(IOS_MEMORY_PRESSURE_CRITICAL).unwrap(),
            HostMemoryPressureLevel::Critical
        );
        assert!(decode_ios_memory_pressure_level(u32::MAX).is_err());
    }

    #[test]
    fn test_decode_thermal_codes() {
        assert_eq!(
            decode_ios_thermal_state(IOS_THERMAL_NOMINAL).unwrap(),
            HostThermalState::Nominal
        );
        assert_eq!(
            decode_ios_thermal_state(IOS_THERMAL_FAIR).unwrap(),
            HostThermalState::Fair
        );
        assert_eq!(
            decode_ios_thermal_state(IOS_THERMAL_SERIOUS).unwrap(),
            HostThermalState::Serious
        );
        assert_eq!(
            decode_ios_thermal_state(IOS_THERMAL_CRITICAL).unwrap(),
            HostThermalState::Critical
        );
        assert!(decode_ios_thermal_state(u32::MAX).is_err());
    }

    #[test]
    fn test_decode_power_mode_codes() {
        assert_eq!(
            decode_ios_power_mode(IOS_POWER_MODE_NORMAL).unwrap(),
            HostPowerMode::Normal
        );
        assert_eq!(
            decode_ios_power_mode(IOS_POWER_MODE_LOW_POWER).unwrap(),
            HostPowerMode::LowPower
        );
        assert!(decode_ios_power_mode(u32::MAX).is_err());
    }
}
