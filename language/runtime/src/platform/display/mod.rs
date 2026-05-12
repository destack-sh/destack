#[path = "abi.generated.rs"]
pub(crate) mod abi_generated;

pub(crate) use abi_generated::*;

pub(crate) mod core;
#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
pub(crate) mod options;
mod state;
#[cfg(unix)]
mod unix;
mod unsupported;
#[cfg(windows)]
mod windows;

pub(crate) use state::*;
#[cfg(target_os = "macos")]
pub(crate) use unix::appkit;
#[cfg(target_os = "linux")]
pub(crate) use unix::wayland;
#[cfg(target_os = "linux")]
pub(crate) use unix::x11;
