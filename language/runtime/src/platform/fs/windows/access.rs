use windows_sys::Win32::Foundation::{CloseHandle, HLOCAL, LocalFree};
use windows_sys::Win32::Security::Authorization::{
    GetNamedSecurityInfoW, SE_FILE_OBJECT, SetNamedSecurityInfoW,
};
use windows_sys::Win32::Security::{
    AccessCheck, DACL_SECURITY_INFORMATION, GENERIC_MAPPING, GROUP_SECURITY_INFORMATION,
    MapGenericMask, OWNER_SECURITY_INFORMATION, PRIVILEGE_SET, SecurityImpersonation,
    TOKEN_DUPLICATE, TOKEN_QUERY,
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ALL_ACCESS, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_READONLY, FILE_GENERIC_EXECUTE,
    FILE_GENERIC_READ, FILE_GENERIC_WRITE, GetFileAttributesW, SetFileAttributesW,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::abi::NativeAbi;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::{
    AccessMode, AtFlags, DirectoryHandle, FileMode, OsPath, PathBytes, PathBytesAbi, PathUtf16,
    core as core_fs,
};
use crate::runtime::BindingCallContext;

/// Build one explicit access denied error.
fn access_denied(operation: &str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoPermissionDenied),
        None,
        None,
        Some(operation.to_string()),
        None,
        "access denied",
    ))
    .boxed()
}

/// Validate one `AtFlags` payload for Windows attribute-relative lanes that support no flags.
fn validate_noat_flags(flags: AtFlags, binding_name: &'static str) -> RuntimeResult<()> {
    // reject unknown flag bits explicitly
    let unknown_bits = flags.0 & !(AT_SYMLINK_NOFOLLOW | AT_REMOVEDIR);
    if unknown_bits != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            format!("unsupported {binding_name} flags: {unknown_bits:#x}"),
        ))
        .boxed());
    }

    // surface known but unsupported `*at` flag semantics directly
    if flags.0 != 0 {
        return Err(RuntimeError::from(PlatformError::not_supported(binding_name)).boxed());
    }

    Ok(())
}

/// Build one generic mapping for Windows file objects.
fn file_generic_mapping() -> GENERIC_MAPPING {
    GENERIC_MAPPING {
        GenericRead: FILE_GENERIC_READ,
        GenericWrite: FILE_GENERIC_WRITE,
        GenericExecute: FILE_GENERIC_EXECUTE,
        GenericAll: FILE_ALL_ACCESS,
    }
}

/// Map one Destack access mode into one Windows desired access mask.
fn desired_access_mask(mode: AccessMode) -> u32 {
    let mut desired_access = 0u32;

    if mode.0 & 0o444 != 0 {
        desired_access |= FILE_GENERIC_READ;
    }

    if mode.0 & 0o222 != 0 {
        desired_access |= FILE_GENERIC_WRITE;
    }

    if mode.0 & 0o111 != 0 {
        desired_access |= FILE_GENERIC_EXECUTE;
    }

    desired_access
}

/// Duplicate the current process token into one impersonation token for access checks.
fn current_impersonation_token() -> RuntimeResult<isize> {
    // open the current process token
    let mut token = 0isize;
    let rc = unsafe {
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_QUERY | TOKEN_DUPLICATE,
            &mut token,
        )
    };
    if rc == 0 {
        return Err(last_os_error("OpenProcessToken", None));
    }

    // duplicate it into an impersonation token
    let mut impersonation = 0isize;
    let rc = unsafe {
        windows_sys::Win32::Security::DuplicateToken(
            token,
            SecurityImpersonation,
            &mut impersonation,
        )
    };
    unsafe {
        CloseHandle(token);
    }
    if rc == 0 {
        return Err(last_os_error("DuplicateToken", None));
    }

    Ok(impersonation)
}

