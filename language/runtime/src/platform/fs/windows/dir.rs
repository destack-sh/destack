use windows_sys::Wdk::Storage::FileSystem::{FILE_CREATE, FILE_DIRECTORY_FILE};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_NO_MORE_FILES, HANDLE_FLAG_INHERIT, INVALID_HANDLE_VALUE,
    SetHandleInformation,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateDirectoryW, CreateFileW, FILE_ATTRIBUTE_DIRECTORY, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    FindClose, FindFirstFileW, FindNextFileW, OPEN_EXISTING, WIN32_FIND_DATAW,
};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::{
    DirectoryHandle, Dirent, DirentKind, DirentNext, FileMode, OsPath, PathBytes, PathUtf16,
    WatchBatch, WatchOptions, core as core_fs,
};
use crate::platform::resource::{ResourceEntry, ResourceKind, WatchHandle};
use crate::platform::{NativeArray, PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;

/// Open a directory and return a handle.
///
/// Open the target resource with the requested flags and return the host handle exposed by the kernel.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses opendir/readdir on Unix and FindFirstFileW directory enumeration on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_opendir_bytes(
    context: &BindingCallContext,
    out: *mut DirectoryHandle,
    path: PathBytes,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the path
    let wide = wide_from_bytes(path, "path")?;

    // open the directory handle
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            0,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(last_os_error("CreateFileW", None));
    }

    // disable handle inheritance
    let rc = unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0) };
    if rc == 0 {
        unsafe {
            CloseHandle(handle);
        }
        return Err(last_os_error("SetHandleInformation", None));
    }

    // register the resource handle
    let entry = ResourceEntry::new(ResourceKind::Directory)
        .with_handle(handle as _)
        .with_finalizer(HandleFinalizer::new(handle));
    let resource_id = context.runtime().resources.insert(entry);
    unsafe {
        *out = DirectoryHandle(resource_id);
    }

    Ok(())
}

