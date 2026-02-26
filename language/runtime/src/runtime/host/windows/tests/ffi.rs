use super::{
    WINDOWS_LIFECYCLE_ACTIVATED, WINDOWS_LIFECYCLE_CREATED, WINDOWS_LIFECYCLE_DESTROYED,
    WINDOWS_LIFECYCLE_RESUMED, WINDOWS_LIFECYCLE_STOPPING, WINDOWS_LIFECYCLE_SUSPENDED,
    WINDOWS_MEMORY_PRESSURE_CRITICAL, WINDOWS_MEMORY_PRESSURE_NORMAL,
    WINDOWS_MEMORY_PRESSURE_WARNING, WINDOWS_POWER_MODE_LOW_POWER, WINDOWS_POWER_MODE_NORMAL,
    WINDOWS_THERMAL_CRITICAL, WINDOWS_THERMAL_FAIR, WINDOWS_THERMAL_NOMINAL,
    WINDOWS_THERMAL_SERIOUS, decode_windows_application_lifecycle,
    decode_windows_memory_pressure_level, decode_windows_power_mode, decode_windows_thermal_state,
};
use crate::runtime::host::{
    HostMemoryPressureLevel, HostPowerMode, HostThermalState, WindowsApplicationLifecycle,
};

#[test]
fn test_decode_windows_lifecycle_created() {
    let lifecycle = decode_windows_application_lifecycle(WINDOWS_LIFECYCLE_CREATED).unwrap();
    assert_eq!(lifecycle, WindowsApplicationLifecycle::Created);
}

#[test]
fn test_decode_windows_lifecycle_activated() {
    let lifecycle = decode_windows_application_lifecycle(WINDOWS_LIFECYCLE_ACTIVATED).unwrap();
    assert_eq!(lifecycle, WindowsApplicationLifecycle::Activated);
}

#[test]
fn test_decode_windows_lifecycle_resumed() {
    let lifecycle = decode_windows_application_lifecycle(WINDOWS_LIFECYCLE_RESUMED).unwrap();
    assert_eq!(lifecycle, WindowsApplicationLifecycle::Resumed);
}

#[test]
fn test_decode_windows_lifecycle_suspended() {
    let lifecycle = decode_windows_application_lifecycle(WINDOWS_LIFECYCLE_SUSPENDED).unwrap();
    assert_eq!(lifecycle, WindowsApplicationLifecycle::Suspended);
}

#[test]
fn test_decode_windows_lifecycle_stopping() {
    let lifecycle = decode_windows_application_lifecycle(WINDOWS_LIFECYCLE_STOPPING).unwrap();
    assert_eq!(lifecycle, WindowsApplicationLifecycle::Stopping);
}

#[test]
fn test_decode_windows_lifecycle_destroyed() {
    let lifecycle = decode_windows_application_lifecycle(WINDOWS_LIFECYCLE_DESTROYED).unwrap();
    assert_eq!(lifecycle, WindowsApplicationLifecycle::Destroyed);
}

#[test]
fn test_decode_windows_lifecycle_rejects_unknown_code() {
    let result = decode_windows_application_lifecycle(u32::MAX);
    assert!(result.is_err());
}

#[test]
fn test_decode_windows_memory_pressure_codes() {
    assert_eq!(
        decode_windows_memory_pressure_level(WINDOWS_MEMORY_PRESSURE_NORMAL).unwrap(),
        HostMemoryPressureLevel::Normal
    );
    assert_eq!(
        decode_windows_memory_pressure_level(WINDOWS_MEMORY_PRESSURE_WARNING).unwrap(),
        HostMemoryPressureLevel::Warning
    );
    assert_eq!(
        decode_windows_memory_pressure_level(WINDOWS_MEMORY_PRESSURE_CRITICAL).unwrap(),
        HostMemoryPressureLevel::Critical
    );
    assert!(decode_windows_memory_pressure_level(u32::MAX).is_err());
}

#[test]
fn test_decode_windows_thermal_codes() {
    assert_eq!(
        decode_windows_thermal_state(WINDOWS_THERMAL_NOMINAL).unwrap(),
        HostThermalState::Nominal
    );
    assert_eq!(
        decode_windows_thermal_state(WINDOWS_THERMAL_FAIR).unwrap(),
        HostThermalState::Fair
    );
    assert_eq!(
        decode_windows_thermal_state(WINDOWS_THERMAL_SERIOUS).unwrap(),
        HostThermalState::Serious
    );
    assert_eq!(
        decode_windows_thermal_state(WINDOWS_THERMAL_CRITICAL).unwrap(),
        HostThermalState::Critical
    );
    assert!(decode_windows_thermal_state(u32::MAX).is_err());
}

#[test]
fn test_decode_windows_power_mode_codes() {
    assert_eq!(
        decode_windows_power_mode(WINDOWS_POWER_MODE_NORMAL).unwrap(),
        HostPowerMode::Normal
    );
    assert_eq!(
        decode_windows_power_mode(WINDOWS_POWER_MODE_LOW_POWER).unwrap(),
        HostPowerMode::LowPower
    );
    assert!(decode_windows_power_mode(u32::MAX).is_err());
}
