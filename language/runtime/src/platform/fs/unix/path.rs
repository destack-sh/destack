use super::core::*;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{core as core_fs, *};
use crate::platform::resource::*;
use crate::platform::{core as core_platform, *};
use crate::runtime::BindingCallContext;

use std::ffi::CStr;

/// Create a hard link.
///
/// Create a hard-link entry that points to an existing inode without copying file contents.
/// Source and destination remain independent path entries with shared storage identity.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses link(2) on Unix and CreateHardLinkW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.link`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_link_bytes(
    _binding: &BindingCallContext,
    existingpath: PathBytes,
    newpath: PathBytes,
) -> RuntimeResult<()> {
    // create the link on unix platforms
    let existing = resolve_path_bytes_cstring(existingpath, "existingPath")?;
    let newpath = resolve_path_bytes_cstring(newpath, "newPath")?;
    let rc = unsafe { libc::link(existing.as_ptr(), newpath.as_ptr()) };
    if rc == 0 {
        return Ok(());
    }

    Err(core_platform::io_error("link", None))
}

/// Create a hard link.
///
/// Create a hard-link entry that points to an existing inode without copying file contents.
/// Source and destination remain independent path entries with shared storage identity.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses link(2) on Unix and CreateHardLinkW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.link`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_link_utf16(
    binding: &BindingCallContext,
    existingpath: PathUtf16,
    newpath: PathUtf16,
) -> RuntimeResult<()> {
    // create the link by converting utf16 path inputs
    core_fs::with_utf16_pair_as_bytes(
        existingpath,
        newpath,
        "path",
        |existingpath, newpath| unsafe { destack_fs_link_bytes(binding, existingpath, newpath) },
    )
}

/// Read a symbolic link.
///
/// Read the link payload stored at the target path and return it as an `OsPath`.
/// The returned path is link data and is not canonicalized or dereferenced.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses readlink(2) on Unix and reparse-point target query on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_readlink_bytes(
    binding: &BindingCallContext,
    out: *mut PathBytes,
    path: PathBytes,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read the symlink target on unix platforms
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let bytes = readlink_bytes_at(libc::AT_FDCWD, &c_path)?;
    unsafe {
        *out = PathBytesAbi::<NativeAbi>(binding.store_array(bytes));
    }
    Ok(())
}

#[allow(dead_code)]
/// Read a symbolic link.
///
/// Read the link payload stored at the target path and return it as an `OsPath`.
/// The returned path is link data and is not canonicalized or dereferenced.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses readlink(2) on Unix and reparse-point target query on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_readlink_utf16(
    binding: &BindingCallContext,
    out: *mut PathUtf16,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read one symlink target by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| {
        let mut bytes_output = core_fs::empty_path_bytes();
        unsafe { destack_fs_readlink_bytes(binding, &mut bytes_output, path) }?;
        let utf16_output = core_fs::path_utf16_from_bytes(binding, bytes_output, "path")?;
        unsafe {
            *out = utf16_output;
        }
        Ok(())
    })
}

/// Resolve a path to its canonical form.
///
/// Resolve the input path to a canonical absolute form using host path-resolution rules.
/// Canonicalization follows host symlink, mount, and case-normalization behavior.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses realpath(3) on Unix and GetFinalPathNameByHandleW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_realpath_bytes(
    binding: &BindingCallContext,
    out: *mut PathBytes,
    path: PathBytes,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the canonical path on unix platforms
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let resolved = unsafe { libc::realpath(c_path.as_ptr(), std::ptr::null_mut()) };
    if resolved.is_null() {
        return Err(core_platform::io_error("realpath", None));
    }

    let bytes = unsafe { CStr::from_ptr(resolved) }.to_bytes().to_vec();
    unsafe {
        libc::free(resolved as *mut libc::c_void);
    }
    unsafe {
        *out = PathBytesAbi::<NativeAbi>(binding.store_array(bytes));
    }
    Ok(())
}

#[allow(dead_code)]
/// Resolve a path to its canonical form.
///
/// Resolve the input path to a canonical absolute form using host path-resolution rules.
/// Canonicalization follows host symlink, mount, and case-normalization behavior.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses realpath(3) on Unix and GetFinalPathNameByHandleW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_realpath_utf16(
    binding: &BindingCallContext,
    out: *mut PathUtf16,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve one canonical path by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| {
        let mut bytes_output = core_fs::empty_path_bytes();
        unsafe { destack_fs_realpath_bytes(binding, &mut bytes_output, path) }?;
        let utf16_output = core_fs::path_utf16_from_bytes(binding, bytes_output, "path")?;
        unsafe {
            *out = utf16_output;
        }
        Ok(())
    })
}

