mod backend;
mod delivery;
mod ingress;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod runtime;
#[cfg(target_os = "linux")]
mod storage;
mod submit;
mod time;
#[cfg(all(unix, not(target_os = "macos")))]
mod unix;
#[cfg(not(any(target_os = "macos", all(unix, not(target_os = "macos")), windows)))]
mod unsupported;
#[cfg(windows)]
mod windows;
#[cfg(target_os = "linux")]
mod wrapper;

pub(crate) use ingress::{service_notification_ingress, unregister_notification_runtime};
#[cfg(test)]
pub(crate) use runtime::with_notification_test_mode;
pub(crate) use submit::submit_notification_request;
