use std::mem;

use windows_sys::Wdk::Storage::FileSystem::{
    FILE_FULL_EA_INFORMATION, FILE_GET_EA_INFORMATION, ZwQueryEaFile, ZwSetEaFile,
};
use windows_sys::Win32::Foundation::{
    CloseHandle, HANDLE, INVALID_HANDLE_VALUE, NTSTATUS, RtlNtStatusToDosError,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_READ_EA, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_WRITE_EA,
    OPEN_EXISTING,
};
use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;

use super::util::{file_handle, last_os_error, wide_from_bytes, wide_from_utf16};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::{OsPath, PathBytes, PathUtf16, XattrFlags, core as core_fs};
use crate::platform::resource::FileHandle;
use crate::platform::{NativeArray, PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;

const STATUS_SUCCESS: NTSTATUS = 0;
const STATUS_BUFFER_OVERFLOW: NTSTATUS = 0x8000_0005u32 as i32;
const STATUS_BUFFER_TOO_SMALL: NTSTATUS = 0xC000_0023u32 as i32;
const STATUS_EA_NOT_FOUND: NTSTATUS = 0xC000_0051u32 as i32;
const STATUS_NO_EAS_ON_FILE: NTSTATUS = 0xC000_0052u32 as i32;
const EA_QUERY_INITIAL_BUFFER_CAPACITY: usize = 256;

fn nt_status_error(status: NTSTATUS, syscall: &str) -> Box<RuntimeError> {
    // convert the NTSTATUS into a Win32 error code
    let code = unsafe { RtlNtStatusToDosError(status) } as i32;

    // build the platform error
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        Some(code),
        Some(syscall.to_string()),
        None,
        format!("{syscall} failed: {code}"),
    ))
    .boxed()
}

fn xattr_already_exists(syscall: &str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoAlreadyExists),
        None,
        None,
        Some(syscall.to_string()),
        None,
        "extended attribute already exists",
    ))
    .boxed()
}

fn resolve_xattr_name_bytes(name: NativeSlice<u8>) -> RuntimeResult<Vec<u8>> {
    // decode the raw name bytes
    let name = unsafe { name.as_slice()? };

    // validate the name bytes
    if name.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "name",
            "attribute name cannot be empty",
        ))
        .boxed());
    }
    if name.contains(&0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "name",
            "attribute name contains nul byte",
        ))
        .boxed());
    }
    if name.len() > u8::MAX as usize {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "name",
            "attribute name too long",
        ))
        .boxed());
    }

    Ok(name.to_vec())
}

fn xattr_name_slice_from_string(
    binding: &BindingCallContext,
    name: NativeStringRef,
) -> RuntimeResult<NativeSlice<u8>> {
    // decode the string name
    let name = unsafe { name.as_str()? };

    // store a stable raw name slice
    Ok(binding.store_slice_copy(name.as_bytes()))
}

fn xattr_name_strings_from_bytes(
    binding: &BindingCallContext,
    names: NativeArray<NativeArray<u8>>,
) -> RuntimeResult<NativeArray<NativeStringRef>> {
    // decode the raw name arrays
    let names = unsafe { names.as_slice()? };
    let mut decoded = Vec::with_capacity(names.len());

    // decode each name as utf8
    for name in names {
        let bytes = unsafe { name.as_slice()? };
        let name = std::str::from_utf8(bytes).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "xattr",
                "attribute name is not valid utf8",
            ))
            .boxed()
        })?;
        decoded.push(binding.store_string(name));
    }

    Ok(binding.store_array(decoded))
}

fn open_xattr_path(path: &[u16], write: bool, follow_symlink: bool) -> RuntimeResult<HANDLE> {
    // compute access flags
    let access = if write {
        FILE_READ_EA | FILE_WRITE_EA
    } else {
        FILE_READ_EA
    };

    // build open attributes
    let mut attributes = FILE_ATTRIBUTE_NORMAL | FILE_FLAG_BACKUP_SEMANTICS;
    if !follow_symlink {
        attributes |= FILE_FLAG_OPEN_REPARSE_POINT;
    }

    // open the handle
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            attributes,
            0,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(last_os_error("CreateFileW", None));
    }

    Ok(handle)
}