/// Check access against the path security descriptor and readonly attribute state.
fn check_path_access(wide: &[u16], attrs: u32, mode: AccessMode) -> RuntimeResult<()> {
    // access mode zero only checks for existence
    let desired_access = desired_access_mask(mode);
    if desired_access == 0 {
        return Ok(());
    }

    // preserve readonly semantics for regular file writes
    let is_directory = attrs & FILE_ATTRIBUTE_DIRECTORY != 0;
    if !is_directory && mode.0 & 0o222 != 0 && attrs & FILE_ATTRIBUTE_READONLY != 0 {
        return Err(access_denied("access"));
    }

    // load the path security descriptor
    let mut owner = std::ptr::null_mut();
    let mut group = std::ptr::null_mut();
    let mut dacl = std::ptr::null_mut();
    let mut descriptor = std::ptr::null_mut();
    let rc = unsafe {
        GetNamedSecurityInfoW(
            wide.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            &mut group,
            &mut dacl,
            std::ptr::null_mut(),
            &mut descriptor,
        )
    };
    if rc != 0 {
        return Err(win32_error("GetNamedSecurityInfoW", rc));
    }

    // free the security descriptor after the check
    struct SecurityDescriptorGuard(*mut core::ffi::c_void);
    impl Drop for SecurityDescriptorGuard {
        fn drop(&mut self) {
            unsafe {
                LocalFree(self.0 as HLOCAL);
            }
        }
    }
    let _descriptor_guard = SecurityDescriptorGuard(descriptor as *mut _);

    // duplicate one impersonation token for AccessCheck
    let token = current_impersonation_token()?;

    // close the token after the access decision
    struct HandleGuard(isize);
    impl Drop for HandleGuard {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    let _token_guard = HandleGuard(token);

    // map generic rights and run the access check
    let mapping = file_generic_mapping();
    let mut desired_access = desired_access;
    unsafe {
        MapGenericMask(&mut desired_access, &mapping);
    }

    let mut privilege_bytes = std::mem::size_of::<PRIVILEGE_SET>() as u32;
    let mut privilege_set = vec![0u8; privilege_bytes as usize];
    let privilege_set = privilege_set.as_mut_ptr() as *mut PRIVILEGE_SET;
    let mut granted_access = 0u32;
    let mut access_status = 0i32;
    let rc = unsafe {
        AccessCheck(
            descriptor,
            token,
            desired_access,
            &mapping,
            privilege_set,
            &mut privilege_bytes,
            &mut granted_access,
            &mut access_status,
        )
    };
    if rc == 0 {
        return Err(last_os_error("AccessCheck", None));
    }
    if access_status == 0 {
        return Err(access_denied("access"));
    }

    Ok(())
}

/// Check file access permissions.
pub(crate) unsafe fn destack_fs_access_bytes(
    _binding: &BindingCallContext,
    path: PathBytes,
    mode: AccessMode,
) -> RuntimeResult<()> {
    // decode the path
    let wide = wide_from_bytes(path, "path")?;

    // check whether the path exists
    let attrs = unsafe { GetFileAttributesW(wide.as_ptr()) };
    if attrs == u32::MAX {
        return Err(last_os_error("GetFileAttributesW", None));
    }

    check_path_access(&wide, attrs, mode)
}

/// Check file access permissions.
pub(crate) unsafe fn destack_fs_access_utf16(
    _binding: &BindingCallContext,
    path: PathUtf16,
    mode: AccessMode,
) -> RuntimeResult<()> {
    // decode the path
    let wide = wide_from_utf16(path, "path")?;

    // check whether the path exists
    let attrs = unsafe { GetFileAttributesW(wide.as_ptr()) };
    if attrs == u32::MAX {
        return Err(last_os_error("GetFileAttributesW", None));
    }

    check_path_access(&wide, attrs, mode)
}

/// Change file permissions.
pub(crate) unsafe fn destack_fs_chmod_bytes(
    _binding: &BindingCallContext,
    path: PathBytes,
    mode: FileMode,
) -> RuntimeResult<()> {
    // decode the path
    let wide = wide_from_bytes(path, "path")?;

    // load the current attributes
    let mut attrs = unsafe { GetFileAttributesW(wide.as_ptr()) };
    if attrs == u32::MAX {
        return Err(last_os_error("GetFileAttributesW", None));
    }

    // map write bits to readonly attribute
    if mode.0 & 0o222 == 0 {
        attrs |= FILE_ATTRIBUTE_READONLY;
    } else {
        attrs &= !FILE_ATTRIBUTE_READONLY;
    }

    // update the file attributes
    let rc = unsafe { SetFileAttributesW(wide.as_ptr(), attrs) };
    if rc == 0 {
        return Err(last_os_error("SetFileAttributesW", None));
    }

    Ok(())
}

/// Change file permissions.
pub(crate) unsafe fn destack_fs_chmod_utf16(
    _binding: &BindingCallContext,
    path: PathUtf16,
    mode: FileMode,
) -> RuntimeResult<()> {
    // decode the path
    let wide = wide_from_utf16(path, "path")?;

    // load the current attributes
    let mut attrs = unsafe { GetFileAttributesW(wide.as_ptr()) };
    if attrs == u32::MAX {
        return Err(last_os_error("GetFileAttributesW", None));
    }

    // map write bits to readonly attribute
    if mode.0 & 0o222 == 0 {
        attrs |= FILE_ATTRIBUTE_READONLY;
    } else {
        attrs &= !FILE_ATTRIBUTE_READONLY;
    }

    // update the file attributes
    let rc = unsafe { SetFileAttributesW(wide.as_ptr(), attrs) };
    if rc == 0 {
        return Err(last_os_error("SetFileAttributesW", None));
    }

    Ok(())
}

/// Change file permissions relative to a directory handle.
pub(crate) unsafe fn destack_fs_fchmodat_bytes(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    mode: FileMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // reject unsupported flags on windows
    validate_noat_flags(flags, "destack.fs.fchmodat")?;

    // resolve the path and delegate to chmod
    let pathbuf = pathbuf_from_bytes(path, "path")?;
    let full_path = if pathbuf.is_absolute() {
        pathbuf
    } else {
        let mut base = directory_path(binding, dir)?;
        base.push(pathbuf);
        base
    };
    let bytes = bytes_from_pathbuf(&full_path, "path")?;
    let path = PathBytesAbi::<NativeAbi>(binding.store_array(bytes));
    unsafe { destack_fs_chmod_bytes(binding, path, mode) }
}

/// Change file permissions relative to a directory handle.
pub(crate) unsafe fn destack_fs_fchmodat_utf16(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    mode: FileMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // reject unsupported flags on windows
    validate_noat_flags(flags, "destack.fs.fchmodat")?;

    // resolve the path and delegate to chmod
    let pathbuf = pathbuf_from_utf16(path, "path")?;
    let full_path = if pathbuf.is_absolute() {
        pathbuf
    } else {
        let mut base = directory_path(binding, dir)?;
        base.push(pathbuf);
        base
    };
    let path = path_utf16_from_pathbuf(binding, &full_path);
    unsafe { destack_fs_chmod_utf16(binding, path, mode) }
}

/// Change file owner and group.
pub(crate) unsafe fn destack_fs_chown_bytes(
    binding: &BindingCallContext,
    path: PathBytes,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    // resolve inputs
    let wide = wide_from_bytes(path, "path")?;
    let (owner, group) = posix_sids(binding, uid, gid)?;

    // update ownership information
    let rc = unsafe {
        SetNamedSecurityInfoW(
            wide.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION,
            owner.as_ptr(),
            group.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if rc != 0 {
        return Err(win32_error("SetNamedSecurityInfoW", rc));
    }

    Ok(())
}

/// Change file owner and group relative to a directory handle.
pub(crate) unsafe fn destack_fs_fchownat_bytes(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    uid: u32,
    gid: u32,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // reject unsupported flags on windows
    validate_noat_flags(flags, "destack.fs.fchownat")?;

    // resolve the path and delegate to chown
    let pathbuf = pathbuf_from_bytes(path, "path")?;
    let full_path = if pathbuf.is_absolute() {
        pathbuf
    } else {
        let mut base = directory_path(binding, dir)?;
        base.push(pathbuf);
        base
    };
    let bytes = bytes_from_pathbuf(&full_path, "path")?;
    let path = PathBytesAbi::<NativeAbi>(binding.store_array(bytes));
    unsafe { destack_fs_chown_bytes(binding, path, uid, gid) }
}

/// Change file owner and group relative to a directory handle.
pub(crate) unsafe fn destack_fs_fchownat_utf16(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    uid: u32,
    gid: u32,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // reject unsupported flags on windows
    validate_noat_flags(flags, "destack.fs.fchownat")?;

    // resolve the path and delegate to chown
    let pathbuf = pathbuf_from_utf16(path, "path")?;
    let full_path = if pathbuf.is_absolute() {
        pathbuf
    } else {
        let mut base = directory_path(binding, dir)?;
        base.push(pathbuf);
        base
    };
    let path = path_utf16_from_pathbuf(binding, &full_path);
    unsafe { destack_fs_chown_utf16(binding, path, uid, gid) }
}

/// Change file owner and group.
pub(crate) unsafe fn destack_fs_chown_utf16(
    binding: &BindingCallContext,
    path: PathUtf16,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    // resolve inputs
    let wide = wide_from_utf16(path, "path")?;
    let (owner, group) = posix_sids(binding, uid, gid)?;

    // update ownership information
    let rc = unsafe {
        SetNamedSecurityInfoW(
            wide.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION,
            owner.as_ptr(),
            group.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if rc != 0 {
        return Err(win32_error("SetNamedSecurityInfoW", rc));
    }

    Ok(())
}

/// Check file access permissions.
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
pub(crate) unsafe fn destack_fs_accessat(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: OsPath,
    mode: AccessMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // reject unsupported flags on windows
    validate_noat_flags(flags, "destack.fs.accessat")?;

    core_fs::with_path_ref(
        path,
        "path",
        |path| {
            // resolve the path and delegate to access
            let pathbuf = pathbuf_from_bytes(path, "path")?;
            let full_path = if pathbuf.is_absolute() {
                pathbuf
            } else {
                let mut base = directory_path(binding, dir)?;
                base.push(pathbuf);
                base
            };
            let bytes = bytes_from_pathbuf(&full_path, "path")?;
            let path = PathBytesAbi::<NativeAbi>(binding.store_array(bytes));

            unsafe { destack_fs_access_bytes(binding, path, mode) }
        },
        |path| {
            // resolve the path and delegate to access
            let pathbuf = pathbuf_from_utf16(path, "path")?;
            let full_path = if pathbuf.is_absolute() {
                pathbuf
            } else {
                let mut base = directory_path(binding, dir)?;
                base.push(pathbuf);
                base
            };
            let path = path_utf16_from_pathbuf(binding, &full_path);

            unsafe { destack_fs_access_utf16(binding, path, mode) }
        },
    )
}
