use crate::host::macos::MacosApplicationLifecycle;
use crate::host::macos::ingress::ffi::{
    MACOS_LIFECYCLE_DID_BECOME_ACTIVE, MACOS_LIFECYCLE_DID_FINISH_LAUNCHING,
    MACOS_LIFECYCLE_WILL_RESIGN_ACTIVE, MACOS_LIFECYCLE_WILL_TERMINATE,
    MACOS_MEMORY_PRESSURE_CRITICAL, MACOS_MEMORY_PRESSURE_NORMAL, MACOS_MEMORY_PRESSURE_WARNING,
    MACOS_POWER_MODE_LOW_POWER, MACOS_POWER_MODE_NORMAL, MACOS_THERMAL_CRITICAL,
    MACOS_THERMAL_FAIR, MACOS_THERMAL_NOMINAL, MACOS_THERMAL_SERIOUS,
    decode_macos_application_lifecycle, decode_macos_memory_pressure_level,
    decode_macos_power_mode, decode_macos_thermal_state,
};
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};

#[test]
fn test_decode_lifecycle_did_finish_launching() {
    let lifecycle =
        decode_macos_application_lifecycle(MACOS_LIFECYCLE_DID_FINISH_LAUNCHING).unwrap();
    assert_eq!(lifecycle, MacosApplicationLifecycle::DidFinishLaunching);
}

#[test]
fn test_decode_lifecycle_did_become_active() {
    let lifecycle = decode_macos_application_lifecycle(MACOS_LIFECYCLE_DID_BECOME_ACTIVE).unwrap();
    assert_eq!(lifecycle, MacosApplicationLifecycle::DidBecomeActive);
}

#[test]
fn test_decode_lifecycle_will_resign_active() {
    let lifecycle = decode_macos_application_lifecycle(MACOS_LIFECYCLE_WILL_RESIGN_ACTIVE).unwrap();
    assert_eq!(lifecycle, MacosApplicationLifecycle::WillResignActive);
}

#[test]
fn test_decode_lifecycle_will_terminate() {
    let lifecycle = decode_macos_application_lifecycle(MACOS_LIFECYCLE_WILL_TERMINATE).unwrap();
    assert_eq!(lifecycle, MacosApplicationLifecycle::WillTerminate);
}

#[test]
fn test_decode_lifecycle_rejects_unknown_code() {
    let result = decode_macos_application_lifecycle(u32::MAX);
    assert!(result.is_err());
}

#[test]
fn test_decode_memory_pressure_codes() {
    assert_eq!(
        decode_macos_memory_pressure_level(MACOS_MEMORY_PRESSURE_NORMAL).unwrap(),
        HostMemoryPressureLevel::Normal
    );
    assert_eq!(
        decode_macos_memory_pressure_level(MACOS_MEMORY_PRESSURE_WARNING).unwrap(),
        HostMemoryPressureLevel::Warning
    );
    assert_eq!(
        decode_macos_memory_pressure_level(MACOS_MEMORY_PRESSURE_CRITICAL).unwrap(),
        HostMemoryPressureLevel::Critical
    );
    assert!(decode_macos_memory_pressure_level(u32::MAX).is_err());
}

#[test]
fn test_decode_thermal_codes() {
    assert_eq!(
        decode_macos_thermal_state(MACOS_THERMAL_NOMINAL).unwrap(),
        HostThermalState::Nominal
    );
    assert_eq!(
        decode_macos_thermal_state(MACOS_THERMAL_FAIR).unwrap(),
        HostThermalState::Fair
    );
    assert_eq!(
        decode_macos_thermal_state(MACOS_THERMAL_SERIOUS).unwrap(),
        HostThermalState::Serious
    );
    assert_eq!(
        decode_macos_thermal_state(MACOS_THERMAL_CRITICAL).unwrap(),
        HostThermalState::Critical
    );
    assert!(decode_macos_thermal_state(u32::MAX).is_err());
}

#[test]
fn test_decode_power_mode_codes() {
    assert_eq!(
        decode_macos_power_mode(MACOS_POWER_MODE_NORMAL).unwrap(),
        HostPowerMode::Normal
    );
    assert_eq!(
        decode_macos_power_mode(MACOS_POWER_MODE_LOW_POWER).unwrap(),
        HostPowerMode::LowPower
    );
    assert!(decode_macos_power_mode(u32::MAX).is_err());
}
