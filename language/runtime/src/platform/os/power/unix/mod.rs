#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod target;
#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "macos")))]
mod unsupported;

#[cfg(all(test, target_os = "linux"))]
pub(super) use linux::set_test_suspend_hook;
#[cfg(all(test, target_os = "macos"))]
pub(super) use macos::set_test_suspend_hook;
pub(crate) use target::*;
