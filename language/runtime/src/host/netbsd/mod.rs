#[cfg(target_os = "netbsd")]
mod backend;
#[cfg(any(test, target_os = "netbsd"))]
mod callback;
#[cfg(any(test, target_os = "netbsd"))]
mod ffi;
#[cfg(test)]
mod tests;

#[cfg(target_os = "netbsd")]
pub(crate) use backend::NetBsdHost;
#[cfg(any(test, target_os = "netbsd"))]
pub use callback::{
    NetBsdApplicationLifecycle, netbsd_notify_application_lifecycle,
    netbsd_notify_interruption_changed, netbsd_notify_memory_pressure_changed,
    netbsd_notify_permission_result, netbsd_notify_power_mode_changed,
    netbsd_notify_thermal_state_changed, netbsd_notify_wake, netbsd_notify_wall_clock_changed,
};
#[cfg(any(test, target_os = "netbsd"))]
pub use ffi::{
    destack_host_netbsd_notify_application_lifecycle,
    destack_host_netbsd_notify_interruption_changed,
    destack_host_netbsd_notify_memory_pressure_changed,
    destack_host_netbsd_notify_permission_result, destack_host_netbsd_notify_power_mode_changed,
    destack_host_netbsd_notify_thermal_state_changed, destack_host_netbsd_notify_wake,
    destack_host_netbsd_notify_wall_clock_changed,
};
