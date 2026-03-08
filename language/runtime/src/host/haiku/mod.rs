#[cfg(target_os = "haiku")]
mod backend;
#[cfg(any(test, target_os = "haiku"))]
mod callback;
#[cfg(any(test, target_os = "haiku"))]
mod ffi;
#[cfg(test)]
mod tests;

#[cfg(target_os = "haiku")]
pub(crate) use backend::HaikuHost;
#[cfg(any(test, target_os = "haiku"))]
pub use callback::{
    HaikuApplicationLifecycle, haiku_notify_application_lifecycle,
    haiku_notify_interruption_changed, haiku_notify_memory_pressure_changed,
    haiku_notify_permission_result, haiku_notify_power_mode_changed,
    haiku_notify_thermal_state_changed, haiku_notify_wake, haiku_notify_wall_clock_changed,
};
#[cfg(any(test, target_os = "haiku"))]
pub use ffi::{
    destack_host_haiku_notify_application_lifecycle,
    destack_host_haiku_notify_interruption_changed,
    destack_host_haiku_notify_memory_pressure_changed, destack_host_haiku_notify_permission_result,
    destack_host_haiku_notify_power_mode_changed, destack_host_haiku_notify_thermal_state_changed,
    destack_host_haiku_notify_wake, destack_host_haiku_notify_wall_clock_changed,
};