fn build_get_ea_list(name: &[u8]) -> RuntimeResult<Vec<u8>> {
    // compute the buffer size
    let name_len = name.len();
    if name_len > u8::MAX as usize {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "name",
            "attribute name too long",
        ))
        .boxed());
    }
    let total_len = mem::size_of::<FILE_GET_EA_INFORMATION>() + name_len;

    // build the buffer
    let mut buffer = vec![0u8; total_len];
    let entry = buffer.as_mut_ptr() as *mut FILE_GET_EA_INFORMATION;
    unsafe {
        (*entry).NextEntryOffset = 0;
        (*entry).EaNameLength = name_len as u8;
        let name_ptr = (*entry).EaName.as_mut_ptr();
        std::ptr::copy_nonoverlapping(name.as_ptr(), name_ptr, name_len);
        *name_ptr.add(name_len) = 0;
    }

    Ok(buffer)
}

fn query_ea_entry(handle: HANDLE, name: &[u8]) -> RuntimeResult<Vec<u8>> {
    // build the EA list buffer
    let ealist = build_get_ea_list(name)?;

    // attempt to read the entry with a growing buffer
    let mut buffer_len = EA_QUERY_INITIAL_BUFFER_CAPACITY
        .max(ealist.len() + mem::size_of::<FILE_FULL_EA_INFORMATION>());
    loop {
        // allocate the output buffer
        let mut buffer = vec![0u8; buffer_len];
        let mut iosb = IO_STATUS_BLOCK {
            Anonymous: windows_sys::Win32::System::IO::IO_STATUS_BLOCK_0 { Status: 0 },
            Information: 0,
        };

        // issue the query
        let status = unsafe {
            ZwQueryEaFile(
                handle,
                &mut iosb,
                buffer.as_mut_ptr() as *mut _,
                buffer.len() as u32,
                1,
                ealist.as_ptr() as *const _,
                ealist.len() as u32,
                std::ptr::null(),
                1,
            )
        };

        // handle common outcomes
        if status == STATUS_SUCCESS {
            buffer.truncate(iosb.Information);
            return Ok(buffer);
        }
        if status == STATUS_EA_NOT_FOUND || status == STATUS_NO_EAS_ON_FILE {
            return Err(core_platform::io_not_found(
                "ZwQueryEaFile",
                "extended attribute not found",
            ));
        }
        if status == STATUS_BUFFER_OVERFLOW || status == STATUS_BUFFER_TOO_SMALL {
            let suggested = iosb.Information;
            buffer_len = suggested.max(buffer_len * 2);
            continue;
        }

        return Err(nt_status_error(status, "ZwQueryEaFile"));
    }
}

fn query_ea_list(handle: HANDLE) -> RuntimeResult<Vec<u8>> {
    // start small and grow from the kernel hint when needed
    let mut buffer_len = EA_QUERY_INITIAL_BUFFER_CAPACITY;

    loop {
        // allocate the output buffer
        let mut buffer = vec![0u8; buffer_len];
        let mut iosb = IO_STATUS_BLOCK {
            Anonymous: windows_sys::Win32::System::IO::IO_STATUS_BLOCK_0 { Status: 0 },
            Information: 0,
        };

        // issue the query for all entries
        let status = unsafe {
            ZwQueryEaFile(
                handle,
                &mut iosb,
                buffer.as_mut_ptr() as *mut _,
                buffer.len() as u32,
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                1,
            )
        };

        // handle common outcomes
        if status == STATUS_SUCCESS {
            buffer.truncate(iosb.Information);
            return Ok(buffer);
        }
        if status == STATUS_NO_EAS_ON_FILE {
            return Ok(Vec::new());
        }
        if status == STATUS_BUFFER_OVERFLOW || status == STATUS_BUFFER_TOO_SMALL {
            let suggested = iosb.Information;
            buffer_len = suggested.max(buffer_len * 2);
            continue;
        }

        return Err(nt_status_error(status, "ZwQueryEaFile"));
    }
}

