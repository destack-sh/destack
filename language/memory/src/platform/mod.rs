#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux;
#[cfg(all(unix, not(any(target_os = "linux", target_os = "android"))))]
mod posix;
#[cfg(unix)]
mod unix;
#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(windows)]
mod windows;

#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) use linux::*;
#[cfg(all(unix, not(any(target_os = "linux", target_os = "android"))))]
pub(crate) use posix::*;
#[cfg(target_arch = "wasm32")]
pub(crate) use wasm::*;
#[cfg(windows)]
pub(crate) use windows::*;
