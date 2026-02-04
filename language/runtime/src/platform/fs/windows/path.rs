use std::os::windows::ffi::OsStrExt;

use windows_sys::Wdk::Storage::FileSystem::{
    FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE, FILE_OPEN, FILE_OPEN_REPARSE_POINT,
};
use windows_sys::Win32::Foundation::CloseHandle;
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
    AtFlags, DirectoryHandle, PathBytes, PathBytesAbi, PathUtf16, PathUtf16Abi, SymlinkType,
};
use crate::runtime::RuntimeCallContext;

/// Create a hard link with byte paths.
pub(crate) unsafe fn destack_fs_link_bytes(
    _context: &RuntimeCallContext,
    from: PathBytes,
    to: PathBytes,
) -> RuntimeResult<()> {
    let from = wide_from_bytes(from, "from")?;
    let to = wide_from_bytes(to, "to")?;
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
    let from = wide_from_utf16(from, "from")?;
    let to = wide_from_utf16(to, "to")?;
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
    let existing_pathbuf = pathbuf_from_bytes(existing, "existing")?;
    let new_pathbuf = pathbuf_from_bytes(new, "new")?;
    let _ = flags;
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
    let existing_bytes = bytes_from_pathbuf(&existing_path, "existing")?;
    let new_bytes = bytes_from_pathbuf(&new_path, "new")?;
    let existing = PathBytesAbi::<NativeAbi>(context.store_array(existing_bytes));
    let new = PathBytesAbi::<NativeAbi>(context.store_array(new_bytes));
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
    let existing_pathbuf = pathbuf_from_utf16(existing, "existing")?;
    let new_pathbuf = pathbuf_from_utf16(new, "new")?;
    let _ = flags;
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
    let existing = PathUtf16Abi::<NativeAbi>(
        context.store_array(existing_path.as_os_str().encode_wide().collect()),
    );
    let new = PathUtf16Abi::<NativeAbi>(
        context.store_array(new_path.as_os_str().encode_wide().collect()),
    );
    unsafe { destack_fs_link_utf16(context, existing, new) }
}

/// Create a symlink with byte paths.
pub(crate) unsafe fn destack_fs_symlink_bytes(
    _context: &RuntimeCallContext,
    target: PathBytes,
    path: PathBytes,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let target = pathbuf_from_bytes(target, "target")?;
    let path = pathbuf_from_bytes(path, "path")?;
    let target_wide = wide_from_pathbuf(&target);
    let path_wide = wide_from_pathbuf(&path);
    let mut flags = SYMBOLIC_LINK_FLAG_ALLOW_UNPRIVILEGED_CREATE;
    let is_directory = match kind {
        SymlinkType::Auto => target.is_dir(),
        SymlinkType::File => false,
        SymlinkType::Directory => true,
    };
    if is_directory {
        flags |= SYMBOLIC_LINK_FLAG_DIRECTORY;
    }
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
    let target = pathbuf_from_utf16(target, "target")?;
    let path = pathbuf_from_utf16(path, "path")?;
    let target_wide = wide_from_pathbuf(&target);
    let path_wide = wide_from_pathbuf(&path);
    let mut flags = SYMBOLIC_LINK_FLAG_ALLOW_UNPRIVILEGED_CREATE;
    let is_directory = match kind {
        SymlinkType::Auto => target.is_dir(),
        SymlinkType::File => false,
        SymlinkType::Directory => true,
    };
    if is_directory {
        flags |= SYMBOLIC_LINK_FLAG_DIRECTORY;
    }
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
    let pathbuf = pathbuf_from_bytes(path, "path")?;
    if pathbuf.is_absolute() {
        return unsafe { destack_fs_symlink_bytes(context, target, path, kind) };
    }
    let mut base = directory_path(context, dir)?;
    base.push(pathbuf);
    let bytes = bytes_from_pathbuf(&base, "path")?;
    let path = PathBytesAbi::<NativeAbi>(context.store_array(bytes));
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
    let pathbuf = pathbuf_from_utf16(path, "path")?;
    if pathbuf.is_absolute() {
        return unsafe { destack_fs_symlink_utf16(context, target, path, kind) };
    }
    let mut base = directory_path(context, dir)?;
    base.push(pathbuf);
    let wide: Vec<u16> = base.as_os_str().encode_wide().collect();
    let path = PathUtf16Abi::<NativeAbi>(context.store_array(wide));
    unsafe { destack_fs_symlink_utf16(context, target, path, kind) }
}