fn parse_ea_entries(buffer: &[u8]) -> RuntimeResult<Vec<(Vec<u8>, Vec<u8>)>> {
    // walk the EA buffer entries
    let mut entries = Vec::new();
    let mut offset = 0usize;
    while offset < buffer.len() {
        // ensure the entry header is available
        if buffer.len() - offset < mem::size_of::<FILE_FULL_EA_INFORMATION>() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "xattr",
                "invalid EA buffer",
            ))
            .boxed());
        }

        // read the entry header
        let entry = unsafe { &*(buffer.as_ptr().add(offset) as *const FILE_FULL_EA_INFORMATION) };
        let name_len = entry.EaNameLength as usize;
        let value_len = entry.EaValueLength as usize;
        let entry_len = if entry.NextEntryOffset == 0 {
            buffer.len() - offset
        } else {
            entry.NextEntryOffset as usize
        };
        if entry_len < mem::size_of::<FILE_FULL_EA_INFORMATION>() + name_len + value_len {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "xattr",
                "invalid EA entry size",
            ))
            .boxed());
        }

        // decode name and value
        let name_ptr = entry.EaName.as_ptr();
        let name_bytes = unsafe { std::slice::from_raw_parts(name_ptr, name_len) };
        let value_ptr = unsafe { name_ptr.add(name_len + 1) };
        let value_bytes = unsafe { std::slice::from_raw_parts(value_ptr, value_len) };
        entries.push((name_bytes.to_vec(), value_bytes.to_vec()));

        // advance to the next entry
        if entry.NextEntryOffset == 0 {
            break;
        }
        offset += entry.NextEntryOffset as usize;
    }

    Ok(entries)
}

fn extract_single_value(buffer: &[u8]) -> RuntimeResult<Vec<u8>> {
    // parse the first EA entry
    let mut entries = parse_ea_entries(buffer)?;
    if let Some((_name, value)) = entries.pop() {
        return Ok(value);
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "xattr",
        "attribute value missing",
    ))
    .boxed())
}

fn xattr_exists(handle: HANDLE, name: &[u8]) -> RuntimeResult<bool> {
    // build the EA list buffer
    let ealist = build_get_ea_list(name)?;

    // issue a header-sized query to detect existence
    let mut buffer = vec![0u8; mem::size_of::<FILE_FULL_EA_INFORMATION>()];
    let mut iosb = IO_STATUS_BLOCK {
        Anonymous: windows_sys::Win32::System::IO::IO_STATUS_BLOCK_0 { Status: 0 },
        Information: 0,
    };
    let status = unsafe {
        ZwQueryEaFile(
            handle,
            &mut iosb,
            buffer.as_mut_ptr() as *mut _,
            buffer.len() as u32,
            1,
            ealist.as_ptr() as *const _,
            ealist.len() as u32,
            std::ptr::null(),
            1,
        )
    };

    if status == STATUS_SUCCESS {
        return Ok(true);
    }
    if status == STATUS_BUFFER_OVERFLOW || status == STATUS_BUFFER_TOO_SMALL {
        return Ok(true);
    }
    if status == STATUS_EA_NOT_FOUND || status == STATUS_NO_EAS_ON_FILE {
        return Ok(false);
    }

    Err(nt_status_error(status, "ZwQueryEaFile"))
}

fn set_ea_entry(handle: HANDLE, name: &[u8], value: &[u8]) -> RuntimeResult<()> {
    // validate lengths
    if name.len() > u8::MAX as usize {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "name",
            "attribute name too long",
        ))
        .boxed());
    }
    if value.len() > u16::MAX as usize {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "value",
            "attribute value too long",
        ))
        .boxed());
    }

    // build the EA buffer
    let total_len = mem::size_of::<FILE_FULL_EA_INFORMATION>() + name.len() + value.len();
    let mut buffer = vec![0u8; total_len];
    let entry = buffer.as_mut_ptr() as *mut FILE_FULL_EA_INFORMATION;
    unsafe {
        (*entry).NextEntryOffset = 0;
        (*entry).Flags = 0;
        (*entry).EaNameLength = name.len() as u8;
        (*entry).EaValueLength = value.len() as u16;
        let name_ptr = (*entry).EaName.as_mut_ptr();
        std::ptr::copy_nonoverlapping(name.as_ptr(), name_ptr, name.len());
        *name_ptr.add(name.len()) = 0;
        let value_ptr = name_ptr.add(name.len() + 1);
        std::ptr::copy_nonoverlapping(value.as_ptr(), value_ptr, value.len());
    }

    // issue the set call
    let mut iosb = IO_STATUS_BLOCK {
        Anonymous: windows_sys::Win32::System::IO::IO_STATUS_BLOCK_0 { Status: 0 },
        Information: 0,
    };
    let status = unsafe {
        ZwSetEaFile(
            handle,
            &mut iosb,
            buffer.as_ptr() as *const _,
            buffer.len() as u32,
        )
    };
    if status != STATUS_SUCCESS {
        return Err(nt_status_error(status, "ZwSetEaFile"));
    }

    Ok(())
}

