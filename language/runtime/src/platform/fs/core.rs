use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{
    AccessMode, AtFlags, CopyFlags, DirectoryHandle, DirentNext, FdFlags, FileHandle, FileMode,
    FileOffset, NodeDevice, OpenFlags, OpenOptions, OsPath, PathBytes, PathBytesAbi, PathEncoding,
    PathUtf16, PathUtf16Abi, RenameFlags, Stat, StatFs, StatusFlags, Statx, StatxFlags, StatxMask,
    SymlinkType, WatchBatch, WatchOptions, XattrFlags,
};
#[cfg(unix)]
use crate::platform::fs::{Dirent, DirentKind};
use crate::platform::resource::{ResourceEntry, ResourceKind, WatchHandle};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, PlatformError, ResourceId};
use crate::runtime::RuntimeCallContext;
#[cfg(unix)]
use std::ffi::CString;
#[cfg(unix)]
use std::os::unix::io::RawFd;

/// Resolve a file or directory handle to its resource entry.
#[cfg_attr(not(any(unix, windows)), allow(dead_code))]
pub(crate) fn require_resource<T>(
    context: &RuntimeCallContext,
    id: ResourceId,
    kind: ResourceKind,
    label: &str,
    with_entry: impl FnOnce(&ResourceEntry) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let resolved = context
        .runtime()
        .resources
        .with_entry(id, |entry| {
            if entry.kind != kind {
                return None;
            }
            Some(with_entry(entry))
        })
        .flatten();

    match resolved {
        Some(Ok(value)) => Ok(value),
        Some(Err(error)) => Err(error),
        None => Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            format!("unknown {label} handle"),
        ))
        .boxed()),
    }
}

/// Finalizer that closes a unix descriptor.
#[cfg(unix)]
#[derive(Debug)]
struct DescriptorFinalizer {
    /// The descriptor to close.
    fd: RawFd,
}

#[cfg(unix)]
impl crate::platform::resource::ResourceFinalizer for DescriptorFinalizer {
    /// Close the descriptor when the resource is finalized.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

/// Resolve a file handle into a unix descriptor.
#[cfg(unix)]
fn file_descriptor(context: &RuntimeCallContext, handle: FileHandle) -> RuntimeResult<RawFd> {
    require_resource(context, handle.0, ResourceKind::File, "file", |entry| {
        let Some(fd) = entry.fd() else {
            return Err(RuntimeError::Internal {
                message: "file payload missing descriptor".to_string(),
            }
            .boxed());
        };

        Ok(fd)
    })
}

/// Resolve a directory handle into a unix descriptor.
#[cfg(unix)]
fn directory_descriptor(
    context: &RuntimeCallContext,
    handle: DirectoryHandle,
) -> RuntimeResult<RawFd> {
    require_resource(
        context,
        handle.0,
        ResourceKind::Directory,
        "directory",
        |entry| {
            let Some(fd) = entry.fd() else {
                return Err(RuntimeError::Internal {
                    message: "directory payload missing descriptor".to_string(),
                }
                .boxed());
            };

            Ok(fd)
        },
    )
}

/// Resolve byte paths into C strings.
#[cfg(unix)]
fn path_bytes_to_cstring(path: PathBytes, label: &str) -> RuntimeResult<CString> {
    let bytes = unsafe { path.0.as_slice()? };
    CString::new(bytes).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "path contains nul byte",
        ))
        .boxed()
    })
}

/// Map unix dirent kind values to the ABI enum.
#[cfg(unix)]
fn dirent_kind_from_type(kind: libc::c_uchar) -> DirentKind {
    if kind == libc::DT_REG {
        return DirentKind::File;
    }
    if kind == libc::DT_DIR {
        return DirentKind::Directory;
    }
    if kind == libc::DT_LNK {
        return DirentKind::Symlink;
    }
    if kind == libc::DT_BLK {
        return DirentKind::BlockDevice;
    }
    if kind == libc::DT_CHR {
        return DirentKind::CharDevice;
    }
    if kind == libc::DT_FIFO {
        return DirentKind::Fifo;
    }
    if kind == libc::DT_SOCK {
        return DirentKind::Socket;
    }

    DirentKind::Unknown
}

/// Build an empty byte path payload.
pub(crate) fn empty_path_bytes() -> PathBytes {
    PathBytesAbi::<NativeAbi>(NativeArray {
        data: std::ptr::null_mut(),
        len: 0,
        capacity: 0,
    })
}

/// Build an empty UTF-16 path payload.
pub(crate) fn empty_path_utf16() -> PathUtf16 {
    PathUtf16Abi::<NativeAbi>(NativeArray {
        data: std::ptr::null_mut(),
        len: 0,
        capacity: 0,
    })
}

/// Build a OsPath from raw byte data.
pub(crate) fn path_ref_from_bytes(bytes: PathBytes) -> OsPath {
    OsPath {
        encoding: PathEncoding::Bytes,
        bytes,
        utf16: empty_path_utf16(),
    }
}

/// Build a OsPath from UTF-16 data.
pub(crate) fn path_ref_from_utf16(utf16: PathUtf16) -> OsPath {
    OsPath {
        encoding: PathEncoding::Utf16,
        bytes: empty_path_bytes(),
        utf16,
    }
}

/// Return a mismatch error for a pair of paths.
fn path_ref_mismatch<T>(label: &str) -> RuntimeResult<T> {
    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        label,
        "mismatched path encoding",
    ))
    .boxed())
}

