mod backend;
#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "macos")))]
mod unsupported;

pub(crate) use backend::*;
#[cfg(all(test, target_os = "linux"))]
pub(crate) use linux::set_test_suspend_hook;
#[cfg(all(test, target_os = "macos"))]
pub(crate) use macos::set_test_suspend_hook;
