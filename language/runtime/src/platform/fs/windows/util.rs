use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;

use windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES;
use windows_sys::Wdk::Storage::FileSystem::{
    FILE_CREATE, FILE_OPEN, FILE_OPEN_IF, FILE_OVERWRITE, FILE_OVERWRITE_IF, NtCreateFile,
};
use windows_sys::Win32::Foundation::{
    CloseHandle, FILETIME, HANDLE, INVALID_HANDLE_VALUE, NTSTATUS, RtlNtStatusToDosError,
    UNICODE_STRING,
};
use windows_sys::Win32::Storage::FileSystem::{
    CREATE_ALWAYS, CREATE_NEW, CreateFileW, FILE_APPEND_DATA, FILE_ATTRIBUTE_DIRECTORY,
    FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_READONLY, FILE_DISPOSITION_INFO,
    FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ,
    FILE_GENERIC_WRITE, FILE_READ_ATTRIBUTES, FILE_RENAME_INFO, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, FILE_WRITE_ATTRIBUTES, FileDispositionInfo, FileRenameInfo,
    GetDiskFreeSpaceExW, GetDiskFreeSpaceW, GetFileInformationByHandle, GetFinalPathNameByHandleW,
    GetVolumeInformationW, GetVolumePathNameW, MAXIMUM_REPARSE_DATA_BUFFER_SIZE, OPEN_ALWAYS,
    OPEN_EXISTING, SetFileInformationByHandle, SetFileTime, TRUNCATE_EXISTING,
};
use windows_sys::Win32::System::IO::{DeviceIoControl, IO_STATUS_BLOCK};
use windows_sys::Win32::System::Ioctl::FSCTL_GET_REPARSE_POINT;
use windows_sys::Win32::System::Kernel::OBJ_CASE_INSENSITIVE;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::{
    FileMode, FileSize, OpenFlags, PathBytes, PathUtf16, Stat, StatFs, core as core_fs,
};
use crate::platform::resource::{DirectoryHandle, FileHandle, ResourceFinalizer, ResourceKind};
use crate::platform::{PlatformError, ResourceId};
use crate::runtime::RuntimeCallContext;

/// Random characters used for mkdtemp suffixes.
pub(super) const MKDTEMP_CHARS: &[u8; 62] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
/// AtFlags value for nofollow behavior.
pub(super) const AT_SYMLINK_NOFOLLOW: u32 = 0x100;
/// AtFlags value for directory removal.
pub(super) const AT_REMOVEDIR: u32 = 0x200;
/// Access right used for delete operations.
pub(super) const DELETE_ACCESS: u32 = 0x0001_0000;
/// Reparse tag for symlinks.
pub(super) const REPARSE_TAG_SYMLINK: u32 = 0xA000000C;
/// Reparse tag for mount points.
pub(super) const REPARSE_TAG_MOUNT_POINT: u32 = 0xA0000003;

/// Finalizer that closes a raw handle.
#[derive(Debug)]
pub(super) struct HandleFinalizer {
    /// Raw handle to close.
    handle: HANDLE,
}

impl HandleFinalizer {
    /// Create a handle finalizer for a raw handle.
    pub(super) fn new(handle: HANDLE) -> Self {
        Self { handle }
    }
}

impl ResourceFinalizer for HandleFinalizer {
    /// Close the handle when the resource is finalized.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

/// Resolve a UTF-8 byte path into a wide string.
pub(super) fn wide_from_bytes(path: PathBytes, name: &str) -> RuntimeResult<Vec<u16>> {
    let bytes = unsafe { path.0.as_slice()? };
    if bytes.contains(&0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            name,
            "path contains nul byte",
        ))
        .boxed());
    }

    let value = std::str::from_utf8(bytes).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            name,
            "path contains invalid utf8",
        ))
        .boxed()
    })?;
    let mut wide: Vec<u16> = OsString::from(value).as_os_str().encode_wide().collect();
    if wide.contains(&0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            name,
            "path contains nul code unit",
        ))
        .boxed());
    }
    wide.push(0);
    Ok(wide)
}

