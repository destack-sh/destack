#[cfg(all(unix, not(target_os = "android")))]
use super::unix;
#[cfg(not(any(all(unix, not(target_os = "android")), windows)))]
use super::unsupported;
#[cfg(windows)]
use super::windows;

#[cfg(all(unix, not(target_os = "android")))]
pub(super) use unix::{read_power_state, request_suspend};
#[cfg(not(any(all(unix, not(target_os = "android")), windows)))]
pub(super) use unsupported::{read_power_state, request_suspend};
#[cfg(windows)]
pub(super) use windows::{read_power_state, request_suspend};
