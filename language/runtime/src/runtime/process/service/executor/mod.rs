#[cfg(any(target_os = "macos", windows))]
pub(crate) mod host;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "ios"))]
pub(crate) mod inline;
pub(crate) mod periodic;
pub(crate) mod state;
#[cfg(windows)]
pub(crate) mod thread;
