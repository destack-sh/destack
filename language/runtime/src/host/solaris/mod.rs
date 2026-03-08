#[cfg(target_os = "solaris")]
mod backend;
#[cfg(any(test, target_os = "solaris"))]
mod callback;
#[cfg(any(test, target_os = "solaris"))]
mod ffi;
#[cfg(test)]
mod tests;

#[cfg(target_os = "solaris")]
pub(crate) use backend::SolarisHost;
#[cfg(any(test, target_os = "solaris"))]
pub use callback::{
    SolarisApplicationLifecycle, solaris_notify_application_lifecycle,
    solaris_notify_interruption_changed, solaris_notify_memory_pressure_changed,
    solaris_notify_permission_result, solaris_notify_power_mode_changed,
    solaris_notify_thermal_state_changed, solaris_notify_wake, solaris_notify_wall_clock_changed,
};
#[cfg(any(test, target_os = "solaris"))]
pub use ffi::{
    destack_host_solaris_notify_application_lifecycle,
    destack_host_solaris_notify_interruption_changed,
    destack_host_solaris_notify_memory_pressure_changed,
    destack_host_solaris_notify_permission_result, destack_host_solaris_notify_power_mode_changed,
    destack_host_solaris_notify_thermal_state_changed, destack_host_solaris_notify_wake,
    destack_host_solaris_notify_wall_clock_changed,
};