/// Resolve a UTF-16 path into a wide string.
pub(super) fn wide_from_utf16(path: PathUtf16, name: &str) -> RuntimeResult<Vec<u16>> {
    let slice = unsafe { path.0.as_slice()? };
    if slice.contains(&0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            name,
            "path contains nul code unit",
        ))
        .boxed());
    }
    let mut wide = slice.to_vec();
    wide.push(0);
    Ok(wide)
}

/// Resolve a UTF-16 path into a PathBuf.
pub(super) fn pathbuf_from_utf16(path: PathUtf16, name: &str) -> RuntimeResult<PathBuf> {
    let slice = unsafe { path.0.as_slice()? };
    if slice.contains(&0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            name,
            "path contains nul code unit",
        ))
        .boxed());
    }

    Ok(PathBuf::from(OsString::from_wide(slice)))
}

/// Resolve a UTF-8 byte path into a PathBuf.
pub(super) fn pathbuf_from_bytes(path: PathBytes, name: &str) -> RuntimeResult<PathBuf> {
    let bytes = unsafe { path.0.as_slice()? };
    if bytes.contains(&0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            name,
            "path contains nul byte",
        ))
        .boxed());
    }

    let value = std::str::from_utf8(bytes).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            name,
            "path contains invalid utf8",
        ))
        .boxed()
    })?;
    Ok(PathBuf::from(value))
}

/// Resolve a UTF-8 byte path into a String.
pub(super) fn string_from_bytes(path: PathBytes, name: &str) -> RuntimeResult<String> {
    let bytes = unsafe { path.0.as_slice()? };
    if bytes.contains(&0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            name,
            "path contains nul byte",
        ))
        .boxed());
    }

    let value = std::str::from_utf8(bytes).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            name,
            "path contains invalid utf8",
        ))
        .boxed()
    })?;
    Ok(value.to_string())
}

/// Generate a random mkdtemp suffix.
pub(super) fn mkdtemp_suffix() -> RuntimeResult<[u8; 6]> {
    let mut bytes = [0u8; 6];
    getrandom::fill(&mut bytes)
        .map_err(|error| RuntimeError::from(PlatformError::io(error.to_string())).boxed())?;
    for byte in &mut bytes {
        *byte = MKDTEMP_CHARS[(*byte as usize) % MKDTEMP_CHARS.len()];
    }
    Ok(bytes)
}

/// Create a temporary directory from a template string.
pub(super) fn mkdtemp_from_template(template: &str) -> RuntimeResult<PathBuf> {
    let index = template.rfind("XXXXXX").ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "template",
            "template must contain XXXXXX",
        ))
        .boxed()
    })?;

    let prefix = &template[..index];
    let suffix = &template[index + 6..];
    for _ in 0..128 {
        let suffix_bytes = mkdtemp_suffix()?;
        let random: String = suffix_bytes.iter().map(|value| *value as char).collect();
        let candidate = format!("{prefix}{random}{suffix}");
        match std::fs::create_dir(&candidate) {
            Ok(()) => return Ok(PathBuf::from(candidate)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(RuntimeError::from(PlatformError::io(error.to_string())).boxed());
            }
        }
    }

    Err(RuntimeError::from(PlatformError::io("mkdtemp could not find a unique name")).boxed())
}

/// Convert a PathBuf into UTF-8 bytes.
pub(super) fn bytes_from_pathbuf(path: &PathBuf, name: &str) -> RuntimeResult<Vec<u8>> {
    let value = path.to_str().ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            name,
            "path contains invalid utf8",
        ))
        .boxed()
    })?;
    Ok(value.as_bytes().to_vec())
}

/// Convert a PathBuf into a wide string.
pub(super) fn wide_from_pathbuf(path: &PathBuf) -> Vec<u16> {
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    wide
}

/// Convert a PathBuf into a wide string without a trailing nul.
pub(super) fn wide_from_pathbuf_no_nul(path: &PathBuf) -> Vec<u16> {
    let mut wide = wide_from_pathbuf(path);
    if wide.last() == Some(&0) {
        wide.pop();
    }
    wide
}

