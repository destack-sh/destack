#[cfg(target_os = "android")]
mod adapter;
#[cfg(any(test, target_os = "android"))]
mod callback;
#[cfg(any(test, target_os = "android"))]
mod ffi;

#[cfg(target_os = "android")]
pub(super) use adapter::AndroidHostAdapter;
#[cfg(any(test, target_os = "android"))]
pub use callback::{
    AndroidActivityLifecycle, android_notify_activity_lifecycle,
    android_notify_interruption_changed, android_notify_memory_pressure_changed,
    android_notify_permission_request_in_flight, android_notify_permission_result,
    android_notify_power_mode_changed, android_notify_thermal_state_changed, android_notify_wake,
    android_notify_wall_clock_changed, android_notify_window_available,
    android_notify_window_focus_changed, android_notify_window_resized,
    android_notify_window_terminated,
};
#[cfg(any(test, target_os = "android"))]
pub use ffi::{
    destack_runtime_host_android_notify_activity_lifecycle,
    destack_runtime_host_android_notify_interruption_changed,
    destack_runtime_host_android_notify_memory_pressure_changed,
    destack_runtime_host_android_notify_permission_request_in_flight,
    destack_runtime_host_android_notify_permission_result,
    destack_runtime_host_android_notify_power_mode_changed,
    destack_runtime_host_android_notify_thermal_state_changed,
    destack_runtime_host_android_notify_wake,
    destack_runtime_host_android_notify_wall_clock_changed,
    destack_runtime_host_android_notify_window_available,
    destack_runtime_host_android_notify_window_focus_changed,
    destack_runtime_host_android_notify_window_resized,
    destack_runtime_host_android_notify_window_terminated,
};
