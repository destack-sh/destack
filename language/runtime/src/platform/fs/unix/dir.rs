use super::core::*;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{core as core_fs, *};
use crate::platform::resource::*;
use crate::platform::{core as core_platform, *};
use crate::runtime::BindingCallContext;

use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;

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
    binding: &BindingCallContext,
    out: *mut NativeArray<Dirent>,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read directory entries on unix platforms
    let resource = directory_resource(binding, handle)?;
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
        let name = PathBytesAbi::<NativeAbi>(binding.store_array(name_bytes.to_vec()));
        dirents.push(Dirent {
            name: core_fs::path_ref_from_bytes(name),
            kind,
        });
    }

    let array = binding.store_array(dirents);
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
    _binding: &BindingCallContext,
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
    binding: &BindingCallContext,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // remove the directory by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| unsafe {
        destack_fs_rmdir_bytes(binding, path)
    })
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
    _binding: &BindingCallContext,
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
    binding: &BindingCallContext,
    path: PathUtf16,
    mode: FileMode,
) -> RuntimeResult<()> {
    // create the directory by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| unsafe {
        destack_fs_mkdir_bytes(binding, path, mode)
    })
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
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    mode: FileMode,
) -> RuntimeResult<()> {
    // create the directory on unix platforms
    let resource = directory_resource(binding, dir)?;
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
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    mode: FileMode,
) -> RuntimeResult<()> {
    // create the directory by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| unsafe {
        destack_fs_mkdirat_bytes(binding, dir, path, mode)
    })
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
    binding: &BindingCallContext,
    path: OsPath,
    mode: FileMode,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_mkdir_bytes(binding, path, mode) },
        |path| unsafe { destack_fs_mkdir_utf16(binding, path, mode) },
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
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: OsPath,
    mode: FileMode,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_mkdirat_bytes(binding, dir, path, mode) },
        |path| unsafe { destack_fs_mkdirat_utf16(binding, dir, path, mode) },
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
    binding: &BindingCallContext,
    path: OsPath,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_rmdir_bytes(binding, path) },
        |path| unsafe { destack_fs_rmdir_utf16(binding, path) },
    )
}

/// Read a single directory entry from an open directory handle.
///
/// Read at most one entry from the current directory cursor and advance the host iterator.
/// Callers can iterate deterministically by repeatedly invoking this operation until `entry` is void.
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
    binding: &BindingCallContext,
    out: *mut DirentNext,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the directory resource and its iteration cursor
    let resource = directory_resource(binding, handle)?;
    let mut cursor = resource.cursor.lock().map_err(|_| {
        RuntimeError::from(PlatformError::generic(
            None,
            "directory cursor lock is poisoned",
        ))
        .boxed()
    })?;
    let current_index = *cursor;

    // open a fresh directory descriptor so host offsets do not leak across calls
    let c_path = CString::new(resource.path.as_os_str().as_bytes()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "path contains nul byte",
        ))
        .boxed()
    })?;
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC;
    let duplicate_fd = unsafe { libc::open(c_path.as_ptr(), flags) };
    if duplicate_fd < 0 {
        return Err(core_platform::io_error(
            "open",
            Some(resource.path.to_string_lossy().as_ref()),
        ));
    }

    // open a transient directory stream for this read step
    let directory = unsafe { libc::fdopendir(duplicate_fd) };
    if directory.is_null() {
        unsafe {
            libc::close(duplicate_fd);
        }
        return Err(core_platform::io_error("fdopendir", None));
    }

    // close the transient stream automatically on all exits
    struct DirGuard(*mut libc::DIR);
    impl Drop for DirGuard {
        fn drop(&mut self) {
            unsafe {
                libc::closedir(self.0);
            }
        }
    }
    let _guard = DirGuard(directory);

    // walk entries until we reach the indexed cursor position
    let mut visible_index = 0_u64;
    loop {
        core_platform::set_errno(0);
        let entry = unsafe { libc::readdir(directory) };
        if entry.is_null() {
            if core_platform::get_errno() != 0 {
                return Err(core_platform::io_error(
                    "readdir",
                    Some(resource.path.to_string_lossy().as_ref()),
                ));
            }

            // mark end-of-directory
            unsafe {
                *out = DirentNext::DirentNextEnd(DirentNextEnd { kind: "end".into() });
            }
            return Ok(());
        }

        // filter dot and dot-dot entries from the visible index stream
        let name_pointer = unsafe { (*entry).d_name.as_ptr() };
        let name = unsafe { std::ffi::CStr::from_ptr(name_pointer) };
        let name_bytes = name.to_bytes();
        if name_bytes == b"." || name_bytes == b".." {
            continue;
        }

        // skip entries before the current cursor
        if visible_index < current_index {
            visible_index = visible_index.saturating_add(1);
            continue;
        }

        // emit one entry and advance the cursor index
        let mut kind = dirent_kind_from_type(unsafe { (*entry).d_type });
        if kind == DirentKind::Unknown {
            let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
            let rc = unsafe {
                libc::fstatat(
                    resource.fd,
                    name_pointer,
                    stat.as_mut_ptr(),
                    libc::AT_SYMLINK_NOFOLLOW,
                )
            };
            if rc == 0 {
                let stat = unsafe { stat.assume_init() };
                kind = dirent_kind_from_mode(stat.st_mode);
            }
        }

        let name = PathBytesAbi::<NativeAbi>(binding.store_array(name_bytes.to_vec()));
        *cursor = current_index.saturating_add(1);
        unsafe {
            *out = DirentNext::DirentNextEntry(DirentNextEntry {
                kind: "entry".into(),
                entry: Dirent {
                    name: core_fs::path_ref_from_bytes(name),
                    kind,
                },
            });
        }

        return Ok(());
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
    binding: &BindingCallContext,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    // resolve the directory resource and reset its cursor
    let resource = directory_resource(binding, handle)?;
    let mut cursor = resource.cursor.lock().map_err(|_| {
        RuntimeError::from(PlatformError::generic(
            None,
            "directory cursor lock is poisoned",
        ))
        .boxed()
    })?;
    *cursor = 0;

    Ok(())
}