/// Build a UNICODE_STRING for NtCreateFile.
pub(super) fn unicode_string_from_slice(path: &[u16]) -> RuntimeResult<UNICODE_STRING> {
    let length_bytes = path
        .len()
        .checked_mul(2)
        .ok_or_else(|| RuntimeError::from(PlatformError::io("path too long")).boxed())?;
    let length_bytes = u16::try_from(length_bytes)
        .map_err(|_| RuntimeError::from(PlatformError::io("path too long")).boxed())?;

    Ok(UNICODE_STRING {
        Length: length_bytes,
        MaximumLength: length_bytes,
        Buffer: path.as_ptr() as *mut u16,
    })
}

/// Map open flags to NtCreateFile dispositions.
pub(super) fn nt_disposition_from_flags(flags: OpenFlags) -> u32 {
    let flags = flags.0;
    let create = flags & libc::O_CREAT as u32 != 0;
    let excl = flags & libc::O_EXCL as u32 != 0;
    let trunc = flags & libc::O_TRUNC as u32 != 0;
    if create && excl {
        return FILE_CREATE;
    }
    if create && trunc {
        return FILE_OVERWRITE_IF;
    }
    if create {
        return FILE_OPEN_IF;
    }
    if trunc {
        return FILE_OVERWRITE;
    }
    FILE_OPEN
}

/// Create a handle relative to a directory using NtCreateFile.
pub(super) fn nt_create_file_at(
    root: HANDLE,
    path: &[u16],
    desired_access: u32,
    share_mode: u32,
    disposition: u32,
    options: u32,
    file_attributes: u32,
) -> RuntimeResult<HANDLE> {
    let unicode = unicode_string_from_slice(path)?;
    let mut object_attributes = OBJECT_ATTRIBUTES {
        Length: std::mem::size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: root,
        ObjectName: &unicode as *const _,
        Attributes: OBJ_CASE_INSENSITIVE as u32,
        SecurityDescriptor: std::ptr::null(),
        SecurityQualityOfService: std::ptr::null(),
    };

    let mut iosb = IO_STATUS_BLOCK {
        Anonymous: windows_sys::Win32::System::IO::IO_STATUS_BLOCK_0 { Status: 0 },
        Information: 0,
    };
    let mut handle = INVALID_HANDLE_VALUE;
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            desired_access,
            &mut object_attributes,
            &mut iosb,
            std::ptr::null(),
            file_attributes,
            share_mode,
            disposition,
            options,
            std::ptr::null(),
            0,
        )
    };
    if status != 0 {
        let code = unsafe { RtlNtStatusToDosError(status as NTSTATUS) } as i32;
        let message = format!("NtCreateFile failed: {code}");
        return Err(RuntimeError::from(PlatformError::io_with(
            None,
            None,
            Some(code),
            Some("NtCreateFile".to_string()),
            None,
            message,
        ))
        .boxed());
    }
    Ok(handle)
}

/// Update rename information for an open handle.
pub(super) fn set_rename_info(handle: HANDLE, root: HANDLE, name: &[u16]) -> RuntimeResult<()> {
    let name_len_bytes = name
        .len()
        .checked_mul(2)
        .ok_or_else(|| RuntimeError::from(PlatformError::io("path too long")).boxed())?;
    let name_len_bytes = u32::try_from(name_len_bytes)
        .map_err(|_| RuntimeError::from(PlatformError::io("path too long")).boxed())?;
    let buffer_len =
        std::mem::size_of::<FILE_RENAME_INFO>() + (name_len_bytes as usize).saturating_sub(2);
    let mut buffer = vec![0u8; buffer_len];
    let info = buffer.as_mut_ptr() as *mut FILE_RENAME_INFO;
    unsafe {
        (*info).Anonymous.ReplaceIfExists = 1;
        (*info).RootDirectory = root;
        (*info).FileNameLength = name_len_bytes;
        let target = (*info).FileName.as_mut_ptr();
        std::ptr::copy_nonoverlapping(name.as_ptr(), target, name.len());
    }

    let rc = unsafe {
        SetFileInformationByHandle(handle, FileRenameInfo, info as *const _, buffer_len as u32)
    };
    if rc == 0 {
        return Err(last_os_error("SetFileInformationByHandle", None));
    }
    Ok(())
}

