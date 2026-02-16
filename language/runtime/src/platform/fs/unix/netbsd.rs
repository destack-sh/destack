use std::ffi::CStr;
use std::os::unix::io::RawFd;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::fs::StatFs;

use super::{nanos_from_secs_and_nanos, statfs_u64};

/// Return nanosecond timestamps for stat fields on netbsd.
pub(crate) fn stat_times(stat: libc::stat) -> (u64, u64, u64, u64) {
    (
        nanos_from_secs_and_nanos(stat.st_atime as u64, stat.st_atimensec as u64),
        nanos_from_secs_and_nanos(stat.st_mtime as u64, stat.st_mtimensec as u64),
        nanos_from_secs_and_nanos(stat.st_ctime as u64, stat.st_ctimensec as u64),
        nanos_from_secs_and_nanos(stat.st_birthtime as u64, stat.st_birthtimensec as u64),
    )
}

/// Convert libc statvfs into the ABI StatFs shape.
fn statvfs_from_libc(stat: libc::statvfs) -> StatFs {
    StatFs {
        bsize: statfs_u64(stat.f_bsize),
        frsize: statfs_u64(stat.f_frsize),
        blocks: statfs_u64(stat.f_blocks),
        bfree: statfs_u64(stat.f_bfree),
        bavail: statfs_u64(stat.f_bavail),
        files: statfs_u64(stat.f_files),
        ffree: statfs_u64(stat.f_ffree),
        fsid: fsid_to_u64(stat.f_fsidx),
        flags: statfs_u64(stat.f_flag),
        namelen: statfs_u64(stat.f_namemax),
    }
}

/// Convert a libc fsid_t into a stable u64.
fn fsid_to_u64(fsid: libc::fsid_t) -> u64 {
    let raw: [libc::c_int; 2] = unsafe { std::mem::transmute(fsid) };
    (raw[0] as u32 as u64) | ((raw[1] as u32 as u64) << 32)
}

/// Fetch statfs data for a file descriptor.
pub(crate) fn statfs_for_fd(fd: RawFd) -> RuntimeResult<StatFs> {
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::fstatvfs(fd, &mut stat) };
    if rc != 0 {
        return Err(core_platform::io_error("fstatvfs", None));
    }

    Ok(statvfs_from_libc(stat))
}

/// Fetch statfs data for a path.
pub(crate) fn statfs_for_path(path: &CStr) -> RuntimeResult<StatFs> {
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statvfs(path.as_ptr(), &mut stat) };
    if rc != 0 {
        return Err(core_platform::io_error("statvfs", None));
    }

    Ok(statvfs_from_libc(stat))
}

/// Call fdatasync on netbsd.
pub(crate) fn fdatasync(fd: RawFd) -> i32 {
    unsafe { libc::fsync(fd) }
}
