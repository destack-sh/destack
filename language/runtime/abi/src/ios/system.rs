use crate::diagnostic::RuntimeStatus;
use crate::host::apple::abi::ingress::{
    destack_host_ios_notify_interruption_changed as runtime_notify_interruption_changed,
    destack_host_ios_notify_memory_pressure_changed as runtime_notify_memory_pressure_changed,
    destack_host_ios_notify_power_mode_changed as runtime_notify_power_mode_changed,
    destack_host_ios_notify_thermal_state_changed as runtime_notify_thermal_state_changed,
    destack_host_ios_notify_wake as runtime_notify_wake,
    destack_host_ios_notify_wall_clock_changed as runtime_notify_wall_clock_changed,
};

/// Forward the Apple interruption ingress through the ABI artifact.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeStatus {
    unsafe { runtime_notify_interruption_changed(runtime_id, interrupted) }
}

/// Forward the Apple memory-pressure ingress through the ABI artifact.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_memory_pressure_changed(
    runtime_id: u64,
    level_code: u32,
) -> RuntimeStatus {
    unsafe { runtime_notify_memory_pressure_changed(runtime_id, level_code) }
}

/// Forward the Apple thermal-state ingress through the ABI artifact.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_thermal_state_changed(
    runtime_id: u64,
    thermal_code: u32,
) -> RuntimeStatus {
    unsafe { runtime_notify_thermal_state_changed(runtime_id, thermal_code) }
}

/// Forward the Apple power-mode ingress through the ABI artifact.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_power_mode_changed(
    runtime_id: u64,
    power_mode_code: u32,
) -> RuntimeStatus {
    unsafe { runtime_notify_power_mode_changed(runtime_id, power_mode_code) }
}

/// Forward the Apple wall-clock ingress through the ABI artifact.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_wall_clock_changed(
    runtime_id: u64,
) -> RuntimeStatus {
    unsafe { runtime_notify_wall_clock_changed(runtime_id) }
}

/// Forward the Apple wake ingress through the ABI artifact.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_wake(runtime_id: u64) -> RuntimeStatus {
    unsafe { runtime_notify_wake(runtime_id) }
}
