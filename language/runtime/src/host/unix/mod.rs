mod callback;
mod ffi;
#[cfg(test)]
mod tests;

pub use callback::{
    UnixApplicationLifecycle, unix_notify_application_lifecycle, unix_notify_interruption_changed,
    unix_notify_memory_pressure_changed, unix_notify_permission_result,
    unix_notify_power_mode_changed, unix_notify_thermal_state_changed, unix_notify_wake,
    unix_notify_wall_clock_changed, unix_notify_window_available, unix_notify_window_focus_changed,
    unix_notify_window_resized, unix_notify_window_terminated,
};
pub(crate) use ffi::{
    decode_unix_application_lifecycle, decode_unix_memory_pressure_level,
    decode_unix_permission_name, decode_unix_power_mode, decode_unix_thermal_state,
    unix_runtime_status,
};
