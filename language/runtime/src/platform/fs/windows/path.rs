use windows_sys::Wdk::Storage::FileSystem::{
    FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE, FILE_OPEN, FILE_OPEN_REPARSE_POINT,
};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateHardLinkW, CreateSymbolicLinkW, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    MOVEFILE_REPLACE_EXISTING, MoveFileExW, SYMBOLIC_LINK_FLAG_ALLOW_UNPRIVILEGED_CREATE,
    SYMBOLIC_LINK_FLAG_DIRECTORY,
};

use super::dir::{destack_fs_rmdir_bytes, destack_fs_rmdir_utf16};
use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{
    AtFlags, DirectoryHandle, FileMode, NodeDevice, OsPath, PathBytes, PathBytesAbi, PathUtf16,
    RenameFlags, SymlinkType, core as core_fs,
};
use crate::runtime::BindingCallContext;

/// Rename flag to disallow replacing existing entries.
const RENAME_NOREPLACE: u32 = 0x1;
/// Rename flag to exchange paths.
const RENAME_EXCHANGE: u32 = 0x2;
/// Rename flag to create a whiteout entry.
const RENAME_WHITEOUT: u32 = 0x4;

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
    _context: &BindingCallContext,
    from: PathBytes,
    to: PathBytes,
) -> RuntimeResult<()> {
    // decode the source and target paths
    let from = wide_from_bytes(from, "from")?;
    let to = wide_from_bytes(to, "to")?;

    // create the hard link
    let rc = unsafe { CreateHardLinkW(to.as_ptr(), from.as_ptr(), std::ptr::null_mut()) };
    if rc == 0 {
        return Err(last_os_error("CreateHardLinkW", None));
    }

    Ok(())
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
    _context: &BindingCallContext,
    from: PathUtf16,
    to: PathUtf16,
) -> RuntimeResult<()> {
    // decode the source and target paths
    let from = wide_from_utf16(from, "from")?;
    let to = wide_from_utf16(to, "to")?;

    // create the hard link
    let rc = unsafe { CreateHardLinkW(to.as_ptr(), from.as_ptr(), std::ptr::null_mut()) };
    if rc == 0 {
        return Err(last_os_error("CreateHardLinkW", None));
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
pub(crate) unsafe fn destack_fs_linkat_bytes(
    context: &BindingCallContext,
    existing_dir: DirectoryHandle,
    existing: PathBytes,
    new_dir: DirectoryHandle,
    new: PathBytes,
    _flags: AtFlags,
) -> RuntimeResult<()> {
    // decode the relative paths
    let existing_pathbuf = pathbuf_from_bytes(existing, "existing")?;
    let new_pathbuf = pathbuf_from_bytes(new, "new")?;
    // resolve relative paths to full paths
    let existing_path = if existing_pathbuf.is_absolute() {
        existing_pathbuf
    } else {
        let mut base = directory_path(context, existing_dir)?;
        base.push(existing_pathbuf);
        base
    };
    let new_path = if new_pathbuf.is_absolute() {
        new_pathbuf
    } else {
        let mut base = directory_path(context, new_dir)?;
        base.push(new_pathbuf);
        base
    };

    // re-encode the resolved paths
    let existing_bytes = bytes_from_pathbuf(&existing_path, "existing")?;
    let new_bytes = bytes_from_pathbuf(&new_path, "new")?;
    let existing = PathBytesAbi::<NativeAbi>(context.store_array(existing_bytes));
    let new = PathBytesAbi::<NativeAbi>(context.store_array(new_bytes));

    // create the hard link
    unsafe { destack_fs_link_bytes(context, existing, new) }
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
    context: &BindingCallContext,
    existing_dir: DirectoryHandle,
    existing: PathUtf16,
    new_dir: DirectoryHandle,
    new: PathUtf16,
    _flags: AtFlags,
) -> RuntimeResult<()> {
    // decode the relative paths
    let existing_pathbuf = pathbuf_from_utf16(existing, "existing")?;
    let new_pathbuf = pathbuf_from_utf16(new, "new")?;
    // resolve relative paths to full paths
    let existing_path = if existing_pathbuf.is_absolute() {
        existing_pathbuf
    } else {
        let mut base = directory_path(context, existing_dir)?;
        base.push(existing_pathbuf);
        base
    };
    let new_path = if new_pathbuf.is_absolute() {
        new_pathbuf
    } else {
        let mut base = directory_path(context, new_dir)?;
        base.push(new_pathbuf);
        base
    };

    // re-encode the resolved paths
    let existing = path_utf16_from_pathbuf(context, &existing_path);
    let new = path_utf16_from_pathbuf(context, &new_path);

    // create the hard link
    unsafe { destack_fs_link_utf16(context, existing, new) }
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
    _context: &BindingCallContext,
    target: PathBytes,
    path: PathBytes,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    // decode the target and link paths
    let target = pathbuf_from_bytes(target, "target")?;
    let path = pathbuf_from_bytes(path, "path")?;

    // build wide strings for the syscall
    let target_wide = wide_from_pathbuf(&target);
    let path_wide = wide_from_pathbuf(&path);

    // choose symlink flags
    let mut flags = SYMBOLIC_LINK_FLAG_ALLOW_UNPRIVILEGED_CREATE;
    let is_directory = match kind {
        SymlinkType::Auto => target.is_dir(),
        SymlinkType::File => false,
        SymlinkType::Directory => true,
    };
    if is_directory {
        flags |= SYMBOLIC_LINK_FLAG_DIRECTORY;
    }

    // create the symlink
    let rc = unsafe { CreateSymbolicLinkW(path_wide.as_ptr(), target_wide.as_ptr(), flags) };
    if rc == 0 {
        return Err(last_os_error("CreateSymbolicLinkW", None));
    }

    Ok(())
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
    _context: &BindingCallContext,
    target: PathUtf16,
    path: PathUtf16,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    // decode the target and link paths
    let target = pathbuf_from_utf16(target, "target")?;
    let path = pathbuf_from_utf16(path, "path")?;

    // build wide strings for the syscall
    let target_wide = wide_from_pathbuf(&target);
    let path_wide = wide_from_pathbuf(&path);

    // choose symlink flags
    let mut flags = SYMBOLIC_LINK_FLAG_ALLOW_UNPRIVILEGED_CREATE;
    let is_directory = match kind {
        SymlinkType::Auto => target.is_dir(),
        SymlinkType::File => false,
        SymlinkType::Directory => true,
    };
    if is_directory {
        flags |= SYMBOLIC_LINK_FLAG_DIRECTORY;
    }

    // create the symlink
    let rc = unsafe { CreateSymbolicLinkW(path_wide.as_ptr(), target_wide.as_ptr(), flags) };
    if rc == 0 {
        return Err(last_os_error("CreateSymbolicLinkW", None));
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
pub(crate) unsafe fn destack_fs_symlinkat_bytes(
    context: &BindingCallContext,
    target: PathBytes,
    dir: DirectoryHandle,
    path: PathBytes,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    // decode the relative path
    let pathbuf = pathbuf_from_bytes(path, "path")?;
    if pathbuf.is_absolute() {
        return unsafe { destack_fs_symlink_bytes(context, target, path, kind) };
    }

    // resolve the directory handle
    let mut base = directory_path(context, dir)?;
    base.push(pathbuf);

    // re-encode the resolved path
    let bytes = bytes_from_pathbuf(&base, "path")?;
    let path = PathBytesAbi::<NativeAbi>(context.store_array(bytes));

    // create the symlink
    unsafe { destack_fs_symlink_bytes(context, target, path, kind) }
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
    context: &BindingCallContext,
    target: PathUtf16,
    dir: DirectoryHandle,
    path: PathUtf16,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    // decode the relative path
    let pathbuf = pathbuf_from_utf16(path, "path")?;
    if pathbuf.is_absolute() {
        return unsafe { destack_fs_symlink_utf16(context, target, path, kind) };
    }

    // resolve the directory handle
    let mut base = directory_path(context, dir)?;
    base.push(pathbuf);

    // re-encode the resolved path
    let path = path_utf16_from_pathbuf(context, &base);

    // create the symlink
    unsafe { destack_fs_symlink_utf16(context, target, path, kind) }
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
    context: &BindingCallContext,
    out: *mut PathBytes,
    path: PathBytes,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // open the reparse point
    let wide = wide_from_bytes(path, "path")?;
    let handle = open_for_reparse(&wide)?;
    let result = readlink_from_handle(handle);
    unsafe {
        CloseHandle(handle);
    }

    // decode the link target
    let target = result?;
    let bytes = target.as_bytes().to_vec();

    // write the output
    unsafe {
        *out = PathBytesAbi::<NativeAbi>(context.store_array(bytes));
    }

    Ok(())
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
pub(crate) unsafe fn destack_fs_readlink_utf16(
    context: &BindingCallContext,
    out: *mut PathUtf16,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // open the reparse point
    let wide = wide_from_utf16(path, "path")?;
    let handle = open_for_reparse(&wide)?;
    let result = readlink_from_handle(handle);
    unsafe {
        CloseHandle(handle);
    }

    // decode the link target
    let target = result?;
    let wide: Vec<u16> = target.encode_utf16().collect();

    // write the output
    unsafe {
        *out = path_utf16_from_units(context, &wide);
    }

    Ok(())
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
    context: &BindingCallContext,
    out: *mut PathBytes,
    dir: DirectoryHandle,
    path: PathBytes,
) -> RuntimeResult<()> {
    // decode the relative path
    let pathbuf = pathbuf_from_bytes(path, "path")?;
    if pathbuf.is_absolute() {
        return unsafe { destack_fs_readlink_bytes(context, out, path) };
    }

    // resolve the directory handle
    let mut base = directory_path(context, dir)?;
    base.push(pathbuf);

    // re-encode the resolved path
    let bytes = bytes_from_pathbuf(&base, "path")?;
    let path = PathBytesAbi::<NativeAbi>(context.store_array(bytes));

    // read the symlink
    unsafe { destack_fs_readlink_bytes(context, out, path) }
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
pub(crate) unsafe fn destack_fs_readlinkat_utf16(
    context: &BindingCallContext,
    out: *mut PathUtf16,
    dir: DirectoryHandle,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // decode the relative path
    let pathbuf = pathbuf_from_utf16(path, "path")?;
    if pathbuf.is_absolute() {
        return unsafe { destack_fs_readlink_utf16(context, out, path) };
    }

    // resolve the directory handle
    let mut base = directory_path(context, dir)?;
    base.push(pathbuf);

    // re-encode the resolved path
    let path = path_utf16_from_pathbuf(context, &base);

    // read the symlink
    unsafe { destack_fs_readlink_utf16(context, out, path) }
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
    context: &BindingCallContext,
    out: *mut PathBytes,
    path: PathBytes,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // open a handle for resolving the final path
    let wide = wide_from_bytes(path, "path")?;
    let handle = open_for_metadata(&wide, true)?;

    // close the handle after resolution
    struct HandleGuard(HANDLE);
    impl Drop for HandleGuard {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    let _guard = HandleGuard(handle);

    // resolve the final path
    let wide = final_path_from_handle(handle)?;
    let resolved = string_from_wide(&wide, "path")?;
    let resolved = normalize_reparse_target(resolved);
    let bytes = resolved.as_bytes().to_vec();

    // write the output
    unsafe {
        *out = PathBytesAbi::<NativeAbi>(context.store_array(bytes));
    }

    Ok(())
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
pub(crate) unsafe fn destack_fs_realpath_utf16(
    context: &BindingCallContext,
    out: *mut PathUtf16,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // open a handle for resolving the final path
    let wide = wide_from_utf16(path, "path")?;
    let handle = open_for_metadata(&wide, true)?;

    // close the handle after resolution
    struct HandleGuard(HANDLE);
    impl Drop for HandleGuard {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    let _guard = HandleGuard(handle);

    // resolve the final path
    let wide = final_path_from_handle(handle)?;
    let resolved = string_from_wide(&wide, "path")?;
    let resolved = normalize_reparse_target(resolved);
    let wide: Vec<u16> = resolved.encode_utf16().collect();

    // write the output
    unsafe {
        *out = path_utf16_from_units(context, &wide);
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
pub(crate) unsafe fn destack_fs_rename_bytes(
    _context: &BindingCallContext,
    from: PathBytes,
    to: PathBytes,
) -> RuntimeResult<()> {
    // decode the paths
    let from = wide_from_bytes(from, "from")?;
    let to = wide_from_bytes(to, "to")?;

    // issue the rename
    let rc = unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), MOVEFILE_REPLACE_EXISTING) };
    if rc == 0 {
        return Err(last_os_error("MoveFileExW", None));
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
    _context: &BindingCallContext,
    from: PathUtf16,
    to: PathUtf16,
) -> RuntimeResult<()> {
    // decode the paths
    let from = wide_from_utf16(from, "from")?;
    let to = wide_from_utf16(to, "to")?;

    // issue the rename
    let rc = unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), MOVEFILE_REPLACE_EXISTING) };
    if rc == 0 {
        return Err(last_os_error("MoveFileExW", None));
    }

    Ok(())
}

/// Rename a path with byte paths and explicit replace behavior.
fn rename_paths_with_flags(from: PathBytes, to: PathBytes, replace: bool) -> RuntimeResult<()> {
    // decode the paths
    let from = wide_from_bytes(from, "from")?;
    let to = wide_from_bytes(to, "to")?;

    // issue the rename
    let flags = if replace {
        MOVEFILE_REPLACE_EXISTING
    } else {
        0
    };
    let rc = unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), flags) };
    if rc == 0 {
        return Err(last_os_error("MoveFileExW", None));
    }

    Ok(())
}

/// Rename a path with UTF-16 paths and explicit replace behavior.
fn rename_paths_with_flags_utf16(
    from: PathUtf16,
    to: PathUtf16,
    replace: bool,
) -> RuntimeResult<()> {
    // decode the paths
    let from = wide_from_utf16(from, "from")?;
    let to = wide_from_utf16(to, "to")?;

    // issue the rename
    let flags = if replace {
        MOVEFILE_REPLACE_EXISTING
    } else {
        0
    };
    let rc = unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), flags) };
    if rc == 0 {
        return Err(last_os_error("MoveFileExW", None));
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
pub(crate) unsafe fn destack_fs_renameat_bytes(
    context: &BindingCallContext,
    from_dir: DirectoryHandle,
    from: PathBytes,
    to_dir: DirectoryHandle,
    to: PathBytes,
) -> RuntimeResult<()> {
    // decode the relative paths
    let from_pathbuf = pathbuf_from_bytes(from, "from")?;
    let to_pathbuf = pathbuf_from_bytes(to, "to")?;

    // fall back to the absolute path variant when needed
    if from_pathbuf.is_absolute() || to_pathbuf.is_absolute() {
        let from_path = if from_pathbuf.is_absolute() {
            from_pathbuf
        } else {
            let mut base = directory_path(context, from_dir)?;
            base.push(from_pathbuf);
            base
        };
        let to_path = if to_pathbuf.is_absolute() {
            to_pathbuf
        } else {
            let mut base = directory_path(context, to_dir)?;
            base.push(to_pathbuf);
            base
        };
        let from =
            PathBytesAbi::<NativeAbi>(context.store_array(bytes_from_pathbuf(&from_path, "from")?));
        let to =
            PathBytesAbi::<NativeAbi>(context.store_array(bytes_from_pathbuf(&to_path, "to")?));
        return unsafe { destack_fs_rename_bytes(context, from, to) };
    }

    // resolve the directory handles
    let from_root = directory_handle(context, from_dir)?;
    let to_root = directory_handle(context, to_dir)?;
    let from_path = wide_from_pathbuf_no_nul(&from_pathbuf);
    let to_path = wide_from_pathbuf_no_nul(&to_pathbuf);

    // open the existing entry
    let handle = nt_create_file_at(
        from_root,
        &from_path,
        DELETE_ACCESS,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        FILE_OPEN,
        FILE_OPEN_REPARSE_POINT,
        0,
    )?;

    // issue the rename and close the handle
    let result = set_rename_info(handle, to_root, &to_path);
    unsafe {
        CloseHandle(handle);
    }

    result
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
    context: &BindingCallContext,
    from_dir: DirectoryHandle,
    from: PathUtf16,
    to_dir: DirectoryHandle,
    to: PathUtf16,
) -> RuntimeResult<()> {
    // decode the relative paths
    let from_pathbuf = pathbuf_from_utf16(from, "from")?;
    let to_pathbuf = pathbuf_from_utf16(to, "to")?;

    // fall back to the absolute path variant when needed
    if from_pathbuf.is_absolute() || to_pathbuf.is_absolute() {
        let from_path = if from_pathbuf.is_absolute() {
            from_pathbuf
        } else {
            let mut base = directory_path(context, from_dir)?;
            base.push(from_pathbuf);
            base
        };
        let to_path = if to_pathbuf.is_absolute() {
            to_pathbuf
        } else {
            let mut base = directory_path(context, to_dir)?;
            base.push(to_pathbuf);
            base
        };
        let from = path_utf16_from_pathbuf(context, &from_path);
        let to = path_utf16_from_pathbuf(context, &to_path);
        return unsafe { destack_fs_rename_utf16(context, from, to) };
    }

    // resolve the directory handles
    let from_root = directory_handle(context, from_dir)?;
    let to_root = directory_handle(context, to_dir)?;
    let from_path = wide_from_pathbuf_no_nul(&from_pathbuf);
    let to_path = wide_from_pathbuf_no_nul(&to_pathbuf);

    // open the existing entry
    let handle = nt_create_file_at(
        from_root,
        &from_path,
        DELETE_ACCESS,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        FILE_OPEN,
        FILE_OPEN_REPARSE_POINT,
        0,
    )?;

    // issue the rename and close the handle
    let result = set_rename_info(handle, to_root, &to_path);
    unsafe {
        CloseHandle(handle);
    }

    result
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
    context: &BindingCallContext,
    from_dir: DirectoryHandle,
    from: PathBytes,
    to_dir: DirectoryHandle,
    to: PathBytes,
    flags: RenameFlags,
) -> RuntimeResult<()> {
    // reject unsupported rename flags
    let unsupported = flags.0 & !(RENAME_NOREPLACE | RENAME_EXCHANGE | RENAME_WHITEOUT);
    if unsupported != 0 || flags.0 & (RENAME_EXCHANGE | RENAME_WHITEOUT) != 0 {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.fs.renameat2")).boxed(),
        );
    }
    let replace = flags.0 & RENAME_NOREPLACE == 0;

    // decode the relative paths
    let from_pathbuf = pathbuf_from_bytes(from, "from")?;
    let to_pathbuf = pathbuf_from_bytes(to, "to")?;

    // fall back to the absolute path variant when needed
    if from_pathbuf.is_absolute() || to_pathbuf.is_absolute() {
        let from_path = if from_pathbuf.is_absolute() {
            from_pathbuf
        } else {
            let mut base = directory_path(context, from_dir)?;
            base.push(from_pathbuf);
            base
        };
        let to_path = if to_pathbuf.is_absolute() {
            to_pathbuf
        } else {
            let mut base = directory_path(context, to_dir)?;
            base.push(to_pathbuf);
            base
        };
        let from =
            PathBytesAbi::<NativeAbi>(context.store_array(bytes_from_pathbuf(&from_path, "from")?));
        let to =
            PathBytesAbi::<NativeAbi>(context.store_array(bytes_from_pathbuf(&to_path, "to")?));
        return rename_paths_with_flags(from, to, replace);
    }

    // resolve the directory handles
    let from_root = directory_handle(context, from_dir)?;
    let to_root = directory_handle(context, to_dir)?;
    let from_path = wide_from_pathbuf_no_nul(&from_pathbuf);
    let to_path = wide_from_pathbuf_no_nul(&to_pathbuf);

    // open the existing entry
    let handle = nt_create_file_at(
        from_root,
        &from_path,
        DELETE_ACCESS,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        FILE_OPEN,
        FILE_OPEN_REPARSE_POINT,
        0,
    )?;

    // issue the rename and close the handle
    let result = set_rename_info_with_replace(handle, to_root, &to_path, replace);
    unsafe {
        CloseHandle(handle);
    }

    result
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
    context: &BindingCallContext,
    from_dir: DirectoryHandle,
    from: PathUtf16,
    to_dir: DirectoryHandle,
    to: PathUtf16,
    flags: RenameFlags,
) -> RuntimeResult<()> {
    // reject unsupported rename flags
    let unsupported = flags.0 & !(RENAME_NOREPLACE | RENAME_EXCHANGE | RENAME_WHITEOUT);
    if unsupported != 0 || flags.0 & (RENAME_EXCHANGE | RENAME_WHITEOUT) != 0 {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.fs.renameat2")).boxed(),
        );
    }
    let replace = flags.0 & RENAME_NOREPLACE == 0;

    // decode the relative paths
    let from_pathbuf = pathbuf_from_utf16(from, "from")?;
    let to_pathbuf = pathbuf_from_utf16(to, "to")?;

    // fall back to the absolute path variant when needed
    if from_pathbuf.is_absolute() || to_pathbuf.is_absolute() {
        let from_path = if from_pathbuf.is_absolute() {
            from_pathbuf
        } else {
            let mut base = directory_path(context, from_dir)?;
            base.push(from_pathbuf);
            base
        };
        let to_path = if to_pathbuf.is_absolute() {
            to_pathbuf
        } else {
            let mut base = directory_path(context, to_dir)?;
            base.push(to_pathbuf);
            base
        };
        let from = path_utf16_from_pathbuf(context, &from_path);
        let to = path_utf16_from_pathbuf(context, &to_path);
        return rename_paths_with_flags_utf16(from, to, replace);
    }

    // resolve the directory handles
    let from_root = directory_handle(context, from_dir)?;
    let to_root = directory_handle(context, to_dir)?;
    let from_path = wide_from_pathbuf_no_nul(&from_pathbuf);
    let to_path = wide_from_pathbuf_no_nul(&to_pathbuf);

    // open the existing entry
    let handle = nt_create_file_at(
        from_root,
        &from_path,
        DELETE_ACCESS,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        FILE_OPEN,
        FILE_OPEN_REPARSE_POINT,
        0,
    )?;

    // issue the rename and close the handle
    let result = set_rename_info_with_replace(handle, to_root, &to_path, replace);
    unsafe {
        CloseHandle(handle);
    }

    result
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
    _context: &BindingCallContext,
    path: PathBytes,
) -> RuntimeResult<()> {
    // decode the path
    let wide = wide_from_bytes(path, "path")?;

    // open the file for delete and set disposition
    let handle = open_for_delete_path(&wide)?;
    let result = set_disposition_info(handle);
    unsafe {
        CloseHandle(handle);
    }

    result
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
    _context: &BindingCallContext,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // decode the path
    let wide = wide_from_utf16(path, "path")?;

    // open the file for delete and set disposition
    let handle = open_for_delete_path(&wide)?;
    let result = set_disposition_info(handle);
    unsafe {
        CloseHandle(handle);
    }

    result
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
    context: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // decode the relative path
    let pathbuf = pathbuf_from_bytes(path, "path")?;
    if pathbuf.is_absolute() {
        let remove_dir = flags.0 & AT_REMOVEDIR != 0;
        if remove_dir {
            return unsafe { destack_fs_rmdir_bytes(context, path) };
        }
        return unsafe { destack_fs_unlink_bytes(context, path) };
    }

    // resolve the directory handle
    let root = directory_handle(context, dir)?;
    let path = wide_from_pathbuf_no_nul(&pathbuf);
    let remove_dir = flags.0 & AT_REMOVEDIR != 0;
    let mut options = if remove_dir {
        FILE_DIRECTORY_FILE
    } else {
        FILE_NON_DIRECTORY_FILE
    };
    options |= FILE_OPEN_REPARSE_POINT;

    // open the path and set disposition
    let handle = nt_create_file_at(
        root,
        &path,
        DELETE_ACCESS,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        FILE_OPEN,
        options,
        0,
    )?;
    let result = set_disposition_info(handle);
    unsafe {
        CloseHandle(handle);
    }

    result
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
    context: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // decode the relative path
    let pathbuf = pathbuf_from_utf16(path, "path")?;
    if pathbuf.is_absolute() {
        let remove_dir = flags.0 & AT_REMOVEDIR != 0;
        if remove_dir {
            return unsafe { destack_fs_rmdir_utf16(context, path) };
        }
        return unsafe { destack_fs_unlink_utf16(context, path) };
    }

    // resolve the directory handle
    let root = directory_handle(context, dir)?;
    let path = wide_from_pathbuf_no_nul(&pathbuf);
    let remove_dir = flags.0 & AT_REMOVEDIR != 0;
    let mut options = if remove_dir {
        FILE_DIRECTORY_FILE
    } else {
        FILE_NON_DIRECTORY_FILE
    };
    options |= FILE_OPEN_REPARSE_POINT;

    // open the path and set disposition
    let handle = nt_create_file_at(
        root,
        &path,
        DELETE_ACCESS,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        FILE_OPEN,
        options,
        0,
    )?;
    let result = set_disposition_info(handle);
    unsafe {
        CloseHandle(handle);
    }

    result
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
    context: &BindingCallContext,
    from: OsPath,
    to: OsPath,
) -> RuntimeResult<()> {
    core_fs::with_path_ref_pair(
        from,
        to,
        "path",
        |from, to| unsafe { destack_fs_rename_bytes(context, from, to) },
        |from, to| unsafe { destack_fs_rename_utf16(context, from, to) },
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
    context: &BindingCallContext,
    from_dir: DirectoryHandle,
    from: OsPath,
    to_dir: DirectoryHandle,
    to: OsPath,
) -> RuntimeResult<()> {
    core_fs::with_path_ref_pair(
        from,
        to,
        "path",
        |from, to| unsafe { destack_fs_renameat_bytes(context, from_dir, from, to_dir, to) },
        |from, to| unsafe { destack_fs_renameat_utf16(context, from_dir, from, to_dir, to) },
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
    context: &BindingCallContext,
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
            destack_fs_renameat2_bytes(context, from_dir, from, to_dir, to, flags)
        },
        |from, to| unsafe {
            destack_fs_renameat2_utf16(context, from_dir, from, to_dir, to, flags)
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
    context: &BindingCallContext,
    path: OsPath,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_unlink_bytes(context, path) },
        |path| unsafe { destack_fs_unlink_utf16(context, path) },
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
    context: &BindingCallContext,
    dir: DirectoryHandle,
    path: OsPath,
    flags: AtFlags,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_unlinkat_bytes(context, dir, path, flags) },
        |path| unsafe { destack_fs_unlinkat_utf16(context, dir, path, flags) },
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
    context: &BindingCallContext,
    existing_path: OsPath,
    new_path: OsPath,
) -> RuntimeResult<()> {
    core_fs::with_path_ref_pair(
        existing_path,
        new_path,
        "path",
        |existing_path, new_path| unsafe {
            destack_fs_link_bytes(context, existing_path, new_path)
        },
        |existing_path, new_path| unsafe {
            destack_fs_link_utf16(context, existing_path, new_path)
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
    context: &BindingCallContext,
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
                context,
                existing_dir,
                existing_path,
                new_dir,
                new_path,
                flags,
            )
        },
        |existing_path, new_path| unsafe {
            destack_fs_linkat_utf16(
                context,
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
    context: &BindingCallContext,
    target: OsPath,
    path: OsPath,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    core_fs::with_path_ref_pair(
        target,
        path,
        "path",
        |target, path| unsafe { destack_fs_symlink_bytes(context, target, path, kind) },
        |target, path| unsafe { destack_fs_symlink_utf16(context, target, path, kind) },
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
    context: &BindingCallContext,
    target: OsPath,
    dir: DirectoryHandle,
    path: OsPath,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    core_fs::with_path_ref_pair(
        target,
        path,
        "path",
        |target, path| unsafe { destack_fs_symlinkat_bytes(context, target, dir, path, kind) },
        |target, path| unsafe { destack_fs_symlinkat_utf16(context, target, dir, path, kind) },
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
    context: &BindingCallContext,
    out: *mut OsPath,
    path: OsPath,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    #[cfg(unix)]
    {
        match path.encoding {
            PathEncoding::Bytes => {
                let mut inner = core_fs::empty_path_bytes();
                unsafe { destack_fs_readlink_bytes(context, &mut inner, path.bytes) }?;
                unsafe {
                    *out = core_fs::path_ref_from_bytes(inner);
                }
                Ok(())
            }
            PathEncoding::Utf16 => {
                let bytes = core_fs::with_utf16_as_bytes(path.utf16, "path", |path| {
                    let mut inner = core_fs::empty_path_bytes();
                    unsafe { destack_fs_readlink_bytes(context, &mut inner, path) }?;
                    Ok(inner)
                })?;
                let utf16 = core_fs::path_utf16_from_bytes(context, bytes, "path")?;
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
            unsafe { destack_fs_readlink_bytes(context, &mut inner, path) }?;
            unsafe {
                *out = core_fs::path_ref_from_bytes(inner);
            }
            Ok(())
        },
        |path| {
            let mut inner = core_fs::empty_path_utf16();
            unsafe { destack_fs_readlink_utf16(context, &mut inner, path) }?;
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
    context: &BindingCallContext,
    out: *mut OsPath,
    dir: DirectoryHandle,
    path: OsPath,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    #[cfg(unix)]
    {
        match path.encoding {
            PathEncoding::Bytes => {
                let mut inner = core_fs::empty_path_bytes();
                unsafe { destack_fs_readlinkat_bytes(context, &mut inner, dir, path.bytes) }?;
                unsafe {
                    *out = core_fs::path_ref_from_bytes(inner);
                }
                Ok(())
            }
            PathEncoding::Utf16 => {
                let bytes = core_fs::with_utf16_as_bytes(path.utf16, "path", |path| {
                    let mut inner = core_fs::empty_path_bytes();
                    unsafe { destack_fs_readlinkat_bytes(context, &mut inner, dir, path) }?;
                    Ok(inner)
                })?;
                let utf16 = core_fs::path_utf16_from_bytes(context, bytes, "path")?;
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
            unsafe { destack_fs_readlinkat_bytes(context, &mut inner, dir, path) }?;
            unsafe {
                *out = core_fs::path_ref_from_bytes(inner);
            }
            Ok(())
        },
        |path| {
            let mut inner = core_fs::empty_path_utf16();
            unsafe { destack_fs_readlinkat_utf16(context, &mut inner, dir, path) }?;
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
    context: &BindingCallContext,
    out: *mut OsPath,
    path: OsPath,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    #[cfg(unix)]
    {
        match path.encoding {
            PathEncoding::Bytes => {
                let mut inner = core_fs::empty_path_bytes();
                unsafe { destack_fs_realpath_bytes(context, &mut inner, path.bytes) }?;
                unsafe {
                    *out = core_fs::path_ref_from_bytes(inner);
                }
                Ok(())
            }
            PathEncoding::Utf16 => {
                let bytes = core_fs::with_utf16_as_bytes(path.utf16, "path", |path| {
                    let mut inner = core_fs::empty_path_bytes();
                    unsafe { destack_fs_realpath_bytes(context, &mut inner, path) }?;
                    Ok(inner)
                })?;
                let utf16 = core_fs::path_utf16_from_bytes(context, bytes, "path")?;
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
            unsafe { destack_fs_realpath_bytes(context, &mut inner, path) }?;
            unsafe {
                *out = core_fs::path_ref_from_bytes(inner);
            }
            Ok(())
        },
        |path| {
            let mut inner = core_fs::empty_path_utf16();
            unsafe { destack_fs_realpath_utf16(context, &mut inner, path) }?;
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
    _context: &BindingCallContext,
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
                let _ = (_context, mode, path);
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
    context: &BindingCallContext,
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
                let directory_fd = directory_descriptor(context, dir)?;
                let path = path_bytes_to_cstring(path, "path")?;
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
                let _ = (context, dir, path, mode);
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
    _context: &BindingCallContext,
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
                let _ = (_context, mode, device, path);
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
    context: &BindingCallContext,
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
                let directory_fd = directory_descriptor(context, dir)?;
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
                let _ = (context, dir, mode, device, path);
                Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mknodat")).boxed())
            }
        },
        |_path| Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mknodat")).boxed()),
    )
}
