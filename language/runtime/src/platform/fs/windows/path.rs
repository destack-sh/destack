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
    AtFlags, DirectoryHandle, PathBytes, PathBytesAbi, PathUtf16, RenameFlags, SymlinkType,
};
use crate::runtime::RuntimeCallContext;

/// Rename flag to disallow replacing existing entries.
const RENAME_NOREPLACE: u32 = 0x1;
/// Rename flag to exchange paths.
const RENAME_EXCHANGE: u32 = 0x2;
/// Rename flag to create a whiteout entry.
const RENAME_WHITEOUT: u32 = 0x4;

/// Create a hard link with byte paths.
pub(crate) unsafe fn destack_fs_link_bytes(
    _context: &RuntimeCallContext,
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

/// Create a hard link with UTF-16 paths.
pub(crate) unsafe fn destack_fs_link_utf16(
    _context: &RuntimeCallContext,
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

/// Create a hard link relative to directory handles with byte paths.
pub(crate) unsafe fn destack_fs_linkat_bytes(
    context: &RuntimeCallContext,
    existing_dir: DirectoryHandle,
    existing: PathBytes,
    new_dir: DirectoryHandle,
    new: PathBytes,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // decode the relative paths
    let existing_pathbuf = pathbuf_from_bytes(existing, "existing")?;
    let new_pathbuf = pathbuf_from_bytes(new, "new")?;
    let _ = flags;

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

/// Create a hard link relative to directory handles with UTF-16 paths.
pub(crate) unsafe fn destack_fs_linkat_utf16(
    context: &RuntimeCallContext,
    existing_dir: DirectoryHandle,
    existing: PathUtf16,
    new_dir: DirectoryHandle,
    new: PathUtf16,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // decode the relative paths
    let existing_pathbuf = pathbuf_from_utf16(existing, "existing")?;
    let new_pathbuf = pathbuf_from_utf16(new, "new")?;
    let _ = flags;

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

/// Create a symlink with byte paths.
pub(crate) unsafe fn destack_fs_symlink_bytes(
    _context: &RuntimeCallContext,
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

/// Create a symlink with UTF-16 paths.
pub(crate) unsafe fn destack_fs_symlink_utf16(
    _context: &RuntimeCallContext,
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

/// Create a symlink relative to a directory handle with byte paths.
pub(crate) unsafe fn destack_fs_symlinkat_bytes(
    context: &RuntimeCallContext,
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

/// Create a symlink relative to a directory handle with UTF-16 paths.
pub(crate) unsafe fn destack_fs_symlinkat_utf16(
    context: &RuntimeCallContext,
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

/// Read a symlink with byte paths.
pub(crate) unsafe fn destack_fs_readlink_bytes(
    context: &RuntimeCallContext,
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

/// Read a symlink with UTF-16 paths.
pub(crate) unsafe fn destack_fs_readlink_utf16(
    context: &RuntimeCallContext,
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

/// Read a symlink relative to a directory handle with byte paths.
pub(crate) unsafe fn destack_fs_readlinkat_bytes(
    context: &RuntimeCallContext,
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

/// Read a symlink relative to a directory handle with UTF-16 paths.
pub(crate) unsafe fn destack_fs_readlinkat_utf16(
    context: &RuntimeCallContext,
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

/// Resolve a path to its canonical form with byte paths.
pub(crate) unsafe fn destack_fs_realpath_bytes(
    context: &RuntimeCallContext,
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

/// Resolve a path to its canonical form with UTF-16 paths.
pub(crate) unsafe fn destack_fs_realpath_utf16(
    context: &RuntimeCallContext,
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

/// Rename a path with byte paths.
pub(crate) unsafe fn destack_fs_rename_bytes(
    _context: &RuntimeCallContext,
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

/// Rename a path with UTF-16 paths.
pub(crate) unsafe fn destack_fs_rename_utf16(
    _context: &RuntimeCallContext,
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

/// Rename a path relative to a directory handle with byte paths.
pub(crate) unsafe fn destack_fs_renameat_bytes(
    context: &RuntimeCallContext,
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

/// Rename a path relative to a directory handle with UTF-16 paths.
pub(crate) unsafe fn destack_fs_renameat_utf16(
    context: &RuntimeCallContext,
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

/// Rename a path relative to a directory handle with renameat2 semantics and byte paths.
pub(crate) unsafe fn destack_fs_renameat2_bytes(
    context: &RuntimeCallContext,
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

/// Rename a path relative to a directory handle with renameat2 semantics and UTF-16 paths.
pub(crate) unsafe fn destack_fs_renameat2_utf16(
    context: &RuntimeCallContext,
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

/// Unlink a path with byte paths.
pub(crate) unsafe fn destack_fs_unlink_bytes(
    _context: &RuntimeCallContext,
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

/// Unlink a path with UTF-16 paths.
pub(crate) unsafe fn destack_fs_unlink_utf16(
    _context: &RuntimeCallContext,
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

/// Unlink a path relative to a directory handle with byte paths.
pub(crate) unsafe fn destack_fs_unlinkat_bytes(
    context: &RuntimeCallContext,
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

/// Unlink a path relative to a directory handle with UTF-16 paths.
pub(crate) unsafe fn destack_fs_unlinkat_utf16(
    context: &RuntimeCallContext,
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
