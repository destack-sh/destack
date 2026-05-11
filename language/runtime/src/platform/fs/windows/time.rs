use windows_sys::Win32::Foundation::CloseHandle;

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{
    AtFlags, DirectoryHandle, OsPath, PathBytes, PathBytesAbi, PathUtf16, core as core_fs,
};
use crate::runtime::BindingCallContext;

/// Validate one `utimensat` flag payload on Windows.
fn validate_utimensat_flags(flags: AtFlags) -> RuntimeResult<bool> {
    // reject unknown flag bits explicitly
    let unknown_bits = flags.0 & !(AT_SYMLINK_NOFOLLOW | AT_REMOVEDIR);
    if unknown_bits != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            format!("unsupported destack.fs.utimensat flags: {unknown_bits:#x}"),
        ))
        .boxed());
    }

    // surface known but unsupported directory-removal semantics directly
    if flags.0 & AT_REMOVEDIR != 0 {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.utimensat")).boxed(),
        );
    }

    Ok(flags.0 & AT_SYMLINK_NOFOLLOW != 0)
}

/// Update access and modification times.
pub(crate) unsafe fn destack_fs_utimes_bytes(
    _binding: &BindingCallContext,
    path: PathBytes,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    // open the file with write-attributes access
    let wide = wide_from_bytes(path, "path")?;
    let handle = open_for_write_attributes(&wide, true)?;

    // update timestamps
    let result = set_handle_times(handle, atime_ns, mtime_ns);
    unsafe {
        CloseHandle(handle);
    }

    result
}

/// Update access and modification times.
pub(crate) unsafe fn destack_fs_utimes_utf16(
    _binding: &BindingCallContext,
    path: PathUtf16,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    // open the file with write-attributes access
    let wide = wide_from_utf16(path, "path")?;
    let handle = open_for_write_attributes(&wide, true)?;

    // update timestamps
    let result = set_handle_times(handle, atime_ns, mtime_ns);
    unsafe {
        CloseHandle(handle);
    }

    result
}

/// Update access and modification times without following symlinks.
pub(crate) unsafe fn destack_fs_lutimes_bytes(
    _binding: &BindingCallContext,
    path: PathBytes,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    // open the reparse point for write-attributes access
    let wide = wide_from_bytes(path, "path")?;
    let handle = open_for_write_attributes(&wide, false)?;

    // update timestamps
    let result = set_handle_times(handle, atime_ns, mtime_ns);
    unsafe {
        CloseHandle(handle);
    }

    result
}

/// Update access and modification times without following symlinks.
pub(crate) unsafe fn destack_fs_lutimes_utf16(
    _binding: &BindingCallContext,
    path: PathUtf16,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    // open the reparse point for write-attributes access
    let wide = wide_from_utf16(path, "path")?;
    let handle = open_for_write_attributes(&wide, false)?;

    // update timestamps
    let result = set_handle_times(handle, atime_ns, mtime_ns);
    unsafe {
        CloseHandle(handle);
    }

    result
}

/// Update access and modification times relative to a directory handle.
pub(crate) unsafe fn destack_fs_utimensat_bytes(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    atime_ns: u64,
    mtime_ns: u64,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let nofollow = validate_utimensat_flags(flags)?;

    // preserve nofollow semantics through the lutimes path
    if nofollow {
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

        return unsafe { destack_fs_lutimes_bytes(binding, path, atime_ns, mtime_ns) };
    }

    // resolve the path and delegate to utimes
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
    unsafe { destack_fs_utimes_bytes(binding, path, atime_ns, mtime_ns) }
}

/// Update access and modification times relative to a directory handle.
pub(crate) unsafe fn destack_fs_utimensat_utf16(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    atime_ns: u64,
    mtime_ns: u64,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let nofollow = validate_utimensat_flags(flags)?;

    // preserve nofollow semantics through the lutimes path
    if nofollow {
        let pathbuf = pathbuf_from_utf16(path, "path")?;
        let full_path = if pathbuf.is_absolute() {
            pathbuf
        } else {
            let mut base = directory_path(binding, dir)?;
            base.push(pathbuf);
            base
        };
        let path = path_utf16_from_pathbuf(binding, &full_path);

        return unsafe { destack_fs_lutimes_utf16(binding, path, atime_ns, mtime_ns) };
    }

    // resolve the path and delegate to utimes
    let pathbuf = pathbuf_from_utf16(path, "path")?;
    let full_path = if pathbuf.is_absolute() {
        pathbuf
    } else {
        let mut base = directory_path(binding, dir)?;
        base.push(pathbuf);
        base
    };
    let path = path_utf16_from_pathbuf(binding, &full_path);
    unsafe { destack_fs_utimes_utf16(binding, path, atime_ns, mtime_ns) }
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
