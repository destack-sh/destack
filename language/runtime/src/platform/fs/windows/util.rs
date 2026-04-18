use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES;
use windows_sys::Wdk::Storage::FileSystem::{
    FILE_CREATE, FILE_OPEN, FILE_OPEN_IF, FILE_OVERWRITE, FILE_OVERWRITE_IF, NtCreateFile,
};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, FILETIME, HANDLE, HLOCAL, INVALID_HANDLE_VALUE, LocalFree,
    NTSTATUS, PSID, RtlNtStatusToDosError, UNICODE_STRING,
};
use windows_sys::Win32::Networking::WinSock::SOCKET;
use windows_sys::Win32::Security::Authorization::ConvertStringSidToSidW;
use windows_sys::Win32::Storage::FileSystem::{
    CREATE_ALWAYS, CREATE_NEW, CreateDirectoryW, CreateFileW, FILE_APPEND_DATA,
    FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_READONLY,
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO, FILE_BASIC_INFO, FILE_DISPOSITION_INFO,
    FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ,
    FILE_GENERIC_WRITE, FILE_READ_ATTRIBUTES, FILE_RENAME_INFO, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, FILE_WRITE_ATTRIBUTES, FileAttributeTagInfo, FileBasicInfo,
    FileDispositionInfo, FileRenameInfo, GetDiskFreeSpaceExW, GetDiskFreeSpaceW,
    GetFileInformationByHandle, GetFileInformationByHandleEx, GetFinalPathNameByHandleW,
    GetVolumeInformationW, GetVolumePathNameW, MAXIMUM_REPARSE_DATA_BUFFER_SIZE, OPEN_ALWAYS,
    OPEN_EXISTING, SetFileInformationByHandle, SetFileTime, TRUNCATE_EXISTING,
};
use windows_sys::Win32::System::IO::{DeviceIoControl, IO_STATUS_BLOCK};
use windows_sys::Win32::System::Ioctl::FSCTL_GET_REPARSE_POINT;
use windows_sys::Win32::System::Kernel::OBJ_CASE_INSENSITIVE;

use parking_lot::Mutex;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeAbi;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::{
    FileMode, FileSize, OpenFlags, PathBytes, PathUtf16, PathUtf16Abi, Stat, StatFs, StatFsFlags,
    StatusFlags, core as core_fs,
};
use crate::platform::net::{SocketHandle, core as core_net};
use crate::platform::resource::{
    DirectoryHandle, FileHandle, ResourceEntry, ResourceFinalizer, ResourceKind,
};
use crate::platform::{PlatformError, ResourceId, core as core_platform};
use crate::runtime::BindingCallContext;

/// Random characters used for mkdtemp suffixes.
pub(super) const MKDTEMP_CHARS: &[u8; 62] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
/// Required template suffix for mkdtemp-style directory creation.
pub(super) const MKDTEMP_TEMPLATE_SUFFIX: &str = "XXXXXX";
/// Maximum number of random-name attempts for mkdtemp emulation.
const MKDTEMP_MAX_ATTEMPTS: usize = 128;
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
/// Synthetic mode tag used for symlink-like reparse points.
pub(super) const WINDOWS_S_IFLNK_MODE: u32 = 0o120000;
/// Buffer capacity for legacy windows path APIs.
const WINDOWS_LEGACY_PATH_CAPACITY: usize = 260;
/// Initial buffer capacity for dynamic windows path APIs.
const WINDOWS_DYNAMIC_PATH_INITIAL_CAPACITY: usize = 512;

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
    // validate the byte path
    let bytes = unsafe { path.0.as_slice()? };
    if bytes.contains(&0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            name,
            "path contains nul byte",
        ))
        .boxed());
    }

    // decode utf-8 into a wide string
    core_platform::wide_from_utf8(name, bytes)
}

/// Resolve a UTF-16 path into a wide string.
pub(super) fn wide_from_utf16(path: PathUtf16, name: &str) -> RuntimeResult<Vec<u16>> {
    let units = utf16_units(path, name)?;
    core_platform::wide_from_utf16(name, &units)
}

