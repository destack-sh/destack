#[cfg(target_os = "freebsd")]
mod backend;
#[cfg(any(test, target_os = "freebsd"))]
mod callback;
#[cfg(any(test, target_os = "freebsd"))]
mod ffi;
#[cfg(test)]
mod tests;

#[cfg(target_os = "freebsd")]
pub(crate) use backend::FreeBsdHost;
#[cfg(any(test, target_os = "freebsd"))]
pub use callback::{
    FreeBsdApplicationLifecycle, freebsd_notify_application_lifecycle,
    freebsd_notify_interruption_changed, freebsd_notify_memory_pressure_changed,
    freebsd_notify_permission_result, freebsd_notify_power_mode_changed,
    freebsd_notify_thermal_state_changed, freebsd_notify_wake, freebsd_notify_wall_clock_changed,
};
#[cfg(any(test, target_os = "freebsd"))]
pub use ffi::{
    destack_host_freebsd_notify_application_lifecycle,
    destack_host_freebsd_notify_interruption_changed,
    destack_host_freebsd_notify_memory_pressure_changed,
    destack_host_freebsd_notify_permission_result, destack_host_freebsd_notify_power_mode_changed,
    destack_host_freebsd_notify_thermal_state_changed, destack_host_freebsd_notify_wake,
    destack_host_freebsd_notify_wall_clock_changed,
};
