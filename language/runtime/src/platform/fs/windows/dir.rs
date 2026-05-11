use windows_sys::Wdk::Storage::FileSystem::{FILE_CREATE, FILE_DIRECTORY_FILE};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_FILE_NOT_FOUND, ERROR_NO_MORE_FILES, HANDLE_FLAG_INHERIT,
    INVALID_HANDLE_VALUE, SetHandleInformation,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateDirectoryW, CreateFileW, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
    FILE_FLAG_BACKUP_SEMANTICS, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_READ_ATTRIBUTES,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FindClose, FindFirstFileW, FindNextFileW,
    OPEN_EXISTING, WIN32_FIND_DATAW,
};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::{
    DirectoryHandle, Dirent, DirentKind, DirentNext, DirentNextEnd, DirentNextEntry, FileMode,
    OsPath, PathBytes, PathUtf16, WatchBatch, WatchOptions, core as core_fs,
};
use crate::platform::resource::{
    ResourceEntry, ResourceFinalizer, ResourceId, ResourceKind, WatchHandle,
};
use crate::platform::{NativeArray, PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;
use std::sync::{Arc, Mutex};

/// Directory payload stored in the resource table.
struct DirectoryResource {
    /// Directory enumeration state.
    enumerator: Arc<Mutex<DirectoryEnumerator>>,
}

impl std::fmt::Debug for DirectoryResource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DirectoryResource")
            .finish_non_exhaustive()
    }
}

/// Directory enumeration state stored behind one handle-local mutex.
struct DirectoryEnumerator {
    /// Stable search pattern for the opened directory.
    search_pattern: Vec<u16>,
    /// Active `FindFirstFileW` handle when enumeration has started.
    find_handle: isize,
    /// Pending first or next result that has not yet been consumed.
    pending: Option<WIN32_FIND_DATAW>,
    /// Whether enumeration has started.
    started: bool,
    /// Whether enumeration has reached end-of-directory.
    finished: bool,
}

impl std::fmt::Debug for DirectoryEnumerator {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DirectoryEnumerator")
            .field("started", &self.started)
            .field("finished", &self.finished)
            .finish_non_exhaustive()
    }
}

impl DirectoryEnumerator {
    /// Create one directory enumerator from a stable search pattern.
    fn new(search_pattern: Vec<u16>) -> Self {
        Self {
            search_pattern,
            find_handle: INVALID_HANDLE_VALUE,
            pending: None,
            started: false,
            finished: false,
        }
    }
}

/// Finalizer that closes one directory handle and any active enumeration handle.
#[derive(Debug)]
struct DirectoryHandleFinalizer {
    /// Raw directory handle to close.
    handle: isize,
    /// Enumeration state whose find handle must be closed.
    enumerator: Arc<Mutex<DirectoryEnumerator>>,
}

impl DirectoryHandleFinalizer {
    /// Create one finalizer for a directory resource.
    fn new(handle: isize, enumerator: Arc<Mutex<DirectoryEnumerator>>) -> Self {
        Self { handle, enumerator }
    }
}

impl ResourceFinalizer for DirectoryHandleFinalizer {
    /// Close the active find handle and directory handle during finalization.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        let mut enumerator = match self.enumerator.lock() {
            Ok(enumerator) => enumerator,
            Err(poisoned) => poisoned.into_inner(),
        };

        close_directory_enumerator(&mut enumerator);

        unsafe {
            CloseHandle(self.handle);
        }
    }
}

/// Require one opened handle to describe a directory for `opendir`.
fn require_directory_handle(handle: isize) -> RuntimeResult<()> {
    // stat the opened handle and reject non-directory targets
    let stat = stat_from_handle(handle)?;
    if stat.mode.0 & libc::S_IFMT as u32 == libc::S_IFDIR as u32 {
        return Ok(());
    }

    Err(RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotDirectory),
        None,
        None,
        Some("opendir".to_string()),
        None,
        "path is not a directory",
    ))
    .boxed())
}