/// Resolve a UTF-16 path into a PathBuf.
pub(super) fn pathbuf_from_utf16(path: PathUtf16, name: &str) -> RuntimeResult<PathBuf> {
    let units = utf16_units(path, name)?;
    core_platform::pathbuf_from_utf16(name, &units)
}

/// Resolve a UTF-8 byte path into a PathBuf.
pub(super) fn pathbuf_from_bytes(path: PathBytes, name: &str) -> RuntimeResult<PathBuf> {
    let bytes = unsafe { path.0.as_slice()? };
    core_platform::pathbuf_from_utf8(name, bytes)
}

/// Resolve a UTF-8 byte path into a String.
pub(super) fn string_from_bytes(path: PathBytes, name: &str) -> RuntimeResult<String> {
    let bytes = unsafe { path.0.as_slice()? };
    core_platform::string_from_utf8(name, bytes)
}

/// Generate a random mkdtemp suffix.
pub(super) fn mkdtemp_suffix() -> RuntimeResult<[u8; 6]> {
    // generate random bytes
    let mut bytes = [0u8; 6];
    getrandom::fill(&mut bytes)
        .map_err(|error| RuntimeError::from(PlatformError::io(error.to_string())).boxed())?;

    // map bytes into the allowed alphabet
    for byte in &mut bytes {
        *byte = MKDTEMP_CHARS[(*byte as usize) % MKDTEMP_CHARS.len()];
    }

    Ok(bytes)
}

/// Create a temporary directory from a template string.
pub(super) fn mkdtemp_from_template(template: &str) -> RuntimeResult<PathBuf> {
    // require the standard trailing suffix
    let prefix = template
        .strip_suffix(MKDTEMP_TEMPLATE_SUFFIX)
        .ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "template",
                "template must end with XXXXXX",
            ))
            .boxed()
        })?;

    // try to create a unique directory
    for _ in 0..MKDTEMP_MAX_ATTEMPTS {
        let suffix_bytes = mkdtemp_suffix()?;
        let random: String = suffix_bytes.iter().map(|value| *value as char).collect();
        let candidate = format!("{prefix}{random}");
        let mut wide: Vec<u16> = OsString::from(&candidate).encode_wide().collect();
        wide.push(0);
        let rc = unsafe { CreateDirectoryW(wide.as_ptr(), std::ptr::null()) };
        if rc != 0 {
            return Ok(PathBuf::from(candidate));
        }

        let error = core_platform::last_error_code() as u32;
        if error == ERROR_ALREADY_EXISTS {
            continue;
        }

        return Err(last_os_error("CreateDirectoryW", Some(&candidate)));
    }

    Err(RuntimeError::from(PlatformError::io("mkdtemp could not find a unique name")).boxed())
}

/// Convert a PathBuf into UTF-8 bytes.
pub(super) fn bytes_from_pathbuf(path: &Path, name: &str) -> RuntimeResult<Vec<u8>> {
    // decode the path as utf-8
    let value = path.to_str().ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            name,
            "path contains invalid utf8",
        ))
        .boxed()
    })?;

    // return the bytes
    Ok(value.as_bytes().to_vec())
}

/// Convert a PathBuf into a wide string.
pub(super) fn wide_from_pathbuf(path: &Path) -> Vec<u16> {
    // encode the path as a nul-terminated wide string
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    wide
}

/// Convert a PathBuf into a wide string without a trailing nul.
pub(super) fn wide_from_pathbuf_no_nul(path: &Path) -> Vec<u16> {
    // strip the trailing nul after encoding
    let mut wide = wide_from_pathbuf(path);
    if wide.last() == Some(&0) {
        wide.pop();
    }

    wide
}

/// Decode UTF-16 units from the path byte payload.
pub(super) fn utf16_units(path: PathUtf16, name: &str) -> RuntimeResult<Vec<u16>> {
    // decode raw utf16 storage
    let units = unsafe { path.0.as_slice()? };
    let units = units.to_vec();
    let _ = name;

    Ok(units)
}

/// Build a UTF-16 path payload from units.
pub(super) fn path_utf16_from_units(binding: &BindingCallContext, units: &[u16]) -> PathUtf16 {
    PathUtf16Abi::<NativeAbi>(binding.store_array_copy(units))
}

