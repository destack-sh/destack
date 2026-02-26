#[cfg(target_os = "linux")]
mod adapter;
#[cfg(any(test, target_os = "linux"))]
mod callback;
#[cfg(any(test, target_os = "linux"))]
mod ffi;
#[cfg(test)]
mod tests;

#[cfg(target_os = "linux")]
pub(super) use adapter::LinuxHostAdapter;
#[cfg(any(test, target_os = "linux"))]
pub use callback::{
    LinuxApplicationLifecycle, linux_notify_application_lifecycle,
    linux_notify_interruption_changed, linux_notify_memory_pressure_changed,
    linux_notify_permission_result, linux_notify_power_mode_changed,
    linux_notify_thermal_state_changed, linux_notify_wake, linux_notify_wall_clock_changed,
    linux_notify_window_available, linux_notify_window_focus_changed, linux_notify_window_resized,
    linux_notify_window_terminated,
};
#[cfg(any(test, target_os = "linux"))]
pub use ffi::{
    destack_runtime_host_linux_notify_application_lifecycle,
    destack_runtime_host_linux_notify_interruption_changed,
    destack_runtime_host_linux_notify_memory_pressure_changed,
    destack_runtime_host_linux_notify_permission_result,
    destack_runtime_host_linux_notify_power_mode_changed,
    destack_runtime_host_linux_notify_thermal_state_changed,
    destack_runtime_host_linux_notify_wake, destack_runtime_host_linux_notify_wall_clock_changed,
    destack_runtime_host_linux_notify_window_available,
    destack_runtime_host_linux_notify_window_focus_changed,
    destack_runtime_host_linux_notify_window_resized,
    destack_runtime_host_linux_notify_window_terminated,
};
