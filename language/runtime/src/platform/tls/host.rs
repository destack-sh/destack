#[cfg(unix)]
#[path = "unix/mod.rs"]
mod unix;
#[cfg(unix)]
pub(crate) use unix::*;

#[cfg(windows)]
#[path = "windows/mod.rs"]
mod windows;
#[cfg(windows)]
pub(crate) use windows::*;

#[cfg(not(any(unix, windows)))]
pub(crate) use super::unsupported::*;