/// Mark a handle for deletion.
pub(super) fn set_disposition_info(handle: HANDLE) -> RuntimeResult<()> {
    let info = FILE_DISPOSITION_INFO { DeleteFile: 1 };
    let rc = unsafe {
        SetFileInformationByHandle(
            handle,
            FileDispositionInfo,
            &info as *const _ as *const _,
            std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
        )
    };
    if rc == 0 {
        return Err(last_os_error("SetFileInformationByHandle", None));
    }
    Ok(())
}

/// Open a path for delete operations.
pub(super) fn open_for_delete_path(path: &[u16]) -> RuntimeResult<HANDLE> {
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            DELETE_ACCESS,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
            0,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(last_os_error("CreateFileW", None));
    }
    Ok(handle)
}

/// Open a path for reparse inspection.
pub(super) fn open_for_reparse(path: &[u16]) -> RuntimeResult<HANDLE> {
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
            0,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(last_os_error("CreateFileW", None));
    }
    Ok(handle)
}

/// Read a symlink target from a handle.
pub(super) fn readlink_from_handle(handle: HANDLE) -> RuntimeResult<String> {
    let mut buffer = vec![0u8; MAXIMUM_REPARSE_DATA_BUFFER_SIZE as usize];
    let mut returned = 0u32;
    let rc = unsafe {
        DeviceIoControl(
            handle,
            FSCTL_GET_REPARSE_POINT,
            std::ptr::null_mut(),
            0,
            buffer.as_mut_ptr() as *mut _,
            buffer.len() as u32,
            &mut returned,
            std::ptr::null_mut(),
        )
    };
    if rc == 0 {
        return Err(last_os_error("DeviceIoControl", None));
    }
    buffer.truncate(returned as usize);

    let target = parse_reparse_target(&buffer)?;
    let value = String::from_utf16(&target).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "reparse target contains invalid utf16",
        ))
        .boxed()
    })?;
    Ok(normalize_reparse_target(value))
}

/// Parse a reparse buffer into a UTF-16 path.
pub(super) fn parse_reparse_target(buffer: &[u8]) -> RuntimeResult<Vec<u16>> {
    if buffer.len() < 8 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "reparse buffer too small",
        ))
        .boxed());
    }
    let tag = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
    let data_len = u16::from_le_bytes([buffer[4], buffer[5]]) as usize;
    let data_start = 8;
    let data_end = data_start + data_len;
    if buffer.len() < data_end {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "reparse buffer truncated",
        ))
        .boxed());
    }
    let data = &buffer[data_start..data_end];

    match tag {
        REPARSE_TAG_SYMLINK => parse_symlink_reparse(data),
        REPARSE_TAG_MOUNT_POINT => parse_mount_point_reparse(data),
        _ => Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readlink")).boxed()),
    }
}

/// Parse a symlink reparse buffer.
pub(super) fn parse_symlink_reparse(data: &[u8]) -> RuntimeResult<Vec<u16>> {
    if data.len() < 12 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "symlink reparse buffer too small",
        ))
        .boxed());
    }
    let substitute_offset = u16::from_le_bytes([data[0], data[1]]) as usize;
    let substitute_length = u16::from_le_bytes([data[2], data[3]]) as usize;
    let print_offset = u16::from_le_bytes([data[4], data[5]]) as usize;
    let print_length = u16::from_le_bytes([data[6], data[7]]) as usize;
    let path_data = &data[12..];
    let (offset, length) = if print_length > 0 {
        (print_offset, print_length)
    } else {
        (substitute_offset, substitute_length)
    };
    extract_utf16_path(path_data, offset, length)
}

/// Parse a mount point reparse buffer.
pub(super) fn parse_mount_point_reparse(data: &[u8]) -> RuntimeResult<Vec<u16>> {
    if data.len() < 8 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "mount point reparse buffer too small",
        ))
        .boxed());
    }
    let substitute_offset = u16::from_le_bytes([data[0], data[1]]) as usize;
    let substitute_length = u16::from_le_bytes([data[2], data[3]]) as usize;
    let print_offset = u16::from_le_bytes([data[4], data[5]]) as usize;
    let print_length = u16::from_le_bytes([data[6], data[7]]) as usize;
    let path_data = &data[8..];
    let (offset, length) = if print_length > 0 {
        (print_offset, print_length)
    } else {
        (substitute_offset, substitute_length)
    };
    extract_utf16_path(path_data, offset, length)
}

