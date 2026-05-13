#[cfg(target_os = "linux")]
mod linux;
#[cfg(all(unix, not(target_os = "linux")))]
mod posix;
#[cfg(unix)]
mod unix;
#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
pub(crate) use linux::*;
#[cfg(all(unix, not(target_os = "linux")))]
pub(crate) use posix::*;
#[cfg(target_arch = "wasm32")]
pub(crate) use wasm::*;
#[cfg(windows)]
pub(crate) use windows::*;