/// Decode one UTF-16 byte payload into native code units.
#[cfg(unix)]
fn decode_utf16_bytes(path: PathUtf16, label: &str) -> RuntimeResult<Vec<u16>> {
    let units = unsafe { path.0.as_slice()? }.to_vec();
    let _ = label;

    Ok(units)
}

/// Convert a UTF-16 path into a byte path for Unix platforms.
#[cfg(unix)]
pub(crate) fn utf16_path_to_utf8_bytes(path: PathUtf16, label: &str) -> RuntimeResult<Vec<u8>> {
    let utf16 = decode_utf16_bytes(path, label)?;
    let decoded = String::from_utf16(&utf16).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "path contains invalid utf16",
        ))
        .boxed()
    })?;

    let bytes = decoded.into_bytes();
    if bytes.len() > u32::MAX as usize {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "path is too large",
        ))
        .boxed());
    }

    Ok(bytes)
}

/// Convert a UTF-16 path into a byte path for Unix platforms.
#[cfg(unix)]
fn with_utf16_as_bytes<T>(
    path: PathUtf16,
    label: &str,
    on_bytes: impl FnOnce(PathBytes) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let mut bytes = utf16_path_to_utf8_bytes(path, label)?;

    let path_bytes = PathBytesAbi::<NativeAbi>(NativeArray {
        data: bytes.as_mut_ptr(),
        len: bytes.len() as u32,
        capacity: bytes.len() as u32,
    });
    let result = on_bytes(path_bytes);
    drop(bytes);
    result
}

/// Convert a pair of UTF-16 paths into byte paths for Unix platforms.
#[cfg(unix)]
fn with_utf16_pair_as_bytes<T>(
    left: PathUtf16,
    right: PathUtf16,
    label: &str,
    on_bytes: impl FnOnce(PathBytes, PathBytes) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let left_utf16 = decode_utf16_bytes(left, label)?;
    let right_utf16 = decode_utf16_bytes(right, label)?;

    let left_decoded = String::from_utf16(&left_utf16).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "path contains invalid utf16",
        ))
        .boxed()
    })?;
    let right_decoded = String::from_utf16(&right_utf16).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "path contains invalid utf16",
        ))
        .boxed()
    })?;

    let mut left_bytes = left_decoded.into_bytes();
    let mut right_bytes = right_decoded.into_bytes();
    if left_bytes.len() > u32::MAX as usize || right_bytes.len() > u32::MAX as usize {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "path is too large",
        ))
        .boxed());
    }

    let left_path = PathBytesAbi::<NativeAbi>(NativeArray {
        data: left_bytes.as_mut_ptr(),
        len: left_bytes.len() as u32,
        capacity: left_bytes.len() as u32,
    });
    let right_path = PathBytesAbi::<NativeAbi>(NativeArray {
        data: right_bytes.as_mut_ptr(),
        len: right_bytes.len() as u32,
        capacity: right_bytes.len() as u32,
    });

    let result = on_bytes(left_path, right_path);
    drop((left_bytes, right_bytes));
    result
}

/// Convert a byte path into a UTF-16 path for Unix platforms.
#[cfg(unix)]
fn path_utf16_from_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    label: &str,
) -> RuntimeResult<PathUtf16> {
    let bytes = unsafe { path.0.as_slice()? };
    let text = std::str::from_utf8(bytes).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "path contains invalid utf8",
        ))
        .boxed()
    })?;
    let utf16: Vec<u16> = text.encode_utf16().collect();

    Ok(PathUtf16Abi::<NativeAbi>(context.store_array(utf16)))
}

/// Dispatch a OsPath into a bytes or UTF-16 handler.
fn with_path_ref<T>(
    path: OsPath,
    label: &str,
    on_bytes: impl FnOnce(PathBytes) -> RuntimeResult<T>,
    _on_utf16: impl FnOnce(PathUtf16) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    #[cfg(not(unix))]
    let _ = label;

    match path.encoding {
        PathEncoding::Bytes => on_bytes(path.bytes),
        PathEncoding::Utf16 => {
            #[cfg(unix)]
            {
                with_utf16_as_bytes(path.utf16, label, on_bytes)
            }
            #[cfg(not(unix))]
            {
                _on_utf16(path.utf16)
            }
        }
    }
}

/// Dispatch two OsPath values into a bytes or UTF-16 handler.
fn with_path_ref_pair<T>(
    left: OsPath,
    right: OsPath,
    label: &str,
    on_bytes: impl FnOnce(PathBytes, PathBytes) -> RuntimeResult<T>,
    _on_utf16: impl FnOnce(PathUtf16, PathUtf16) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    match (left.encoding, right.encoding) {
        (PathEncoding::Bytes, PathEncoding::Bytes) => on_bytes(left.bytes, right.bytes),
        (PathEncoding::Utf16, PathEncoding::Utf16) => {
            #[cfg(unix)]
            {
                with_utf16_pair_as_bytes(left.utf16, right.utf16, label, on_bytes)
            }
            #[cfg(not(unix))]
            {
                _on_utf16(left.utf16, right.utf16)
            }
        }
        _ => path_ref_mismatch(label),
    }
}

/// Check file access permissions.
pub(crate) unsafe fn destack_fs_access(
    context: &RuntimeCallContext,
    path: OsPath,
    mode: AccessMode,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_access_bytes(context, path, mode) },
        |path| unsafe { destack_fs_access_utf16(context, path, mode) },
    )
}

