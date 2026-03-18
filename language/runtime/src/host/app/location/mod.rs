#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod unsupported;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
pub(crate) use linux::unregister_location_runtime;
#[cfg(target_os = "macos")]
pub(crate) use macos::unregister_location_runtime;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
pub(crate) use unsupported::unregister_location_runtime;
#[cfg(windows)]
pub(crate) use windows::unregister_location_runtime;
