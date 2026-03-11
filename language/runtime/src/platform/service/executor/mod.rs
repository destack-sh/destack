#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) mod caller;
#[cfg(windows)]
pub(crate) mod dedicated;
#[cfg(any(target_os = "macos", windows))]
pub(crate) mod host;
