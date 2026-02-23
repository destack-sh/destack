#[cfg(target_os = "freebsd")]
mod adapter;
#[cfg(any(test, target_os = "freebsd"))]
mod callback;
#[cfg(any(test, target_os = "freebsd"))]
mod ffi;

#[cfg(target_os = "freebsd")]
pub(super) use adapter::FreeBsdHostAdapter;
#[cfg(any(test, target_os = "freebsd"))]
pub use callback::{
    FreeBsdApplicationLifecycle, freebsd_notify_application_lifecycle,
    freebsd_notify_interruption_changed, freebsd_notify_memory_pressure_changed,
    freebsd_notify_permission_result, freebsd_notify_power_mode_changed,
    freebsd_notify_thermal_state_changed, freebsd_notify_wake, freebsd_notify_wall_clock_changed,
    freebsd_notify_window_available, freebsd_notify_window_focus_changed,
    freebsd_notify_window_resized, freebsd_notify_window_terminated,
};
#[cfg(any(test, target_os = "freebsd"))]
pub use ffi::{
    destack_runtime_host_freebsd_notify_application_lifecycle,
    destack_runtime_host_freebsd_notify_interruption_changed,
    destack_runtime_host_freebsd_notify_memory_pressure_changed,
    destack_runtime_host_freebsd_notify_permission_result,
    destack_runtime_host_freebsd_notify_power_mode_changed,
    destack_runtime_host_freebsd_notify_thermal_state_changed,
    destack_runtime_host_freebsd_notify_wake,
    destack_runtime_host_freebsd_notify_wall_clock_changed,
    destack_runtime_host_freebsd_notify_window_available,
    destack_runtime_host_freebsd_notify_window_focus_changed,
    destack_runtime_host_freebsd_notify_window_resized,
    destack_runtime_host_freebsd_notify_window_terminated,
};
