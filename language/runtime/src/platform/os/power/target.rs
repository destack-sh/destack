#[cfg(unix)]
use super::unix;
#[cfg(not(any(unix, windows)))]
use super::unsupported;
#[cfg(windows)]
use super::windows;

#[cfg(unix)]
pub(super) use unix::{read_power_state, request_suspend};
#[cfg(not(any(unix, windows)))]
pub(super) use unsupported::{read_power_state, request_suspend};
#[cfg(windows)]
pub(super) use windows::{read_power_state, request_suspend};