/// Close one active Windows directory enumerator handle.
fn close_directory_enumerator(enumerator: &mut DirectoryEnumerator) {
    if enumerator.find_handle != INVALID_HANDLE_VALUE {
        unsafe {
            FindClose(enumerator.find_handle);
        }
        enumerator.find_handle = INVALID_HANDLE_VALUE;
    }

    enumerator.pending = None;
}

/// Reset one directory enumerator to the unopened state.
fn reset_directory_enumerator(enumerator: &mut DirectoryEnumerator) {
    close_directory_enumerator(enumerator);
    enumerator.started = false;
    enumerator.finished = false;
}

/// Ensure one directory enumerator has started.
fn ensure_directory_enumerator_started(enumerator: &mut DirectoryEnumerator) -> RuntimeResult<()> {
    if enumerator.started {
        return Ok(());
    }

    let mut data = unsafe { std::mem::zeroed::<WIN32_FIND_DATAW>() };
    let find_handle = unsafe { FindFirstFileW(enumerator.search_pattern.as_ptr(), &mut data) };
    enumerator.started = true;

    if find_handle == INVALID_HANDLE_VALUE {
        let code = core_platform::last_error_code() as u32;
        if code == ERROR_NO_MORE_FILES || code == ERROR_FILE_NOT_FOUND {
            enumerator.finished = true;
            return Ok(());
        }

        return Err(last_os_error("FindFirstFileW", None));
    }

    enumerator.find_handle = find_handle;
    enumerator.pending = Some(data);

    Ok(())
}

/// Read the next raw directory result from one Windows enumerator.
fn next_directory_data(
    enumerator: &mut DirectoryEnumerator,
) -> RuntimeResult<Option<WIN32_FIND_DATAW>> {
    ensure_directory_enumerator_started(enumerator)?;
    if enumerator.finished {
        return Ok(None);
    }

    if let Some(data) = enumerator.pending.take() {
        return Ok(Some(data));
    }

    let mut data = unsafe { std::mem::zeroed::<WIN32_FIND_DATAW>() };
    let rc = unsafe { FindNextFileW(enumerator.find_handle, &mut data) };
    if rc != 0 {
        return Ok(Some(data));
    }

    let code = core_platform::last_error_code() as u32;
    if code == ERROR_NO_MORE_FILES {
        close_directory_enumerator(enumerator);
        enumerator.finished = true;
        return Ok(None);
    }

    Err(last_os_error("FindNextFileW", None))
}

/// Read the next visible entry from one Windows directory enumerator.
fn read_next_visible_dirent(
    binding: &BindingCallContext,
    enumerator: &mut DirectoryEnumerator,
) -> RuntimeResult<Option<Dirent>> {
    loop {
        let Some(data) = next_directory_data(enumerator)? else {
            return Ok(None);
        };

        let name_length = data
            .cFileName
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(data.cFileName.len());
        let is_dot = name_length == 1 && data.cFileName[0] == 0x2e;
        let is_dot_dot = name_length == 2 && data.cFileName[0] == 0x2e && data.cFileName[1] == 0x2e;
        if is_dot || is_dot_dot {
            continue;
        }

        let kind = dirent_kind_from_find_data(&data);
        let name = path_utf16_from_units(binding, &data.cFileName[..name_length]);
        return Ok(Some(Dirent {
            name: core_fs::path_ref_from_utf16(name),
            kind,
        }));
    }
}

/// Resolve a directory enumerator from one directory handle.
fn directory_enumerator(
    binding: &BindingCallContext,
    handle: DirectoryHandle,
) -> RuntimeResult<Arc<Mutex<DirectoryEnumerator>>> {
    // resolve the enumerator payload for this directory handle
    core_fs::require_resource(
        binding,
        handle.0,
        ResourceKind::Directory,
        "directory",
        |entry| {
            entry
                .payload_ref::<DirectoryResource>()
                .map(|directory| directory.enumerator.clone())
                .ok_or_else(|| {
                    RuntimeError::from(PlatformError::generic(
                        None,
                        "directory enumerator missing payload",
                    ))
                    .boxed()
                })
        },
    )
}

