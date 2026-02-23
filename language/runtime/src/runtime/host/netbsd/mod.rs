#[cfg(target_os = "netbsd")]
mod adapter;
#[cfg(any(test, target_os = "netbsd"))]
mod callback;
#[cfg(any(test, target_os = "netbsd"))]
mod ffi;

#[cfg(target_os = "netbsd")]
pub(super) use adapter::NetBsdHostAdapter;
#[cfg(any(test, target_os = "netbsd"))]
pub use callback::{
    NetBsdApplicationLifecycle, netbsd_notify_application_lifecycle,
    netbsd_notify_interruption_changed, netbsd_notify_memory_pressure_changed,
    netbsd_notify_permission_result, netbsd_notify_power_mode_changed,
    netbsd_notify_thermal_state_changed, netbsd_notify_wake, netbsd_notify_wall_clock_changed,
    netbsd_notify_window_available, netbsd_notify_window_focus_changed,
    netbsd_notify_window_resized, netbsd_notify_window_terminated,
};
#[cfg(any(test, target_os = "netbsd"))]
pub use ffi::{
    destack_runtime_host_netbsd_notify_application_lifecycle,
    destack_runtime_host_netbsd_notify_interruption_changed,
    destack_runtime_host_netbsd_notify_memory_pressure_changed,
    destack_runtime_host_netbsd_notify_permission_result,
    destack_runtime_host_netbsd_notify_power_mode_changed,
    destack_runtime_host_netbsd_notify_thermal_state_changed,
    destack_runtime_host_netbsd_notify_wake, destack_runtime_host_netbsd_notify_wall_clock_changed,
    destack_runtime_host_netbsd_notify_window_available,
    destack_runtime_host_netbsd_notify_window_focus_changed,
    destack_runtime_host_netbsd_notify_window_resized,
    destack_runtime_host_netbsd_notify_window_terminated,
};
