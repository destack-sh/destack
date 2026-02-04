use std::ffi::CStr;
use std::os::unix::io::RawFd;

use crate::diagnostic::RuntimeResult;
use crate::platform::fs::StatFs;

use super::statfs::statfs_from_libc;
use super::{last_os_error, nanos_from_secs_and_nanos, statfs_u64};

/// Return nanosecond timestamps for stat fields on apple targets.
pub(super) fn stat_times(stat: libc::stat) -> (u64, u64, u64, u64) {
    (
        nanos_from_secs_and_nanos(stat.st_atime as u64, stat.st_atime_nsec as u64),
        nanos_from_secs_and_nanos(stat.st_mtime as u64, stat.st_mtime_nsec as u64),
        nanos_from_secs_and_nanos(stat.st_ctime as u64, stat.st_ctime_nsec as u64),
        nanos_from_secs_and_nanos(stat.st_birthtime as u64, stat.st_birthtime_nsec as u64),
    )
}

/// Fetch statfs data for a file descriptor.
pub(super) fn statfs_for_fd(fd: RawFd) -> RuntimeResult<StatFs> {
    let mut statfs: libc::statfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::fstatfs(fd, &mut statfs) };
    if rc != 0 {
        return Err(last_os_error("fstatfs", None));
    }

    let frsize = statfs_u64(statfs.f_bsize);
    Ok(statfs_from_libc(statfs, frsize, 0))
}

/// Fetch statfs data for a path.
pub(super) fn statfs_for_path(path: &CStr) -> RuntimeResult<StatFs> {
    let mut statfs: libc::statfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statfs(path.as_ptr(), &mut statfs) };
    if rc != 0 {
        return Err(last_os_error("statfs", None));
    }

    let frsize = statfs_u64(statfs.f_bsize);
    Ok(statfs_from_libc(statfs, frsize, 0))
}

/// Return the current errno pointer on apple targets.
pub(super) fn errno_location() -> *mut libc::c_int {
    unsafe { libc::__error() }
}

/// Call fdatasync on apple targets.
pub(super) fn fdatasync(fd: RawFd) -> i32 {
    unsafe { libc::fsync(fd) }
}