/// Extract a UTF-16 path from a byte buffer.
pub(super) fn extract_utf16_path(
    data: &[u8],
    offset: usize,
    length: usize,
) -> RuntimeResult<Vec<u16>> {
    let end = offset + length;
    if end > data.len() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "reparse path out of bounds",
        ))
        .boxed());
    }
    if offset % 2 != 0 || length % 2 != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "reparse path misaligned",
        ))
        .boxed());
    }
    let slice = &data[offset..end];
    let mut out = Vec::with_capacity(slice.len() / 2);
    for chunk in slice.chunks_exact(2) {
        out.push(u16::from_le_bytes([chunk[0], chunk[1]]));
    }
    Ok(out)
}

/// Normalize the reparse target prefix.
pub(super) fn normalize_reparse_target(value: String) -> String {
    if let Some(stripped) = value.strip_prefix(r"\\?\\") {
        return stripped.to_string();
    }
    if let Some(stripped) = value.strip_prefix(r"\\??\\") {
        return stripped.to_string();
    }
    value
}

/// Build a runtime error from the last OS error.
pub(super) fn last_os_error(syscall: &str, path: Option<&str>) -> Box<RuntimeError> {
    let error = std::io::Error::last_os_error();
    let errno = error.raw_os_error();
    let message = format!("{syscall} failed: {error}");
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        errno,
        Some(syscall.to_string()),
        path.map(|path| path.to_string()),
        message,
    ))
    .boxed()
}

/// Convert a Windows wide buffer into a String.
pub(super) fn string_from_wide(buffer: &[u16], name: &str) -> RuntimeResult<String> {
    String::from_utf16(buffer).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            name,
            "path contains invalid utf16",
        ))
        .boxed()
    })
}

/// Resolve a resource entry for a file handle.
pub(super) fn file_handle(
    context: &RuntimeCallContext,
    handle: FileHandle,
) -> RuntimeResult<HANDLE> {
    let handle =
        core_fs::require_resource(context, handle.0, ResourceKind::File, "file", |entry| {
            entry.handle()
        })?;

    Ok(handle as HANDLE)
}

/// Resolve a resource entry for a directory handle.
pub(super) fn directory_handle(
    context: &RuntimeCallContext,
    handle: DirectoryHandle,
) -> RuntimeResult<HANDLE> {
    let handle = core_fs::require_resource(
        context,
        handle.0,
        ResourceKind::Directory,
        "directory",
        |entry| entry.handle(),
    )?;

    Ok(handle as HANDLE)
}

/// Resolve a directory handle into a PathBuf.
pub(super) fn directory_path(
    context: &RuntimeCallContext,
    handle: DirectoryHandle,
) -> RuntimeResult<PathBuf> {
    let handle = directory_handle(context, handle)?;
    let wide = final_path_from_handle(handle)?;
    Ok(PathBuf::from(OsString::from_wide(&wide)))
}

/// Map open flags to desired access.
pub(super) fn desired_access_from_flags(flags: OpenFlags) -> u32 {
    let flags = flags.0;
    let is_write = flags & libc::O_WRONLY as u32 != 0;
    let is_readwrite = flags & libc::O_RDWR as u32 != 0;
    let is_append = flags & libc::O_APPEND as u32 != 0;
    if is_append {
        return FILE_APPEND_DATA | FILE_GENERIC_READ;
    }
    if is_readwrite {
        return FILE_GENERIC_READ | FILE_GENERIC_WRITE;
    }
    if is_write {
        return FILE_GENERIC_WRITE;
    }
    FILE_GENERIC_READ
}

/// Map open flags to creation disposition.
pub(super) fn creation_from_flags(flags: OpenFlags) -> u32 {
    let flags = flags.0;
    let create = flags & libc::O_CREAT as u32 != 0;
    let excl = flags & libc::O_EXCL as u32 != 0;
    let trunc = flags & libc::O_TRUNC as u32 != 0;
    if create && excl {
        return CREATE_NEW;
    }
    if create && trunc {
        return CREATE_ALWAYS;
    }
    if create {
        return OPEN_ALWAYS;
    }
    if trunc {
        return TRUNCATE_EXISTING;
    }
    OPEN_EXISTING
}

