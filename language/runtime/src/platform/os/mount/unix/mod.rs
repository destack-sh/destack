#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux;
#[cfg(all(unix, not(any(target_os = "linux", target_os = "android"))))]
mod posix;
mod target;

pub(crate) use target::*;
