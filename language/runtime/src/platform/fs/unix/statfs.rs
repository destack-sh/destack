use crate::platform::fs::StatFs;

use super::statfs_u64;

/// Convert a libc fsid_t into a stable u64.
pub(super) fn fsid_to_u64(fsid: libc::fsid_t) -> u64 {
    let raw: [libc::c_int; 2] = unsafe { std::mem::transmute(fsid) };
    (raw[0] as u32 as u64) | ((raw[1] as u32 as u64) << 32)
}

/// Convert libc statfs into the ABI StatFs shape with explicit fields.
pub(super) fn statfs_from_libc(statfs: libc::statfs, frsize: u64, namelen: u64) -> StatFs {
    StatFs {
        bsize: statfs_u64(statfs.f_bsize),
        frsize,
        blocks: statfs_u64(statfs.f_blocks),
        bfree: statfs_u64(statfs.f_bfree),
        bavail: statfs_u64(statfs.f_bavail),
        files: statfs_u64(statfs.f_files),
        ffree: statfs_u64(statfs.f_ffree),
        fsid: fsid_to_u64(statfs.f_fsid),
        flags: statfs_u64(statfs.f_flags),
        namelen,
    }
}
