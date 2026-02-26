#[cfg(windows)]
mod adapter;
#[cfg(any(test, windows))]
mod callback;
#[cfg(any(test, windows))]
mod ffi;
#[cfg(test)]
mod tests;

#[cfg(windows)]
pub(super) use adapter::WindowsHost;
#[cfg(any(test, windows))]
pub use callback::{
    WindowsApplicationLifecycle, windows_notify_application_lifecycle,
    windows_notify_interruption_changed, windows_notify_memory_pressure_changed,
    windows_notify_permission_result, windows_notify_power_mode_changed,
    windows_notify_thermal_state_changed, windows_notify_wake, windows_notify_wall_clock_changed,
    windows_notify_window_available, windows_notify_window_focus_changed,
    windows_notify_window_resized, windows_notify_window_terminated,
};
#[cfg(any(test, windows))]
pub use ffi::{
    destack_host_windows_notify_application_lifecycle,
    destack_host_windows_notify_interruption_changed,
    destack_host_windows_notify_memory_pressure_changed,
    destack_host_windows_notify_permission_result, destack_host_windows_notify_power_mode_changed,
    destack_host_windows_notify_thermal_state_changed, destack_host_windows_notify_wake,
    destack_host_windows_notify_wall_clock_changed, destack_host_windows_notify_window_available,
    destack_host_windows_notify_window_focus_changed, destack_host_windows_notify_window_resized,
    destack_host_windows_notify_window_terminated,
};
