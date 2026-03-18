#[cfg(any(target_os = "linux", target_os = "macos", windows))]
mod core;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod unsupported;
#[cfg(windows)]
mod windows;

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) use unix::*;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
pub(crate) use unsupported::*;
#[cfg(windows)]
pub(crate) use windows::*;

#[cfg(test)]
mod tests;
