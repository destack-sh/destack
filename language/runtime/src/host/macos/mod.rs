#[cfg(target_os = "macos")]
mod backend;
mod callback;
mod ffi;
#[cfg(test)]
mod tests;

#[cfg(target_os = "macos")]
pub(crate) use backend::MacosHost;
pub use callback::{
    MacosApplicationLifecycle, macos_notify_application_lifecycle,
    macos_notify_interruption_changed, macos_notify_memory_pressure_changed,
    macos_notify_permission_result, macos_notify_power_mode_changed,
    macos_notify_thermal_state_changed, macos_notify_wake, macos_notify_wall_clock_changed,
};
pub use ffi::{
    destack_host_macos_notify_application_lifecycle,
    destack_host_macos_notify_interruption_changed,
    destack_host_macos_notify_memory_pressure_changed, destack_host_macos_notify_permission_result,
    destack_host_macos_notify_power_mode_changed, destack_host_macos_notify_thermal_state_changed,
    destack_host_macos_notify_wake, destack_host_macos_notify_wall_clock_changed,
};