fn remove_ea_entry(handle: HANDLE, name: &[u8]) -> RuntimeResult<()> {
    // build the removal buffer
    let total_len = mem::size_of::<FILE_FULL_EA_INFORMATION>() + name.len();
    let mut buffer = vec![0u8; total_len];
    let entry = buffer.as_mut_ptr() as *mut FILE_FULL_EA_INFORMATION;
    unsafe {
        (*entry).NextEntryOffset = 0;
        (*entry).Flags = 0;
        (*entry).EaNameLength = name.len() as u8;
        (*entry).EaValueLength = 0;
        let name_ptr = (*entry).EaName.as_mut_ptr();
        std::ptr::copy_nonoverlapping(name.as_ptr(), name_ptr, name.len());
        *name_ptr.add(name.len()) = 0;
    }

    // issue the set call
    let mut iosb = IO_STATUS_BLOCK {
        Anonymous: windows_sys::Win32::System::IO::IO_STATUS_BLOCK_0 { Status: 0 },
        Information: 0,
    };
    let status = unsafe {
        ZwSetEaFile(
            handle,
            &mut iosb,
            buffer.as_ptr() as *const _,
            buffer.len() as u32,
        )
    };
    if status == STATUS_EA_NOT_FOUND || status == STATUS_NO_EAS_ON_FILE {
        return Err(core_platform::io_not_found(
            "ZwSetEaFile",
            "extended attribute not found",
        ));
    }
    if status != STATUS_SUCCESS {
        return Err(nt_status_error(status, "ZwSetEaFile"));
    }

    Ok(())
}

fn decode_xattr_list_bytes(
    binding: &BindingCallContext,
    buffer: Vec<u8>,
) -> RuntimeResult<NativeArray<NativeArray<u8>>> {
    // parse the EA entries
    let entries = parse_ea_entries(&buffer)?;

    // build the name list
    let mut names = Vec::with_capacity(entries.len());
    for (name, _) in entries {
        names.push(binding.store_array(name));
    }

    Ok(binding.store_array(names))
}

fn with_handle<F, T>(handle: HANDLE, f: F) -> RuntimeResult<T>
where
    F: FnOnce(HANDLE) -> RuntimeResult<T>,
{
    // run the operation
    let result = f(handle);

    // close the handle
    unsafe {
        CloseHandle(handle);
    }

    result
}

/// Read an extended attribute by path with a raw name payload.
pub(crate) unsafe fn destack_fs_getxattr_bytes(
    binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    path: PathBytes,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve inputs
    let name = resolve_xattr_name_bytes(name)?;
    let wide = wide_from_bytes(path, "path")?;

    // open the handle and query the entry
    let handle = open_xattr_path(&wide, false, true)?;
    let buffer = with_handle(handle, |handle| query_ea_entry(handle, &name))?;
    let value = extract_single_value(&buffer)?;

    // write the output
    unsafe {
        *out = binding.store_array(value);
    }

    Ok(())
}

/// Read an extended attribute by path.
pub(crate) unsafe fn destack_fs_getxattr_utf16(
    binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    path: PathUtf16,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve inputs
    let name = resolve_xattr_name_bytes(name)?;
    let wide = wide_from_utf16(path, "path")?;

    // open the handle and query the entry
    let handle = open_xattr_path(&wide, false, true)?;
    let buffer = with_handle(handle, |handle| query_ea_entry(handle, &name))?;
    let value = extract_single_value(&buffer)?;

    // write the output
    unsafe {
        *out = binding.store_array(value);
    }

    Ok(())
}

/// Read an extended attribute without following symlinks, using a raw name payload.
pub(crate) unsafe fn destack_fs_lgetxattr_bytes(
    binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    path: PathBytes,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve inputs
    let name = resolve_xattr_name_bytes(name)?;
    let wide = wide_from_bytes(path, "path")?;

    // open the handle and query the entry
    let handle = open_xattr_path(&wide, false, false)?;
    let buffer = with_handle(handle, |handle| query_ea_entry(handle, &name))?;
    let value = extract_single_value(&buffer)?;

    // write the output
    unsafe {
        *out = binding.store_array(value);
    }

    Ok(())
}

/// Read an extended attribute without following symlinks.
pub(crate) unsafe fn destack_fs_lgetxattr_utf16(
    binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    path: PathUtf16,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve inputs
    let name = resolve_xattr_name_bytes(name)?;
    let wide = wide_from_utf16(path, "path")?;

    // open the handle and query the entry
    let handle = open_xattr_path(&wide, false, false)?;
    let buffer = with_handle(handle, |handle| query_ea_entry(handle, &name))?;
    let value = extract_single_value(&buffer)?;

    // write the output
    unsafe {
        *out = binding.store_array(value);
    }

    Ok(())
}

