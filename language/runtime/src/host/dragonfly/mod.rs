#[cfg(target_os = "dragonfly")]
mod adapter;
#[cfg(any(test, target_os = "dragonfly"))]
mod callback;
#[cfg(any(test, target_os = "dragonfly"))]
mod ffi;
#[cfg(test)]
mod tests;

#[cfg(target_os = "dragonfly")]
pub(super) use adapter::DragonflyHost;
#[cfg(any(test, target_os = "dragonfly"))]
pub use callback::{
    DragonflyApplicationLifecycle, dragonfly_notify_application_lifecycle,
    dragonfly_notify_interruption_changed, dragonfly_notify_memory_pressure_changed,
    dragonfly_notify_permission_result, dragonfly_notify_power_mode_changed,
    dragonfly_notify_thermal_state_changed, dragonfly_notify_wake,
    dragonfly_notify_wall_clock_changed, dragonfly_notify_window_available,
    dragonfly_notify_window_focus_changed, dragonfly_notify_window_resized,
    dragonfly_notify_window_terminated,
};
#[cfg(any(test, target_os = "dragonfly"))]
pub use ffi::{
    destack_host_dragonfly_notify_application_lifecycle,
    destack_host_dragonfly_notify_interruption_changed,
    destack_host_dragonfly_notify_memory_pressure_changed,
    destack_host_dragonfly_notify_permission_result,
    destack_host_dragonfly_notify_power_mode_changed,
    destack_host_dragonfly_notify_thermal_state_changed, destack_host_dragonfly_notify_wake,
    destack_host_dragonfly_notify_wall_clock_changed,
    destack_host_dragonfly_notify_window_available,
    destack_host_dragonfly_notify_window_focus_changed,
    destack_host_dragonfly_notify_window_resized, destack_host_dragonfly_notify_window_terminated,
};
