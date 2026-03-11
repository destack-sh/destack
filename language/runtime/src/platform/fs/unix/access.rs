use super::core::*;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::{core as core_fs, *};
use crate::platform::resource::*;
use crate::platform::{core as core_platform, *};
use crate::runtime::BindingCallContext;

/// Check file access permissions.
///
/// Check file access permissions via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses access(2) on Unix and GetFileAttributesW plus ACL checks on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_access_bytes(
    _binding: &BindingCallContext,
    path: PathBytes,
    mode: AccessMode,
) -> RuntimeResult<()> {
    // run the access check on unix platforms
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let rc = unsafe { libc::access(c_path.as_ptr(), mode.0 as libc::c_int) };
    if rc == 0 {
        return Ok(());
    }

    Err(core_platform::io_error("access", None))
}

/// Check file access permissions.
///
/// Check file access permissions via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses access(2) on Unix and GetFileAttributesW plus ACL checks on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_access_utf16(
    binding: &BindingCallContext,
    path: PathUtf16,
    mode: AccessMode,
) -> RuntimeResult<()> {
    // run the access check by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| unsafe {
        destack_fs_access_bytes(binding, path, mode)
    })
}

/// Change file permissions.
///
/// Change file permissions via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses chmod(2) on Unix and file attribute/security updates on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.chmod`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_chmod_bytes(
    _binding: &BindingCallContext,
    path: PathBytes,
    mode: FileMode,
) -> RuntimeResult<()> {
    // apply permissions on unix platforms
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let rc = unsafe { libc::chmod(c_path.as_ptr(), mode.0 as libc::mode_t) };
    if rc == 0 {
        return Ok(());
    }

    Err(core_platform::io_error("chmod", None))
}

/// Change file permissions.
///
/// Change file permissions via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses chmod(2) on Unix and file attribute/security updates on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.chmod`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_chmod_utf16(
    binding: &BindingCallContext,
    path: PathUtf16,
    mode: FileMode,
) -> RuntimeResult<()> {
    // apply permissions by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| unsafe {
        destack_fs_chmod_bytes(binding, path, mode)
    })
}

/// Change file permissions relative to a directory handle.
///
/// Change file permissions relative to a directory handle via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fchmodat(2) on Unix and handle-relative mode updates on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.chmod`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_fchmodat_bytes(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    mode: FileMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // apply permissions relative to the directory on unix platforms
    let resource = directory_resource(binding, dir)?;
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let rc = unsafe {
        libc::fchmodat(
            resource.fd,
            c_path.as_ptr(),
            mode.0 as libc::mode_t,
            flags.0 as libc::c_int,
        )
    };
    if rc == 0 {
        return Ok(());
    }

    Err(core_platform::io_error("fchmodat", None))
}

/// Change file permissions relative to a directory handle.
///
/// Change file permissions relative to a directory handle via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fchmodat(2) on Unix and handle-relative mode updates on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.chmod`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_fchmodat_utf16(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    mode: FileMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // apply permissions by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| unsafe {
        destack_fs_fchmodat_bytes(binding, dir, path, mode, flags)
    })
}

/// Change file owner and group.
///
/// Change file owner and group via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses chown(2) on Unix and token/owner updates where supported on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.chown`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_chown_bytes(
    _binding: &BindingCallContext,
    path: PathBytes,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    // apply ownership on unix platforms
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let rc = unsafe { libc::chown(c_path.as_ptr(), uid, gid) };
    if rc == 0 {
        return Ok(());
    }

    Err(core_platform::io_error("chown", None))
}

/// Change file owner and group.
///
/// Change file owner and group via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses chown(2) on Unix and token/owner updates where supported on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.chown`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_chown_utf16(
    binding: &BindingCallContext,
    path: PathUtf16,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    // apply ownership by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| unsafe {
        destack_fs_chown_bytes(binding, path, uid, gid)
    })
}