/// Rename or move a file.
///
/// Rename one path entry to a new absolute or relative path in the current process namespace.
/// The operation targets plain path names and does not expose directory-handle scoping.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses rename(2) on Unix and MoveFileExW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_rename_bytes(
    _binding: &BindingCallContext,
    from: PathBytes,
    to: PathBytes,
) -> RuntimeResult<()> {
    // rename the path on unix platforms
    let from = resolve_path_bytes_cstring(from, "from")?;
    let to = resolve_path_bytes_cstring(to, "to")?;
    let result = unsafe { libc::rename(from.as_ptr(), to.as_ptr()) };
    if result != 0 {
        return Err(core_platform::io_error("rename", None));
    }

    Ok(())
}

/// Rename or move a file.
///
/// Rename one path entry to a new absolute or relative path in the current process namespace.
/// The operation targets plain path names and does not expose directory-handle scoping.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses rename(2) on Unix and MoveFileExW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_rename_utf16(
    binding: &BindingCallContext,
    from: PathUtf16,
    to: PathUtf16,
) -> RuntimeResult<()> {
    // rename one path by converting utf16 path inputs
    core_fs::with_utf16_pair_as_bytes(from, to, "path", |from, to| unsafe {
        destack_fs_rename_bytes(binding, from, to)
    })
}

/// Create a symbolic link.
///
/// Create a symbolic-link entry that stores the provided target path payload.
/// Target bytes are persisted as link data and are not resolved during creation.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses symlink(2) on Unix and CreateSymbolicLinkW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.link`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_symlink_bytes(
    _binding: &BindingCallContext,
    target: PathBytes,
    path: PathBytes,
    _kind: SymlinkType,
) -> RuntimeResult<()> {
    // create the symlink on unix platforms
    let target = resolve_path_bytes_cstring(target, "target")?;
    let path = resolve_path_bytes_cstring(path, "path")?;
    let rc = unsafe { libc::symlink(target.as_ptr(), path.as_ptr()) };
    if rc == 0 {
        return Ok(());
    }

    Err(core_platform::io_error("symlink", None))
}

/// Create a symbolic link.
///
/// Create a symbolic-link entry that stores the provided target path payload.
/// Target bytes are persisted as link data and are not resolved during creation.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses symlink(2) on Unix and CreateSymbolicLinkW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.link`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_symlink_utf16(
    binding: &BindingCallContext,
    target: PathUtf16,
    path: PathUtf16,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    // create one symlink by converting utf16 path inputs
    core_fs::with_utf16_pair_as_bytes(target, path, "path", |target, path| unsafe {
        destack_fs_symlink_bytes(binding, target, path, kind)
    })
}

/// Unlink a file.
///
/// Remove one directory entry that names a non-directory filesystem object.
/// Data blocks are reclaimed by the host once link count and open-handle rules allow.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses unlink(2) on Unix and DeleteFileW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_unlink_bytes(
    _binding: &BindingCallContext,
    path: PathBytes,
) -> RuntimeResult<()> {
    // unlink the file on unix platforms
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let rc = unsafe { libc::unlink(c_path.as_ptr()) };
    if rc == 0 {
        return Ok(());
    }

    Err(core_platform::io_error("unlink", None))
}

/// Unlink a file.
///
/// Remove one directory entry that names a non-directory filesystem object.
/// Data blocks are reclaimed by the host once link count and open-handle rules allow.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses unlink(2) on Unix and DeleteFileW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_unlink_utf16(
    binding: &BindingCallContext,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // unlink one file by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| unsafe {
        destack_fs_unlink_bytes(binding, path)
    })
}

/// Rename or move a file relative to directory handles.
///
/// Rename one path entry where both source and destination are resolved relative to explicit directory handles.
/// This avoids ambient current-working-directory resolution for both sides of the rename.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses renameat(2) on Unix and handle-relative rename on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_renameat_bytes(
    binding: &BindingCallContext,
    from_dir: DirectoryHandle,
    from: PathBytes,
    to_dir: DirectoryHandle,
    to: PathBytes,
) -> RuntimeResult<()> {
    // rename the path on unix platforms
    let from_resource = directory_resource(binding, from_dir)?;
    let to_resource = directory_resource(binding, to_dir)?;
    let from = resolve_path_bytes_cstring(from, "from")?;
    let to = resolve_path_bytes_cstring(to, "to")?;
    let result =
        unsafe { libc::renameat(from_resource.fd, from.as_ptr(), to_resource.fd, to.as_ptr()) };
    if result != 0 {
        return Err(core_platform::io_error("renameat", None));
    }
    Ok(())
}