/// Change file permissions.
pub(crate) unsafe fn destack_fs_chmod(
    context: &RuntimeCallContext,
    path: OsPath,
    mode: FileMode,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_chmod_bytes(context, path, mode) },
        |path| unsafe { destack_fs_chmod_utf16(context, path, mode) },
    )
}

/// Change file permissions relative to a directory handle.
pub(crate) unsafe fn destack_fs_fchmodat(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: OsPath,
    mode: FileMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_fchmodat_bytes(context, dir, path, mode, flags) },
        |path| unsafe { destack_fs_fchmodat_utf16(context, dir, path, mode, flags) },
    )
}

/// Change file owner and group.
pub(crate) unsafe fn destack_fs_chown(
    context: &RuntimeCallContext,
    path: OsPath,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_chown_bytes(context, path, uid, gid) },
        |path| unsafe { destack_fs_chown_utf16(context, path, uid, gid) },
    )
}

/// Change file owner and group relative to a directory handle.
pub(crate) unsafe fn destack_fs_fchownat(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: OsPath,
    uid: u32,
    gid: u32,
    flags: AtFlags,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_fchownat_bytes(context, dir, path, uid, gid, flags) },
        |path| unsafe { destack_fs_fchownat_utf16(context, dir, path, uid, gid, flags) },
    )
}

/// Update access and modification times.
pub(crate) unsafe fn destack_fs_utimes(
    context: &RuntimeCallContext,
    path: OsPath,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_utimes_bytes(context, path, atime_ns, mtime_ns) },
        |path| unsafe { destack_fs_utimes_utf16(context, path, atime_ns, mtime_ns) },
    )
}

/// Update access and modification times without following symlinks.
pub(crate) unsafe fn destack_fs_lutimes(
    context: &RuntimeCallContext,
    path: OsPath,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_lutimes_bytes(context, path, atime_ns, mtime_ns) },
        |path| unsafe { destack_fs_lutimes_utf16(context, path, atime_ns, mtime_ns) },
    )
}

/// Update access and modification times relative to a directory handle.
pub(crate) unsafe fn destack_fs_utimensat(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: OsPath,
    atime_ns: u64,
    mtime_ns: u64,
    flags: AtFlags,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_utimensat_bytes(context, dir, path, atime_ns, mtime_ns, flags) },
        |path| unsafe { destack_fs_utimensat_utf16(context, dir, path, atime_ns, mtime_ns, flags) },
    )
}

/// Create a directory.
pub(crate) unsafe fn destack_fs_mkdir(
    context: &RuntimeCallContext,
    path: OsPath,
    mode: FileMode,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_mkdir_bytes(context, path, mode) },
        |path| unsafe { destack_fs_mkdir_utf16(context, path, mode) },
    )
}

/// Create a directory relative to a directory handle.
pub(crate) unsafe fn destack_fs_mkdirat(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: OsPath,
    mode: FileMode,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_mkdirat_bytes(context, dir, path, mode) },
        |path| unsafe { destack_fs_mkdirat_utf16(context, dir, path, mode) },
    )
}

/// Remove a directory.
pub(crate) unsafe fn destack_fs_rmdir(
    context: &RuntimeCallContext,
    path: OsPath,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_rmdir_bytes(context, path) },
        |path| unsafe { destack_fs_rmdir_utf16(context, path) },
    )
}

/// Open a directory and return a handle.
pub(crate) unsafe fn destack_fs_opendir(
    context: &RuntimeCallContext,
    out: *mut DirectoryHandle,
    path: OsPath,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_opendir_bytes(context, out, path) },
        |path| unsafe { destack_fs_opendir_utf16(context, out, path) },
    )
}

/// Create a temporary directory.
pub(crate) unsafe fn destack_fs_mkdtemp(
    context: &RuntimeCallContext,
    out: *mut OsPath,
    template: OsPath,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    #[cfg(unix)]
    {
        match template.encoding {
            PathEncoding::Bytes => {
                let mut inner = empty_path_bytes();
                unsafe { destack_fs_mkdtemp_bytes(context, &mut inner, template.bytes) }?;
                unsafe {
                    *out = path_ref_from_bytes(inner);
                }
                Ok(())
            }
            PathEncoding::Utf16 => {
                let bytes = with_utf16_as_bytes(template.utf16, "template", |template| {
                    let mut inner = empty_path_bytes();
                    unsafe { destack_fs_mkdtemp_bytes(context, &mut inner, template) }?;
                    Ok(inner)
                })?;
                let utf16 = path_utf16_from_bytes(context, bytes, "template")?;
                unsafe {
                    *out = path_ref_from_utf16(utf16);
                }
                Ok(())
            }
        }
    }
    #[cfg(not(unix))]
    with_path_ref(
        template,
        "template",
        |template| {
            let mut inner = empty_path_bytes();
            unsafe { destack_fs_mkdtemp_bytes(context, &mut inner, template) }?;
            unsafe {
                *out = path_ref_from_bytes(inner);
            }
            Ok(())
        },
        |template| {
            let mut inner = empty_path_utf16();
            unsafe { destack_fs_mkdtemp_utf16(context, &mut inner, template) }?;
            unsafe {
                *out = path_ref_from_utf16(inner);
            }
            Ok(())
        },
    )
}

