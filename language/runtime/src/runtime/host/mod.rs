#[cfg(any(test, target_os = "android"))]
mod android;
mod core;
#[cfg(target_os = "dragonfly")]
mod dragonfly;
#[cfg(target_os = "freebsd")]
mod freebsd;
#[cfg(target_os = "haiku")]
mod haiku;
#[cfg(target_os = "illumos")]
mod illumos;
#[cfg(any(test, target_os = "ios"))]
mod ios;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "netbsd")]
mod netbsd;
#[cfg(target_os = "openbsd")]
mod openbsd;
#[cfg(target_os = "solaris")]
mod solaris;
#[cfg(not(any(
    target_os = "android",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "haiku",
    target_os = "illumos",
    target_os = "ios",
    target_os = "linux",
    target_os = "macos",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "solaris",
    windows,
)))]
mod unsupported;
#[cfg(any(test, windows))]
mod windows;

pub use core::{
    HostAdapter, HostEvent, HostEventKind, HostInterruptionEvent, HostInterruptionService,
    HostLifecycleEvent, HostLifecycleService, HostLifecycleState, HostPermissionEvent,
    HostPermissionService, HostPlatform, HostRuntime, HostServices, HostWindowEvent,
    HostWindowService, default_host_adapter,
};

#[cfg(any(test, target_os = "android"))]
pub use android::{
    AndroidActivityLifecycle, android_notify_activity_lifecycle,
    android_notify_interruption_changed, android_notify_permission_request_in_flight,
    android_notify_permission_result, android_notify_wake, android_notify_window_available,
    android_notify_window_resized, android_notify_window_terminated,
    destack_runtime_host_android_notify_activity_lifecycle,
    destack_runtime_host_android_notify_interruption_changed,
    destack_runtime_host_android_notify_permission_request_in_flight,
    destack_runtime_host_android_notify_permission_result,
    destack_runtime_host_android_notify_wake, destack_runtime_host_android_notify_window_available,
    destack_runtime_host_android_notify_window_resized,
    destack_runtime_host_android_notify_window_terminated,
};

#[cfg(target_os = "macos")]
pub use macos::{
    MacosApplicationLifecycle, destack_runtime_host_macos_notify_application_lifecycle,
    destack_runtime_host_macos_notify_interruption_changed,
    destack_runtime_host_macos_notify_permission_result, destack_runtime_host_macos_notify_wake,
    destack_runtime_host_macos_notify_window_available,
    destack_runtime_host_macos_notify_window_resized,
    destack_runtime_host_macos_notify_window_terminated, macos_notify_application_lifecycle,
    macos_notify_interruption_changed, macos_notify_permission_result, macos_notify_wake,
    macos_notify_window_available, macos_notify_window_resized, macos_notify_window_terminated,
};

#[cfg(any(test, target_os = "ios"))]
pub use ios::{
    IosApplicationLifecycle, destack_runtime_host_ios_notify_application_lifecycle,
    destack_runtime_host_ios_notify_interruption_changed,
    destack_runtime_host_ios_notify_permission_result, destack_runtime_host_ios_notify_wake,
    destack_runtime_host_ios_notify_window_available,
    destack_runtime_host_ios_notify_window_resized,
    destack_runtime_host_ios_notify_window_terminated, ios_notify_application_lifecycle,
    ios_notify_interruption_changed, ios_notify_permission_result, ios_notify_wake,
    ios_notify_window_available, ios_notify_window_resized, ios_notify_window_terminated,
};

#[cfg(any(test, windows))]
pub use windows::{
    WindowsApplicationLifecycle, destack_runtime_host_windows_notify_application_lifecycle,
    destack_runtime_host_windows_notify_interruption_changed,
    destack_runtime_host_windows_notify_permission_result,
    destack_runtime_host_windows_notify_wake, destack_runtime_host_windows_notify_window_available,
    destack_runtime_host_windows_notify_window_resized,
    destack_runtime_host_windows_notify_window_terminated, windows_notify_application_lifecycle,
    windows_notify_interruption_changed, windows_notify_permission_result, windows_notify_wake,
    windows_notify_window_available, windows_notify_window_resized,
    windows_notify_window_terminated,
};
