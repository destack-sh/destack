#[cfg(any(target_os = "linux", target_os = "android"))]
use super::linux;
#[cfg(all(unix, not(any(target_os = "linux", target_os = "android"))))]
use super::posix;

#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) use linux::read_mount_entries;
#[cfg(all(unix, not(any(target_os = "linux", target_os = "android"))))]
pub(crate) use posix::read_mount_entries;