/// Rename or move a file relative to directory handles.
///
/// Rename one path entry where both source and destination are resolved relative to explicit directory handles.
/// This avoids ambient current-working-directory resolution for both sides of the rename.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses renameat(2) on Unix and handle-relative rename on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_renameat_utf16(
    binding: &BindingCallContext,
    from_dir: DirectoryHandle,
    from: PathUtf16,
    to_dir: DirectoryHandle,
    to: PathUtf16,
) -> RuntimeResult<()> {
    // rename one path by converting utf16 path inputs
    core_fs::with_utf16_pair_as_bytes(from, to, "path", |from, to| unsafe {
        destack_fs_renameat_bytes(binding, from_dir, from, to_dir, to)
    })
}

/// Rename or move a file relative to directory handles with renameat2 semantics.
///
/// Rename one path entry with explicit rename flags controlling replace and exchange behavior.
/// Flag handling follows host support levels and returns notSupported when the requested mode is unavailable.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses renameat2(2) on linux and runtime emulation/fallback on other targets.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_renameat2_bytes(
    binding: &BindingCallContext,
    from_dir: DirectoryHandle,
    from: PathBytes,
    to_dir: DirectoryHandle,
    to: PathBytes,
    flags: RenameFlags,
) -> RuntimeResult<()> {
    // rename the path with renameat2 on linux platforms
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let from_resource = directory_resource(binding, from_dir)?;
        let to_resource = directory_resource(binding, to_dir)?;
        let from = resolve_path_bytes_cstring(from, "from")?;
        let to = resolve_path_bytes_cstring(to, "to")?;
        let rc = unsafe {
            libc::syscall(
                libc::SYS_renameat2,
                from_resource.fd,
                from.as_ptr(),
                to_resource.fd,
                to.as_ptr(),
                flags.0 as libc::c_uint,
            )
        } as libc::c_int;
        if rc != 0 {
            return Err(core_platform::io_error("renameat2", None));
        }
        Ok(())
    }

    // fall back to renameat when no flags are requested on other unix platforms
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        if flags.0 != 0 {
            return Err(
                RuntimeError::from(PlatformError::not_supported("destack.fs.renameat2")).boxed(),
            );
        }
        unsafe { destack_fs_renameat_bytes(binding, from_dir, from, to_dir, to) }
    }
}

/// Rename or move a file relative to directory handles with renameat2 semantics.
///
/// Rename one path entry with explicit rename flags controlling replace and exchange behavior.
/// Flag handling follows host support levels and returns notSupported when the requested mode is unavailable.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses renameat2(2) on linux and runtime emulation/fallback on other targets.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_renameat2_utf16(
    binding: &BindingCallContext,
    from_dir: DirectoryHandle,
    from: PathUtf16,
    to_dir: DirectoryHandle,
    to: PathUtf16,
    flags: RenameFlags,
) -> RuntimeResult<()> {
    // rename one path by converting utf16 path inputs
    core_fs::with_utf16_pair_as_bytes(from, to, "path", |from, to| unsafe {
        destack_fs_renameat2_bytes(binding, from_dir, from, to_dir, to, flags)
    })
}

/// Unlink a file relative to a directory handle.
///
/// Remove one directory entry resolved from `dir` for a non-directory filesystem object.
/// Relative unlink avoids ambient cwd traversal and keeps deletion scope explicit.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses unlinkat(2) on Unix and handle-relative delete on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_unlinkat_bytes(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // unlink the path on unix platforms
    let resource = directory_resource(binding, dir)?;
    let path = resolve_path_bytes_cstring(path, "path")?;
    let result = unsafe { libc::unlinkat(resource.fd, path.as_ptr(), flags.0 as libc::c_int) };
    if result != 0 {
        return Err(core_platform::io_error("unlinkat", None));
    }
    Ok(())
}