/// Build a UTF-16 path payload from a PathBuf.
pub(super) fn path_utf16_from_pathbuf(binding: &BindingCallContext, path: &Path) -> PathUtf16 {
    let units: Vec<u16> = path.as_os_str().encode_wide().collect();
    path_utf16_from_units(binding, &units)
}

/// Build a UNICODE_STRING for NtCreateFile.
pub(super) fn unicode_string_from_slice(path: &[u16]) -> RuntimeResult<UNICODE_STRING> {
    // compute the byte length
    let length_bytes = path
        .len()
        .checked_mul(2)
        .ok_or_else(|| RuntimeError::from(PlatformError::io("path too long")).boxed())?;
    let length_bytes = u16::try_from(length_bytes)
        .map_err(|_| RuntimeError::from(PlatformError::io("path too long")).boxed())?;

    // build the unicode string
    Ok(UNICODE_STRING {
        Length: length_bytes,
        MaximumLength: length_bytes,
        Buffer: path.as_ptr() as *mut u16,
    })
}

/// Map open flags to NtCreateFile dispositions.
pub(super) fn nt_disposition_from_flags(flags: OpenFlags) -> u32 {
    // decode the flags
    let flags = flags.0;
    let create = flags & libc::O_CREAT as u32 != 0;
    let excl = flags & libc::O_EXCL as u32 != 0;
    let trunc = flags & libc::O_TRUNC as u32 != 0;

    // pick the right disposition
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
    // build the object attributes
    let unicode = unicode_string_from_slice(path)?;
    let object_attributes = OBJECT_ATTRIBUTES {
        Length: std::mem::size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: root,
        ObjectName: &unicode as *const _,
        Attributes: OBJ_CASE_INSENSITIVE as u32,
        SecurityDescriptor: std::ptr::null(),
        SecurityQualityOfService: std::ptr::null(),
    };

    // issue the NtCreateFile call
    let mut iosb = IO_STATUS_BLOCK {
        Anonymous: windows_sys::Win32::System::IO::IO_STATUS_BLOCK_0 { Status: 0 },
        Information: 0,
    };
    let mut handle = INVALID_HANDLE_VALUE;
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            desired_access,
            &object_attributes,
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
    set_rename_info_with_replace(handle, root, name, true)
}

/// Update rename information for an open handle with replace control.
pub(super) fn set_rename_info_with_replace(
    handle: HANDLE,
    root: HANDLE,
    name: &[u16],
    replace: bool,
) -> RuntimeResult<()> {
    // compute the buffer length
    let name_len_bytes = name
        .len()
        .checked_mul(2)
        .ok_or_else(|| RuntimeError::from(PlatformError::io("path too long")).boxed())?;
    let name_len_bytes = u32::try_from(name_len_bytes)
        .map_err(|_| RuntimeError::from(PlatformError::io("path too long")).boxed())?;
    let buffer_len =
        std::mem::size_of::<FILE_RENAME_INFO>() + (name_len_bytes as usize).saturating_sub(2);
    let mut buffer = vec![0u8; buffer_len];

    // fill the rename info buffer
    let info = buffer.as_mut_ptr() as *mut FILE_RENAME_INFO;
    unsafe {
        (*info).Anonymous.ReplaceIfExists = if replace { 1 } else { 0 };
        (*info).RootDirectory = root;
        (*info).FileNameLength = name_len_bytes;
        let target = (*info).FileName.as_mut_ptr();
        std::ptr::copy_nonoverlapping(name.as_ptr(), target, name.len());
    }

    // issue the rename
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
    // build the disposition info
    let info = FILE_DISPOSITION_INFO { DeleteFile: 1 };

    // issue the disposition update
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
    // open the handle with delete access
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
    // open the handle with reparse access
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
    // issue the reparse point query
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

    // parse the reparse buffer
    let target = parse_reparse_target(&buffer)?;
    let value = String::from_utf16(&target).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "reparse target contains invalid utf16",
        ))
        .boxed()
    })?;

    // normalize the target and return it
    Ok(normalize_reparse_target(value))
}

