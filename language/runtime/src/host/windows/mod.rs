#[cfg(windows)]
mod backend;
#[cfg(any(test, windows))]
mod callback;
#[cfg(any(test, windows))]
mod ffi;
#[cfg(windows)]
mod message;
#[cfg(test)]
mod tests;

#[cfg(windows)]
pub(crate) use backend::WindowsHost;
#[cfg(any(test, windows))]
pub use callback::{
    WindowsApplicationLifecycle, windows_notify_application_lifecycle,
    windows_notify_interruption_changed, windows_notify_memory_pressure_changed,
    windows_notify_permission_result, windows_notify_power_mode_changed,
    windows_notify_thermal_state_changed, windows_notify_wake, windows_notify_wall_clock_changed,
};
#[cfg(any(test, windows))]
pub use ffi::{
    destack_host_windows_notify_application_lifecycle,
    destack_host_windows_notify_interruption_changed,
    destack_host_windows_notify_memory_pressure_changed,
    destack_host_windows_notify_permission_result, destack_host_windows_notify_power_mode_changed,
    destack_host_windows_notify_thermal_state_changed, destack_host_windows_notify_wake,
    destack_host_windows_notify_wall_clock_changed,
};
#[cfg(windows)]
pub(crate) use message::process_ingress_loop;
