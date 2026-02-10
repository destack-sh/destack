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
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{
    DirectoryHandle, Dirent, DirentKind, FileMode, PathBytes, PathUtf16, PathUtf16Abi,
    core as core_fs,
};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{NativeArray, PlatformError, core as core_platform};
use crate::runtime::RuntimeCallContext;

/// Open a directory with byte paths.
pub(crate) unsafe fn destack_fs_opendir_bytes(
    context: &RuntimeCallContext,
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

/// Open a directory with UTF-16 paths.
pub(crate) unsafe fn destack_fs_opendir_utf16(
    context: &RuntimeCallContext,
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

/// Read directory entries from a directory handle.
pub(crate) unsafe fn destack_fs_readdir(
    context: &RuntimeCallContext,
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
            let name =
                PathUtf16Abi::<NativeAbi>(context.store_array(data.cFileName[..name_len].to_vec()));
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

/// Create a directory with byte paths.
pub(crate) unsafe fn destack_fs_mkdir_bytes(
    _context: &RuntimeCallContext,
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

/// Create a directory with UTF-16 paths.
pub(crate) unsafe fn destack_fs_mkdir_utf16(
    _context: &RuntimeCallContext,
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

/// Create a directory relative to a directory handle with byte paths.
pub(crate) unsafe fn destack_fs_mkdirat_bytes(
    context: &RuntimeCallContext,
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

/// Create a directory relative to a directory handle with UTF-16 paths.
pub(crate) unsafe fn destack_fs_mkdirat_utf16(
    context: &RuntimeCallContext,
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

/// Remove a directory with byte paths.
pub(crate) unsafe fn destack_fs_rmdir_bytes(
    _context: &RuntimeCallContext,
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

/// Remove a directory with UTF-16 paths.
pub(crate) unsafe fn destack_fs_rmdir_utf16(
    _context: &RuntimeCallContext,
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
