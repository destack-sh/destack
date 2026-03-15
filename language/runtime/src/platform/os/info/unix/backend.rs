#[cfg(target_vendor = "apple")]
use super::apple;
#[cfg(not(target_vendor = "apple"))]
use super::posix;

#[cfg(target_vendor = "apple")]
pub(crate) use apple::{
    read_boot_time_unix_ns, read_load_average, read_system_snapshot, read_uptime_ns,
};
#[cfg(not(target_vendor = "apple"))]
pub(crate) use posix::{
    read_boot_time_unix_ns, read_load_average, read_system_snapshot, read_uptime_ns,
};
