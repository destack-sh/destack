use super::{
    SolarisApplicationLifecycle, solaris_notify_application_lifecycle,
    solaris_notify_interruption_changed, solaris_notify_memory_pressure_changed,
    solaris_notify_permission_result, solaris_notify_power_mode_changed,
    solaris_notify_thermal_state_changed, solaris_notify_wake, solaris_notify_wall_clock_changed,
};
use crate::diagnostic::RuntimeStatus;
use crate::host::unix::{
    decode_unix_application_lifecycle, decode_unix_memory_pressure_level,
    decode_unix_permission_name, decode_unix_power_mode, decode_unix_thermal_state,
    unix_runtime_status,
};
use crate::runtime::NativeStringRef;

/// Notify the runtime host state about one Solaris application lifecycle transition.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_solaris_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle_code: u32,
) -> RuntimeStatus {
    let result = decode_unix_application_lifecycle(lifecycle_code).and_then(
        |lifecycle: SolarisApplicationLifecycle| {
            solaris_notify_application_lifecycle(runtime_id, lifecycle)
        },
    );

    unix_runtime_status(result)
}

/// Notify the runtime host state with one Solaris permission result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_solaris_notify_permission_result(
    runtime_id: u64,
    permission: NativeStringRef,
    granted: bool,
) -> RuntimeStatus {
    let result = decode_unix_permission_name(permission).and_then(|permission: String| {
        solaris_notify_permission_result(runtime_id, permission.as_str(), granted)
    });

    unix_runtime_status(result)
}

/// Notify the runtime host state that interruption state changed on Solaris.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_solaris_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeStatus {
    unix_runtime_status(solaris_notify_interruption_changed(runtime_id, interrupted))
}

/// Notify the runtime host state that memory pressure changed on Solaris.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_solaris_notify_memory_pressure_changed(
    runtime_id: u64,
    level_code: u32,
) -> RuntimeStatus {
    let result = decode_unix_memory_pressure_level(level_code)
        .and_then(|level| solaris_notify_memory_pressure_changed(runtime_id, level));

    unix_runtime_status(result)
}

/// Notify the runtime host state that thermal state changed on Solaris.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_solaris_notify_thermal_state_changed(
    runtime_id: u64,
    thermal_code: u32,
) -> RuntimeStatus {
    let result = decode_unix_thermal_state(thermal_code)
        .and_then(|state| solaris_notify_thermal_state_changed(runtime_id, state));

    unix_runtime_status(result)
}

/// Notify the runtime host state that power mode changed on Solaris.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_solaris_notify_power_mode_changed(
    runtime_id: u64,
    power_mode_code: u32,
) -> RuntimeStatus {
    let result = decode_unix_power_mode(power_mode_code)
        .and_then(|mode| solaris_notify_power_mode_changed(runtime_id, mode));

    unix_runtime_status(result)
}

/// Notify the runtime host state that wall clock changed on Solaris.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_solaris_notify_wall_clock_changed(
    runtime_id: u64,
) -> RuntimeStatus {
    unix_runtime_status(solaris_notify_wall_clock_changed(runtime_id))
}

/// Wake one blocked host event poll operation for Solaris.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_solaris_notify_wake(runtime_id: u64) -> RuntimeStatus {
    unix_runtime_status(solaris_notify_wake(runtime_id))
}
