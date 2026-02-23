#[cfg(target_os = "illumos")]
mod adapter;
#[cfg(any(test, target_os = "illumos"))]
mod callback;
#[cfg(any(test, target_os = "illumos"))]
mod ffi;

#[cfg(target_os = "illumos")]
pub(super) use adapter::IllumosHostAdapter;
#[cfg(any(test, target_os = "illumos"))]
pub use callback::{
    IllumosApplicationLifecycle, illumos_notify_application_lifecycle,
    illumos_notify_interruption_changed, illumos_notify_memory_pressure_changed,
    illumos_notify_permission_result, illumos_notify_power_mode_changed,
    illumos_notify_thermal_state_changed, illumos_notify_wake, illumos_notify_wall_clock_changed,
    illumos_notify_window_available, illumos_notify_window_focus_changed,
    illumos_notify_window_resized, illumos_notify_window_terminated,
};
#[cfg(any(test, target_os = "illumos"))]
pub use ffi::{
    destack_runtime_host_illumos_notify_application_lifecycle,
    destack_runtime_host_illumos_notify_interruption_changed,
    destack_runtime_host_illumos_notify_memory_pressure_changed,
    destack_runtime_host_illumos_notify_permission_result,
    destack_runtime_host_illumos_notify_power_mode_changed,
    destack_runtime_host_illumos_notify_thermal_state_changed,
    destack_runtime_host_illumos_notify_wake,
    destack_runtime_host_illumos_notify_wall_clock_changed,
    destack_runtime_host_illumos_notify_window_available,
    destack_runtime_host_illumos_notify_window_focus_changed,
    destack_runtime_host_illumos_notify_window_resized,
    destack_runtime_host_illumos_notify_window_terminated,
};