/// Unlink a file relative to a directory handle.
///
/// Remove one directory entry resolved from `dir` for a non-directory filesystem object.
/// Relative unlink avoids ambient cwd traversal and keeps deletion scope explicit.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses unlinkat(2) on Unix and handle-relative delete on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_unlinkat_utf16(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // unlink one path by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| unsafe {
        destack_fs_unlinkat_bytes(binding, dir, path, flags)
    })
}

/// Create a hard link relative to directory handles.
///
/// Create a hard-link entry using directory-relative paths for both source and destination.
/// Relative resolution keeps both lookup roots explicit and avoids ambient cwd lookup.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses linkat(2) on Unix and handle-relative hard-link creation on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.link`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_linkat_bytes(
    binding: &BindingCallContext,
    existing_dir: DirectoryHandle,
    existing_path: PathBytes,
    new_dir: DirectoryHandle,
    new_path: PathBytes,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // link the paths on unix platforms
    let existing_resource = directory_resource(binding, existing_dir)?;
    let new_resource = directory_resource(binding, new_dir)?;
    let existing_path = resolve_path_bytes_cstring(existing_path, "existingPath")?;
    let new_path = resolve_path_bytes_cstring(new_path, "newPath")?;
    let result = unsafe {
        libc::linkat(
            existing_resource.fd,
            existing_path.as_ptr(),
            new_resource.fd,
            new_path.as_ptr(),
            flags.0 as libc::c_int,
        )
    };
    if result != 0 {
        return Err(core_platform::io_error("linkat", None));
    }
    Ok(())
}

/// Create a hard link relative to directory handles.
///
/// Create a hard-link entry using directory-relative paths for both source and destination.
/// Relative resolution keeps both lookup roots explicit and avoids ambient cwd lookup.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses linkat(2) on Unix and handle-relative hard-link creation on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.link`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_linkat_utf16(
    binding: &BindingCallContext,
    existing_dir: DirectoryHandle,
    existing_path: PathUtf16,
    new_dir: DirectoryHandle,
    new_path: PathUtf16,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // create one link by converting utf16 path inputs
    core_fs::with_utf16_pair_as_bytes(
        existing_path,
        new_path,
        "path",
        |existing_path, new_path| unsafe {
            destack_fs_linkat_bytes(
                binding,
                existing_dir,
                existing_path,
                new_dir,
                new_path,
                flags,
            )
        },
    )
}

/// Create a symbolic link relative to a directory handle.
///
/// Create a symbolic-link entry using a directory-relative destination path.
/// Destination lookup uses `dir` while `target` bytes are stored verbatim by the host.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses symlinkat(2) on Unix and handle-relative symlink creation on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.link`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_symlinkat_bytes(
    binding: &BindingCallContext,
    target: PathBytes,
    dir: DirectoryHandle,
    path: PathBytes,
    _kind: SymlinkType,
) -> RuntimeResult<()> {
    // create the symlink on unix platforms
    let resource = directory_resource(binding, dir)?;
    let target = resolve_path_bytes_cstring(target, "target")?;
    let path = resolve_path_bytes_cstring(path, "path")?;
    let result = unsafe { libc::symlinkat(target.as_ptr(), resource.fd, path.as_ptr()) };
    if result != 0 {
        return Err(core_platform::io_error("symlinkat", None));
    }
    Ok(())
}

/// Create a symbolic link relative to a directory handle.
///
/// Create a symbolic-link entry using a directory-relative destination path.
/// Destination lookup uses `dir` while `target` bytes are stored verbatim by the host.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses symlinkat(2) on Unix and handle-relative symlink creation on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.link`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_symlinkat_utf16(
    binding: &BindingCallContext,
    target: PathUtf16,
    dir: DirectoryHandle,
    path: PathUtf16,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    // create one symlink by converting utf16 path inputs
    core_fs::with_utf16_pair_as_bytes(target, path, "path", |target, path| unsafe {
        destack_fs_symlinkat_bytes(binding, target, dir, path, kind)
    })
}

/// Read a symbolic link relative to a directory handle.
///
/// Read the link payload stored at a directory-relative target path.
/// The returned path is raw link data and is not dereferenced during the read.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses readlinkat(2) on Unix and handle-relative target query on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_readlinkat_bytes(
    binding: &BindingCallContext,
    out: *mut PathBytes,
    dir: DirectoryHandle,
    path: PathBytes,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read the symlink target on unix platforms
    let resource = directory_resource(binding, dir)?;
    let path = resolve_path_bytes_cstring(path, "path")?;
    let bytes = readlink_bytes_at(resource.fd, &path)?;
    unsafe {
        *out = PathBytesAbi::<NativeAbi>(binding.store_array(bytes));
    }
    Ok(())
}