/// Build the attribute flags for CreateFile.
pub(super) fn attributes_from_mode(mode: FileMode) -> u32 {
    let write_bits = mode.0 & 0o222;
    if write_bits == 0 {
        return FILE_ATTRIBUTE_READONLY;
    }
    FILE_ATTRIBUTE_NORMAL
}

/// Build a Stat struct from file information.
pub(super) fn stat_from_info(
    info: windows_sys::Win32::Storage::FileSystem::BY_HANDLE_FILE_INFORMATION,
) -> Stat {
    let ino = ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64;
    let nlink = info.nNumberOfLinks as u32;
    let mode = if info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
        libc::S_IFDIR as u32
    } else {
        libc::S_IFREG as u32
    };
    let size = ((info.nFileSizeHigh as u64) << 32) | info.nFileSizeLow as u64;
    let atime_ns = filetime_to_nanos(info.ftLastAccessTime);
    let mtime_ns = filetime_to_nanos(info.ftLastWriteTime);
    let birthtime_ns = filetime_to_nanos(info.ftCreationTime);

    Stat {
        dev: 0,
        ino,
        mode,
        nlink,
        uid: 0,
        gid: 0,
        rdev: 0,
        size: FileSize(size),
        blksize: 0,
        blocks: 0,
        atime_ns,
        mtime_ns,
        ctime_ns: birthtime_ns,
        birthtime_ns,
    }
}

/// Convert FILETIME to nanoseconds since the unix epoch.
pub(super) fn filetime_to_nanos(filetime: FILETIME) -> u64 {
    let ticks = ((filetime.dwHighDateTime as u64) << 32) | filetime.dwLowDateTime as u64;
    if ticks < 116444736000000000 {
        return 0;
    }
    (ticks - 116444736000000000) * 100
}

/// Convert nanoseconds since the unix epoch to FILETIME.
pub(super) fn filetime_from_nanos(nanos: u64) -> FILETIME {
    let ticks = (nanos / 100) + 116444736000000000u64;
    FILETIME {
        dwLowDateTime: ticks as u32,
        dwHighDateTime: (ticks >> 32) as u32,
    }
}

/// Set access and modification timestamps for a handle.
pub(super) fn set_handle_times(handle: HANDLE, atime_ns: u64, mtime_ns: u64) -> RuntimeResult<()> {
    let atime = filetime_from_nanos(atime_ns);
    let mtime = filetime_from_nanos(mtime_ns);
    let rc = unsafe { SetFileTime(handle, std::ptr::null(), &atime, &mtime) };
    if rc == 0 {
        return Err(last_os_error("SetFileTime", None));
    }
    Ok(())
}

/// Resolve a file handle from a handle and build Stat.
pub(super) fn stat_from_handle(handle: HANDLE) -> RuntimeResult<Stat> {
    let mut info = std::mem::MaybeUninit::uninit();
    let rc = unsafe { GetFileInformationByHandle(handle, info.as_mut_ptr()) };
    if rc == 0 {
        return Err(last_os_error("GetFileInformationByHandle", None));
    }
    let info = unsafe { info.assume_init() };
    Ok(stat_from_info(info))
}

/// Resolve a stat structure for a path.
pub(super) fn stat_from_path(path: &[u16], follow_symlink: bool) -> RuntimeResult<Stat> {
    let handle = open_for_metadata(path, follow_symlink)?;
    let stat = stat_from_handle(handle)?;
    unsafe {
        CloseHandle(handle);
    }
    Ok(stat)
}

/// Ensure a path is nul-terminated.
pub(super) fn ensure_wide_nul(path: &[u16]) -> Vec<u16> {
    if path.last() == Some(&0) {
        path.to_vec()
    } else {
        let mut wide = path.to_vec();
        wide.push(0);
        wide
    }
}

/// Resolve the volume root for a path.
pub(super) fn volume_path_from_path(path: &[u16]) -> RuntimeResult<Vec<u16>> {
    let path = ensure_wide_nul(path);
    let mut buffer = vec![0u16; 260];
    let rc = unsafe { GetVolumePathNameW(path.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32) };
    if rc == 0 {
        return Err(last_os_error("GetVolumePathNameW", None));
    }
    let len = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    buffer.truncate(len);
    buffer.push(0);
    Ok(buffer)
}