/// Open a file and return a handle.
pub(crate) unsafe fn destack_fs_open(
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    path: OsPath,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_open_bytes(context, out, path, flags, mode) },
        |path| unsafe { destack_fs_open_utf16(context, out, path, flags, mode) },
    )
}

/// Open a file relative to a directory handle.
pub(crate) unsafe fn destack_fs_openat(
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    dir: DirectoryHandle,
    path: OsPath,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_openat_bytes(context, out, dir, path, flags, mode) },
        |path| unsafe { destack_fs_openat_utf16(context, out, dir, path, flags, mode) },
    )
}

/// Open a file relative to a directory handle with openat2 semantics.
pub(crate) unsafe fn destack_fs_openat2(
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    dir: DirectoryHandle,
    path: OsPath,
    how: OpenOptions,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_openat2_bytes(context, out, dir, path, how) },
        |path| unsafe { destack_fs_openat2_utf16(context, out, dir, path, how) },
    )
}

/// Truncate a file.
pub(crate) unsafe fn destack_fs_truncate(
    context: &RuntimeCallContext,
    path: OsPath,
    size: FileOffset,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_truncate_bytes(context, path, size) },
        |path| unsafe { destack_fs_truncate_utf16(context, path, size) },
    )
}

/// Rename or move a file.
pub(crate) unsafe fn destack_fs_rename(
    context: &RuntimeCallContext,
    from: OsPath,
    to: OsPath,
) -> RuntimeResult<()> {
    with_path_ref_pair(
        from,
        to,
        "path",
        |from, to| unsafe { destack_fs_rename_bytes(context, from, to) },
        |from, to| unsafe { destack_fs_rename_utf16(context, from, to) },
    )
}

/// Rename or move a file relative to directory handles.
pub(crate) unsafe fn destack_fs_renameat(
    context: &RuntimeCallContext,
    from_dir: DirectoryHandle,
    from: OsPath,
    to_dir: DirectoryHandle,
    to: OsPath,
) -> RuntimeResult<()> {
    with_path_ref_pair(
        from,
        to,
        "path",
        |from, to| unsafe { destack_fs_renameat_bytes(context, from_dir, from, to_dir, to) },
        |from, to| unsafe { destack_fs_renameat_utf16(context, from_dir, from, to_dir, to) },
    )
}

/// Rename or move a file relative to directory handles with renameat2 semantics.
pub(crate) unsafe fn destack_fs_renameat2(
    context: &RuntimeCallContext,
    from_dir: DirectoryHandle,
    from: OsPath,
    to_dir: DirectoryHandle,
    to: OsPath,
    flags: RenameFlags,
) -> RuntimeResult<()> {
    with_path_ref_pair(
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
pub(crate) unsafe fn destack_fs_unlink(
    context: &RuntimeCallContext,
    path: OsPath,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_unlink_bytes(context, path) },
        |path| unsafe { destack_fs_unlink_utf16(context, path) },
    )
}

/// Unlink a file relative to a directory handle.
pub(crate) unsafe fn destack_fs_unlinkat(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: OsPath,
    flags: AtFlags,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_unlinkat_bytes(context, dir, path, flags) },
        |path| unsafe { destack_fs_unlinkat_utf16(context, dir, path, flags) },
    )
}

/// Create a hard link.
pub(crate) unsafe fn destack_fs_link(
    context: &RuntimeCallContext,
    existing_path: OsPath,
    new_path: OsPath,
) -> RuntimeResult<()> {
    with_path_ref_pair(
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
pub(crate) unsafe fn destack_fs_linkat(
    context: &RuntimeCallContext,
    existing_dir: DirectoryHandle,
    existing_path: OsPath,
    new_dir: DirectoryHandle,
    new_path: OsPath,
    flags: AtFlags,
) -> RuntimeResult<()> {
    with_path_ref_pair(
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
pub(crate) unsafe fn destack_fs_symlink(
    context: &RuntimeCallContext,
    target: OsPath,
    path: OsPath,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    with_path_ref_pair(
        target,
        path,
        "path",
        |target, path| unsafe { destack_fs_symlink_bytes(context, target, path, kind) },
        |target, path| unsafe { destack_fs_symlink_utf16(context, target, path, kind) },
    )
}

/// Create a symbolic link relative to a directory handle.
pub(crate) unsafe fn destack_fs_symlinkat(
    context: &RuntimeCallContext,
    target: OsPath,
    dir: DirectoryHandle,
    path: OsPath,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    with_path_ref_pair(
        target,
        path,
        "path",
        |target, path| unsafe { destack_fs_symlinkat_bytes(context, target, dir, path, kind) },
        |target, path| unsafe { destack_fs_symlinkat_utf16(context, target, dir, path, kind) },
    )
}

/// Read a symbolic link.
pub(crate) unsafe fn destack_fs_readlink(
    context: &RuntimeCallContext,
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
                let mut inner = empty_path_bytes();
                unsafe { destack_fs_readlink_bytes(context, &mut inner, path.bytes) }?;
                unsafe {
                    *out = path_ref_from_bytes(inner);
                }
                Ok(())
            }
            PathEncoding::Utf16 => {
                let bytes = with_utf16_as_bytes(path.utf16, "path", |path| {
                    let mut inner = empty_path_bytes();
                    unsafe { destack_fs_readlink_bytes(context, &mut inner, path) }?;
                    Ok(inner)
                })?;
                let utf16 = path_utf16_from_bytes(context, bytes, "path")?;
                unsafe {
                    *out = path_ref_from_utf16(utf16);
                }
                Ok(())
            }
        }
    }
    #[cfg(not(unix))]
    with_path_ref(
        path,
        "path",
        |path| {
            let mut inner = empty_path_bytes();
            unsafe { destack_fs_readlink_bytes(context, &mut inner, path) }?;
            unsafe {
                *out = path_ref_from_bytes(inner);
            }
            Ok(())
        },
        |path| {
            let mut inner = empty_path_utf16();
            unsafe { destack_fs_readlink_utf16(context, &mut inner, path) }?;
            unsafe {
                *out = path_ref_from_utf16(inner);
            }
            Ok(())
        },
    )
}

/// Read a symbolic link relative to a directory handle.
pub(crate) unsafe fn destack_fs_readlinkat(
    context: &RuntimeCallContext,
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
                let mut inner = empty_path_bytes();
                unsafe { destack_fs_readlinkat_bytes(context, &mut inner, dir, path.bytes) }?;
                unsafe {
                    *out = path_ref_from_bytes(inner);
                }
                Ok(())
            }
            PathEncoding::Utf16 => {
                let bytes = with_utf16_as_bytes(path.utf16, "path", |path| {
                    let mut inner = empty_path_bytes();
                    unsafe { destack_fs_readlinkat_bytes(context, &mut inner, dir, path) }?;
                    Ok(inner)
                })?;
                let utf16 = path_utf16_from_bytes(context, bytes, "path")?;
                unsafe {
                    *out = path_ref_from_utf16(utf16);
                }
                Ok(())
            }
        }
    }
    #[cfg(not(unix))]
    with_path_ref(
        path,
        "path",
        |path| {
            let mut inner = empty_path_bytes();
            unsafe { destack_fs_readlinkat_bytes(context, &mut inner, dir, path) }?;
            unsafe {
                *out = path_ref_from_bytes(inner);
            }
            Ok(())
        },
        |path| {
            let mut inner = empty_path_utf16();
            unsafe { destack_fs_readlinkat_utf16(context, &mut inner, dir, path) }?;
            unsafe {
                *out = path_ref_from_utf16(inner);
            }
            Ok(())
        },
    )
}

