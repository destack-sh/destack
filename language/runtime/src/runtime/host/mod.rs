#[cfg(target_os = "android")]
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
#[cfg(target_os = "ios")]
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
#[cfg(windows)]
mod windows;

pub use core::{
    HostAdapter, HostAssetService, HostDisplayService, HostEvent, HostHapticsService,
    HostInterruptionEvent, HostInterruptionService, HostJniService, HostLifecycleEvent,
    HostLifecycleService, HostLifecycleState, HostPermissionEvent, HostPermissionService,
    HostPlatform, HostPowerService, HostRuntime, HostServices, HostTextInputService,
    HostWindowEvent, HostWindowService, default_host_adapter,
};