#[allow(dead_code)]
/// Read a symbolic link relative to a directory handle.
///
/// Read the link payload stored at a directory-relative target path.
/// The returned path is raw link data and is not dereferenced during the read.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses readlinkat(2) on Unix and handle-relative target query on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_readlinkat_utf16(
    binding: &BindingCallContext,
    out: *mut PathUtf16,
    dir: DirectoryHandle,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read one symlink target by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| {
        let mut bytes_output = core_fs::empty_path_bytes();
        unsafe { destack_fs_readlinkat_bytes(binding, &mut bytes_output, dir, path) }?;
        let utf16_output = core_fs::path_utf16_from_bytes(binding, bytes_output, "path")?;
        unsafe {
            *out = utf16_output;
        }
        Ok(())
    })
}

/// Rename or move a file.
///
/// Rename one path entry to a new absolute or relative path in the current process namespace.
/// The operation targets plain path names and does not expose directory-handle scoping.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses rename(2) on Unix and MoveFileExW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_rename(
    binding: &BindingCallContext,
    from: OsPath,
    to: OsPath,
) -> RuntimeResult<()> {
    core_fs::with_path_ref_pair(
        from,
        to,
        "path",
        |from, to| unsafe { destack_fs_rename_bytes(binding, from, to) },
        |from, to| unsafe { destack_fs_rename_utf16(binding, from, to) },
    )
}

/// Rename or move a file relative to directory handles.
///
/// Rename one path entry where both source and destination are resolved relative to explicit directory handles.
/// This avoids ambient current-working-directory resolution for both sides of the rename.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses renameat(2) on Unix and handle-relative rename on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_renameat(
    binding: &BindingCallContext,
    from_dir: DirectoryHandle,
    from: OsPath,
    to_dir: DirectoryHandle,
    to: OsPath,
) -> RuntimeResult<()> {
    core_fs::with_path_ref_pair(
        from,
        to,
        "path",
        |from, to| unsafe { destack_fs_renameat_bytes(binding, from_dir, from, to_dir, to) },
        |from, to| unsafe { destack_fs_renameat_utf16(binding, from_dir, from, to_dir, to) },
    )
}

/// Rename or move a file relative to directory handles with renameat2 semantics.
///
/// Rename one path entry with explicit rename flags controlling replace and exchange behavior.
/// Flag handling follows host support levels and returns notSupported when the requested mode is unavailable.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses renameat2(2) on linux and runtime emulation/fallback on other targets.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_renameat2(
    binding: &BindingCallContext,
    from_dir: DirectoryHandle,
    from: OsPath,
    to_dir: DirectoryHandle,
    to: OsPath,
    flags: RenameFlags,
) -> RuntimeResult<()> {
    core_fs::with_path_ref_pair(
        from,
        to,
        "path",
        |from, to| unsafe {
            destack_fs_renameat2_bytes(binding, from_dir, from, to_dir, to, flags)
        },
        |from, to| unsafe {
            destack_fs_renameat2_utf16(binding, from_dir, from, to_dir, to, flags)
        },
    )
}

/// Unlink a file.
///
/// Remove one directory entry that names a non-directory filesystem object.
/// Data blocks are reclaimed by the host once link count and open-handle rules allow.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses unlink(2) on Unix and DeleteFileW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_unlink(
    binding: &BindingCallContext,
    path: OsPath,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_unlink_bytes(binding, path) },
        |path| unsafe { destack_fs_unlink_utf16(binding, path) },
    )
}

/// Unlink a file relative to a directory handle.
///
/// Remove one directory entry resolved from `dir` for a non-directory filesystem object.
/// Relative unlink avoids ambient cwd traversal and keeps deletion scope explicit.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses unlinkat(2) on Unix and handle-relative delete on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_unlinkat(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: OsPath,
    flags: AtFlags,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_unlinkat_bytes(binding, dir, path, flags) },
        |path| unsafe { destack_fs_unlinkat_utf16(binding, dir, path, flags) },
    )
}

