mod core;
mod delivery;
mod ingress;
pub(crate) mod runtime;
#[cfg(target_os = "linux")]
pub(crate) mod storage;
pub(crate) mod time;
#[cfg(not(any(
    windows,
    target_os = "macos",
    all(
        unix,
        not(any(target_os = "android", target_os = "ios", target_os = "macos"))
    )
)))]
mod unsupported;
#[cfg(target_os = "linux")]
pub(crate) mod wrapper;

pub(crate) use core::*;
pub(crate) use ingress::{service_notification_ingress, unregister_notification_runtime};
#[cfg(test)]
pub(crate) use runtime::with_notification_test_mode;
