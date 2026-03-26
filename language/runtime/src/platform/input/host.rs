#[cfg(all(unix, not(any(target_os = "android", target_os = "ios"))))]
#[path = "unix/mod.rs"]
mod unix;
#[cfg(all(unix, not(any(target_os = "android", target_os = "ios"))))]
pub(crate) use unix::*;

#[cfg(windows)]
#[path = "windows/mod.rs"]
mod windows;
#[cfg(windows)]
pub(crate) use windows::*;

#[cfg(any(target_os = "android", target_os = "ios"))]
#[path = "unsupported.rs"]
mod mobile;
#[cfg(any(target_os = "android", target_os = "ios"))]
pub(crate) use mobile::*;

#[cfg(not(any(unix, windows, target_os = "android", target_os = "ios")))]
#[path = "unsupported.rs"]
mod unsupported;
#[cfg(not(any(unix, windows, target_os = "android", target_os = "ios")))]
pub(crate) use unsupported::*;