/// Read an extended attribute by handle.
pub(crate) unsafe fn destack_fs_fgetxattr_handle(
    binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    handle: FileHandle,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve inputs
    let name = resolve_xattr_name_bytes(name)?;
    let handle = file_handle(binding, handle)?;

    // query the entry
    let buffer = query_ea_entry(handle, &name)?;
    let value = extract_single_value(&buffer)?;
    // write the output
    unsafe {
        *out = binding.store_array(value);
    }

    Ok(())
}

/// Set an extended attribute by path with a raw name payload.
pub(crate) unsafe fn destack_fs_setxattr_bytes(
    _binding: &BindingCallContext,
    path: PathBytes,
    name: NativeSlice<u8>,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    // resolve inputs
    let name = resolve_xattr_name_bytes(name)?;
    let value = unsafe { value.as_slice()? };
    let wide = wide_from_bytes(path, "path")?;

    // open the handle
    let handle = open_xattr_path(&wide, true, true)?;

    // run the set operation
    with_handle(handle, |handle| {
        // validate flags
        core_fs::validate_xattr_flags(flags)?;

        // enforce create/replace flags
        if flags.0 & 0x1 != 0 || flags.0 & 0x2 != 0 {
            let exists = xattr_exists(handle, &name)?;
            if flags.0 & 0x1 != 0 && exists {
                return Err(xattr_already_exists("ZwQueryEaFile"));
            }
            if flags.0 & 0x2 != 0 && !exists {
                return Err(core_platform::io_not_found(
                    "ZwQueryEaFile",
                    "extended attribute not found",
                ));
            }
        }

        set_ea_entry(handle, &name, value)
    })
}

/// Set an extended attribute by path.
pub(crate) unsafe fn destack_fs_setxattr_utf16(
    _binding: &BindingCallContext,
    path: PathUtf16,
    name: NativeSlice<u8>,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    // resolve inputs
    let name = resolve_xattr_name_bytes(name)?;
    let value = unsafe { value.as_slice()? };
    let wide = wide_from_utf16(path, "path")?;

    // open the handle
    let handle = open_xattr_path(&wide, true, true)?;

    // run the set operation
    with_handle(handle, |handle| {
        // validate flags
        core_fs::validate_xattr_flags(flags)?;

        // enforce create/replace flags
        if flags.0 & 0x1 != 0 || flags.0 & 0x2 != 0 {
            let exists = xattr_exists(handle, &name)?;
            if flags.0 & 0x1 != 0 && exists {
                return Err(xattr_already_exists("ZwQueryEaFile"));
            }
            if flags.0 & 0x2 != 0 && !exists {
                return Err(core_platform::io_not_found(
                    "ZwQueryEaFile",
                    "extended attribute not found",
                ));
            }
        }

        set_ea_entry(handle, &name, value)
    })
}

/// Set an extended attribute without following symlinks, using a raw name payload.
pub(crate) unsafe fn destack_fs_lsetxattr_bytes(
    _binding: &BindingCallContext,
    path: PathBytes,
    name: NativeSlice<u8>,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    // resolve inputs
    let name = resolve_xattr_name_bytes(name)?;
    let value = unsafe { value.as_slice()? };
    let wide = wide_from_bytes(path, "path")?;

    // open the handle
    let handle = open_xattr_path(&wide, true, false)?;

    // run the set operation
    with_handle(handle, |handle| {
        // validate flags
        core_fs::validate_xattr_flags(flags)?;

        // enforce create/replace flags
        if flags.0 & 0x1 != 0 || flags.0 & 0x2 != 0 {
            let exists = xattr_exists(handle, &name)?;
            if flags.0 & 0x1 != 0 && exists {
                return Err(xattr_already_exists("ZwQueryEaFile"));
            }
            if flags.0 & 0x2 != 0 && !exists {
                return Err(core_platform::io_not_found(
                    "ZwQueryEaFile",
                    "extended attribute not found",
                ));
            }
        }

        set_ea_entry(handle, &name, value)
    })
}

