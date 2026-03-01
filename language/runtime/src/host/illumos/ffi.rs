use super::{
    IllumosApplicationLifecycle, illumos_notify_application_lifecycle,
    illumos_notify_interruption_changed, illumos_notify_memory_pressure_changed,
    illumos_notify_permission_result, illumos_notify_power_mode_changed,
    illumos_notify_thermal_state_changed, illumos_notify_wake, illumos_notify_wall_clock_changed,
    illumos_notify_window_available, illumos_notify_window_focus_changed,
    illumos_notify_window_resized, illumos_notify_window_terminated,
};
use crate::diagnostic::RuntimeStatus;
use crate::host::unix::{
    decode_unix_application_lifecycle, decode_unix_memory_pressure_level,
    decode_unix_permission_name, decode_unix_power_mode, decode_unix_thermal_state,
    unix_runtime_status,
};
use crate::platform::NativeStringRef;

/// Notify the runtime host state about one illumos application lifecycle transition.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_illumos_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle_code: u32,
) -> RuntimeStatus {
    let result = decode_unix_application_lifecycle(lifecycle_code).and_then(
        |lifecycle: IllumosApplicationLifecycle| {
            illumos_notify_application_lifecycle(runtime_id, lifecycle)
        },
    );

    unix_runtime_status(result)
}

/// Notify the runtime host state that one illumos window became available.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_illumos_notify_window_available(
    runtime_id: u64,
    window_id: u64,
) -> RuntimeStatus {
    unix_runtime_status(illumos_notify_window_available(runtime_id, window_id))
}

/// Notify the runtime host state that one illumos window terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_illumos_notify_window_terminated(
    runtime_id: u64,
    window_id: u64,
) -> RuntimeStatus {
    unix_runtime_status(illumos_notify_window_terminated(runtime_id, window_id))
}

/// Notify the runtime host state that one illumos window resized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_illumos_notify_window_resized(
    runtime_id: u64,
    window_id: u64,
    width_px: u32,
    height_px: u32,
) -> RuntimeStatus {
    unix_runtime_status(illumos_notify_window_resized(
        runtime_id, window_id, width_px, height_px,
    ))
}

/// Notify the runtime host state that one illumos window focus changed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_illumos_notify_window_focus_changed(
    runtime_id: u64,
    window_id: u64,
    is_focused: bool,
) -> RuntimeStatus {
    unix_runtime_status(illumos_notify_window_focus_changed(
        runtime_id, window_id, is_focused,
    ))
}

/// Notify the runtime host state with one illumos permission result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_illumos_notify_permission_result(
    runtime_id: u64,
    permission: NativeStringRef,
    granted: bool,
) -> RuntimeStatus {
    let result = decode_unix_permission_name(permission).and_then(|permission| {
        illumos_notify_permission_result(runtime_id, permission.as_str(), granted)
    });

    unix_runtime_status(result)
}

/// Notify the runtime host state that interruption state changed on illumos.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_illumos_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeStatus {
    unix_runtime_status(illumos_notify_interruption_changed(runtime_id, interrupted))
}

/// Notify the runtime host state that memory pressure changed on illumos.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_illumos_notify_memory_pressure_changed(
    runtime_id: u64,
    level_code: u32,
) -> RuntimeStatus {
    let result = decode_unix_memory_pressure_level(level_code)
        .and_then(|level| illumos_notify_memory_pressure_changed(runtime_id, level));

    unix_runtime_status(result)
}

/// Notify the runtime host state that thermal state changed on illumos.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_illumos_notify_thermal_state_changed(
    runtime_id: u64,
    thermal_code: u32,
) -> RuntimeStatus {
    let result = decode_unix_thermal_state(thermal_code)
        .and_then(|state| illumos_notify_thermal_state_changed(runtime_id, state));

    unix_runtime_status(result)
}

/// Notify the runtime host state that power mode changed on illumos.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_illumos_notify_power_mode_changed(
    runtime_id: u64,
    power_mode_code: u32,
) -> RuntimeStatus {
    let result = decode_unix_power_mode(power_mode_code)
        .and_then(|mode| illumos_notify_power_mode_changed(runtime_id, mode));

    unix_runtime_status(result)
}

/// Notify the runtime host state that wall clock changed on illumos.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_illumos_notify_wall_clock_changed(
    runtime_id: u64,
) -> RuntimeStatus {
    unix_runtime_status(illumos_notify_wall_clock_changed(runtime_id))
}

/// Wake one blocked host event poll operation for illumos.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_illumos_notify_wake(runtime_id: u64) -> RuntimeStatus {
    unix_runtime_status(illumos_notify_wake(runtime_id))
}
