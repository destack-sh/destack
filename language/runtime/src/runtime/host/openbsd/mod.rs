#[cfg(target_os = "openbsd")]
mod adapter;
#[cfg(any(test, target_os = "openbsd"))]
mod callback;
#[cfg(any(test, target_os = "openbsd"))]
mod ffi;

#[cfg(target_os = "openbsd")]
pub(super) use adapter::OpenBsdHostAdapter;
#[cfg(any(test, target_os = "openbsd"))]
pub use callback::{
    OpenBsdApplicationLifecycle, openbsd_notify_application_lifecycle,
    openbsd_notify_interruption_changed, openbsd_notify_memory_pressure_changed,
    openbsd_notify_permission_result, openbsd_notify_power_mode_changed,
    openbsd_notify_thermal_state_changed, openbsd_notify_wake, openbsd_notify_wall_clock_changed,
    openbsd_notify_window_available, openbsd_notify_window_focus_changed,
    openbsd_notify_window_resized, openbsd_notify_window_terminated,
};
#[cfg(any(test, target_os = "openbsd"))]
pub use ffi::{
    destack_runtime_host_openbsd_notify_application_lifecycle,
    destack_runtime_host_openbsd_notify_interruption_changed,
    destack_runtime_host_openbsd_notify_memory_pressure_changed,
    destack_runtime_host_openbsd_notify_permission_result,
    destack_runtime_host_openbsd_notify_power_mode_changed,
    destack_runtime_host_openbsd_notify_thermal_state_changed,
    destack_runtime_host_openbsd_notify_wake,
    destack_runtime_host_openbsd_notify_wall_clock_changed,
    destack_runtime_host_openbsd_notify_window_available,
    destack_runtime_host_openbsd_notify_window_focus_changed,
    destack_runtime_host_openbsd_notify_window_resized,
    destack_runtime_host_openbsd_notify_window_terminated,
};