/// Set an extended attribute without following symlinks.
pub(crate) unsafe fn destack_fs_lsetxattr_utf16(
    _binding: &BindingCallContext,
    path: PathUtf16,
    name: NativeSlice<u8>,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    // resolve inputs
    let name = resolve_xattr_name_bytes(name)?;
    let value = unsafe { value.as_slice()? };
    let wide = wide_from_utf16(path, "path")?;

    // open the handle
    let handle = open_xattr_path(&wide, true, false)?;

    // run the set operation
    with_handle(handle, |handle| {
        // validate flags
        core_fs::validate_xattr_flags(flags)?;

        // enforce create/replace flags
        if flags.0 & 0x1 != 0 || flags.0 & 0x2 != 0 {
            let exists = xattr_exists(handle, &name)?;
            if flags.0 & 0x1 != 0 && exists {
                return Err(xattr_already_exists("ZwQueryEaFile"));
            }
            if flags.0 & 0x2 != 0 && !exists {
                return Err(core_platform::io_not_found(
                    "ZwQueryEaFile",
                    "extended attribute not found",
                ));
            }
        }

        set_ea_entry(handle, &name, value)
    })
}

/// Set an extended attribute by handle.
pub(crate) unsafe fn destack_fs_fsetxattr_handle(
    binding: &BindingCallContext,
    handle: FileHandle,
    name: NativeSlice<u8>,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    // resolve inputs
    let name = resolve_xattr_name_bytes(name)?;
    let value = unsafe { value.as_slice()? };
    let handle = file_handle(binding, handle)?;

    // validate flags
    core_fs::validate_xattr_flags(flags)?;

    // enforce create/replace flags
    if flags.0 & 0x1 != 0 || flags.0 & 0x2 != 0 {
        let exists = xattr_exists(handle, &name)?;
        if flags.0 & 0x1 != 0 && exists {
            return Err(xattr_already_exists("ZwQueryEaFile"));
        }
        if flags.0 & 0x2 != 0 && !exists {
            return Err(core_platform::io_not_found(
                "ZwQueryEaFile",
                "extended attribute not found",
            ));
        }
    }

    set_ea_entry(handle, &name, value)
}

/// List extended attribute names by path as raw byte payloads.
pub(crate) unsafe fn destack_fs_listxattr_bytes(
    binding: &BindingCallContext,
    out: *mut NativeArray<NativeArray<u8>>,
    path: PathBytes,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve inputs
    let wide = wide_from_bytes(path, "path")?;

    // open the handle and read the list
    let handle = open_xattr_path(&wide, false, true)?;
    let buffer = with_handle(handle, query_ea_list)?;

    // write the output
    unsafe {
        *out = decode_xattr_list_bytes(binding, buffer)?;
    }

    Ok(())
}

/// List extended attribute names by path.
pub(crate) unsafe fn destack_fs_listxattr_utf16(
    binding: &BindingCallContext,
    out: *mut NativeArray<NativeArray<u8>>,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve inputs
    let wide = wide_from_utf16(path, "path")?;

    // open the handle and read the list
    let handle = open_xattr_path(&wide, false, true)?;
    let buffer = with_handle(handle, query_ea_list)?;

    // write the output
    unsafe {
        *out = decode_xattr_list_bytes(binding, buffer)?;
    }

    Ok(())
}

/// List extended attribute names without following symlinks as raw byte payloads.
pub(crate) unsafe fn destack_fs_llistxattr_bytes(
    binding: &BindingCallContext,
    out: *mut NativeArray<NativeArray<u8>>,
    path: PathBytes,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve inputs
    let wide = wide_from_bytes(path, "path")?;

    // open the handle and read the list
    let handle = open_xattr_path(&wide, false, false)?;
    let buffer = with_handle(handle, query_ea_list)?;

    // write the output
    unsafe {
        *out = decode_xattr_list_bytes(binding, buffer)?;
    }

    Ok(())
}

/// List extended attribute names without following symlinks.
pub(crate) unsafe fn destack_fs_llistxattr_utf16(
    binding: &BindingCallContext,
    out: *mut NativeArray<NativeArray<u8>>,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve inputs
    let wide = wide_from_utf16(path, "path")?;

    // open the handle and read the list
    let handle = open_xattr_path(&wide, false, false)?;
    let buffer = with_handle(handle, query_ea_list)?;

    // write the output
    unsafe {
        *out = decode_xattr_list_bytes(binding, buffer)?;
    }

    Ok(())
}

/// List extended attribute names by handle.
pub(crate) unsafe fn destack_fs_flistxattr_handle(
    binding: &BindingCallContext,
    out: *mut NativeArray<NativeArray<u8>>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve inputs
    let handle = file_handle(binding, handle)?;

    // read the list
    let buffer = query_ea_list(handle)?;
    // write the output
    unsafe {
        *out = decode_xattr_list_bytes(binding, buffer)?;
    }

    Ok(())
}

