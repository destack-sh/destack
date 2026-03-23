use crate::host::unix::abi::ingress::{
    UNIX_LIFECYCLE_CREATED, UNIX_LIFECYCLE_DESTROYED, UNIX_LIFECYCLE_PAUSED,
    UNIX_LIFECYCLE_RUNNING, UNIX_LIFECYCLE_STOPPED, UNIX_MEMORY_PRESSURE_CRITICAL,
    UNIX_MEMORY_PRESSURE_NORMAL, UNIX_MEMORY_PRESSURE_WARNING, UNIX_POWER_MODE_LOW_POWER,
    UNIX_POWER_MODE_NORMAL, UNIX_THERMAL_CRITICAL, UNIX_THERMAL_FAIR, UNIX_THERMAL_NOMINAL,
    UNIX_THERMAL_SERIOUS, decode_unix_application_lifecycle, decode_unix_memory_pressure_level,
    decode_unix_power_mode, decode_unix_thermal_state,
};
use crate::host::unix::ingress::notify::UnixApplicationLifecycle;
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};

#[test]
fn test_decode_unix_lifecycle_codes() {
    assert_eq!(
        decode_unix_application_lifecycle(UNIX_LIFECYCLE_CREATED).unwrap(),
        UnixApplicationLifecycle::Created
    );
    assert_eq!(
        decode_unix_application_lifecycle(UNIX_LIFECYCLE_RUNNING).unwrap(),
        UnixApplicationLifecycle::Running
    );
    assert_eq!(
        decode_unix_application_lifecycle(UNIX_LIFECYCLE_PAUSED).unwrap(),
        UnixApplicationLifecycle::Paused
    );
    assert_eq!(
        decode_unix_application_lifecycle(UNIX_LIFECYCLE_STOPPED).unwrap(),
        UnixApplicationLifecycle::Stopped
    );
    assert_eq!(
        decode_unix_application_lifecycle(UNIX_LIFECYCLE_DESTROYED).unwrap(),
        UnixApplicationLifecycle::Destroyed
    );
}

#[test]
fn test_decode_unix_lifecycle_rejects_unknown_code() {
    let result = decode_unix_application_lifecycle(u32::MAX);
    assert!(result.is_err());
}

#[test]
fn test_decode_unix_memory_pressure_codes() {
    assert_eq!(
        decode_unix_memory_pressure_level(UNIX_MEMORY_PRESSURE_NORMAL).unwrap(),
        HostMemoryPressureLevel::Normal
    );
    assert_eq!(
        decode_unix_memory_pressure_level(UNIX_MEMORY_PRESSURE_WARNING).unwrap(),
        HostMemoryPressureLevel::Warning
    );
    assert_eq!(
        decode_unix_memory_pressure_level(UNIX_MEMORY_PRESSURE_CRITICAL).unwrap(),
        HostMemoryPressureLevel::Critical
    );
    assert!(decode_unix_memory_pressure_level(u32::MAX).is_err());
}

#[test]
fn test_decode_unix_thermal_codes() {
    assert_eq!(
        decode_unix_thermal_state(UNIX_THERMAL_NOMINAL).unwrap(),
        HostThermalState::Nominal
    );
    assert_eq!(
        decode_unix_thermal_state(UNIX_THERMAL_FAIR).unwrap(),
        HostThermalState::Fair
    );
    assert_eq!(
        decode_unix_thermal_state(UNIX_THERMAL_SERIOUS).unwrap(),
        HostThermalState::Serious
    );
    assert_eq!(
        decode_unix_thermal_state(UNIX_THERMAL_CRITICAL).unwrap(),
        HostThermalState::Critical
    );
    assert!(decode_unix_thermal_state(u32::MAX).is_err());
}

#[test]
fn test_decode_unix_power_mode_codes() {
    assert_eq!(
        decode_unix_power_mode(UNIX_POWER_MODE_NORMAL).unwrap(),
        HostPowerMode::Normal
    );
    assert_eq!(
        decode_unix_power_mode(UNIX_POWER_MODE_LOW_POWER).unwrap(),
        HostPowerMode::LowPower
    );
    assert!(decode_unix_power_mode(u32::MAX).is_err());
}