/// Classify one Windows directory entry using the native attribute payload.
fn dirent_kind_from_find_data(data: &WIN32_FIND_DATAW) -> DirentKind {
    // preserve symlink-like reparse points before collapsing to file or directory
    if data.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        && matches!(
            data.dwReserved0,
            REPARSE_TAG_SYMLINK | REPARSE_TAG_MOUNT_POINT
        )
    {
        return DirentKind::Symlink;
    }

    if data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
        return DirentKind::Directory;
    }

    DirentKind::File
}

/// Open a directory and return a handle.
pub(crate) unsafe fn destack_fs_opendir_bytes(
    binding: &BindingCallContext,
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
            FILE_READ_ATTRIBUTES,
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

    // reject non-directory targets explicitly
    if let Err(error) = require_directory_handle(handle) {
        unsafe {
            CloseHandle(handle);
        }
        return Err(error);
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
    let mut search_pattern = final_path_from_handle(handle)?;
    search_pattern.push('\\' as u16);
    search_pattern.push('*' as u16);
    search_pattern.push(0);
    let enumerator = Arc::new(Mutex::new(DirectoryEnumerator::new(search_pattern)));
    let directory = DirectoryResource {
        enumerator: enumerator.clone(),
    };
    let entry = ResourceEntry::new(ResourceKind::Directory)
        .with_handle(handle as _)
        .with_payload(directory)
        .with_finalizer(DirectoryHandleFinalizer::new(handle, enumerator));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));
    unsafe {
        *out = DirectoryHandle(resource_id);
    }

    Ok(())
}

/// Open a directory and return a handle.
pub(crate) unsafe fn destack_fs_opendir_utf16(
    binding: &BindingCallContext,
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
            FILE_READ_ATTRIBUTES,
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

    // reject non-directory targets explicitly
    if let Err(error) = require_directory_handle(handle) {
        unsafe {
            CloseHandle(handle);
        }
        return Err(error);
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
    let mut search_pattern = final_path_from_handle(handle)?;
    search_pattern.push('\\' as u16);
    search_pattern.push('*' as u16);
    search_pattern.push(0);
    let enumerator = Arc::new(Mutex::new(DirectoryEnumerator::new(search_pattern)));
    let directory = DirectoryResource {
        enumerator: enumerator.clone(),
    };
    let entry = ResourceEntry::new(ResourceKind::Directory)
        .with_handle(handle as _)
        .with_payload(directory)
        .with_finalizer(DirectoryHandleFinalizer::new(handle, enumerator));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));
    unsafe {
        *out = DirectoryHandle(resource_id);
    }

    Ok(())
}

/// Read directory entries from an open directory handle.
pub(crate) unsafe fn destack_fs_readdir(
    binding: &BindingCallContext,
    out: *mut NativeArray<Dirent>,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the shared directory enumerator
    let enumerator = directory_enumerator(binding, handle)?;
    let mut enumerator = enumerator.lock().map_err(|_| {
        RuntimeError::from(PlatformError::generic(
            None,
            "directory enumerator lock is poisoned",
        ))
        .boxed()
    })?;
    let mut entries = Vec::new();

    // consume the remaining native enumeration stream
    while let Some(entry) = read_next_visible_dirent(binding, &mut enumerator)? {
        entries.push(entry);
    }

    unsafe {
        *out = binding.store_array(entries);
    }
    Ok(())
}

