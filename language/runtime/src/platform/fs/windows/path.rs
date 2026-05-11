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
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{
    AtFlags, DirectoryHandle, FileMode, NodeDevice, OsPath, PathBytes, PathBytesAbi, PathUtf16,
    RenameFlags, SymlinkType, core as core_fs,
};
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;
use std::path::Path;

/// Rename flag to disallow replacing existing entries.
const RENAME_NOREPLACE: u32 = 0x1;
/// Rename flag to exchange paths.
const RENAME_EXCHANGE: u32 = 0x2;
/// Rename flag to create a whiteout entry.
const RENAME_WHITEOUT: u32 = 0x4;
/// Known renameat2 flag bits.
const RENAME_KNOWN_FLAGS: u32 = RENAME_NOREPLACE | RENAME_EXCHANGE | RENAME_WHITEOUT;
/// `linkat` flag bit for following a source symlink.
const LINKAT_FLAG_SYMLINK_FOLLOW: u32 = 0x400;

/// Resolve the Windows symlink-directory bit from one target payload and link path.
fn symlink_is_directory(target: &Path, path: &Path, kind: SymlinkType) -> bool {
    match kind {
        SymlinkType::Auto => {
            let inferred_target = if target.is_absolute() {
                target.to_path_buf()
            } else {
                match path.parent() {
                    Some(parent) => parent.join(target),
                    None => target.to_path_buf(),
                }
            };

            inferred_target.is_dir()
        }
        SymlinkType::File => false,
        SymlinkType::Directory => true,
    }
}

/// Validate one Windows renameat2 flag payload.
fn decode_windows_renameat2_replace(flags: RenameFlags) -> RuntimeResult<bool> {
    // reject unknown flag bits explicitly
    let unknown_bits = flags.0 & !RENAME_KNOWN_FLAGS;
    if unknown_bits != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            format!("unknown renameat2 flag bits: {unknown_bits:#x}"),
        ))
        .boxed());
    }

    // reject known but unsupported rename semantics
    if flags.0 & (RENAME_EXCHANGE | RENAME_WHITEOUT) != 0 {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.fs.path.renameat2")).boxed(),
        );
    }

    Ok(flags.0 & RENAME_NOREPLACE == 0)
}

/// Validate one Windows `linkat` flag payload.
fn validate_windows_linkat_flags(flags: AtFlags) -> RuntimeResult<()> {
    // reject unknown bits explicitly
    let unknown_bits = flags.0 & !LINKAT_FLAG_SYMLINK_FOLLOW;
    if unknown_bits != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            format!("unknown linkat flag bits: {unknown_bits:#x}"),
        ))
        .boxed());
    }

    // windows does not yet honor follow-symlink hard-link semantics
    if flags.0 & LINKAT_FLAG_SYMLINK_FOLLOW != 0 {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.fs.path.linkat")).boxed(),
        );
    }

    Ok(())
}