/// Resolve a path to its canonical form.
pub(crate) unsafe fn destack_fs_realpath(
    context: &RuntimeCallContext,
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
                let mut inner = empty_path_bytes();
                unsafe { destack_fs_realpath_bytes(context, &mut inner, path.bytes) }?;
                unsafe {
                    *out = path_ref_from_bytes(inner);
                }
                Ok(())
            }
            PathEncoding::Utf16 => {
                let bytes = with_utf16_as_bytes(path.utf16, "path", |path| {
                    let mut inner = empty_path_bytes();
                    unsafe { destack_fs_realpath_bytes(context, &mut inner, path) }?;
                    Ok(inner)
                })?;
                let utf16 = path_utf16_from_bytes(context, bytes, "path")?;
                unsafe {
                    *out = path_ref_from_utf16(utf16);
                }
                Ok(())
            }
        }
    }
    #[cfg(not(unix))]
    with_path_ref(
        path,
        "path",
        |path| {
            let mut inner = empty_path_bytes();
            unsafe { destack_fs_realpath_bytes(context, &mut inner, path) }?;
            unsafe {
                *out = path_ref_from_bytes(inner);
            }
            Ok(())
        },
        |path| {
            let mut inner = empty_path_utf16();
            unsafe { destack_fs_realpath_utf16(context, &mut inner, path) }?;
            unsafe {
                *out = path_ref_from_utf16(inner);
            }
            Ok(())
        },
    )
}

/// Copy a file.
pub(crate) unsafe fn destack_fs_copyfile(
    context: &RuntimeCallContext,
    from: OsPath,
    to: OsPath,
    flags: CopyFlags,
) -> RuntimeResult<()> {
    with_path_ref_pair(
        from,
        to,
        "path",
        |from, to| unsafe { destack_fs_copyfile_bytes(context, from, to, flags) },
        |from, to| unsafe { destack_fs_copyfile_utf16(context, from, to, flags) },
    )
}

/// Read an extended attribute by path.
pub(crate) unsafe fn destack_fs_getxattr(
    context: &RuntimeCallContext,
    out: *mut NativeArray<u8>,
    path: OsPath,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_getxattr_bytes(context, out, path, name) },
        |path| unsafe { destack_fs_getxattr_utf16(context, out, path, name) },
    )
}

/// Read an extended attribute without following symlinks.
pub(crate) unsafe fn destack_fs_lgetxattr(
    context: &RuntimeCallContext,
    out: *mut NativeArray<u8>,
    path: OsPath,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_lgetxattr_bytes(context, out, path, name) },
        |path| unsafe { destack_fs_lgetxattr_utf16(context, out, path, name) },
    )
}

/// Read an extended attribute by handle.
pub(crate) unsafe fn destack_fs_fgetxattr(
    context: &RuntimeCallContext,
    out: *mut NativeArray<u8>,
    handle: FileHandle,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { destack_fs_fgetxattr_handle(context, out, handle, name) }
}

/// Set an extended attribute by path.
pub(crate) unsafe fn destack_fs_setxattr(
    context: &RuntimeCallContext,
    path: OsPath,
    name: NativeStringRef,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_setxattr_bytes(context, path, name, value, flags) },
        |path| unsafe { destack_fs_setxattr_utf16(context, path, name, value, flags) },
    )
}

