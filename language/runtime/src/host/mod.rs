#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

#[cfg(not(feature = "generator"))]
pub(crate) mod abi;
#[cfg(feature = "generator")]
pub mod abi;
#[cfg(any(test, target_os = "android"))]
#[cfg(not(feature = "generator"))]
pub(crate) mod android;
#[cfg(any(target_os = "ios", target_os = "macos"))]
#[cfg(not(feature = "generator"))]
pub(crate) mod apple;
#[cfg(not(feature = "generator"))]
mod bootstrap;
#[cfg(not(feature = "generator"))]
pub(crate) mod core;
#[cfg(target_os = "dragonfly")]
#[cfg(not(feature = "generator"))]
mod dragonfly;
#[cfg(target_os = "freebsd")]
#[cfg(not(feature = "generator"))]
mod freebsd;
#[cfg(target_os = "haiku")]
#[cfg(not(feature = "generator"))]
mod haiku;
#[cfg(target_os = "illumos")]
#[cfg(not(feature = "generator"))]
mod illumos;
#[cfg(any(test, target_os = "ios"))]
#[cfg(not(feature = "generator"))]
mod ios;
#[cfg(target_os = "linux")]
#[cfg(not(feature = "generator"))]
pub(crate) mod linux;
#[cfg(target_os = "macos")]
#[cfg(not(feature = "generator"))]
pub(crate) mod macos;
#[cfg(target_os = "netbsd")]
#[cfg(not(feature = "generator"))]
mod netbsd;
#[cfg(target_os = "openbsd")]
#[cfg(not(feature = "generator"))]
mod openbsd;
#[cfg(not(feature = "generator"))]
pub(crate) mod operation;
#[cfg(not(feature = "generator"))]
pub(crate) mod policy;
#[cfg(target_os = "solaris")]
#[cfg(not(feature = "generator"))]
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
#[cfg(not(feature = "generator"))]
pub(crate) mod unix;
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
#[cfg(not(feature = "generator"))]
mod unsupported;
#[cfg(any(test, windows))]
#[cfg(not(feature = "generator"))]
pub(crate) mod windows;

#[cfg(not(feature = "generator"))]
pub(crate) use core::HostAdapter;
#[cfg(not(feature = "generator"))]
pub use core::{
    HOST_STATUS_BUFFER_TOO_SMALL, HOST_STATUS_FAILED, HOST_STATUS_INVALID_ARGUMENT,
    HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
    HOST_STATUS_PERMISSION_DENIED, HostBackgroundEvent, HostDocumentEvent, HostEvent,
    HostEventKind, HostIntentEvent, HostIntentPayload, HostInterruptionEvent, HostLifecycleEvent,
    HostLifecycleSourceKind, HostLifecycleState, HostLocationEvent, HostMemoryPressureEvent,
    HostMemoryPressureLevel, HostNotificationEvent, HostPermissionEvent, HostPollOutcome,
    HostPowerMode, HostPowerModeEvent, HostRequestId, HostSession, HostThermalEvent,
    HostThermalState, HostWallClockEvent,
};
#[cfg(not(feature = "generator"))]
pub use destack_artifact::Platform;
#[cfg(windows)]
#[cfg(not(feature = "generator"))]
pub(crate) use windows::ingress::process_ingress_loop;
