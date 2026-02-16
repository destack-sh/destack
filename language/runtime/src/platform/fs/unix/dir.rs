#![allow(unused_imports)]

use super::core::*;
use super::os;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{core as core_fs, *};
use crate::platform::resource::*;
use crate::platform::{core as core_platform, net as platform_net, *};
use crate::runtime::RuntimeCallContext;

use std::ffi::{CStr, CString};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::RawFd;
use std::path::PathBuf;

/// Read directory entries from an open directory handle.
///
/// Read the full directory stream from the current cursor until the host reports end-of-directory.
/// Entry ordering and type classification follow host directory iteration semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses readdir(3) loop on Unix and FindNextFileW loop on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_readdir(
    context: &RuntimeCallContext,
    out: *mut NativeArray<Dirent>,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read directory entries on unix platforms
    let resource = directory_resource(context, handle)?;
    let dup_fd = unsafe { libc::dup(resource.fd) };
    if dup_fd < 0 {
        return Err(core_platform::io_error("dup", None));
    }

    let dirp = unsafe { libc::fdopendir(dup_fd) };
    if dirp.is_null() {
        unsafe {
            libc::close(dup_fd);
        }
        return Err(core_platform::io_error("fdopendir", None));
    }

    struct DirGuard(*mut libc::DIR);
    impl Drop for DirGuard {
        fn drop(&mut self) {
            unsafe {
                libc::closedir(self.0);
            }
        }
    }

    let _guard = DirGuard(dirp);
    let mut dirents = Vec::new();
    loop {
        core_platform::set_errno(0);
        let entry = unsafe { libc::readdir(dirp) };
        if entry.is_null() {
            if core_platform::get_errno() != 0 {
                return Err(core_platform::io_error(
                    "readdir",
                    Some(resource.path.to_string_lossy().as_ref()),
                ));
            }
            break;
        }

        let name_ptr = unsafe { (*entry).d_name.as_ptr() };
        let name = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
        let name_bytes = name.to_bytes();
        if name_bytes == b"." || name_bytes == b".." {
            continue;
        }
        let mut kind = dirent_kind_from_type(unsafe { (*entry).d_type });
        if kind == DirentKind::Unknown {
            let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
            let rc = unsafe {
                libc::fstatat(
                    resource.fd,
                    name_ptr,
                    stat.as_mut_ptr(),
                    libc::AT_SYMLINK_NOFOLLOW,
                )
            };
            if rc == 0 {
                let stat = unsafe { stat.assume_init() };
                kind = dirent_kind_from_mode(stat.st_mode);
            }
        }
        let name = PathBytesAbi::<NativeAbi>(context.store_array(name_bytes.to_vec()));
        dirents.push(Dirent {
            name: core_fs::path_ref_from_bytes(name),
            kind,
        });
    }

    let array = context.store_array(dirents);
    unsafe {
        *out = array;
    }

    Ok(())
}

/// Remove a directory.
///
/// Remove the target resource through a single host namespace operation with no runtime fallback path.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses rmdir(2) on Unix and RemoveDirectoryW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_rmdir_bytes(
    _context: &RuntimeCallContext,
    path: PathBytes,
) -> RuntimeResult<()> {
    // remove the directory on unix platforms
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let rc = unsafe { libc::rmdir(c_path.as_ptr()) };
    if rc == 0 {
        return Ok(());
    }

    Err(core_platform::io_error("rmdir", None))
}

/// Remove a directory.
///
/// Remove the target resource through a single host namespace operation with no runtime fallback path.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses rmdir(2) on Unix and RemoveDirectoryW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_rmdir_utf16(
    _context: &RuntimeCallContext,
    _path: PathUtf16,
) -> RuntimeResult<()> {
    // report unsupported rmdir calls on non-windows platforms
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.rmdirUtf16")).boxed())
}

/// Create a directory.
///
/// Create a single directory entry at the provided path with the supplied mode bits.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses mkdir(2) on Unix and CreateDirectoryW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_mkdir_bytes(
    _context: &RuntimeCallContext,
    path: PathBytes,
    mode: FileMode,
) -> RuntimeResult<()> {
    // create the directory on unix platforms
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let rc = unsafe { libc::mkdir(c_path.as_ptr(), mode.0 as libc::mode_t) };
    if rc == 0 {
        return Ok(());
    }

    Err(core_platform::io_error("mkdir", None))
}

