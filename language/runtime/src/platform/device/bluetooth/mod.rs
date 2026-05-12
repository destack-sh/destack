#[cfg(target_os = "android")]
mod android;
mod core;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod unsupported;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "android")]
pub(crate) use android::*;
#[cfg(target_os = "linux")]
pub(crate) use linux::*;
#[cfg(target_os = "macos")]
pub(crate) use macos::*;
#[cfg(not(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    windows
)))]
pub(crate) use unsupported::*;
#[cfg(windows)]
pub(crate) use windows::*;
