pub(crate) mod affinity;
pub(crate) mod executor;
pub(crate) mod registry;
#[cfg(target_os = "macos")]
pub(crate) mod unix;
#[cfg(windows)]
pub(crate) mod windows;

pub(crate) use registry::{CachedServiceHandle, global_service};