/// Resolve filesystem statistics for a path.
pub(super) fn statfs_from_path(path: &[u16]) -> RuntimeResult<StatFs> {
    let volume = volume_path_from_path(path)?;
    let mut sectors_per_cluster = 0u32;
    let mut bytes_per_sector = 0u32;
    let mut free_clusters = 0u32;
    let mut total_clusters = 0u32;
    let rc = unsafe {
        GetDiskFreeSpaceW(
            volume.as_ptr(),
            &mut sectors_per_cluster,
            &mut bytes_per_sector,
            &mut free_clusters,
            &mut total_clusters,
        )
    };
    if rc == 0 {
        return Err(last_os_error("GetDiskFreeSpaceW", None));
    }

    let bsize = (sectors_per_cluster as u64) * (bytes_per_sector as u64);
    let mut free_bytes = 0u64;
    let mut total_bytes = 0u64;
    let mut total_free = 0u64;
    let path = ensure_wide_nul(path);
    let rc = unsafe {
        GetDiskFreeSpaceExW(
            path.as_ptr(),
            &mut free_bytes,
            &mut total_bytes,
            &mut total_free,
        )
    };
    if rc == 0 {
        return Err(last_os_error("GetDiskFreeSpaceExW", None));
    }

    let mut serial = 0u32;
    let mut max_component = 0u32;
    let mut flags = 0u32;
    let mut volume_name = vec![0u16; 260];
    let mut fs_name = vec![0u16; 260];
    let rc = unsafe {
        GetVolumeInformationW(
            volume.as_ptr(),
            volume_name.as_mut_ptr(),
            volume_name.len() as u32,
            &mut serial,
            &mut max_component,
            &mut flags,
            fs_name.as_mut_ptr(),
            fs_name.len() as u32,
        )
    };
    if rc == 0 {
        return Err(last_os_error("GetVolumeInformationW", None));
    }

    let blocks = if bsize == 0 { 0 } else { total_bytes / bsize };
    let bfree = if bsize == 0 { 0 } else { total_free / bsize };
    let bavail = if bsize == 0 { 0 } else { free_bytes / bsize };

    Ok(StatFs {
        bsize,
        frsize: bsize,
        blocks,
        bfree,
        bavail,
        files: 0,
        ffree: 0,
        fsid: serial as u64,
        flags: flags as u64,
        namelen: max_component as u64,
    })
}

/// Convert a handle to a wide path using GetFinalPathNameByHandleW.
pub(super) fn final_path_from_handle(handle: HANDLE) -> RuntimeResult<Vec<u16>> {
    let mut buffer = vec![0u16; 512];
    loop {
        let len = unsafe {
            GetFinalPathNameByHandleW(handle, buffer.as_mut_ptr(), buffer.len() as u32, 0)
        };
        if len == 0 {
            return Err(last_os_error("GetFinalPathNameByHandleW", None));
        }
        if (len as usize) < buffer.len() {
            buffer.truncate(len as usize);
            break;
        }
        buffer.resize(len as usize + 1, 0);
    }
    Ok(buffer)
}

/// Open a handle for metadata operations.
pub(super) fn open_for_metadata(path: &[u16], follow_symlink: bool) -> RuntimeResult<HANDLE> {
    let mut flags = FILE_FLAG_BACKUP_SEMANTICS;
    if !follow_symlink {
        flags |= FILE_FLAG_OPEN_REPARSE_POINT;
    }
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            FILE_GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            flags,
            0,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(last_os_error("CreateFileW", None));
    }
    Ok(handle)
}

/// Open a handle for attribute updates.
pub(super) fn open_for_write_attributes(
    path: &[u16],
    follow_symlink: bool,
) -> RuntimeResult<HANDLE> {
    let mut flags = FILE_FLAG_BACKUP_SEMANTICS;
    if !follow_symlink {
        flags |= FILE_FLAG_OPEN_REPARSE_POINT;
    }
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            FILE_WRITE_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            flags,
            0,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(last_os_error("CreateFileW", None));
    }
    Ok(handle)
}