/// Start watching a path and return a watch handle.
///
/// Registers the path with the native watch backend and starts event delivery for the selected mask.
/// Event ordering and coalescing behavior are backend defined.
///
/// # Platform
/// Unix and Windows.
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
    binding: &BindingCallContext,
    out: *mut WatchHandle,
    path: OsPath,
    options: WatchOptions,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the path and open the watch resource
    let watch_path = core_fs::with_path_ref(
        path,
        "path",
        |bytes| resolve_path_bytes(bytes, "path"),
        |_utf16| {
            Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "utf16 paths are not supported on unix",
            ))
            .boxed())
        },
    )?;
    let handle = core_fs::open_watch(binding, &watch_path, options)?;

    // store the returned watch handle
    unsafe {
        *out = handle;
    }

    Ok(())
}

/// Close a watch handle.
///
/// Unregisters the watch from the backend and releases associated runtime resources.
/// No further events are delivered after close succeeds.
///
/// # Platform
/// Unix and Windows.
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
    binding: &BindingCallContext,
    handle: WatchHandle,
) -> RuntimeResult<()> {
    core_fs::close_watch(binding, handle)
}

/// Read a batch of events from a watch handle.
///
/// Reads available watch records from the backend queue and reports overflow explicitly when events were dropped.
/// Callers should treat `overflowed` as a signal to resynchronize state.
///
/// # Platform
/// Unix and Windows.
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
    binding: &BindingCallContext,
    out: *mut WatchBatch,
    handle: WatchHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read one pending watch batch
    let batch = core_fs::read_watch(binding, handle)?;
    unsafe {
        *out = batch;
    }

    Ok(())
}

/// Start watching a path relative to a directory handle.
///
/// Resolves the path relative to the supplied directory and registers the resulting entry with the backend watcher.
/// Event ordering and coalescing behavior are backend defined.
///
/// # Platform
/// Unix and Windows.
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
    binding: &BindingCallContext,
    out: *mut WatchHandle,
    directory: DirectoryHandle,
    path: OsPath,
    options: WatchOptions,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the directory base path
    let directory = directory_resource(binding, directory)?;

    // decode the path and resolve it relative to the directory
    let watch_path = core_fs::with_path_ref(
        path,
        "path",
        |bytes| resolve_path_bytes(bytes, "path"),
        |_utf16| {
            Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "utf16 paths are not supported on unix",
            ))
            .boxed())
        },
    )?;
    let watch_path = if watch_path.is_absolute() {
        watch_path
    } else {
        directory.path.join(watch_path)
    };
    let handle = core_fs::open_watch(binding, &watch_path, options)?;

    // store the returned watch handle
    unsafe {
        *out = handle;
    }

    Ok(())
}