/// Open a directory and return a handle.
///
/// Open the target resource with the requested flags and return the host handle exposed by the kernel.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses opendir/readdir on Unix and FindFirstFileW directory enumeration on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_opendir_utf16(
    context: &BindingCallContext,
    out: *mut DirectoryHandle,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the path
    let wide = wide_from_utf16(path, "path")?;

    // open the directory handle
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            0,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(last_os_error("CreateFileW", None));
    }

    // disable handle inheritance
    let rc = unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0) };
    if rc == 0 {
        unsafe {
            CloseHandle(handle);
        }
        return Err(last_os_error("SetHandleInformation", None));
    }

    // register the resource handle
    let entry = ResourceEntry::new(ResourceKind::Directory)
        .with_handle(handle as _)
        .with_finalizer(HandleFinalizer::new(handle));
    let resource_id = context.runtime().resources.insert(entry);
    unsafe {
        *out = DirectoryHandle(resource_id);
    }

    Ok(())
}

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
    context: &BindingCallContext,
    out: *mut NativeArray<Dirent>,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the directory handle
    let handle = directory_handle(context, handle)?;
    let mut entries = Vec::new();
    let mut search: Vec<u16> = final_path_from_handle(handle)?;
    search.push('\\' as u16);
    search.push('*' as u16);
    search.push(0);

    // seed the search
    let mut data = unsafe { std::mem::zeroed::<WIN32_FIND_DATAW>() };
    let find = unsafe { FindFirstFileW(search.as_ptr(), &mut data) };
    if find == INVALID_HANDLE_VALUE {
        return Err(last_os_error("FindFirstFileW", None));
    }

    // walk directory entries
    loop {
        let name_len = data
            .cFileName
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(data.cFileName.len());
        let is_dot = name_len == 1 && data.cFileName[0] == 0x2e;
        let is_dot_dot = name_len == 2 && data.cFileName[0] == 0x2e && data.cFileName[1] == 0x2e;
        if !(is_dot || is_dot_dot) {
            let kind = if data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
                DirentKind::Directory
            } else {
                DirentKind::File
            };
            let name = path_utf16_from_units(context, &data.cFileName[..name_len]);
            entries.push(Dirent {
                name: core_fs::path_ref_from_utf16(name),
                kind,
            });
        }

        // advance to the next entry
        let rc = unsafe { FindNextFileW(find, &mut data) };
        if rc == 0 {
            let code = core_platform::last_error_code() as u32;
            if code == ERROR_NO_MORE_FILES {
                break;
            }
            unsafe {
                FindClose(find);
            }
            return Err(last_os_error("FindNextFileW", None));
        }
    }

    // store the entries
    unsafe {
        FindClose(find);
        *out = context.store_array(entries);
    }
    Ok(())
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
    _context: &BindingCallContext,
    path: PathBytes,
    _mode: FileMode,
) -> RuntimeResult<()> {
    // decode the path
    let wide = wide_from_bytes(path, "path")?;

    // create the directory
    let rc = unsafe { CreateDirectoryW(wide.as_ptr(), std::ptr::null()) };
    if rc == 0 {
        return Err(last_os_error("CreateDirectoryW", None));
    }
    Ok(())
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
    _context: &BindingCallContext,
    path: PathUtf16,
    _mode: FileMode,
) -> RuntimeResult<()> {
    // decode the path
    let wide = wide_from_utf16(path, "path")?;

    // create the directory
    let rc = unsafe { CreateDirectoryW(wide.as_ptr(), std::ptr::null()) };
    if rc == 0 {
        return Err(last_os_error("CreateDirectoryW", None));
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
pub(crate) unsafe fn destack_fs_mkdirat_bytes(
    context: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    mode: FileMode,
) -> RuntimeResult<()> {
    // decode the path
    let pathbuf = pathbuf_from_bytes(path, "path")?;

    // use the absolute path variant when the path is fully qualified
    if pathbuf.is_absolute() {
        return unsafe { destack_fs_mkdir_bytes(context, path, mode) };
    }

    // resolve the directory handle
    let root = directory_handle(context, dir)?;
    let path = wide_from_pathbuf_no_nul(&pathbuf);

    // create the directory
    let handle = nt_create_file_at(
        root,
        &path,
        FILE_GENERIC_READ | FILE_GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        FILE_CREATE,
        FILE_DIRECTORY_FILE,
        attributes_from_mode(mode),
    )?;

    // close the handle before returning
    unsafe {
        CloseHandle(handle);
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
    context: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    mode: FileMode,
) -> RuntimeResult<()> {
    // decode the path
    let pathbuf = pathbuf_from_utf16(path, "path")?;

    // use the absolute path variant when the path is fully qualified
    if pathbuf.is_absolute() {
        return unsafe { destack_fs_mkdir_utf16(context, path, mode) };
    }

    // resolve the directory handle
    let root = directory_handle(context, dir)?;
    let path = wide_from_pathbuf_no_nul(&pathbuf);

    // create the directory
    let handle = nt_create_file_at(
        root,
        &path,
        FILE_GENERIC_READ | FILE_GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        FILE_CREATE,
        FILE_DIRECTORY_FILE,
        attributes_from_mode(mode),
    )?;

    // close the handle before returning
    unsafe {
        CloseHandle(handle);
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
    _context: &BindingCallContext,
    path: PathBytes,
) -> RuntimeResult<()> {
    // decode the path
    let wide = wide_from_bytes(path, "path")?;

    // open the path for deletion
    let handle = open_for_delete_path(&wide)?;
    let result = set_disposition_info(handle);

    // close the handle before returning
    unsafe {
        CloseHandle(handle);
    }
    result
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
    _context: &BindingCallContext,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // decode the path
    let wide = wide_from_utf16(path, "path")?;

    // open the path for deletion
    let handle = open_for_delete_path(&wide)?;
    let result = set_disposition_info(handle);

    // close the handle before returning
    unsafe {
        CloseHandle(handle);
    }
    result
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
    context: &BindingCallContext,
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
    context: &BindingCallContext,
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
    context: &BindingCallContext,
    path: OsPath,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_rmdir_bytes(context, path) },
        |path| unsafe { destack_fs_rmdir_utf16(context, path) },
    )
}

/// Open a directory and return a handle.
///
/// Open the target resource with the requested flags and return the host handle exposed by the kernel.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses opendir/readdir on Unix and FindFirstFileW directory enumeration on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_opendir(
    context: &BindingCallContext,
    out: *mut DirectoryHandle,
    path: OsPath,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_opendir_bytes(context, out, path) },
        |path| unsafe { destack_fs_opendir_utf16(context, out, path) },
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
    context: &BindingCallContext,
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
    context: &BindingCallContext,
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
    context: &BindingCallContext,
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
    context: &BindingCallContext,
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
    context: &BindingCallContext,
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
    context: &BindingCallContext,
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
