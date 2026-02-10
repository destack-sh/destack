use std::ffi::CStr;
use std::os::unix::io::RawFd;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::fs::StatFs;

use super::statfs::statfs_from_libc;
use super::{nanos_from_secs_and_nanos, statfs_u64};

/// Return nanosecond timestamps for stat fields on linux and android.
pub(super) fn stat_times(stat: libc::stat) -> (u64, u64, u64, u64) {
    (
        nanos_from_secs_and_nanos(stat.st_atime as u64, stat.st_atime_nsec as u64),
        nanos_from_secs_and_nanos(stat.st_mtime as u64, stat.st_mtime_nsec as u64),
        nanos_from_secs_and_nanos(stat.st_ctime as u64, stat.st_ctime_nsec as u64),
        0,
    )
}

/// Fetch statfs data for a file descriptor.
pub(super) fn statfs_for_fd(fd: RawFd) -> RuntimeResult<StatFs> {
    let mut statfs: libc::statfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::fstatfs(fd, &mut statfs) };
    if rc != 0 {
        return Err(core_platform::io_error("fstatfs", None));
    }

    let frsize = statfs_u64(statfs.f_frsize);
    let namelen = statfs_u64(statfs.f_namelen);
    let mut statvfs: libc::statvfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::fstatvfs(fd, &mut statvfs) };
    if rc != 0 {
        return Err(core_platform::io_error("fstatvfs", None));
    }
    let flags = statfs_u64(statvfs.f_flag);
    Ok(statfs_from_libc(statfs, frsize, namelen, flags))
}

/// Fetch statfs data for a path.
pub(super) fn statfs_for_path(path: &CStr) -> RuntimeResult<StatFs> {
    let mut statfs: libc::statfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statfs(path.as_ptr(), &mut statfs) };
    if rc != 0 {
        return Err(core_platform::io_error("statfs", None));
    }

    let frsize = statfs_u64(statfs.f_frsize);
    let namelen = statfs_u64(statfs.f_namelen);
    let mut statvfs: libc::statvfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statvfs(path.as_ptr(), &mut statvfs) };
    if rc != 0 {
        return Err(core_platform::io_error("statvfs", None));
    }
    let flags = statfs_u64(statvfs.f_flag);
    Ok(statfs_from_libc(statfs, frsize, namelen, flags))
}

/// Call fdatasync on linux and android.
pub(super) fn fdatasync(fd: RawFd) -> i32 {
    unsafe { libc::fdatasync(fd) }
}