/// Parse a reparse buffer into a UTF-16 path.
pub(super) fn parse_reparse_target(buffer: &[u8]) -> RuntimeResult<Vec<u16>> {
    // validate the header length
    if buffer.len() < 8 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "reparse buffer too small",
        ))
        .boxed());
    }

    // extract the tag and data length
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

    // dispatch by tag
    match tag {
        REPARSE_TAG_SYMLINK => parse_symlink_reparse(data),
        REPARSE_TAG_MOUNT_POINT => parse_mount_point_reparse(data),
        _ => Err(RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoInvalidData),
            None,
            None,
            Some("DeviceIoControl".to_string()),
            None,
            "reparse point is not a symlink or junction".to_string(),
        ))
        .boxed()),
    }
}

/// Parse a symlink reparse buffer.
pub(super) fn parse_symlink_reparse(data: &[u8]) -> RuntimeResult<Vec<u16>> {
    // validate the buffer length
    if data.len() < 12 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "symlink reparse buffer too small",
        ))
        .boxed());
    }

    // decode offsets and lengths
    let substitute_offset = u16::from_le_bytes([data[0], data[1]]) as usize;
    let substitute_length = u16::from_le_bytes([data[2], data[3]]) as usize;
    let print_offset = u16::from_le_bytes([data[4], data[5]]) as usize;
    let print_length = u16::from_le_bytes([data[6], data[7]]) as usize;
    let path_data = &data[12..];

    // choose the best path representation
    let (offset, length) = if print_length > 0 {
        (print_offset, print_length)
    } else {
        (substitute_offset, substitute_length)
    };

    extract_utf16_path(path_data, offset, length)
}

/// Parse a mount point reparse buffer.
pub(super) fn parse_mount_point_reparse(data: &[u8]) -> RuntimeResult<Vec<u16>> {
    // validate the buffer length
    if data.len() < 8 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "mount point reparse buffer too small",
        ))
        .boxed());
    }

    // decode offsets and lengths
    let substitute_offset = u16::from_le_bytes([data[0], data[1]]) as usize;
    let substitute_length = u16::from_le_bytes([data[2], data[3]]) as usize;
    let print_offset = u16::from_le_bytes([data[4], data[5]]) as usize;
    let print_length = u16::from_le_bytes([data[6], data[7]]) as usize;
    let path_data = &data[8..];

    // choose the best path representation
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
    // validate bounds and alignment
    let end = offset + length;
    if end > data.len() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "reparse path out of bounds",
        ))
        .boxed());
    }
    if !offset.is_multiple_of(2) || !length.is_multiple_of(2) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "reparse path misaligned",
        ))
        .boxed());
    }

    // decode the utf-16 data
    let slice = &data[offset..end];
    let mut out = Vec::with_capacity(slice.len() / 2);
    for chunk in slice.chunks_exact(2) {
        out.push(u16::from_le_bytes([chunk[0], chunk[1]]));
    }

    Ok(out)
}

/// Normalize the reparse target prefix.
pub(super) fn normalize_reparse_target(value: String) -> String {
    // strip common reparse prefixes
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
    // extract the os error
    let errno = core_platform::last_error_code();
    let message = core_platform::error_message(syscall, errno);

    // build the platform error
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        Some(errno),
        Some(syscall.to_string()),
        path.map(|path| path.to_string()),
        message,
    ))
    .boxed()
}

/// Stored payload for file handles that need cursor tracking.
#[derive(Debug, Clone)]
pub(super) struct FileResource {
    /// Raw handle for the file.
    pub handle: isize,
    /// Cursor tracking for sequential reads and writes.
    pub cursor: Arc<Mutex<i64>>,
    /// Status-flag tracking for get and set status flags lanes.
    pub status_flags: Arc<Mutex<u32>>,
}

/// Shared runtime file state for cursor and status-flag lanes.
type FileState = (Arc<Mutex<i64>>, Arc<Mutex<u32>>);

/// Access-mode bits that remain fixed for one Windows file handle.
const WINDOWS_STATUS_ACCESS_MASK: u32 = libc::O_WRONLY as u32 | libc::O_RDWR as u32;

/// Status flag bits that are tracked for Windows file handles.
const WINDOWS_STATUS_FLAGS_MASK: u32 = WINDOWS_STATUS_ACCESS_MASK | libc::O_APPEND as u32;