/// Create a hard link.
pub(crate) unsafe fn destack_fs_link_bytes(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_fs_link_utf16(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_fs_linkat_bytes(
    binding: &BindingCallContext,
    existing_dir: DirectoryHandle,
    existing: PathBytes,
    new_dir: DirectoryHandle,
    new: PathBytes,
    flags: AtFlags,
) -> RuntimeResult<()> {
    validate_windows_linkat_flags(flags)?;

    // decode the relative paths
    let existing_pathbuf = pathbuf_from_bytes(existing, "existing")?;
    let new_pathbuf = pathbuf_from_bytes(new, "new")?;
    // resolve relative paths to full paths
    let existing_path = if existing_pathbuf.is_absolute() {
        existing_pathbuf
    } else {
        let mut base = directory_path(binding, existing_dir)?;
        base.push(existing_pathbuf);
        base
    };
    let new_path = if new_pathbuf.is_absolute() {
        new_pathbuf
    } else {
        let mut base = directory_path(binding, new_dir)?;
        base.push(new_pathbuf);
        base
    };

    // re-encode the resolved paths
    let existing_bytes = bytes_from_pathbuf(&existing_path, "existing")?;
    let new_bytes = bytes_from_pathbuf(&new_path, "new")?;
    let existing = PathBytesAbi::<NativeAbi>(binding.store_array(existing_bytes));
    let new = PathBytesAbi::<NativeAbi>(binding.store_array(new_bytes));

    // create the hard link
    unsafe { destack_fs_link_bytes(binding, existing, new) }
}

/// Create a hard link relative to directory handles.
pub(crate) unsafe fn destack_fs_linkat_utf16(
    binding: &BindingCallContext,
    existing_dir: DirectoryHandle,
    existing: PathUtf16,
    new_dir: DirectoryHandle,
    new: PathUtf16,
    flags: AtFlags,
) -> RuntimeResult<()> {
    validate_windows_linkat_flags(flags)?;

    // decode the relative paths
    let existing_pathbuf = pathbuf_from_utf16(existing, "existing")?;
    let new_pathbuf = pathbuf_from_utf16(new, "new")?;
    // resolve relative paths to full paths
    let existing_path = if existing_pathbuf.is_absolute() {
        existing_pathbuf
    } else {
        let mut base = directory_path(binding, existing_dir)?;
        base.push(existing_pathbuf);
        base
    };
    let new_path = if new_pathbuf.is_absolute() {
        new_pathbuf
    } else {
        let mut base = directory_path(binding, new_dir)?;
        base.push(new_pathbuf);
        base
    };

    // re-encode the resolved paths
    let existing = path_utf16_from_pathbuf(binding, &existing_path);
    let new = path_utf16_from_pathbuf(binding, &new_path);

    // create the hard link
    unsafe { destack_fs_link_utf16(binding, existing, new) }
}

/// Create a symbolic link.
pub(crate) unsafe fn destack_fs_symlink_bytes(
    _binding: &BindingCallContext,
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
    let is_directory = symlink_is_directory(&target, &path, kind);
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
pub(crate) unsafe fn destack_fs_symlink_utf16(
    _binding: &BindingCallContext,
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
    let is_directory = symlink_is_directory(&target, &path, kind);
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
pub(crate) unsafe fn destack_fs_symlinkat_bytes(
    binding: &BindingCallContext,
    target: PathBytes,
    dir: DirectoryHandle,
    path: PathBytes,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    // decode the relative path
    let pathbuf = pathbuf_from_bytes(path, "path")?;
    if pathbuf.is_absolute() {
        return unsafe { destack_fs_symlink_bytes(binding, target, path, kind) };
    }

    // resolve the directory handle
    let mut base = directory_path(binding, dir)?;
    base.push(pathbuf);

    // re-encode the resolved path
    let bytes = bytes_from_pathbuf(&base, "path")?;
    let path = PathBytesAbi::<NativeAbi>(binding.store_array(bytes));

    // create the symlink
    unsafe { destack_fs_symlink_bytes(binding, target, path, kind) }
}

/// Create a symbolic link relative to a directory handle.
pub(crate) unsafe fn destack_fs_symlinkat_utf16(
    binding: &BindingCallContext,
    target: PathUtf16,
    dir: DirectoryHandle,
    path: PathUtf16,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    // decode the relative path
    let pathbuf = pathbuf_from_utf16(path, "path")?;
    if pathbuf.is_absolute() {
        return unsafe { destack_fs_symlink_utf16(binding, target, path, kind) };
    }

    // resolve the directory handle
    let mut base = directory_path(binding, dir)?;
    base.push(pathbuf);

    // re-encode the resolved path
    let path = path_utf16_from_pathbuf(binding, &base);

    // create the symlink
    unsafe { destack_fs_symlink_utf16(binding, target, path, kind) }
}

/// Read a symbolic link.
pub(crate) unsafe fn destack_fs_readlink_bytes(
    binding: &BindingCallContext,
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
        *out = PathBytesAbi::<NativeAbi>(binding.store_array(bytes));
    }

    Ok(())
}

/// Read a symbolic link.
pub(crate) unsafe fn destack_fs_readlink_utf16(
    binding: &BindingCallContext,
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
        *out = path_utf16_from_units(binding, &wide);
    }

    Ok(())
}

/// Read a symbolic link relative to a directory handle.
pub(crate) unsafe fn destack_fs_readlinkat_bytes(
    binding: &BindingCallContext,
    out: *mut PathBytes,
    dir: DirectoryHandle,
    path: PathBytes,
) -> RuntimeResult<()> {
    // decode the relative path
    let pathbuf = pathbuf_from_bytes(path, "path")?;
    if pathbuf.is_absolute() {
        return unsafe { destack_fs_readlink_bytes(binding, out, path) };
    }

    // resolve the directory handle
    let mut base = directory_path(binding, dir)?;
    base.push(pathbuf);

    // re-encode the resolved path
    let bytes = bytes_from_pathbuf(&base, "path")?;
    let path = PathBytesAbi::<NativeAbi>(binding.store_array(bytes));

    // read the symlink
    unsafe { destack_fs_readlink_bytes(binding, out, path) }
}

/// Read a symbolic link relative to a directory handle.
pub(crate) unsafe fn destack_fs_readlinkat_utf16(
    binding: &BindingCallContext,
    out: *mut PathUtf16,
    dir: DirectoryHandle,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // decode the relative path
    let pathbuf = pathbuf_from_utf16(path, "path")?;
    if pathbuf.is_absolute() {
        return unsafe { destack_fs_readlink_utf16(binding, out, path) };
    }

    // resolve the directory handle
    let mut base = directory_path(binding, dir)?;
    base.push(pathbuf);

    // re-encode the resolved path
    let path = path_utf16_from_pathbuf(binding, &base);

    // read the symlink
    unsafe { destack_fs_readlink_utf16(binding, out, path) }
}

/// Resolve a path to its canonical form.
pub(crate) unsafe fn destack_fs_realpath_bytes(
    binding: &BindingCallContext,
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
    let resolved = core_platform::string_from_wide("path", &wide)?;
    let resolved = normalize_reparse_target(resolved);
    let bytes = resolved.as_bytes().to_vec();

    // write the output
    unsafe {
        *out = PathBytesAbi::<NativeAbi>(binding.store_array(bytes));
    }

    Ok(())
}

/// Resolve a path to its canonical form.
pub(crate) unsafe fn destack_fs_realpath_utf16(
    binding: &BindingCallContext,
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
    let resolved = core_platform::string_from_wide("path", &wide)?;
    let resolved = normalize_reparse_target(resolved);
    let wide: Vec<u16> = resolved.encode_utf16().collect();

    // write the output
    unsafe {
        *out = path_utf16_from_units(binding, &wide);
    }

    Ok(())
}

/// Rename or move a file.
pub(crate) unsafe fn destack_fs_rename_bytes(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_fs_rename_utf16(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_fs_renameat_bytes(
    binding: &BindingCallContext,
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
            let mut base = directory_path(binding, from_dir)?;
            base.push(from_pathbuf);
            base
        };
        let to_path = if to_pathbuf.is_absolute() {
            to_pathbuf
        } else {
            let mut base = directory_path(binding, to_dir)?;
            base.push(to_pathbuf);
            base
        };
        let from =
            PathBytesAbi::<NativeAbi>(binding.store_array(bytes_from_pathbuf(&from_path, "from")?));
        let to =
            PathBytesAbi::<NativeAbi>(binding.store_array(bytes_from_pathbuf(&to_path, "to")?));
        return unsafe { destack_fs_rename_bytes(binding, from, to) };
    }

    // resolve the directory handles
    let from_root = directory_handle(binding, from_dir)?;
    let to_root = directory_handle(binding, to_dir)?;
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
pub(crate) unsafe fn destack_fs_renameat_utf16(
    binding: &BindingCallContext,
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
            let mut base = directory_path(binding, from_dir)?;
            base.push(from_pathbuf);
            base
        };
        let to_path = if to_pathbuf.is_absolute() {
            to_pathbuf
        } else {
            let mut base = directory_path(binding, to_dir)?;
            base.push(to_pathbuf);
            base
        };
        let from = path_utf16_from_pathbuf(binding, &from_path);
        let to = path_utf16_from_pathbuf(binding, &to_path);
        return unsafe { destack_fs_rename_utf16(binding, from, to) };
    }

    // resolve the directory handles
    let from_root = directory_handle(binding, from_dir)?;
    let to_root = directory_handle(binding, to_dir)?;
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
pub(crate) unsafe fn destack_fs_renameat2_bytes(
    binding: &BindingCallContext,
    from_dir: DirectoryHandle,
    from: PathBytes,
    to_dir: DirectoryHandle,
    to: PathBytes,
    flags: RenameFlags,
) -> RuntimeResult<()> {
    // validate rename flags up front
    let replace = decode_windows_renameat2_replace(flags)?;

    // decode the relative paths
    let from_pathbuf = pathbuf_from_bytes(from, "from")?;
    let to_pathbuf = pathbuf_from_bytes(to, "to")?;

    // fall back to the absolute path variant when needed
    if from_pathbuf.is_absolute() || to_pathbuf.is_absolute() {
        let from_path = if from_pathbuf.is_absolute() {
            from_pathbuf
        } else {
            let mut base = directory_path(binding, from_dir)?;
            base.push(from_pathbuf);
            base
        };
        let to_path = if to_pathbuf.is_absolute() {
            to_pathbuf
        } else {
            let mut base = directory_path(binding, to_dir)?;
            base.push(to_pathbuf);
            base
        };
        let from =
            PathBytesAbi::<NativeAbi>(binding.store_array(bytes_from_pathbuf(&from_path, "from")?));
        let to =
            PathBytesAbi::<NativeAbi>(binding.store_array(bytes_from_pathbuf(&to_path, "to")?));
        return rename_paths_with_flags(from, to, replace);
    }

    // resolve the directory handles
    let from_root = directory_handle(binding, from_dir)?;
    let to_root = directory_handle(binding, to_dir)?;
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
pub(crate) unsafe fn destack_fs_renameat2_utf16(
    binding: &BindingCallContext,
    from_dir: DirectoryHandle,
    from: PathUtf16,
    to_dir: DirectoryHandle,
    to: PathUtf16,
    flags: RenameFlags,
) -> RuntimeResult<()> {
    // validate rename flags up front
    let replace = decode_windows_renameat2_replace(flags)?;

    // decode the relative paths
    let from_pathbuf = pathbuf_from_utf16(from, "from")?;
    let to_pathbuf = pathbuf_from_utf16(to, "to")?;

    // fall back to the absolute path variant when needed
    if from_pathbuf.is_absolute() || to_pathbuf.is_absolute() {
        let from_path = if from_pathbuf.is_absolute() {
            from_pathbuf
        } else {
            let mut base = directory_path(binding, from_dir)?;
            base.push(from_pathbuf);
            base
        };
        let to_path = if to_pathbuf.is_absolute() {
            to_pathbuf
        } else {
            let mut base = directory_path(binding, to_dir)?;
            base.push(to_pathbuf);
            base
        };
        let from = path_utf16_from_pathbuf(binding, &from_path);
        let to = path_utf16_from_pathbuf(binding, &to_path);
        return rename_paths_with_flags_utf16(from, to, replace);
    }

    // resolve the directory handles
    let from_root = directory_handle(binding, from_dir)?;
    let to_root = directory_handle(binding, to_dir)?;
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
pub(crate) unsafe fn destack_fs_unlink_bytes(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_fs_unlink_utf16(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_fs_unlinkat_bytes(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // decode the relative path
    let pathbuf = pathbuf_from_bytes(path, "path")?;
    if pathbuf.is_absolute() {
        let remove_dir = flags.0 & AT_REMOVEDIR != 0;
        if remove_dir {
            return unsafe { destack_fs_rmdir_bytes(binding, path) };
        }
        return unsafe { destack_fs_unlink_bytes(binding, path) };
    }

    // resolve the directory handle
    let root = directory_handle(binding, dir)?;
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
pub(crate) unsafe fn destack_fs_unlinkat_utf16(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // decode the relative path
    let pathbuf = pathbuf_from_utf16(path, "path")?;
    if pathbuf.is_absolute() {
        let remove_dir = flags.0 & AT_REMOVEDIR != 0;
        if remove_dir {
            return unsafe { destack_fs_rmdir_utf16(binding, path) };
        }
        return unsafe { destack_fs_unlink_utf16(binding, path) };
    }

    // resolve the directory handle
    let root = directory_handle(binding, dir)?;
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
pub(crate) unsafe fn destack_fs_readlink(
    binding: &BindingCallContext,
    out: *mut OsPath,
    path: OsPath,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

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
pub(crate) unsafe fn destack_fs_readlinkat(
    binding: &BindingCallContext,
    out: *mut OsPath,
    dir: DirectoryHandle,
    path: OsPath,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

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
pub(crate) unsafe fn destack_fs_realpath(
    binding: &BindingCallContext,
    out: *mut OsPath,
    path: OsPath,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

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
pub(crate) unsafe fn destack_fs_mkfifo(
    binding: &BindingCallContext,
    path: OsPath,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (binding, path, mode);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.mkfifo")).boxed())
}

/// Create a FIFO special file relative to a directory handle.
pub(crate) unsafe fn destack_fs_mkfifoat(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: OsPath,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (binding, dir, path, mode);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.mkfifoat")).boxed())
}

/// Create a filesystem node.
pub(crate) unsafe fn destack_fs_mknod(
    binding: &BindingCallContext,
    path: OsPath,
    mode: FileMode,
    device: NodeDevice,
) -> RuntimeResult<()> {
    let _ = (binding, path, mode, device);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.mknod")).boxed())
}

/// Create a filesystem node relative to a directory handle.
pub(crate) unsafe fn destack_fs_mknodat(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: OsPath,
    mode: FileMode,
    device: NodeDevice,
) -> RuntimeResult<()> {
    let _ = (binding, dir, path, mode, device);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.mknodat")).boxed())
}
