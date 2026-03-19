mod ingress;
#[cfg(target_os = "linux")]
mod linux;
mod macos;
mod runtime;
mod scheduler;
mod storage;
mod submit;
#[cfg(windows)]
mod windows;
mod wrapper;

pub(crate) use ingress::{service_background_ingress, unregister_background_runtime};
#[cfg(test)]
pub(crate) use runtime::with_background_test_mode;
pub(crate) use submit::submit_background_request;