/// Change file owner and group relative to a directory handle.
///
/// Change file owner and group relative to a directory handle via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fchownat(2) on Unix and handle-relative owner updates where supported on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.chown`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_fchownat_bytes(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    uid: u32,
    gid: u32,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // apply ownership relative to the directory on unix platforms
    let resource = directory_resource(binding, dir)?;
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let rc = unsafe { libc::fchownat(resource.fd, c_path.as_ptr(), uid, gid, flags.0 as i32) };
    if rc == 0 {
        return Ok(());
    }

    Err(core_platform::io_error("fchownat", None))
}

/// Change file owner and group relative to a directory handle.
///
/// Change file owner and group relative to a directory handle via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fchownat(2) on Unix and handle-relative owner updates where supported on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.chown`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_fchownat_utf16(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    uid: u32,
    gid: u32,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // apply ownership by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| unsafe {
        destack_fs_fchownat_bytes(binding, dir, path, uid, gid, flags)
    })
}

/// Check file access permissions.
///
/// Check file access permissions via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses access(2) on Unix and GetFileAttributesW plus ACL checks on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_access(
    binding: &BindingCallContext,
    path: OsPath,
    mode: AccessMode,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_access_bytes(binding, path, mode) },
        |path| unsafe { destack_fs_access_utf16(binding, path, mode) },
    )
}

/// Change file permissions.
///
/// Change file permissions via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses chmod(2) on Unix and file attribute/security updates on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.chmod`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_chmod(
    binding: &BindingCallContext,
    path: OsPath,
    mode: FileMode,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_chmod_bytes(binding, path, mode) },
        |path| unsafe { destack_fs_chmod_utf16(binding, path, mode) },
    )
}

/// Change file permissions relative to a directory handle.
///
/// Change file permissions relative to a directory handle via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fchmodat(2) on Unix and handle-relative mode updates on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.chmod`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_fchmodat(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: OsPath,
    mode: FileMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_fchmodat_bytes(binding, dir, path, mode, flags) },
        |path| unsafe { destack_fs_fchmodat_utf16(binding, dir, path, mode, flags) },
    )
}

/// Change file owner and group.
///
/// Change file owner and group via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses chown(2) on Unix and token/owner updates where supported on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.chown`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_chown(
    binding: &BindingCallContext,
    path: OsPath,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_chown_bytes(binding, path, uid, gid) },
        |path| unsafe { destack_fs_chown_utf16(binding, path, uid, gid) },
    )
}

/// Change file owner and group relative to a directory handle.
///
/// Change file owner and group relative to a directory handle via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fchownat(2) on Unix and handle-relative owner updates where supported on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.chown`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_fchownat(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: OsPath,
    uid: u32,
    gid: u32,
    flags: AtFlags,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_fchownat_bytes(binding, dir, path, uid, gid, flags) },
        |path| unsafe { destack_fs_fchownat_utf16(binding, dir, path, uid, gid, flags) },
    )
}

/// Check file access permissions relative to a directory handle.
///
/// Check file access permissions relative to a directory handle via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses faccessat(2) on Unix and relative path checks via native handles on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_accessat(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: OsPath,
    mode: AccessMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| {
            #[cfg(unix)]
            {
                let directory_fd = directory_descriptor(binding, dir)?;
                let path = path_bytes_to_cstring(path, "path")?;
                let result = unsafe {
                    libc::faccessat(
                        directory_fd,
                        path.as_ptr(),
                        mode.0 as libc::c_int,
                        flags.0 as libc::c_int,
                    )
                };
                if result != 0 {
                    return Err(RuntimeError::from(PlatformError::io(
                        "faccessat failed".to_string(),
                    ))
                    .boxed());
                }

                Ok(())
            }
            #[cfg(not(unix))]
            {
                let _ = (binding, dir, path, mode, flags);
                Err(RuntimeError::from(PlatformError::not_supported("destack.fs.accessat")).boxed())
            }
        },
        |_path| {
            Err(RuntimeError::from(PlatformError::not_supported("destack.fs.accessat")).boxed())
        },
    )
}