/// Read a symlink with byte paths.
pub(crate) unsafe fn destack_fs_readlink_bytes(
    context: &RuntimeCallContext,
    out: *mut PathBytes,
    path: PathBytes,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let wide = wide_from_bytes(path, "path")?;
    let handle = open_for_reparse(&wide)?;
    let result = readlink_from_handle(handle);
    unsafe {
        CloseHandle(handle);
    }
    let target = result?;
    let bytes = target.as_bytes().to_vec();
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
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let wide = wide_from_utf16(path, "path")?;
    let handle = open_for_reparse(&wide)?;
    let result = readlink_from_handle(handle);
    unsafe {
        CloseHandle(handle);
    }
    let target = result?;
    let wide: Vec<u16> = target.encode_utf16().collect();
    unsafe {
        *out = PathUtf16Abi::<NativeAbi>(context.store_array(wide));
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
    let pathbuf = pathbuf_from_bytes(path, "path")?;
    if pathbuf.is_absolute() {
        return unsafe { destack_fs_readlink_bytes(context, out, path) };
    }
    let mut base = directory_path(context, dir)?;
    base.push(pathbuf);
    let bytes = bytes_from_pathbuf(&base, "path")?;
    let path = PathBytesAbi::<NativeAbi>(context.store_array(bytes));
    unsafe { destack_fs_readlink_bytes(context, out, path) }
}

/// Read a symlink relative to a directory handle with UTF-16 paths.
pub(crate) unsafe fn destack_fs_readlinkat_utf16(
    context: &RuntimeCallContext,
    out: *mut PathUtf16,
    dir: DirectoryHandle,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let pathbuf = pathbuf_from_utf16(path, "path")?;
    if pathbuf.is_absolute() {
        return unsafe { destack_fs_readlink_utf16(context, out, path) };
    }
    let mut base = directory_path(context, dir)?;
    base.push(pathbuf);
    let wide: Vec<u16> = base.as_os_str().encode_wide().collect();
    let path = PathUtf16Abi::<NativeAbi>(context.store_array(wide));
    unsafe { destack_fs_readlink_utf16(context, out, path) }
}

/// Resolve a path to its canonical form with byte paths.
pub(crate) unsafe fn destack_fs_realpath_bytes(
    context: &RuntimeCallContext,
    out: *mut PathBytes,
    path: PathBytes,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let path = pathbuf_from_bytes(path, "path")?;
    let resolved = std::fs::canonicalize(&path).map_err(|error| {
        RuntimeError::from(PlatformError::io(format!(
            "failed to resolve path: {error}"
        )))
        .boxed()
    })?;
    let bytes = bytes_from_pathbuf(&resolved, "path")?;
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
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let path = pathbuf_from_utf16(path, "path")?;
    let resolved = std::fs::canonicalize(&path).map_err(|error| {
        RuntimeError::from(PlatformError::io(format!(
            "failed to resolve path: {error}"
        )))
        .boxed()
    })?;
    let wide: Vec<u16> = resolved.as_os_str().encode_wide().collect();
    unsafe {
        *out = PathUtf16Abi::<NativeAbi>(context.store_array(wide));
    }
    Ok(())
}

/// Rename a path with byte paths.
pub(crate) unsafe fn destack_fs_rename_bytes(
    _context: &RuntimeCallContext,
    from: PathBytes,
    to: PathBytes,
) -> RuntimeResult<()> {
    let from = wide_from_bytes(from, "from")?;
    let to = wide_from_bytes(to, "to")?;
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
    let from = wide_from_utf16(from, "from")?;
    let to = wide_from_utf16(to, "to")?;
    let rc = unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), MOVEFILE_REPLACE_EXISTING) };
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
    let from_pathbuf = pathbuf_from_bytes(from, "from")?;
    let to_pathbuf = pathbuf_from_bytes(to, "to")?;
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

    let from_root = directory_handle(context, from_dir)?;
    let to_root = directory_handle(context, to_dir)?;
    let from_path = wide_from_pathbuf_no_nul(&from_pathbuf);
    let to_path = wide_from_pathbuf_no_nul(&to_pathbuf);

    let handle = nt_create_file_at(
        from_root,
        &from_path,
        DELETE_ACCESS,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        FILE_OPEN,
        FILE_OPEN_REPARSE_POINT,
        0,
    )?;

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
    let from_pathbuf = pathbuf_from_utf16(from, "from")?;
    let to_pathbuf = pathbuf_from_utf16(to, "to")?;
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
        let from = PathUtf16Abi::<NativeAbi>(
            context.store_array(from_path.as_os_str().encode_wide().collect()),
        );
        let to = PathUtf16Abi::<NativeAbi>(
            context.store_array(to_path.as_os_str().encode_wide().collect()),
        );
        return unsafe { destack_fs_rename_utf16(context, from, to) };
    }

    let from_root = directory_handle(context, from_dir)?;
    let to_root = directory_handle(context, to_dir)?;
    let from_path = wide_from_pathbuf_no_nul(&from_pathbuf);
    let to_path = wide_from_pathbuf_no_nul(&to_pathbuf);

    let handle = nt_create_file_at(
        from_root,
        &from_path,
        DELETE_ACCESS,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        FILE_OPEN,
        FILE_OPEN_REPARSE_POINT,
        0,
    )?;

    let result = set_rename_info(handle, to_root, &to_path);
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
    let wide = wide_from_bytes(path, "path")?;
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
    let wide = wide_from_utf16(path, "path")?;
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
    let pathbuf = pathbuf_from_bytes(path, "path")?;
    if pathbuf.is_absolute() {
        let remove_dir = flags.0 & AT_REMOVEDIR != 0;
        if remove_dir {
            return unsafe { destack_fs_rmdir_bytes(context, path) };
        }
        return unsafe { destack_fs_unlink_bytes(context, path) };
    }

    let root = directory_handle(context, dir)?;
    let path = wide_from_pathbuf_no_nul(&pathbuf);
    let remove_dir = flags.0 & AT_REMOVEDIR != 0;
    let mut options = if remove_dir {
        FILE_DIRECTORY_FILE
    } else {
        FILE_NON_DIRECTORY_FILE
    };
    options |= FILE_OPEN_REPARSE_POINT;
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
    let pathbuf = pathbuf_from_utf16(path, "path")?;
    if pathbuf.is_absolute() {
        let remove_dir = flags.0 & AT_REMOVEDIR != 0;
        if remove_dir {
            return unsafe { destack_fs_rmdir_utf16(context, path) };
        }
        return unsafe { destack_fs_unlink_utf16(context, path) };
    }

    let root = directory_handle(context, dir)?;
    let path = wide_from_pathbuf_no_nul(&pathbuf);
    let remove_dir = flags.0 & AT_REMOVEDIR != 0;
    let mut options = if remove_dir {
        FILE_DIRECTORY_FILE
    } else {
        FILE_NON_DIRECTORY_FILE
    };
    options |= FILE_OPEN_REPARSE_POINT;
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
