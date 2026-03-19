#[cfg(any(target_os = "linux", target_os = "android"))]
use super::linux;
#[cfg(target_os = "macos")]
use super::macos;
#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "macos")))]
use super::unsupported;

#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) use linux::{read_power_state, request_suspend};
#[cfg(target_os = "macos")]
pub(crate) use macos::{read_power_state, request_suspend};
#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "macos")))]
pub(crate) use unsupported::{read_power_state, request_suspend};