/// Remove an extended attribute by path with a raw name payload.
pub(crate) unsafe fn destack_fs_removexattr_bytes(
    _binding: &BindingCallContext,
    path: PathBytes,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // resolve inputs
    let name = resolve_xattr_name_bytes(name)?;
    let wide = wide_from_bytes(path, "path")?;

    // open the handle and remove the entry
    let handle = open_xattr_path(&wide, true, true)?;
    with_handle(handle, |handle| remove_ea_entry(handle, &name))
}

/// Remove an extended attribute by path.
pub(crate) unsafe fn destack_fs_removexattr_utf16(
    _binding: &BindingCallContext,
    path: PathUtf16,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // resolve inputs
    let name = resolve_xattr_name_bytes(name)?;
    let wide = wide_from_utf16(path, "path")?;

    // open the handle and remove the entry
    let handle = open_xattr_path(&wide, true, true)?;
    with_handle(handle, |handle| remove_ea_entry(handle, &name))
}

/// Remove an extended attribute without following symlinks, using a raw name payload.
pub(crate) unsafe fn destack_fs_lremovexattr_bytes(
    _binding: &BindingCallContext,
    path: PathBytes,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // resolve inputs
    let name = resolve_xattr_name_bytes(name)?;
    let wide = wide_from_bytes(path, "path")?;

    // open the handle and remove the entry
    let handle = open_xattr_path(&wide, true, false)?;
    with_handle(handle, |handle| remove_ea_entry(handle, &name))
}

/// Remove an extended attribute without following symlinks.
pub(crate) unsafe fn destack_fs_lremovexattr_utf16(
    _binding: &BindingCallContext,
    path: PathUtf16,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // resolve inputs
    let name = resolve_xattr_name_bytes(name)?;
    let wide = wide_from_utf16(path, "path")?;

    // open the handle and remove the entry
    let handle = open_xattr_path(&wide, true, false)?;
    with_handle(handle, |handle| remove_ea_entry(handle, &name))
}

/// Remove an extended attribute by handle.
pub(crate) unsafe fn destack_fs_fremovexattr_handle(
    binding: &BindingCallContext,
    handle: FileHandle,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // resolve inputs
    let name = resolve_xattr_name_bytes(name)?;
    let handle = file_handle(binding, handle)?;

    // remove the entry
    remove_ea_entry(handle, &name)
}

/// Read an extended attribute by path.
pub(crate) unsafe fn destack_fs_getxattr(
    binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    path: OsPath,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    // resolve the string name as raw bytes
    let name = xattr_name_slice_from_string(binding, name)?;

    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            destack_fs_getxattr_bytes(binding, out, path_bytes.bytes, name)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            destack_fs_getxattr_utf16(binding, out, path_utf16.utf16, name)
        },
    }
}

/// Read an extended attribute without following symlinks.
pub(crate) unsafe fn destack_fs_lgetxattr(
    binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    path: OsPath,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    // resolve the string name as raw bytes
    let name = xattr_name_slice_from_string(binding, name)?;

    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            destack_fs_lgetxattr_bytes(binding, out, path_bytes.bytes, name)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            destack_fs_lgetxattr_utf16(binding, out, path_utf16.utf16, name)
        },
    }
}

/// Read an extended attribute by handle.
pub(crate) unsafe fn destack_fs_fgetxattr(
    binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    handle: FileHandle,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    // resolve the string name as raw bytes
    let name = xattr_name_slice_from_string(binding, name)?;

    // dispatch through the raw handle lane
    unsafe { destack_fs_fgetxattr_handle(binding, out, handle, name) }
}

/// Set an extended attribute by path.
pub(crate) unsafe fn destack_fs_setxattr(
    binding: &BindingCallContext,
    path: OsPath,
    name: NativeStringRef,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    // resolve the string name as raw bytes
    let name = xattr_name_slice_from_string(binding, name)?;

    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            destack_fs_setxattr_bytes(binding, path_bytes.bytes, name, value, flags)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            destack_fs_setxattr_utf16(binding, path_utf16.utf16, name, value, flags)
        },
    }
}

/// Set an extended attribute without following symlinks.
pub(crate) unsafe fn destack_fs_lsetxattr(
    binding: &BindingCallContext,
    path: OsPath,
    name: NativeStringRef,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    // resolve the string name as raw bytes
    let name = xattr_name_slice_from_string(binding, name)?;

    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            destack_fs_lsetxattr_bytes(binding, path_bytes.bytes, name, value, flags)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            destack_fs_lsetxattr_utf16(binding, path_utf16.utf16, name, value, flags)
        },
    }
}

