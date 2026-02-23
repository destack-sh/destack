#[cfg(target_os = "ios")]
mod adapter;
#[cfg(any(test, target_os = "ios"))]
mod callback;
#[cfg(any(test, target_os = "ios"))]
mod ffi;

#[cfg(target_os = "ios")]
pub(super) use adapter::IosHostAdapter;
#[cfg(any(test, target_os = "ios"))]
pub use callback::{
    IosApplicationLifecycle, ios_notify_application_lifecycle, ios_notify_interruption_changed,
    ios_notify_memory_pressure_changed, ios_notify_permission_result,
    ios_notify_power_mode_changed, ios_notify_thermal_state_changed, ios_notify_wake,
    ios_notify_wall_clock_changed, ios_notify_window_available, ios_notify_window_focus_changed,
    ios_notify_window_resized, ios_notify_window_terminated,
};
#[cfg(any(test, target_os = "ios"))]
pub use ffi::{
    destack_runtime_host_ios_notify_application_lifecycle,
    destack_runtime_host_ios_notify_interruption_changed,
    destack_runtime_host_ios_notify_memory_pressure_changed,
    destack_runtime_host_ios_notify_permission_result,
    destack_runtime_host_ios_notify_power_mode_changed,
    destack_runtime_host_ios_notify_thermal_state_changed, destack_runtime_host_ios_notify_wake,
    destack_runtime_host_ios_notify_wall_clock_changed,
    destack_runtime_host_ios_notify_window_available,
    destack_runtime_host_ios_notify_window_focus_changed,
    destack_runtime_host_ios_notify_window_resized,
    destack_runtime_host_ios_notify_window_terminated,
};