/// Create a directory.
pub(crate) unsafe fn destack_fs_mkdir_bytes(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_fs_mkdir_utf16(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_fs_mkdirat_bytes(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    mode: FileMode,
) -> RuntimeResult<()> {
    // decode the path
    let pathbuf = pathbuf_from_bytes(path, "path")?;

    // use the absolute path variant when the path is fully qualified
    if pathbuf.is_absolute() {
        return unsafe { destack_fs_mkdir_bytes(binding, path, mode) };
    }

    // resolve the directory handle
    let root = directory_handle(binding, dir)?;
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
pub(crate) unsafe fn destack_fs_mkdirat_utf16(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    mode: FileMode,
) -> RuntimeResult<()> {
    // decode the path
    let pathbuf = pathbuf_from_utf16(path, "path")?;

    // use the absolute path variant when the path is fully qualified
    if pathbuf.is_absolute() {
        return unsafe { destack_fs_mkdir_utf16(binding, path, mode) };
    }

    // resolve the directory handle
    let root = directory_handle(binding, dir)?;
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
pub(crate) unsafe fn destack_fs_rmdir_bytes(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_fs_rmdir_utf16(
    _binding: &BindingCallContext,
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

/// Open a directory and return a handle.
pub(crate) unsafe fn destack_fs_opendir(
    binding: &BindingCallContext,
    out: *mut DirectoryHandle,
    path: OsPath,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_opendir_bytes(binding, out, path) },
        |path| unsafe { destack_fs_opendir_utf16(binding, out, path) },
    )
}

/// Read a single directory entry from an open directory handle.
pub(crate) unsafe fn destack_fs_readdir_next(
    binding: &BindingCallContext,
    out: *mut DirentNext,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the shared directory enumerator
    let enumerator = directory_enumerator(binding, handle)?;
    let mut enumerator = enumerator.lock().map_err(|_| {
        RuntimeError::from(PlatformError::generic(
            None,
            "directory enumerator lock is poisoned",
        ))
        .boxed()
    })?;

    // emit one entry or mark end-of-directory on the shared enumerator
    if let Some(entry) = read_next_visible_dirent(binding, &mut enumerator)? {
        unsafe {
            *out = DirentNext::DirentNextEntry(DirentNextEntry {
                kind: binding.store_string("entry"),
                entry,
            });
        }
    } else {
        unsafe {
            *out = DirentNext::DirentNextEnd(DirentNextEnd {
                kind: binding.store_string("end"),
            });
        }
    }

    Ok(())
}

/// Reset an open directory handle to the first entry.
pub(crate) unsafe fn destack_fs_rewinddir(
    binding: &BindingCallContext,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    // resolve and reset the shared directory enumerator
    let enumerator = directory_enumerator(binding, handle)?;
    let mut enumerator = enumerator.lock().map_err(|_| {
        RuntimeError::from(PlatformError::generic(
            None,
            "directory enumerator lock is poisoned",
        ))
        .boxed()
    })?;
    reset_directory_enumerator(&mut enumerator);

    Ok(())
}

/// Start watching a path and return a watch handle.
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
        |bytes| pathbuf_from_bytes(bytes, "path"),
        |utf16| pathbuf_from_utf16(utf16, "path"),
    )?;
    let handle = core_fs::open_watch(binding, &watch_path, options)?;

    // store the returned watch handle
    unsafe {
        *out = handle;
    }

    Ok(())
}

/// Close a watch handle.
pub(crate) unsafe fn destack_fs_watch_close(
    binding: &BindingCallContext,
    handle: WatchHandle,
) -> RuntimeResult<()> {
    core_fs::close_watch(binding, handle)
}

/// Read a batch of events from a watch handle.
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
    let directory_path = directory_path(binding, directory)?;

    // decode the path and resolve it relative to the directory
    let watch_path = core_fs::with_path_ref(
        path,
        "path",
        |bytes| pathbuf_from_bytes(bytes, "path"),
        |utf16| pathbuf_from_utf16(utf16, "path"),
    )?;
    let watch_path = if watch_path.is_absolute() {
        watch_path
    } else {
        directory_path.join(watch_path)
    };
    let handle = core_fs::open_watch(binding, &watch_path, options)?;

    // store the returned watch handle
    unsafe {
        *out = handle;
    }

    Ok(())
}
