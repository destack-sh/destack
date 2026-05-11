use super::core::*;
use super::os;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::{core as core_fs, *};
use crate::platform::resource::*;
use crate::platform::{core as core_platform, *};
use crate::runtime::BindingCallContext;

/// Stat a file without following symlinks.
pub(crate) unsafe fn destack_fs_lstat_bytes(
    _binding: &BindingCallContext,
    out: *mut Stat,
    path: PathBytes,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read the stat data without following symlinks
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::lstat(c_path.as_ptr(), &mut stat) };
    if rc != 0 {
        return Err(core_platform::io_error("lstat", None));
    }

    unsafe {
        *out = stat_from_libc(stat);
    }

    Ok(())
}

/// Stat a file without following symlinks.
pub(crate) unsafe fn destack_fs_lstat_utf16(
    binding: &BindingCallContext,
    out: *mut Stat,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read stat data by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| unsafe {
        destack_fs_lstat_bytes(binding, out, path)
    })
}

/// Stat a file.
pub(crate) unsafe fn destack_fs_stat_bytes(
    _binding: &BindingCallContext,
    out: *mut Stat,
    path: PathBytes,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read the stat data on unix platforms
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::stat(c_path.as_ptr(), &mut stat) };
    if rc != 0 {
        return Err(core_platform::io_error("stat", None));
    }

    unsafe {
        *out = stat_from_libc(stat);
    }

    Ok(())
}

/// Stat a file.
pub(crate) unsafe fn destack_fs_stat_utf16(
    binding: &BindingCallContext,
    out: *mut Stat,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read stat data by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| unsafe {
        destack_fs_stat_bytes(binding, out, path)
    })
}

/// Stat a filesystem.
pub(crate) unsafe fn destack_fs_statfs_bytes(
    _binding: &BindingCallContext,
    out: *mut StatFs,
    path: PathBytes,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read the statfs data on unix platforms
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let statfs = os::statfs_for_path(&c_path)?;
    unsafe {
        *out = statfs;
    }
    Ok(())
}

/// Stat a filesystem.
pub(crate) unsafe fn destack_fs_statfs_utf16(
    binding: &BindingCallContext,
    out: *mut StatFs,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read statfs data by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| unsafe {
        destack_fs_statfs_bytes(binding, out, path)
    })
}

/// Stat a file relative to a directory handle.
pub(crate) unsafe fn destack_fs_statat_bytes(
    binding: &BindingCallContext,
    out: *mut Stat,
    dir: DirectoryHandle,
    path: PathBytes,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // stat the path on unix platforms
    let resource = directory_resource(binding, dir)?;
    let path = resolve_path_bytes_cstring(path, "path")?;
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            resource.fd,
            path.as_ptr(),
            stat.as_mut_ptr(),
            flags.0 as libc::c_int,
        )
    };
    if result != 0 {
        return Err(core_platform::io_error("statat", None));
    }
    let stat = unsafe { stat.assume_init() };
    unsafe {
        *out = stat_from_libc(stat);
    }
    Ok(())
}

/// Stat a file relative to a directory handle.
pub(crate) unsafe fn destack_fs_statat_utf16(
    binding: &BindingCallContext,
    out: *mut Stat,
    dir: DirectoryHandle,
    path: PathUtf16,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read stat data by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| unsafe {
        destack_fs_statat_bytes(binding, out, dir, path, flags)
    })
}

/// Stat a file.
pub(crate) unsafe fn destack_fs_stat(
    binding: &BindingCallContext,
    out: *mut Stat,
    path: OsPath,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_stat_bytes(binding, out, path) },
        |path| unsafe { destack_fs_stat_utf16(binding, out, path) },
    )
}

/// Stat a file relative to a directory handle.
pub(crate) unsafe fn destack_fs_statat(
    binding: &BindingCallContext,
    out: *mut Stat,
    dir: DirectoryHandle,
    path: OsPath,
    flags: AtFlags,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_statat_bytes(binding, out, dir, path, flags) },
        |path| unsafe { destack_fs_statat_utf16(binding, out, dir, path, flags) },
    )
}

/// Stat a file without following symlinks.
pub(crate) unsafe fn destack_fs_lstat(
    binding: &BindingCallContext,
    out: *mut Stat,
    path: OsPath,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_lstat_bytes(binding, out, path) },
        |path| unsafe { destack_fs_lstat_utf16(binding, out, path) },
    )
}

/// Stat a filesystem.
pub(crate) unsafe fn destack_fs_statfs(
    binding: &BindingCallContext,
    out: *mut StatFs,
    path: OsPath,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_statfs_bytes(binding, out, path) },
        |path| unsafe { destack_fs_statfs_utf16(binding, out, path) },
    )
}

/// Stat a path with statx semantics.
pub(crate) unsafe fn destack_fs_statx(
    binding: &BindingCallContext,
    out: *mut Statx,
    dir: DirectoryHandle,
    path: OsPath,
    flags: StatxFlags,
    _mask: StatxMask,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    core_fs::with_path_ref(
        path,
        "path",
        |path| {
            let directory_fd = directory_descriptor(binding, dir)?;
            let path = path_bytes_to_cstring(path, "path")?;
            let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
            let result = unsafe {
                libc::fstatat(
                    directory_fd,
                    path.as_ptr(),
                    stat.as_mut_ptr(),
                    flags.0 as libc::c_int,
                )
            };
            if result != 0 {
                return Err(core_platform::io_error("fstatat", None));
            }
            let stat = unsafe { stat.assume_init() };

            let atime_ns = nanos_from_secs_and_nanos(stat.st_atime, stat.st_atime_nsec);
            let btime_ns = 0;
            let ctime_ns = nanos_from_secs_and_nanos(stat.st_ctime, stat.st_ctime_nsec);
            let mtime_ns = nanos_from_secs_and_nanos(stat.st_mtime, stat.st_mtime_nsec);

            unsafe {
                *out = Statx {
                    mask: STATX_BASIC_STATS,
                    blksize: stat.st_blksize as u32,
                    mount_id: 0,
                    dev_major: ((stat.st_dev >> 8) & 0xfff) as u32,
                    dev_minor: ((stat.st_dev & 0xff) | ((stat.st_dev >> 12) & 0xfff00)) as u32,
                    ino: stat.st_ino,
                    mode: FileMode(file_mode_u32(stat.st_mode)),
                    nlink: file_nlink_u32(stat.st_nlink),
                    uid: stat.st_uid,
                    gid: stat.st_gid,
                    rdev_major: ((stat.st_rdev >> 8) & 0xfff) as u32,
                    rdev_minor: ((stat.st_rdev & 0xff) | ((stat.st_rdev >> 12) & 0xfff00)) as u32,
                    size: FileSize(stat_u64(stat.st_size)),
                    blocks: stat_u64(stat.st_blocks),
                    atime_ns,
                    btime_ns,
                    ctime_ns,
                    mtime_ns,
                };
            }
            Ok(())
        },
        |path| unsafe {
            core_fs::with_utf16_as_bytes(path, "path", |path| {
                destack_fs_statx(
                    binding,
                    out,
                    dir,
                    core_fs::path_ref_from_bytes(path),
                    flags,
                    _mask,
                )
            })
        },
    )
}
