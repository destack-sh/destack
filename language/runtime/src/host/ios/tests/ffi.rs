use super::{
    IOS_LIFECYCLE_DID_BECOME_ACTIVE, IOS_LIFECYCLE_DID_ENTER_BACKGROUND,
    IOS_LIFECYCLE_DID_FINISH_LAUNCHING, IOS_LIFECYCLE_WILL_ENTER_FOREGROUND,
    IOS_LIFECYCLE_WILL_RESIGN_ACTIVE, IOS_LIFECYCLE_WILL_TERMINATE, IOS_MEMORY_PRESSURE_CRITICAL,
    IOS_MEMORY_PRESSURE_NORMAL, IOS_MEMORY_PRESSURE_WARNING, IOS_POWER_MODE_LOW_POWER,
    IOS_POWER_MODE_NORMAL, IOS_THERMAL_CRITICAL, IOS_THERMAL_FAIR, IOS_THERMAL_NOMINAL,
    IOS_THERMAL_SERIOUS, decode_ios_application_lifecycle, decode_ios_memory_pressure_level,
    decode_ios_power_mode, decode_ios_thermal_state,
};
use crate::host::{
    HostMemoryPressureLevel, HostPowerMode, HostThermalState, IosApplicationLifecycle,
};

#[test]
fn test_decode_lifecycle_did_finish_launching() {
    let lifecycle = decode_ios_application_lifecycle(IOS_LIFECYCLE_DID_FINISH_LAUNCHING).unwrap();
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
    let lifecycle = decode_ios_application_lifecycle(IOS_LIFECYCLE_DID_ENTER_BACKGROUND).unwrap();
    assert_eq!(lifecycle, IosApplicationLifecycle::DidEnterBackground);
}

#[test]
fn test_decode_lifecycle_will_enter_foreground() {
    let lifecycle = decode_ios_application_lifecycle(IOS_LIFECYCLE_WILL_ENTER_FOREGROUND).unwrap();
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
