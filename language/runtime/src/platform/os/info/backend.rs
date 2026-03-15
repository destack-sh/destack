#[cfg(unix)]
use super::unix;
#[cfg(not(any(unix, windows)))]
use super::unsupported;
#[cfg(windows)]
use super::windows;

#[cfg(unix)]
pub(super) use unix::{
    read_boot_time_unix_ns, read_load_average, read_system_snapshot, read_uptime_ns,
};
#[cfg(not(any(unix, windows)))]
pub(super) use unsupported::{
    read_boot_time_unix_ns, read_load_average, read_system_snapshot, read_uptime_ns,
};
#[cfg(windows)]
pub(super) use windows::{
    read_boot_time_unix_ns, read_load_average, read_system_snapshot, read_uptime_ns,
};
