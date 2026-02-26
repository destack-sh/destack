mod adapter;
mod callback;
mod ffi;
#[cfg(test)]
mod tests;

pub(super) use adapter::MacosHostAdapter;
pub use callback::{
    MacosApplicationLifecycle, macos_notify_application_lifecycle,
    macos_notify_interruption_changed, macos_notify_memory_pressure_changed,
    macos_notify_permission_result, macos_notify_power_mode_changed,
    macos_notify_thermal_state_changed, macos_notify_wake, macos_notify_wall_clock_changed,
    macos_notify_window_available, macos_notify_window_focus_changed, macos_notify_window_resized,
    macos_notify_window_terminated,
};
pub use ffi::{
    destack_runtime_host_macos_notify_application_lifecycle,
    destack_runtime_host_macos_notify_interruption_changed,
    destack_runtime_host_macos_notify_memory_pressure_changed,
    destack_runtime_host_macos_notify_permission_result,
    destack_runtime_host_macos_notify_power_mode_changed,
    destack_runtime_host_macos_notify_thermal_state_changed,
    destack_runtime_host_macos_notify_wake, destack_runtime_host_macos_notify_wall_clock_changed,
    destack_runtime_host_macos_notify_window_available,
    destack_runtime_host_macos_notify_window_focus_changed,
    destack_runtime_host_macos_notify_window_resized,
    destack_runtime_host_macos_notify_window_terminated,
};
