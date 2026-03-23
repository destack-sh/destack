#![allow(unreachable_pub)]

use crate::diagnostic::RuntimeStatus;
use crate::host::linux::ingress::notify::{
    LinuxApplicationLifecycle, linux_notify_application_lifecycle,
    linux_notify_interruption_changed, linux_notify_memory_pressure_changed,
    linux_notify_permission_result, linux_notify_power_mode_changed,
    linux_notify_thermal_state_changed, linux_notify_wake, linux_notify_wall_clock_changed,
};
use crate::host::unix::abi::ingress::{
    decode_unix_application_lifecycle, decode_unix_memory_pressure_level,
    decode_unix_permission_name, decode_unix_power_mode, decode_unix_thermal_state,
    unix_runtime_status,
};
use crate::runtime::NativeStringRef;

/// Notify the runtime host about one Linux application lifecycle transition.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_linux_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle_code: u32,
) -> RuntimeStatus {
    let result = decode_unix_application_lifecycle(lifecycle_code).and_then(
        |lifecycle: LinuxApplicationLifecycle| {
            linux_notify_application_lifecycle(runtime_id, lifecycle)
        },
    );

    unix_runtime_status(result)
}

/// Notify the runtime host with one Linux permission result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_linux_notify_permission_result(
    runtime_id: u64,
    permission: NativeStringRef,
    granted: bool,
) -> RuntimeStatus {
    let result = decode_unix_permission_name(permission).and_then(|permission: String| {
        linux_notify_permission_result(runtime_id, permission.as_str(), granted)
    });

    unix_runtime_status(result)
}

/// Notify the runtime host that interruption state changed on Linux.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_linux_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeStatus {
    unix_runtime_status(linux_notify_interruption_changed(runtime_id, interrupted))
}

/// Notify the runtime host that memory pressure changed on Linux.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_linux_notify_memory_pressure_changed(
    runtime_id: u64,
    level_code: u32,
) -> RuntimeStatus {
    let result = decode_unix_memory_pressure_level(level_code)
        .and_then(|level| linux_notify_memory_pressure_changed(runtime_id, level));

    unix_runtime_status(result)
}

/// Notify the runtime host that thermal state changed on Linux.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_linux_notify_thermal_state_changed(
    runtime_id: u64,
    thermal_code: u32,
) -> RuntimeStatus {
    let result = decode_unix_thermal_state(thermal_code)
        .and_then(|state| linux_notify_thermal_state_changed(runtime_id, state));

    unix_runtime_status(result)
}

/// Notify the runtime host that power mode changed on Linux.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_linux_notify_power_mode_changed(
    runtime_id: u64,
    power_mode_code: u32,
) -> RuntimeStatus {
    let result = decode_unix_power_mode(power_mode_code)
        .and_then(|mode| linux_notify_power_mode_changed(runtime_id, mode));

    unix_runtime_status(result)
}

/// Notify the runtime host that wall clock changed on Linux.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_linux_notify_wall_clock_changed(
    runtime_id: u64,
) -> RuntimeStatus {
    unix_runtime_status(linux_notify_wall_clock_changed(runtime_id))
}

/// Wake one blocked host event poll operation for Linux.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_linux_notify_wake(runtime_id: u64) -> RuntimeStatus {
    unix_runtime_status(linux_notify_wake(runtime_id))
}