/// Heap-allocated SID wrapper that frees on drop.
#[derive(Debug)]
pub(super) struct SidHandle {
    /// Pointer to the SID data.
    sid: PSID,
}

impl SidHandle {
    /// Build a SID from a domain SID and numeric identifier.
    pub(super) fn from_domain(domain_sid: &str, id: u32, name: &str) -> RuntimeResult<Self> {
        // validate the domain SID
        if domain_sid.is_empty() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "windows.posixDomainSid",
                "domain sid is empty",
            ))
            .boxed());
        }

        // format the SID string
        let sid_string = format!("{domain_sid}-{id}");
        let mut wide: Vec<u16> = OsString::from(&sid_string).encode_wide().collect();
        wide.push(0);

        // convert to a SID
        let mut sid: PSID = std::ptr::null_mut();
        let rc = unsafe { ConvertStringSidToSidW(wide.as_ptr(), &mut sid) };
        if rc == 0 || sid.is_null() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                name,
                "failed to convert SID",
            ))
            .boxed());
        }

        Ok(Self { sid })
    }

    /// Return the underlying SID pointer.
    pub(super) const fn as_ptr(&self) -> PSID {
        self.sid
    }
}

impl Drop for SidHandle {
    fn drop(&mut self) {
        if !self.sid.is_null() {
            unsafe {
                LocalFree(self.sid as HLOCAL);
            }
        }
    }
}

/// Resolve a resource entry for a file handle.
pub(super) fn file_handle(
    binding: &BindingCallContext,
    handle: FileHandle,
) -> RuntimeResult<HANDLE> {
    // resolve the resource entry
    let handle =
        core_fs::require_resource(binding, handle.0, ResourceKind::File, "file", |entry| {
            if let Some(resource) = entry.payload_ref::<FileResource>() {
                return Ok(resource.handle as HANDLE);
            }
            entry
                .handle()
                .map(|handle| handle as HANDLE)
                .ok_or_else(|| {
                    RuntimeError::from(PlatformError::generic(None, "file handle missing payload"))
                        .boxed()
                })
        })?;

    // cast to a raw handle
    Ok(handle as HANDLE)
}

/// Resolve Windows SIDs for POSIX-style uid and gid values.
pub(super) fn posix_sids(
    binding: &BindingCallContext,
    uid: u32,
    gid: u32,
) -> RuntimeResult<(SidHandle, SidHandle)> {
    // read the domain SID from configuration
    let domain_sid = binding
        .worker()
        .options
        .platform
        .windows
        .posix_domain_sid
        .as_deref()
        .ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "windows.posixDomainSid",
                "missing domain sid configuration",
            ))
            .boxed()
        })?;

    // build owner and group SIDs
    let owner = SidHandle::from_domain(domain_sid, uid, "uid")?;
    let group = SidHandle::from_domain(domain_sid, gid, "gid")?;

    Ok((owner, group))
}

/// Build a Win32 error from an explicit error code.
pub(super) fn win32_error(syscall: &str, code: u32) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        Some(code as i32),
        Some(syscall.to_string()),
        None,
        format!("{syscall} failed: {code}"),
    ))
    .boxed()
}

/// Resolve a resource entry for a file handle with cursor tracking.
pub(super) fn file_resource(
    binding: &BindingCallContext,
    handle: FileHandle,
) -> RuntimeResult<Arc<Mutex<i64>>> {
    let (cursor, _) = file_state(binding, handle)?;

    Ok(cursor)
}

/// Resolve a resource entry for file cursor and status-flag state.
pub(super) fn file_state(
    binding: &BindingCallContext,
    handle: FileHandle,
) -> RuntimeResult<FileState> {
    // resolve the resource entry
    let state =
        core_fs::require_resource(binding, handle.0, ResourceKind::File, "file", |entry| {
            let Some(resource) = entry.payload_ref::<FileResource>() else {
                return Err(RuntimeError::from(PlatformError::generic(
                    None,
                    "file state missing payload",
                ))
                .boxed());
            };

            Ok((resource.cursor.clone(), resource.status_flags.clone()))
        })?;

    Ok(state)
}

