mod core;
#[cfg(all(test, any(target_os = "macos", windows)))]
mod tests;
#[cfg(unix)]
pub(crate) mod unix;
#[cfg(not(any(unix, windows)))]
mod unsupported;
#[cfg(windows)]
pub(crate) mod windows;

pub(crate) use core::*;
