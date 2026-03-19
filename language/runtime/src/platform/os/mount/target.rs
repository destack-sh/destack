#[cfg(unix)]
use super::unix;
#[cfg(not(any(unix, windows)))]
use super::unsupported;
#[cfg(windows)]
use super::windows;

#[cfg(unix)]
pub(super) use unix::read_mount_entries;
#[cfg(not(any(unix, windows)))]
pub(super) use unsupported::read_mount_entries;
#[cfg(windows)]
pub(super) use windows::read_mount_entries;