/// Set an extended attribute without following symlinks.
pub(crate) unsafe fn destack_fs_lsetxattr(
    context: &RuntimeCallContext,
    path: OsPath,
    name: NativeStringRef,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_lsetxattr_bytes(context, path, name, value, flags) },
        |path| unsafe { destack_fs_lsetxattr_utf16(context, path, name, value, flags) },
    )
}

/// Set an extended attribute by handle.
pub(crate) unsafe fn destack_fs_fsetxattr(
    context: &RuntimeCallContext,
    handle: FileHandle,
    name: NativeStringRef,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    unsafe { destack_fs_fsetxattr_handle(context, handle, name, value, flags) }
}

/// List extended attribute names by path.
pub(crate) unsafe fn destack_fs_listxattr(
    context: &RuntimeCallContext,
    out: *mut NativeArray<NativeStringRef>,
    path: OsPath,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_listxattr_bytes(context, out, path) },
        |path| unsafe { destack_fs_listxattr_utf16(context, out, path) },
    )
}

/// List extended attribute names without following symlinks.
pub(crate) unsafe fn destack_fs_llistxattr(
    context: &RuntimeCallContext,
    out: *mut NativeArray<NativeStringRef>,
    path: OsPath,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_llistxattr_bytes(context, out, path) },
        |path| unsafe { destack_fs_llistxattr_utf16(context, out, path) },
    )
}

/// List extended attribute names by handle.
pub(crate) unsafe fn destack_fs_flistxattr(
    context: &RuntimeCallContext,
    out: *mut NativeArray<NativeStringRef>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    unsafe { destack_fs_flistxattr_handle(context, out, handle) }
}

/// Remove an extended attribute by path.
pub(crate) unsafe fn destack_fs_removexattr(
    context: &RuntimeCallContext,
    path: OsPath,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_removexattr_bytes(context, path, name) },
        |path| unsafe { destack_fs_removexattr_utf16(context, path, name) },
    )
}

/// Remove an extended attribute without following symlinks.
pub(crate) unsafe fn destack_fs_lremovexattr(
    context: &RuntimeCallContext,
    path: OsPath,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_lremovexattr_bytes(context, path, name) },
        |path| unsafe { destack_fs_lremovexattr_utf16(context, path, name) },
    )
}

/// Remove an extended attribute by handle.
pub(crate) unsafe fn destack_fs_fremovexattr(
    context: &RuntimeCallContext,
    handle: FileHandle,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { destack_fs_fremovexattr_handle(context, handle, name) }
}

/// Stat a file.
pub(crate) unsafe fn destack_fs_stat(
    context: &RuntimeCallContext,
    out: *mut Stat,
    path: OsPath,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_stat_bytes(context, out, path) },
        |path| unsafe { destack_fs_stat_utf16(context, out, path) },
    )
}

/// Stat a file relative to a directory handle.
pub(crate) unsafe fn destack_fs_statat(
    context: &RuntimeCallContext,
    out: *mut Stat,
    dir: DirectoryHandle,
    path: OsPath,
    flags: AtFlags,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_statat_bytes(context, out, dir, path, flags) },
        |path| unsafe { destack_fs_statat_utf16(context, out, dir, path, flags) },
    )
}

/// Stat a file without following symlinks.
pub(crate) unsafe fn destack_fs_lstat(
    context: &RuntimeCallContext,
    out: *mut Stat,
    path: OsPath,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_lstat_bytes(context, out, path) },
        |path| unsafe { destack_fs_lstat_utf16(context, out, path) },
    )
}

/// Stat a filesystem.
pub(crate) unsafe fn destack_fs_statfs(
    context: &RuntimeCallContext,
    out: *mut StatFs,
    path: OsPath,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_statfs_bytes(context, out, path) },
        |path| unsafe { destack_fs_statfs_utf16(context, out, path) },
    )
}

/// Check file access permissions relative to a directory handle.
pub(crate) unsafe fn destack_fs_accessat(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: OsPath,
    mode: AccessMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    with_path_ref(
        path,
        "path",
        |path| {
            #[cfg(unix)]
            {
                let directory_fd = directory_descriptor(context, dir)?;
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
                let _ = (context, dir, path, mode, flags);
                Err(RuntimeError::from(PlatformError::not_supported("destack.fs.accessat")).boxed())
            }
        },
        |_path| {
            Err(RuntimeError::from(PlatformError::not_supported("destack.fs.accessat")).boxed())
        },
    )
}