/// Set an extended attribute by handle.
pub(crate) unsafe fn destack_fs_fsetxattr(
    binding: &BindingCallContext,
    handle: FileHandle,
    name: NativeStringRef,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    // resolve the string name as raw bytes
    let name = xattr_name_slice_from_string(binding, name)?;

    // dispatch through the raw handle lane
    unsafe { destack_fs_fsetxattr_handle(binding, handle, name, value, flags) }
}

/// List extended attribute names by path.
pub(crate) unsafe fn destack_fs_listxattr(
    binding: &BindingCallContext,
    out: *mut NativeArray<NativeStringRef>,
    path: OsPath,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read raw names for the selected path encoding
    let mut names = std::mem::MaybeUninit::<NativeArray<NativeArray<u8>>>::uninit();
    match path {
        OsPath::OsPathBytes(path_bytes) => {
            unsafe { destack_fs_listxattr_bytes(binding, names.as_mut_ptr(), path_bytes.bytes) }?;
        }
        OsPath::OsPathUtf16(path_utf16) => {
            unsafe { destack_fs_listxattr_utf16(binding, names.as_mut_ptr(), path_utf16.utf16) }?;
        }
    }
    let names = unsafe { names.assume_init() };
    let names = xattr_name_strings_from_bytes(binding, names)?;

    // write the decoded output
    unsafe {
        *out = names;
    }

    Ok(())
}

/// List extended attribute names without following symlinks.
pub(crate) unsafe fn destack_fs_llistxattr(
    binding: &BindingCallContext,
    out: *mut NativeArray<NativeStringRef>,
    path: OsPath,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read raw names for the selected path encoding
    let mut names = std::mem::MaybeUninit::<NativeArray<NativeArray<u8>>>::uninit();
    match path {
        OsPath::OsPathBytes(path_bytes) => {
            unsafe { destack_fs_llistxattr_bytes(binding, names.as_mut_ptr(), path_bytes.bytes) }?;
        }
        OsPath::OsPathUtf16(path_utf16) => {
            unsafe { destack_fs_llistxattr_utf16(binding, names.as_mut_ptr(), path_utf16.utf16) }?;
        }
    }
    let names = unsafe { names.assume_init() };
    let names = xattr_name_strings_from_bytes(binding, names)?;

    // write the decoded output
    unsafe {
        *out = names;
    }

    Ok(())
}

/// List extended attribute names by handle.
pub(crate) unsafe fn destack_fs_flistxattr(
    binding: &BindingCallContext,
    out: *mut NativeArray<NativeStringRef>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read raw names from the handle lane
    let mut names = std::mem::MaybeUninit::<NativeArray<NativeArray<u8>>>::uninit();
    unsafe { destack_fs_flistxattr_handle(binding, names.as_mut_ptr(), handle) }?;
    let names = unsafe { names.assume_init() };
    let names = xattr_name_strings_from_bytes(binding, names)?;

    // write the decoded output
    unsafe {
        *out = names;
    }

    Ok(())
}

/// Remove an extended attribute by path.
pub(crate) unsafe fn destack_fs_removexattr(
    binding: &BindingCallContext,
    path: OsPath,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    // resolve the string name as raw bytes
    let name = xattr_name_slice_from_string(binding, name)?;

    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            destack_fs_removexattr_bytes(binding, path_bytes.bytes, name)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            destack_fs_removexattr_utf16(binding, path_utf16.utf16, name)
        },
    }
}

/// Remove an extended attribute without following symlinks.
pub(crate) unsafe fn destack_fs_lremovexattr(
    binding: &BindingCallContext,
    path: OsPath,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    // resolve the string name as raw bytes
    let name = xattr_name_slice_from_string(binding, name)?;

    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            destack_fs_lremovexattr_bytes(binding, path_bytes.bytes, name)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            destack_fs_lremovexattr_utf16(binding, path_utf16.utf16, name)
        },
    }
}

/// Remove an extended attribute by handle.
pub(crate) unsafe fn destack_fs_fremovexattr(
    binding: &BindingCallContext,
    handle: FileHandle,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    // resolve the string name as raw bytes
    let name = xattr_name_slice_from_string(binding, name)?;

    // dispatch through the raw handle lane
    unsafe { destack_fs_fremovexattr_handle(binding, handle, name) }
}
