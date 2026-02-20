#[cfg(unix)]
#[path = "unix/mod.rs"]
mod unix;
#[cfg(unix)]
#[allow(unused_imports)]
pub(crate) use unix::*;

#[cfg(windows)]
#[path = "windows/mod.rs"]
mod windows;
#[cfg(windows)]
#[allow(unused_imports)]
pub(crate) use windows::*;

#[cfg(not(any(unix, windows)))]
#[allow(unused_imports)]
pub(crate) use super::unsupported::*;
