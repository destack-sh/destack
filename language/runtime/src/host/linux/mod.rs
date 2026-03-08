#[cfg(target_os = "linux")]
mod backend;
#[cfg(any(test, target_os = "linux"))]
mod callback;
#[cfg(any(test, target_os = "linux"))]
mod ffi;
#[cfg(test)]
mod tests;

#[cfg(target_os = "linux")]
pub(crate) use backend::LinuxHost;
#[cfg(any(test, target_os = "linux"))]
pub use callback::{
    LinuxApplicationLifecycle, linux_notify_application_lifecycle,
    linux_notify_interruption_changed, linux_notify_memory_pressure_changed,
    linux_notify_permission_result, linux_notify_power_mode_changed,
    linux_notify_thermal_state_changed, linux_notify_wake, linux_notify_wall_clock_changed,
};
#[cfg(any(test, target_os = "linux"))]
pub use ffi::{
    destack_host_linux_notify_application_lifecycle,
    destack_host_linux_notify_interruption_changed,
    destack_host_linux_notify_memory_pressure_changed, destack_host_linux_notify_permission_result,
    destack_host_linux_notify_power_mode_changed, destack_host_linux_notify_thermal_state_changed,
    destack_host_linux_notify_wake, destack_host_linux_notify_wall_clock_changed,
};