/// Resolve a resource entry for file status-flag state.
pub(super) fn file_status_flags(
    binding: &BindingCallContext,
    handle: FileHandle,
) -> RuntimeResult<Arc<Mutex<u32>>> {
    let (_, status_flags) = file_state(binding, handle)?;

    Ok(status_flags)
}

/// Derive tracked status flags from one open flag set.
pub(super) fn tracked_status_flags_from_open_flags(flags: OpenFlags) -> u32 {
    flags.0 & WINDOWS_STATUS_FLAGS_MASK
}

/// Normalize one requested status-flag update against existing access mode bits.
pub(super) fn normalize_status_flags(existing: u32, requested: StatusFlags) -> RuntimeResult<u32> {
    // reject unsupported status bits on windows
    if requested.0 & !WINDOWS_STATUS_FLAGS_MASK != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "unsupported status flag bits",
        ))
        .boxed());
    }

    // preserve access mode bits and update the mutable subset
    let access_mode = existing & WINDOWS_STATUS_ACCESS_MASK;
    let mutable_flags = requested.0 & libc::O_APPEND as u32;

    Ok(access_mode | mutable_flags)
}

/// Resolve a resource entry for a directory handle.
pub(super) fn directory_handle(
    binding: &BindingCallContext,
    handle: DirectoryHandle,
) -> RuntimeResult<HANDLE> {
    // resolve the resource entry
    let handle = core_fs::require_resource(
        binding,
        handle.0,
        ResourceKind::Directory,
        "directory",
        |entry| {
            entry.handle().ok_or_else(|| {
                RuntimeError::from(PlatformError::generic(
                    None,
                    "directory handle missing payload",
                ))
                .boxed()
            })
        },
    )?;

    // cast to a raw handle
    Ok(handle as HANDLE)
}

/// Resolve a resource entry for a socket handle.
pub(super) fn socket_handle(
    binding: &BindingCallContext,
    handle: SocketHandle,
) -> RuntimeResult<SOCKET> {
    let socket = core_net::require_resource(
        binding,
        handle.0,
        ResourceKind::Socket,
        "socket",
        |entry: &ResourceEntry| {
            entry.socket().ok_or_else(|| {
                RuntimeError::from(PlatformError::generic(
                    None,
                    "socket handle missing payload",
                ))
                .boxed()
            })
        },
    )?;
    Ok(socket as SOCKET)
}

/// Resolve a directory handle into a PathBuf.
pub(super) fn directory_path(
    binding: &BindingCallContext,
    handle: DirectoryHandle,
) -> RuntimeResult<PathBuf> {
    // resolve the directory path from the handle
    let handle = directory_handle(binding, handle)?;
    let wide = final_path_from_handle(handle)?;
    Ok(PathBuf::from(OsString::from_wide(&wide)))
}

/// Map open flags to desired access.
pub(super) fn desired_access_from_flags(flags: OpenFlags) -> u32 {
    // decode the flags
    let flags = flags.0;
    let is_write = flags & libc::O_WRONLY as u32 != 0;
    let is_readwrite = flags & libc::O_RDWR as u32 != 0;
    let is_append = flags & libc::O_APPEND as u32 != 0;

    // preserve append only handles without silently granting read access
    if is_append && is_write {
        return FILE_APPEND_DATA;
    }

    // allow read plus write access for readwrite opens
    if is_readwrite {
        return FILE_GENERIC_READ | FILE_GENERIC_WRITE;
    }

    // preserve plain write only access
    if is_write {
        return FILE_GENERIC_WRITE;
    }

    // fall back to readonly access
    FILE_GENERIC_READ
}

/// Map open flags to creation disposition.
pub(super) fn creation_from_flags(flags: OpenFlags) -> u32 {
    // decode the flags
    let flags = flags.0;
    let create = flags & libc::O_CREAT as u32 != 0;
    let excl = flags & libc::O_EXCL as u32 != 0;
    let trunc = flags & libc::O_TRUNC as u32 != 0;

    // choose the creation disposition
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
    // map write bits into attributes
    let write_bits = mode.0 & 0o222;
    if write_bits == 0 {
        return FILE_ATTRIBUTE_READONLY;
    }

    FILE_ATTRIBUTE_NORMAL
}

