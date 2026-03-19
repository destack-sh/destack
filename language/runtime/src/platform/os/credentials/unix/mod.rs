#[cfg(target_os = "android")]
mod android;
#[cfg(any(target_os = "macos", target_os = "ios"))]
mod apple;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(not(any(
    target_os = "android",
    target_os = "ios",
    target_os = "linux",
    target_os = "macos",
)))]
mod posix;
mod target;

pub(crate) use target::*;
