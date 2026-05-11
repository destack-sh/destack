use super::core::*;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::fs::{core as core_fs, *};
use crate::platform::resource::*;
use crate::runtime::BindingCallContext;

/// Update access and modification times without following symlinks.
pub(crate) unsafe fn destack_fs_lutimes_bytes(
    _binding: &BindingCallContext,
    path: PathBytes,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    // apply timestamps on unix platforms without following symlinks
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let times = [timespec_from_nanos(atimens), timespec_from_nanos(mtimens)];
    let rc = unsafe {
        libc::utimensat(
            libc::AT_FDCWD,
            c_path.as_ptr(),
            times.as_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if rc == 0 {
        return Ok(());
    }

    Err(core_platform::io_error("utimensat", None))
}

/// Update access and modification times without following symlinks.
pub(crate) unsafe fn destack_fs_lutimes_utf16(
    binding: &BindingCallContext,
    path: PathUtf16,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    // update timestamps by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| unsafe {
        destack_fs_lutimes_bytes(binding, path, atimens, mtimens)
    })
}

/// Update access and modification times relative to a directory handle.
pub(crate) unsafe fn destack_fs_utimensat_bytes(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    atimens: u64,
    mtimens: u64,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // update timestamps relative to the directory on unix platforms
    let resource = directory_resource(binding, dir)?;
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let times = [timespec_from_nanos(atimens), timespec_from_nanos(mtimens)];
    let rc = unsafe {
        libc::utimensat(
            resource.fd,
            c_path.as_ptr(),
            times.as_ptr(),
            flags.0 as libc::c_int,
        )
    };
    if rc == 0 {
        return Ok(());
    }

    Err(core_platform::io_error("utimensat", None))
}

/// Update access and modification times relative to a directory handle.
pub(crate) unsafe fn destack_fs_utimensat_utf16(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    atimens: u64,
    mtimens: u64,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // update timestamps by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| unsafe {
        destack_fs_utimensat_bytes(binding, dir, path, atimens, mtimens, flags)
    })
}

/// Update access and modification times.
pub(crate) unsafe fn destack_fs_utimes_bytes(
    _binding: &BindingCallContext,
    path: PathBytes,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    // apply timestamps on unix platforms
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let times = [timespec_from_nanos(atimens), timespec_from_nanos(mtimens)];
    let rc = unsafe { libc::utimensat(libc::AT_FDCWD, c_path.as_ptr(), times.as_ptr(), 0) };
    if rc == 0 {
        return Ok(());
    }

    Err(core_platform::io_error("utimensat", None))
}

/// Update access and modification times.
pub(crate) unsafe fn destack_fs_utimes_utf16(
    binding: &BindingCallContext,
    path: PathUtf16,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    // update timestamps by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| unsafe {
        destack_fs_utimes_bytes(binding, path, atimens, mtimens)
    })
}

/// Update access and modification times.
pub(crate) unsafe fn destack_fs_utimes(
    binding: &BindingCallContext,
    path: OsPath,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_utimes_bytes(binding, path, atime_ns, mtime_ns) },
        |path| unsafe { destack_fs_utimes_utf16(binding, path, atime_ns, mtime_ns) },
    )
}

/// Update access and modification times without following symlinks.
pub(crate) unsafe fn destack_fs_lutimes(
    binding: &BindingCallContext,
    path: OsPath,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_lutimes_bytes(binding, path, atime_ns, mtime_ns) },
        |path| unsafe { destack_fs_lutimes_utf16(binding, path, atime_ns, mtime_ns) },
    )
}

/// Update access and modification times relative to a directory handle.
pub(crate) unsafe fn destack_fs_utimensat(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: OsPath,
    atime_ns: u64,
    mtime_ns: u64,
    flags: AtFlags,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_utimensat_bytes(binding, dir, path, atime_ns, mtime_ns, flags) },
        |path| unsafe { destack_fs_utimensat_utf16(binding, dir, path, atime_ns, mtime_ns, flags) },
    )
}
