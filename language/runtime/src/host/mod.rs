#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

pub(crate) mod abi;
#[cfg(any(test, target_os = "android"))]
pub(crate) mod android;
#[cfg(any(target_os = "ios", target_os = "macos"))]
pub(crate) mod apple;
pub(crate) mod common;
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
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "netbsd")]
mod netbsd;
#[cfg(target_os = "openbsd")]
mod openbsd;
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
mod windows;

pub use core::{
    HOST_STATUS_BUFFER_TOO_SMALL, HOST_STATUS_FAILED, HOST_STATUS_INVALID_ARGUMENT,
    HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
    HOST_STATUS_PERMISSION_DENIED, HostEvent, HostEventKind, HostIntentEvent, HostIntentPayload,
    HostInterruptionEvent, HostLifecycleEvent, HostLifecycleState, HostMemoryPressureEvent,
    HostMemoryPressureLevel, HostPermissionEvent, HostPollOutcome, HostPowerMode,
    HostPowerModeEvent, HostSession, HostThermalEvent, HostThermalState, HostWallClockEvent,
};
pub(crate) use core::{HostAdapter, HostRequest};
pub use destack_workspace::Platform;
#[cfg(windows)]
pub(crate) use windows::process_ingress_loop;

#[cfg(any(test, target_os = "android"))]
pub use android::*;

#[cfg(all(test, target_os = "macos"))]
pub(crate) use macos::set_test_pick_hook as set_macos_document_test_pick_hook;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(any(test, target_os = "ios"))]
pub use ios::*;

#[cfg(all(test, windows))]
pub(crate) use windows::set_test_pick_hook as set_windows_document_test_pick_hook;
#[cfg(any(test, windows))]
pub use windows::*;

#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "freebsd")]
pub use freebsd::*;

#[cfg(target_os = "dragonfly")]
pub use dragonfly::*;

#[cfg(target_os = "netbsd")]
pub use netbsd::*;

#[cfg(target_os = "openbsd")]
pub use openbsd::*;

#[cfg(target_os = "illumos")]
pub use illumos::*;

#[cfg(target_os = "solaris")]
pub use solaris::*;

#[cfg(target_os = "haiku")]
pub use haiku::*;