/// Create a directory.
///
/// Create a single directory entry at the provided path with the supplied mode bits.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses mkdir(2) on Unix and CreateDirectoryW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_mkdir_utf16(
    _context: &RuntimeCallContext,
    path: PathUtf16,
    mode: FileMode,
) -> RuntimeResult<()> {
    // report unsupported mkdir calls on non-windows platforms
    let _ = (path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdirUtf16")).boxed())
}

/// Create a directory relative to a directory handle.
///
/// Create a single directory entry relative to an existing directory descriptor.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses mkdirat(2) on Unix and handle-relative directory create on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_mkdirat_bytes(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    mode: FileMode,
) -> RuntimeResult<()> {
    // create the directory on unix platforms
    let resource = directory_resource(context, dir)?;
    let path = resolve_path_bytes_cstring(path, "path")?;
    let result = unsafe { libc::mkdirat(resource.fd, path.as_ptr(), mode.0 as libc::mode_t) };
    if result != 0 {
        return Err(core_platform::io_error("mkdirat", None));
    }
    Ok(())
}

/// Create a directory relative to a directory handle.
///
/// Create a single directory entry relative to an existing directory descriptor.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses mkdirat(2) on Unix and handle-relative directory create on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_mkdirat_utf16(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    mode: FileMode,
) -> RuntimeResult<()> {
    // report unsupported mkdirat calls on non-windows platforms
    let _ = (context, dir, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdiratUtf16")).boxed())
}

/// Create a directory.
///
/// Create a single directory entry at the provided path with the supplied mode bits.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses mkdir(2) on Unix and CreateDirectoryW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_mkdir(
    context: &RuntimeCallContext,
    path: OsPath,
    mode: FileMode,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_mkdir_bytes(context, path, mode) },
        |path| unsafe { destack_fs_mkdir_utf16(context, path, mode) },
    )
}

/// Create a directory relative to a directory handle.
///
/// Create a single directory entry relative to an existing directory descriptor.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses mkdirat(2) on Unix and handle-relative directory create on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_mkdirat(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: OsPath,
    mode: FileMode,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_mkdirat_bytes(context, dir, path, mode) },
        |path| unsafe { destack_fs_mkdirat_utf16(context, dir, path, mode) },
    )
}

/// Remove a directory.
///
/// Remove the target resource through a single host namespace operation with no runtime fallback path.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses rmdir(2) on Unix and RemoveDirectoryW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_rmdir(
    context: &RuntimeCallContext,
    path: OsPath,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_rmdir_bytes(context, path) },
        |path| unsafe { destack_fs_rmdir_utf16(context, path) },
    )
}

/// Read a single directory entry from an open directory handle.
///
/// Read at most one entry from the current directory cursor and advance the host iterator.
/// Callers can iterate deterministically by repeatedly invoking this operation until `hasEntry` is false.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses readdir(3) step on Unix and FindNextFileW step on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
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
                            name: core_fs::path_ref_from_bytes(empty_bytes),
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
                        name: core_fs::path_ref_from_bytes(name),
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

/// Reset an open directory handle to the first entry.
///
/// Reset the directory iteration cursor to the beginning of the stream.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses rewinddir(3) on Unix and enumeration reset on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
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

/// Start watching a path and return a watch handle.
///
/// Registers the path with the native watch backend and starts event delivery for the selected mask.
/// Event ordering and coalescing behavior are backend defined.
///
/// # Platform
/// Linux, BSD, macOS, and Windows.
/// Uses inotify on Linux, kqueue on BSD, FSEvents on macOS, and ReadDirectoryChangesW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.watch`.
///
/// # Replay
/// External, recordable.
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

/// Close a watch handle.
///
/// Unregisters the watch from the backend and releases associated runtime resources.
/// No further events are delivered after close succeeds.
///
/// # Platform
/// Linux, BSD, macOS, and Windows.
/// Uses backend specific handle close and unregister operations.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.watch`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_watch_close(
    context: &RuntimeCallContext,
    handle: WatchHandle,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement filesystem watcher close
    let _ = (context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.watchClose")).boxed())
}

/// Read a batch of events from a watch handle.
///
/// Reads available watch records from the backend queue and reports overflow explicitly when events were dropped.
/// Callers should treat `overflowed` as a signal to resynchronize state.
///
/// # Platform
/// Linux, BSD, macOS, and Windows.
/// Uses inotify event reads on Linux, kevent on BSD, FSEvents stream reads on macOS, and ReadDirectoryChangesW reads on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.watch`.
///
/// # Replay
/// External, recordable.
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

/// Start watching a path relative to a directory handle.
///
/// Resolves the path relative to the supplied directory and registers the resulting entry with the backend watcher.
/// Event ordering and coalescing behavior are backend defined.
///
/// # Platform
/// Linux, BSD, macOS, and Windows.
/// Uses inotify on Linux, kqueue on BSD, FSEvents on macOS, and ReadDirectoryChangesW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.watch`.
///
/// # Replay
/// External, recordable.
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
