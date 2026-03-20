#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

pub(crate) mod abi;
#[cfg(any(test, target_os = "android"))]
pub(crate) mod android;
pub(crate) mod app;
#[cfg(any(target_os = "ios", target_os = "macos"))]
pub(crate) mod apple;
mod bootstrap;
#[cfg(any(test, target_os = "android", target_os = "ios"))]
pub(crate) mod callback;
pub(crate) mod core;
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
pub(crate) mod linux;
#[cfg(target_os = "macos")]
pub(crate) mod macos;
#[cfg(target_os = "netbsd")]
mod netbsd;
#[cfg(target_os = "openbsd")]
mod openbsd;
pub(crate) mod operation;
pub(crate) mod policy;
#[cfg(target_os = "solaris")]
mod solaris;
#[cfg(any(
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "haiku",
    target_os = "illumos",
    target_os = "linux",
    target_os = "macos",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "solaris"
))]
pub mod unix;
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
pub(crate) mod windows;

pub(crate) use core::HostAdapter;
pub use core::{
    HOST_STATUS_BUFFER_TOO_SMALL, HOST_STATUS_FAILED, HOST_STATUS_INVALID_ARGUMENT,
    HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
    HOST_STATUS_PERMISSION_DENIED, HostBackgroundEvent, HostEvent, HostEventKind, HostIntentEvent,
    HostIntentPayload, HostInterruptionEvent, HostLifecycleEvent, HostLifecycleState,
    HostLocationEvent, HostMemoryPressureEvent, HostMemoryPressureLevel, HostNotificationEvent,
    HostPermissionEvent, HostPollOutcome, HostPowerMode, HostPowerModeEvent, HostSession,
    HostThermalEvent, HostThermalState, HostWallClockEvent,
};
pub use destack_workspace::Platform;
#[cfg(windows)]
pub(crate) use windows::process_ingress_loop;
