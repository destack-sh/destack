mod backend;
#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux;
#[cfg(all(unix, not(any(target_os = "linux", target_os = "android"))))]
mod posix;

pub(crate) use backend::*;
