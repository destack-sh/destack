use std::ffi::CStr;
use std::os::unix::io::RawFd;

use crate::diagnostic::RuntimeResult;
use crate::platform::fs::StatFs;

use super::statfs::statfs_from_libc;
use super::{last_os_error, nanos_from_secs_and_nanos, statfs_u64};

/// Return nanosecond timestamps for stat fields on linux and android.
pub(super) fn stat_times(stat: libc::stat) -> (u64, u64, u64, u64) {
    let atime = stat.st_atim;
    let mtime = stat.st_mtim;
    let ctime = stat.st_ctim;

    (
        nanos_from_secs_and_nanos(atime.tv_sec as u64, atime.tv_nsec as u64),
        nanos_from_secs_and_nanos(mtime.tv_sec as u64, mtime.tv_nsec as u64),
        nanos_from_secs_and_nanos(ctime.tv_sec as u64, ctime.tv_nsec as u64),
        0,
    )
}

/// Fetch statfs data for a file descriptor.
pub(super) fn statfs_for_fd(fd: RawFd) -> RuntimeResult<StatFs> {
    let mut statfs: libc::statfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::fstatfs(fd, &mut statfs) };
    if rc != 0 {
        return Err(last_os_error("fstatfs", None));
    }

    let frsize = statfs_u64(statfs.f_frsize);
    let namelen = statfs_u64(statfs.f_namelen);
    Ok(statfs_from_libc(statfs, frsize, namelen))
}

/// Fetch statfs data for a path.
pub(super) fn statfs_for_path(path: &CStr) -> RuntimeResult<StatFs> {
    let mut statfs: libc::statfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statfs(path.as_ptr(), &mut statfs) };
    if rc != 0 {
        return Err(last_os_error("statfs", None));
    }

    let frsize = statfs_u64(statfs.f_frsize);
    let namelen = statfs_u64(statfs.f_namelen);
    Ok(statfs_from_libc(statfs, frsize, namelen))
}

/// Return the current errno pointer on unix.
pub(super) fn errno_location() -> *mut libc::c_int {
    unsafe { libc::__errno_location() }
}

/// Call fdatasync on linux and android.
pub(super) fn fdatasync(fd: RawFd) -> i32 {
    unsafe { libc::fdatasync(fd) }
}