/// Create a hard link.
///
/// Create a hard-link entry that points to an existing inode without copying file contents.
/// Source and destination remain independent path entries with shared storage identity.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses link(2) on Unix and CreateHardLinkW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.link`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_link(
    binding: &BindingCallContext,
    existing_path: OsPath,
    new_path: OsPath,
) -> RuntimeResult<()> {
    core_fs::with_path_ref_pair(
        existing_path,
        new_path,
        "path",
        |existing_path, new_path| unsafe {
            destack_fs_link_bytes(binding, existing_path, new_path)
        },
        |existing_path, new_path| unsafe {
            destack_fs_link_utf16(binding, existing_path, new_path)
        },
    )
}

/// Create a hard link relative to directory handles.
///
/// Create a hard-link entry using directory-relative paths for both source and destination.
/// Relative resolution keeps both lookup roots explicit and avoids ambient cwd lookup.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses linkat(2) on Unix and handle-relative hard-link creation on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.link`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_linkat(
    binding: &BindingCallContext,
    existing_dir: DirectoryHandle,
    existing_path: OsPath,
    new_dir: DirectoryHandle,
    new_path: OsPath,
    flags: AtFlags,
) -> RuntimeResult<()> {
    core_fs::with_path_ref_pair(
        existing_path,
        new_path,
        "path",
        |existing_path, new_path| unsafe {
            destack_fs_linkat_bytes(
                binding,
                existing_dir,
                existing_path,
                new_dir,
                new_path,
                flags,
            )
        },
        |existing_path, new_path| unsafe {
            destack_fs_linkat_utf16(
                binding,
                existing_dir,
                existing_path,
                new_dir,
                new_path,
                flags,
            )
        },
    )
}

/// Create a symbolic link.
///
/// Create a symbolic-link entry that stores the provided target path payload.
/// Target bytes are persisted as link data and are not resolved during creation.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses symlink(2) on Unix and CreateSymbolicLinkW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.link`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_symlink(
    binding: &BindingCallContext,
    target: OsPath,
    path: OsPath,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    core_fs::with_path_ref_pair(
        target,
        path,
        "path",
        |target, path| unsafe { destack_fs_symlink_bytes(binding, target, path, kind) },
        |target, path| unsafe { destack_fs_symlink_utf16(binding, target, path, kind) },
    )
}

/// Create a symbolic link relative to a directory handle.
///
/// Create a symbolic-link entry using a directory-relative destination path.
/// Destination lookup uses `dir` while `target` bytes are stored verbatim by the host.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses symlinkat(2) on Unix and handle-relative symlink creation on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.link`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_symlinkat(
    binding: &BindingCallContext,
    target: OsPath,
    dir: DirectoryHandle,
    path: OsPath,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    core_fs::with_path_ref_pair(
        target,
        path,
        "path",
        |target, path| unsafe { destack_fs_symlinkat_bytes(binding, target, dir, path, kind) },
        |target, path| unsafe { destack_fs_symlinkat_utf16(binding, target, dir, path, kind) },
    )
}

/// Read a symbolic link.
///
/// Read the link payload stored at the target path and return it as an `OsPath`.
/// The returned path is link data and is not canonicalized or dereferenced.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses readlink(2) on Unix and reparse-point target query on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_readlink(
    binding: &BindingCallContext,
    out: *mut OsPath,
    path: OsPath,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    #[cfg(unix)]
    {
        match path {
            OsPath::OsPathBytes(path_bytes) => {
                let mut inner = core_fs::empty_path_bytes();
                unsafe { destack_fs_readlink_bytes(binding, &mut inner, path_bytes.bytes) }?;
                unsafe {
                    *out = core_fs::path_ref_from_bytes(inner);
                }
                Ok(())
            }
            OsPath::OsPathUtf16(path_utf16) => {
                let bytes = core_fs::with_utf16_as_bytes(path_utf16.utf16, "path", |path| {
                    let mut inner = core_fs::empty_path_bytes();
                    unsafe { destack_fs_readlink_bytes(binding, &mut inner, path) }?;
                    Ok(inner)
                })?;
                let utf16 = core_fs::path_utf16_from_bytes(binding, bytes, "path")?;
                unsafe {
                    *out = core_fs::path_ref_from_utf16(utf16);
                }
                Ok(())
            }
        }
    }
    #[cfg(not(unix))]
    core_fs::with_path_ref(
        path,
        "path",
        |path| {
            let mut inner = core_fs::empty_path_bytes();
            unsafe { destack_fs_readlink_bytes(binding, &mut inner, path) }?;
            unsafe {
                *out = core_fs::path_ref_from_bytes(inner);
            }
            Ok(())
        },
        |path| {
            let mut inner = core_fs::empty_path_utf16();
            unsafe { destack_fs_readlink_utf16(binding, &mut inner, path) }?;
            unsafe {
                *out = core_fs::path_ref_from_utf16(inner);
            }
            Ok(())
        },
    )
}

