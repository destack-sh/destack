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
#[path = "unsupported.rs"]
mod unsupported;
#[cfg(not(any(unix, windows)))]
pub(crate) use unsupported::*;
