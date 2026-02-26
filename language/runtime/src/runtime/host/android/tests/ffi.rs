use super::{
    ANDROID_LIFECYCLE_CREATED, ANDROID_LIFECYCLE_DESTROYED, ANDROID_LIFECYCLE_PAUSED,
    ANDROID_LIFECYCLE_RESUMED, ANDROID_LIFECYCLE_STARTED, ANDROID_LIFECYCLE_STOPPED,
    ANDROID_MEMORY_PRESSURE_CRITICAL, ANDROID_MEMORY_PRESSURE_NORMAL,
    ANDROID_MEMORY_PRESSURE_WARNING, ANDROID_POWER_MODE_LOW_POWER, ANDROID_POWER_MODE_NORMAL,
    ANDROID_THERMAL_CRITICAL, ANDROID_THERMAL_FAIR, ANDROID_THERMAL_NOMINAL,
    ANDROID_THERMAL_SERIOUS, decode_android_activity_lifecycle,
    decode_android_memory_pressure_level, decode_android_power_mode, decode_android_thermal_state,
};
use crate::runtime::host::{
    AndroidActivityLifecycle, HostMemoryPressureLevel, HostPowerMode, HostThermalState,
};

#[test]
fn test_decode_lifecycle_created() {
    let lifecycle = decode_android_activity_lifecycle(ANDROID_LIFECYCLE_CREATED).unwrap();
    assert_eq!(lifecycle, AndroidActivityLifecycle::Created);
}

#[test]
fn test_decode_lifecycle_resumed() {
    let lifecycle = decode_android_activity_lifecycle(ANDROID_LIFECYCLE_RESUMED).unwrap();
    assert_eq!(lifecycle, AndroidActivityLifecycle::Resumed);
}

#[test]
fn test_decode_lifecycle_started() {
    let lifecycle = decode_android_activity_lifecycle(ANDROID_LIFECYCLE_STARTED).unwrap();
    assert_eq!(lifecycle, AndroidActivityLifecycle::Started);
}

#[test]
fn test_decode_lifecycle_paused() {
    let lifecycle = decode_android_activity_lifecycle(ANDROID_LIFECYCLE_PAUSED).unwrap();
    assert_eq!(lifecycle, AndroidActivityLifecycle::Paused);
}

#[test]
fn test_decode_lifecycle_stopped() {
    let lifecycle = decode_android_activity_lifecycle(ANDROID_LIFECYCLE_STOPPED).unwrap();
    assert_eq!(lifecycle, AndroidActivityLifecycle::Stopped);
}

#[test]
fn test_decode_lifecycle_destroyed() {
    let lifecycle = decode_android_activity_lifecycle(ANDROID_LIFECYCLE_DESTROYED).unwrap();
    assert_eq!(lifecycle, AndroidActivityLifecycle::Destroyed);
}

#[test]
fn test_decode_lifecycle_rejects_unknown_code() {
    let result = decode_android_activity_lifecycle(u32::MAX);
    assert!(result.is_err());
}

#[test]
fn test_decode_memory_pressure_codes() {
    assert_eq!(
        decode_android_memory_pressure_level(ANDROID_MEMORY_PRESSURE_NORMAL).unwrap(),
        HostMemoryPressureLevel::Normal
    );
    assert_eq!(
        decode_android_memory_pressure_level(ANDROID_MEMORY_PRESSURE_WARNING).unwrap(),
        HostMemoryPressureLevel::Warning
    );
    assert_eq!(
        decode_android_memory_pressure_level(ANDROID_MEMORY_PRESSURE_CRITICAL).unwrap(),
        HostMemoryPressureLevel::Critical
    );
    assert!(decode_android_memory_pressure_level(u32::MAX).is_err());
}

#[test]
fn test_decode_thermal_codes() {
    assert_eq!(
        decode_android_thermal_state(ANDROID_THERMAL_NOMINAL).unwrap(),
        HostThermalState::Nominal
    );
    assert_eq!(
        decode_android_thermal_state(ANDROID_THERMAL_FAIR).unwrap(),
        HostThermalState::Fair
    );
    assert_eq!(
        decode_android_thermal_state(ANDROID_THERMAL_SERIOUS).unwrap(),
        HostThermalState::Serious
    );
    assert_eq!(
        decode_android_thermal_state(ANDROID_THERMAL_CRITICAL).unwrap(),
        HostThermalState::Critical
    );
    assert!(decode_android_thermal_state(u32::MAX).is_err());
}

#[test]
fn test_decode_power_mode_codes() {
    assert_eq!(
        decode_android_power_mode(ANDROID_POWER_MODE_NORMAL).unwrap(),
        HostPowerMode::Normal
    );
    assert_eq!(
        decode_android_power_mode(ANDROID_POWER_MODE_LOW_POWER).unwrap(),
        HostPowerMode::LowPower
    );
    assert!(decode_android_power_mode(u32::MAX).is_err());
}