/// Read a symbolic link relative to a directory handle.
///
/// Read the link payload stored at a directory-relative target path.
/// The returned path is raw link data and is not dereferenced during the read.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses readlinkat(2) on Unix and handle-relative target query on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_readlinkat(
    binding: &BindingCallContext,
    out: *mut OsPath,
    dir: DirectoryHandle,
    path: OsPath,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    #[cfg(unix)]
    {
        match path {
            OsPath::OsPathBytes(path_bytes) => {
                let mut inner = core_fs::empty_path_bytes();
                unsafe { destack_fs_readlinkat_bytes(binding, &mut inner, dir, path_bytes.bytes) }?;
                unsafe {
                    *out = core_fs::path_ref_from_bytes(inner);
                }
                Ok(())
            }
            OsPath::OsPathUtf16(path_utf16) => {
                let bytes = core_fs::with_utf16_as_bytes(path_utf16.utf16, "path", |path| {
                    let mut inner = core_fs::empty_path_bytes();
                    unsafe { destack_fs_readlinkat_bytes(binding, &mut inner, dir, path) }?;
                    Ok(inner)
                })?;
                let utf16 = core_fs::path_utf16_from_bytes(binding, bytes, "path")?;
                unsafe {
                    *out = core_fs::path_ref_from_utf16(utf16);
                }
                Ok(())
            }
        }
    }
    #[cfg(not(unix))]
    core_fs::with_path_ref(
        path,
        "path",
        |path| {
            let mut inner = core_fs::empty_path_bytes();
            unsafe { destack_fs_readlinkat_bytes(binding, &mut inner, dir, path) }?;
            unsafe {
                *out = core_fs::path_ref_from_bytes(inner);
            }
            Ok(())
        },
        |path| {
            let mut inner = core_fs::empty_path_utf16();
            unsafe { destack_fs_readlinkat_utf16(binding, &mut inner, dir, path) }?;
            unsafe {
                *out = core_fs::path_ref_from_utf16(inner);
            }
            Ok(())
        },
    )
}

/// Resolve a path to its canonical form.
///
/// Resolve the input path to a canonical absolute form using host path-resolution rules.
/// Canonicalization follows host symlink, mount, and case-normalization behavior.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses realpath(3) on Unix and GetFinalPathNameByHandleW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_realpath(
    binding: &BindingCallContext,
    out: *mut OsPath,
    path: OsPath,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    #[cfg(unix)]
    {
        match path {
            OsPath::OsPathBytes(path_bytes) => {
                let mut inner = core_fs::empty_path_bytes();
                unsafe { destack_fs_realpath_bytes(binding, &mut inner, path_bytes.bytes) }?;
                unsafe {
                    *out = core_fs::path_ref_from_bytes(inner);
                }
                Ok(())
            }
            OsPath::OsPathUtf16(path_utf16) => {
                let bytes = core_fs::with_utf16_as_bytes(path_utf16.utf16, "path", |path| {
                    let mut inner = core_fs::empty_path_bytes();
                    unsafe { destack_fs_realpath_bytes(binding, &mut inner, path) }?;
                    Ok(inner)
                })?;
                let utf16 = core_fs::path_utf16_from_bytes(binding, bytes, "path")?;
                unsafe {
                    *out = core_fs::path_ref_from_utf16(utf16);
                }
                Ok(())
            }
        }
    }
    #[cfg(not(unix))]
    core_fs::with_path_ref(
        path,
        "path",
        |path| {
            let mut inner = core_fs::empty_path_bytes();
            unsafe { destack_fs_realpath_bytes(binding, &mut inner, path) }?;
            unsafe {
                *out = core_fs::path_ref_from_bytes(inner);
            }
            Ok(())
        },
        |path| {
            let mut inner = core_fs::empty_path_utf16();
            unsafe { destack_fs_realpath_utf16(binding, &mut inner, path) }?;
            unsafe {
                *out = core_fs::path_ref_from_utf16(inner);
            }
            Ok(())
        },
    )
}