/// Build a Stat struct from file information.
pub(super) fn stat_from_info(
    info: windows_sys::Win32::Storage::FileSystem::BY_HANDLE_FILE_INFORMATION,
    reparse_tag: Option<u32>,
) -> Stat {
    // decode the raw metadata
    let ino = ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64;
    let nlink = info.nNumberOfLinks;
    let mode = if matches!(
        reparse_tag,
        Some(REPARSE_TAG_SYMLINK | REPARSE_TAG_MOUNT_POINT)
    ) {
        WINDOWS_S_IFLNK_MODE
    } else if info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
        libc::S_IFDIR as u32
    } else {
        libc::S_IFREG as u32
    };
    let size = ((info.nFileSizeHigh as u64) << 32) | info.nFileSizeLow as u64;
    let atime_ns = filetime_to_nanos(info.ftLastAccessTime);
    let mtime_ns = filetime_to_nanos(info.ftLastWriteTime);
    let birthtime_ns = filetime_to_nanos(info.ftCreationTime);

    // build the stat struct
    Stat {
        dev: 0,
        ino,
        mode: FileMode(mode),
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
    // convert ticks to unix nanos
    let ticks = ((filetime.dwHighDateTime as u64) << 32) | filetime.dwLowDateTime as u64;
    if ticks < 116444736000000000 {
        return 0;
    }

    (ticks - 116444736000000000) * 100
}

/// Convert Windows 100ns ticks since 1601 into nanoseconds since the unix epoch.
fn windows_ticks_to_nanos(ticks: i64) -> u64 {
    // saturate negative or pre epoch values to zero
    if ticks <= 0 {
        return 0;
    }

    let ticks = ticks as u64;
    if ticks < 116444736000000000 {
        return 0;
    }

    (ticks - 116444736000000000) * 100
}

/// Convert nanoseconds since the unix epoch to FILETIME.
pub(super) fn filetime_from_nanos(nanos: u64) -> FILETIME {
    // convert unix nanos to filetime ticks
    let ticks = (nanos / 100) + 116444736000000000u64;
    FILETIME {
        dwLowDateTime: ticks as u32,
        dwHighDateTime: (ticks >> 32) as u32,
    }
}

/// Set access and modification timestamps for a handle.
pub(super) fn set_handle_times(handle: HANDLE, atime_ns: u64, mtime_ns: u64) -> RuntimeResult<()> {
    // build filetime structures
    let atime = filetime_from_nanos(atime_ns);
    let mtime = filetime_from_nanos(mtime_ns);

    // update the handle timestamps
    let rc = unsafe { SetFileTime(handle, std::ptr::null(), &atime, &mtime) };
    if rc == 0 {
        return Err(last_os_error("SetFileTime", None));
    }

    Ok(())
}

/// Read the reparse tag for one handle when the file is a reparse point.
pub(super) fn reparse_tag_from_handle(
    handle: HANDLE,
    attributes: u32,
) -> RuntimeResult<Option<u32>> {
    // skip the extra query for non-reparse files
    if attributes & FILE_ATTRIBUTE_REPARSE_POINT == 0 {
        return Ok(None);
    }

    // query the attribute tag payload
    let mut info = FILE_ATTRIBUTE_TAG_INFO {
        FileAttributes: 0,
        ReparseTag: 0,
    };
    let rc = unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileAttributeTagInfo,
            &mut info as *mut _ as *mut _,
            std::mem::size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
        )
    };
    if rc == 0 {
        return Err(last_os_error("GetFileInformationByHandleEx", None));
    }

    Ok(Some(info.ReparseTag))
}

