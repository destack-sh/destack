#[cfg(target_os = "illumos")]
mod backend;
#[cfg(any(test, target_os = "illumos"))]
mod callback;
#[cfg(any(test, target_os = "illumos"))]
mod ffi;
#[cfg(test)]
mod tests;

#[cfg(target_os = "illumos")]
pub(crate) use backend::IllumosHost;
#[cfg(any(test, target_os = "illumos"))]
pub use callback::{
    IllumosApplicationLifecycle, illumos_notify_application_lifecycle,
    illumos_notify_interruption_changed, illumos_notify_memory_pressure_changed,
    illumos_notify_permission_result, illumos_notify_power_mode_changed,
    illumos_notify_thermal_state_changed, illumos_notify_wake, illumos_notify_wall_clock_changed,
};
#[cfg(any(test, target_os = "illumos"))]
pub use ffi::{
    destack_host_illumos_notify_application_lifecycle,
    destack_host_illumos_notify_interruption_changed,
    destack_host_illumos_notify_memory_pressure_changed,
    destack_host_illumos_notify_permission_result, destack_host_illumos_notify_power_mode_changed,
    destack_host_illumos_notify_thermal_state_changed, destack_host_illumos_notify_wake,
    destack_host_illumos_notify_wall_clock_changed,
};