/// Resolve the file descriptor for an open directory handle.
pub(crate) unsafe fn destack_fs_dirfd(
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    #[cfg(unix)]
    {
        let directory_fd = directory_descriptor(context, handle)?;
        let file_fd = unsafe { libc::dup(directory_fd) };
        if file_fd < 0 {
            return Err(RuntimeError::from(PlatformError::io("dup failed".to_string())).boxed());
        }

        let resource = ResourceEntry::new(ResourceKind::File)
            .with_fd(file_fd)
            .with_finalizer(DescriptorFinalizer { fd: file_fd });
        let resource_id = context.runtime().resources.insert(resource);
        unsafe {
            *out = FileHandle(resource_id);
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = (context, handle);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dirfd")).boxed())
    }
}

/// Read file descriptor flags.
pub(crate) unsafe fn destack_fs_get_fd_flags(
    context: &RuntimeCallContext,
    out: *mut FdFlags,
    handle: FileHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    #[cfg(unix)]
    {
        let fd = file_descriptor(context, handle)?;
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        if flags < 0 {
            return Err(RuntimeError::from(PlatformError::io("fcntl failed".to_string())).boxed());
        }
        unsafe {
            *out = FdFlags(flags as u32);
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = (context, handle);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.getFdFlags")).boxed())
    }
}

/// Read file status flags.
pub(crate) unsafe fn destack_fs_get_status_flags(
    context: &RuntimeCallContext,
    out: *mut StatusFlags,
    handle: FileHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    #[cfg(unix)]
    {
        let fd = file_descriptor(context, handle)?;
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 {
            return Err(RuntimeError::from(PlatformError::io("fcntl failed".to_string())).boxed());
        }
        unsafe {
            *out = StatusFlags(flags as u32);
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = (context, handle);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.getStatusFlags")).boxed())
    }
}

/// Create a fifo special file.
pub(crate) unsafe fn destack_fs_mkfifo(
    _context: &RuntimeCallContext,
    path: OsPath,
    mode: FileMode,
) -> RuntimeResult<()> {
    with_path_ref(
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

/// Create a fifo special file relative to a directory handle.
pub(crate) unsafe fn destack_fs_mkfifoat(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: OsPath,
    mode: FileMode,
) -> RuntimeResult<()> {
    with_path_ref(
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
pub(crate) unsafe fn destack_fs_mknod(
    _context: &RuntimeCallContext,
    path: OsPath,
    mode: FileMode,
    device: NodeDevice,
) -> RuntimeResult<()> {
    with_path_ref(
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
pub(crate) unsafe fn destack_fs_mknodat(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: OsPath,
    mode: FileMode,
    device: NodeDevice,
) -> RuntimeResult<()> {
    with_path_ref(
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

/// Read the next directory entry from an open directory handle.
pub(crate) unsafe fn destack_fs_readdir_next(
    context: &RuntimeCallContext,
    out: *mut DirentNext,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    #[cfg(unix)]
    {
        let directory_fd = directory_descriptor(context, handle)?;
        let duplicate_fd = unsafe { libc::dup(directory_fd) };
        if duplicate_fd < 0 {
            return Err(RuntimeError::from(PlatformError::io("dup failed".to_string())).boxed());
        }

        let directory = unsafe { libc::fdopendir(duplicate_fd) };
        if directory.is_null() {
            unsafe {
                libc::close(duplicate_fd);
            }
            return Err(
                RuntimeError::from(PlatformError::io("fdopendir failed".to_string())).boxed(),
            );
        }

        loop {
            let entry = unsafe { libc::readdir(directory) };
            if entry.is_null() {
                unsafe {
                    libc::closedir(directory);
                }
                let empty_bytes = PathBytesAbi::<NativeAbi>(context.store_array(Vec::new()));
                unsafe {
                    *out = DirentNext {
                        has_entry: false,
                        entry: Dirent {
                            name: path_ref_from_bytes(empty_bytes),
                            kind: DirentKind::Unknown,
                        },
                    };
                }
                return Ok(());
            }

            let name_pointer = unsafe { (*entry).d_name.as_ptr() };
            let name = unsafe { std::ffi::CStr::from_ptr(name_pointer) };
            let name_bytes = name.to_bytes();
            if name_bytes == b"." || name_bytes == b".." {
                continue;
            }

            let kind = dirent_kind_from_type(unsafe { (*entry).d_type });
            let name = PathBytesAbi::<NativeAbi>(context.store_array(name_bytes.to_vec()));
            unsafe {
                *out = DirentNext {
                    has_entry: true,
                    entry: Dirent {
                        name: path_ref_from_bytes(name),
                        kind,
                    },
                };
            }
            unsafe {
                libc::closedir(directory);
            }
            return Ok(());
        }
    }

    #[cfg(not(unix))]
    {
        let _ = (context, handle);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readdirNext")).boxed())
    }
}

/// Rewind an open directory handle to the beginning.
pub(crate) unsafe fn destack_fs_rewinddir(
    context: &RuntimeCallContext,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        let directory_fd = directory_descriptor(context, handle)?;
        let duplicate_fd = unsafe { libc::dup(directory_fd) };
        if duplicate_fd < 0 {
            return Err(RuntimeError::from(PlatformError::io("dup failed".to_string())).boxed());
        }
        let directory = unsafe { libc::fdopendir(duplicate_fd) };
        if directory.is_null() {
            unsafe {
                libc::close(duplicate_fd);
            }
            return Err(
                RuntimeError::from(PlatformError::io("fdopendir failed".to_string())).boxed(),
            );
        }
        unsafe {
            libc::rewinddir(directory);
            libc::closedir(directory);
        }
        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = (context, handle);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.rewinddir")).boxed())
    }
}

/// Write file descriptor flags.
pub(crate) unsafe fn destack_fs_set_fd_flags(
    context: &RuntimeCallContext,
    handle: FileHandle,
    flags: FdFlags,
) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        let fd = file_descriptor(context, handle)?;
        let result = unsafe { libc::fcntl(fd, libc::F_SETFD, flags.0 as libc::c_int) };
        if result < 0 {
            return Err(RuntimeError::from(PlatformError::io("fcntl failed".to_string())).boxed());
        }
        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = (context, handle, flags);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.setFdFlags")).boxed())
    }
}

/// Write file status flags.
pub(crate) unsafe fn destack_fs_set_status_flags(
    context: &RuntimeCallContext,
    handle: FileHandle,
    flags: StatusFlags,
) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        let fd = file_descriptor(context, handle)?;
        let result = unsafe { libc::fcntl(fd, libc::F_SETFL, flags.0 as libc::c_int) };
        if result < 0 {
            return Err(RuntimeError::from(PlatformError::io("fcntl failed".to_string())).boxed());
        }
        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = (context, handle, flags);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.setStatusFlags")).boxed())
    }
}

/// Stat a path with statx semantics.
pub(crate) unsafe fn destack_fs_statx(
    context: &RuntimeCallContext,
    out: *mut Statx,
    dir: DirectoryHandle,
    path: OsPath,
    flags: StatxFlags,
    _mask: StatxMask,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    with_path_ref(
        path,
        "path",
        |path| {
            #[cfg(unix)]
            {
                let directory_fd = directory_descriptor(context, dir)?;
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
                    return Err(RuntimeError::from(PlatformError::io(
                        "fstatat failed".to_string(),
                    ))
                    .boxed());
                }
                let stat = unsafe { stat.assume_init() };

                let atime_ns = (stat.st_atime as u64)
                    .saturating_mul(1_000_000_000)
                    .saturating_add(stat.st_atime_nsec as u64);
                let btime_ns = 0;
                let ctime_ns = (stat.st_ctime as u64)
                    .saturating_mul(1_000_000_000)
                    .saturating_add(stat.st_ctime_nsec as u64);
                let mtime_ns = (stat.st_mtime as u64)
                    .saturating_mul(1_000_000_000)
                    .saturating_add(stat.st_mtime_nsec as u64);

                unsafe {
                    *out = Statx {
                        mask: StatxMask(0),
                        blksize: stat.st_blksize as u32,
                        mount_id: 0,
                        dev_major: ((stat.st_dev >> 8) & 0xfff) as u32,
                        dev_minor: ((stat.st_dev & 0xff) | ((stat.st_dev >> 12) & 0xfff00)) as u32,
                        ino: stat.st_ino,
                        mode: FileMode(stat.st_mode as u32),
                        nlink: stat.st_nlink as u32,
                        uid: stat.st_uid,
                        gid: stat.st_gid,
                        rdev_major: ((stat.st_rdev >> 8) & 0xfff) as u32,
                        rdev_minor: ((stat.st_rdev & 0xff) | ((stat.st_rdev >> 12) & 0xfff00))
                            as u32,
                        size: crate::platform::fs::FileSize(stat.st_size as u64),
                        blocks: stat.st_blocks as u64,
                        atime_ns,
                        btime_ns,
                        ctime_ns,
                        mtime_ns,
                    };
                }
                Ok(())
            }
            #[cfg(not(unix))]
            {
                let _ = (context, dir, path, flags);
                Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statx")).boxed())
            }
        },
        |_path| Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statx")).boxed()),
    )
}

/// Synchronize a filesystem by file handle.
pub(crate) unsafe fn destack_fs_syncfs(
    context: &RuntimeCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        let fd = file_descriptor(context, handle)?;
        #[cfg(any(target_os = "linux", target_os = "android"))]
        {
            let result = unsafe { libc::syncfs(fd) };
            if result != 0 {
                return Err(
                    RuntimeError::from(PlatformError::io("syncfs failed".to_string())).boxed(),
                );
            }
            Ok(())
        }
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        {
            let result = unsafe { libc::fsync(fd) };
            if result != 0 {
                return Err(
                    RuntimeError::from(PlatformError::io("fsync failed".to_string())).boxed(),
                );
            }
            Ok(())
        }
    }

    #[cfg(not(unix))]
    {
        let _ = (context, handle);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.syncfs")).boxed())
    }
}

/// Open a filesystem watch for a path.
pub(crate) unsafe fn destack_fs_watch(
    context: &RuntimeCallContext,
    out: *mut WatchHandle,
    path: OsPath,
    options: WatchOptions,
) -> RuntimeResult<()> {
    // validate pointers and mark arguments as used
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, path, options);

    // NOTE #Incomplete: implement filesystem watcher open
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.watch")).boxed())
}

/// Close a filesystem watch handle.
pub(crate) unsafe fn destack_fs_watch_close(
    context: &RuntimeCallContext,
    handle: WatchHandle,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement filesystem watcher close
    let _ = (context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.watchClose")).boxed())
}

/// Read pending watch events.
pub(crate) unsafe fn destack_fs_watch_read(
    context: &RuntimeCallContext,
    out: *mut WatchBatch,
    handle: WatchHandle,
) -> RuntimeResult<()> {
    // validate pointers and mark arguments as used
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle);

    // NOTE #Incomplete: implement filesystem watcher read
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.watchRead")).boxed())
}

/// Open a filesystem watch for a path relative to a directory handle.
pub(crate) unsafe fn destack_fs_watchat(
    context: &RuntimeCallContext,
    out: *mut WatchHandle,
    directory: DirectoryHandle,
    path: OsPath,
    options: WatchOptions,
) -> RuntimeResult<()> {
    // validate pointers and mark arguments as used
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, directory, path, options);

    // NOTE #Incomplete: implement relative filesystem watcher open
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.watchat")).boxed())
}

/// Core filesystem interface exposed to bindings.
/// This provides a stable entrypoint that selects the active OS backend.
pub(crate) use super::host::*;
