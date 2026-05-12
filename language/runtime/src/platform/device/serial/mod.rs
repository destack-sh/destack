#[cfg(any(target_os = "linux", target_os = "macos", windows))]
mod core;
#[cfg(test)]
mod emulator;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod unsupported;
#[cfg(windows)]
mod windows;

#[cfg(test)]
pub(crate) use emulator::*;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) use unix::*;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
pub(crate) use unsupported::*;
#[cfg(windows)]
pub(crate) use windows::*;