/// Create a FIFO special file.
///
/// Create a FIFO special file node at the target path.
/// The created node participates in host pipe semantics when opened for I/O.
///
/// # Platform
/// Unix only. This operation returns `notSupported` on Windows.
/// Uses mkfifo(2) on Unix and notSupported on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.special`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_mkfifo(
    _binding: &BindingCallContext,
    path: OsPath,
    mode: FileMode,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| {
            #[cfg(unix)]
            {
                let path = path_bytes_to_cstring(path, "path")?;
                let result = unsafe { libc::mkfifo(path.as_ptr(), mode.0 as libc::mode_t) };
                if result != 0 {
                    return Err(
                        RuntimeError::from(PlatformError::io("mkfifo failed".to_string())).boxed(),
                    );
                }
                Ok(())
            }
            #[cfg(not(unix))]
            {
                let _ = (binding, mode, path);
                Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkfifo")).boxed())
            }
        },
        |_path| Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkfifo")).boxed()),
    )
}

/// Create a FIFO special file relative to a directory handle.
///
/// Create a FIFO special file node at a directory-relative path.
/// Relative node creation keeps lookup scope anchored to `dir`.
///
/// # Platform
/// Unix only. This operation returns `notSupported` on Windows.
/// Uses mkfifoat(2) on Unix and notSupported on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.special`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_mkfifoat(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: OsPath,
    mode: FileMode,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| {
            #[cfg(unix)]
            {
                let directory_fd = directory_descriptor(binding, dir)?;
                let path = path_bytes_to_cstring(path, "path")?;
                #[cfg(target_os = "android")]
                let result = unsafe {
                    libc::syscall(
                        libc::SYS_mknodat,
                        directory_fd,
                        path.as_ptr(),
                        (mode.0 | libc::S_IFIFO) as libc::mode_t,
                        0 as libc::dev_t,
                    ) as libc::c_int
                };
                #[cfg(not(target_os = "android"))]
                let result =
                    unsafe { libc::mkfifoat(directory_fd, path.as_ptr(), mode.0 as libc::mode_t) };
                if result != 0 {
                    return Err(RuntimeError::from(PlatformError::io(
                        "mkfifoat failed".to_string(),
                    ))
                    .boxed());
                }
                Ok(())
            }
            #[cfg(not(unix))]
            {
                let _ = (binding, dir, path, mode);
                Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkfifoat")).boxed())
            }
        },
        |_path| {
            Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkfifoat")).boxed())
        },
    )
}

/// Create a filesystem node.
///
/// Create a filesystem node with the requested mode and device number.
/// Node interpretation follows host mknod rules for file type and device payload.
///
/// # Platform
/// Unix only. This operation returns `notSupported` on Windows.
/// Uses mknod(2) on Unix and notSupported on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.special`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_mknod(
    _binding: &BindingCallContext,
    path: OsPath,
    mode: FileMode,
    device: NodeDevice,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| {
            #[cfg(unix)]
            {
                let path = path_bytes_to_cstring(path, "path")?;
                let result = unsafe {
                    libc::mknod(
                        path.as_ptr(),
                        mode.0 as libc::mode_t,
                        device.0 as libc::dev_t,
                    )
                };
                if result != 0 {
                    return Err(
                        RuntimeError::from(PlatformError::io("mknod failed".to_string())).boxed(),
                    );
                }
                Ok(())
            }
            #[cfg(not(unix))]
            {
                let _ = (binding, mode, device, path);
                Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mknod")).boxed())
            }
        },
        |_path| Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mknod")).boxed()),
    )
}

/// Create a filesystem node relative to a directory handle.
///
/// Create a filesystem node at a directory-relative path with the requested mode and device number.
/// Relative creation keeps lookup scope anchored to `dir`.
///
/// # Platform
/// Unix only. This operation returns `notSupported` on Windows.
/// Uses mknodat(2) on Unix and notSupported on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.special`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_mknodat(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: OsPath,
    mode: FileMode,
    device: NodeDevice,
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
                    libc::mknodat(
                        directory_fd,
                        path.as_ptr(),
                        mode.0 as libc::mode_t,
                        device.0 as libc::dev_t,
                    )
                };
                if result != 0 {
                    return Err(RuntimeError::from(PlatformError::io(
                        "mknodat failed".to_string(),
                    ))
                    .boxed());
                }
                Ok(())
            }
            #[cfg(not(unix))]
            {
                let _ = (binding, dir, mode, device, path);
                Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mknodat")).boxed())
            }
        },
        |_path| Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mknodat")).boxed()),
    )
}
