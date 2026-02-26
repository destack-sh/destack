#[cfg(target_os = "solaris")]
mod adapter;
#[cfg(any(test, target_os = "solaris"))]
mod callback;
#[cfg(any(test, target_os = "solaris"))]
mod ffi;
#[cfg(test)]
mod tests;

#[cfg(target_os = "solaris")]
pub(super) use adapter::SolarisHostAdapter;
#[cfg(any(test, target_os = "solaris"))]
pub use callback::{
    SolarisApplicationLifecycle, solaris_notify_application_lifecycle,
    solaris_notify_interruption_changed, solaris_notify_memory_pressure_changed,
    solaris_notify_permission_result, solaris_notify_power_mode_changed,
    solaris_notify_thermal_state_changed, solaris_notify_wake, solaris_notify_wall_clock_changed,
    solaris_notify_window_available, solaris_notify_window_focus_changed,
    solaris_notify_window_resized, solaris_notify_window_terminated,
};
#[cfg(any(test, target_os = "solaris"))]
pub use ffi::{
    destack_runtime_host_solaris_notify_application_lifecycle,
    destack_runtime_host_solaris_notify_interruption_changed,
    destack_runtime_host_solaris_notify_memory_pressure_changed,
    destack_runtime_host_solaris_notify_permission_result,
    destack_runtime_host_solaris_notify_power_mode_changed,
    destack_runtime_host_solaris_notify_thermal_state_changed,
    destack_runtime_host_solaris_notify_wake,
    destack_runtime_host_solaris_notify_wall_clock_changed,
    destack_runtime_host_solaris_notify_window_available,
    destack_runtime_host_solaris_notify_window_focus_changed,
    destack_runtime_host_solaris_notify_window_resized,
    destack_runtime_host_solaris_notify_window_terminated,
};