/// Resolve a file handle from a handle and build Stat.
pub(super) fn stat_from_handle(handle: HANDLE) -> RuntimeResult<Stat> {
    // query the file information
    let mut info = std::mem::MaybeUninit::uninit();
    let rc = unsafe { GetFileInformationByHandle(handle, info.as_mut_ptr()) };
    if rc == 0 {
        return Err(last_os_error("GetFileInformationByHandle", None));
    }

    // decode the information
    let info = unsafe { info.assume_init() };
    let reparse_tag = reparse_tag_from_handle(handle, info.dwFileAttributes)?;
    let mut stat = stat_from_info(info, reparse_tag);

    // query the file basic info to preserve real change time semantics
    let mut basic = unsafe { std::mem::zeroed::<FILE_BASIC_INFO>() };
    let rc = unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileBasicInfo,
            &mut basic as *mut _ as *mut _,
            std::mem::size_of::<FILE_BASIC_INFO>() as u32,
        )
    };
    if rc == 0 {
        return Err(last_os_error("GetFileInformationByHandleEx", None));
    }

    stat.ctime_ns = windows_ticks_to_nanos(basic.ChangeTime);

    Ok(stat)
}

/// Resolve a stat structure for a path.
pub(super) fn stat_from_path(path: &[u16], follow_symlink: bool) -> RuntimeResult<Stat> {
    // open the path and stat it
    let handle = open_for_metadata(path, follow_symlink)?;
    let stat = stat_from_handle(handle)?;
    unsafe {
        CloseHandle(handle);
    }

    Ok(stat)
}

/// Ensure a path is nul-terminated.
pub(super) fn ensure_wide_nul(path: &[u16]) -> Vec<u16> {
    // append a nul terminator when needed
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
    // ensure the input is nul-terminated
    let path = ensure_wide_nul(path);

    // query the volume path
    let mut buffer = vec![0u16; WINDOWS_LEGACY_PATH_CAPACITY];
    let rc = unsafe { GetVolumePathNameW(path.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32) };
    if rc == 0 {
        return Err(last_os_error("GetVolumePathNameW", None));
    }

    // trim to the reported length
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
    // resolve the volume path
    let volume = volume_path_from_path(path)?;

    // query disk space
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

    // query exact size info
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

    // query volume metadata
    let mut serial = 0u32;
    let mut max_component = 0u32;
    let mut flags = 0u32;
    let mut volume_name = vec![0u16; WINDOWS_LEGACY_PATH_CAPACITY];
    let mut fs_name = vec![0u16; WINDOWS_LEGACY_PATH_CAPACITY];
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

    // compute statfs values
    let blocks = if bsize == 0 { 0 } else { total_bytes / bsize };
    let bfree = if bsize == 0 { 0 } else { total_free / bsize };
    let bavail = if bsize == 0 { 0 } else { free_bytes / bsize };

    // return the statfs struct
    Ok(StatFs {
        bsize,
        frsize: bsize,
        blocks,
        bfree,
        bavail,
        files: 0,
        ffree: 0,
        fsid: serial as u64,
        flags: StatFsFlags(flags as u64),
        namelen: max_component as u64,
    })
}

/// Convert a handle to a wide path using GetFinalPathNameByHandleW.
pub(crate) fn final_path_from_handle(handle: HANDLE) -> RuntimeResult<Vec<u16>> {
    // grow the buffer until the path fits
    let mut buffer = vec![0u16; WINDOWS_DYNAMIC_PATH_INITIAL_CAPACITY];
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
    // map flags into open options
    let mut flags = FILE_FLAG_BACKUP_SEMANTICS;
    if !follow_symlink {
        flags |= FILE_FLAG_OPEN_REPARSE_POINT;
    }

    // open the handle
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            FILE_READ_ATTRIBUTES,
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
    // map flags into open options
    let mut flags = FILE_FLAG_BACKUP_SEMANTICS;
    if !follow_symlink {
        flags |= FILE_FLAG_OPEN_REPARSE_POINT;
    }

    // open the handle
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

#[cfg(test)]
mod tests {
    use super::parse_reparse_target;
    use crate::platform::diagnostic::PlatformErrorCode;

    /// Reject unsupported reparse tags as invalid link payloads.
    #[test]
    fn test_parse_reparse_target_rejects_unknown_tag() {
        let mut buffer = Vec::new();

        // header: unknown tag plus empty payload
        buffer.extend_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        buffer.extend_from_slice(&0u16.to_le_bytes());
        buffer.extend_from_slice(&0u16.to_le_bytes());

        let error = parse_reparse_target(&buffer).expect_err("unknown tag should fail");
        assert_eq!(error.code(), PlatformErrorCode::IoInvalidData as u16);
    }
}
