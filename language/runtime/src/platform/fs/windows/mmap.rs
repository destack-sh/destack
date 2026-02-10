use std::collections::HashMap;
use std::ptr::null_mut;
use std::sync::OnceLock;

use parking_lot::Mutex;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::Memory::{
    CreateFileMappingW, DiscardVirtualMemory, FILE_MAP_COPY, FILE_MAP_EXECUTE, FILE_MAP_READ,
    FILE_MAP_WRITE, FlushViewOfFile, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE,
    MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile, PAGE_EXECUTE, PAGE_EXECUTE_READ,
    PAGE_EXECUTE_READWRITE, PAGE_NOACCESS, PAGE_READONLY, PAGE_READWRITE, PrefetchVirtualMemory,
    UnmapViewOfFile, VirtualAlloc, VirtualFree, VirtualProtect, WIN32_MEMORY_RANGE_ENTRY,
};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::{
    FileHandle, FileOffset, FileSize, MmapAdvice, MmapFlags, MmapProt, MmapSyncFlags,
};
use crate::platform::{NativeSlice, PlatformError};
use crate::runtime::RuntimeCallContext;

/// Stored metadata for a Windows mapping.
#[derive(Debug, Clone, Copy)]
struct MappingEntry {
    /// Mapping handle for file-backed mappings.
    mapping: HANDLE,
    /// Whether the mapping is anonymous.
    is_anon: bool,
}

/// Return the global mapping table.
fn mapping_table() -> &'static Mutex<HashMap<usize, MappingEntry>> {
    static TABLE: OnceLock<Mutex<HashMap<usize, MappingEntry>>> = OnceLock::new();
    TABLE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Insert a mapping entry for a pointer.
fn insert_mapping(ptr: *mut u8, mapping: HANDLE, is_anon: bool) {
    let mut table = mapping_table().lock();
    table.insert(ptr as usize, MappingEntry { mapping, is_anon });
}

/// Remove a mapping entry for a pointer.
fn remove_mapping(ptr: *mut u8) -> Option<MappingEntry> {
    let mut table = mapping_table().lock();
    table.remove(&(ptr as usize))
}

/// Convert a MmapProt mask into a Windows protection value.
fn map_protection(prot: MmapProt) -> u32 {
    let read = prot.0 & 0x1 != 0;
    let write = prot.0 & 0x2 != 0;
    let exec = prot.0 & 0x4 != 0;

    match (read, write, exec) {
        (false, false, false) => PAGE_NOACCESS,
        (true, false, false) => PAGE_READONLY,
        (true, true, false) => PAGE_READWRITE,
        (true, false, true) => PAGE_EXECUTE_READ,
        (true, true, true) => PAGE_EXECUTE_READWRITE,
        (false, false, true) => PAGE_EXECUTE,
        (false, true, false) => PAGE_READWRITE,
        (false, true, true) => PAGE_EXECUTE_READWRITE,
    }
}

/// Convert protection and flags into map view access.
fn map_view_access(prot: MmapProt, flags: MmapFlags) -> RuntimeResult<u32> {
    let is_private = flags.0 & 0x2 != 0;
    let is_shared = flags.0 & 0x1 != 0;
    if is_private && is_shared {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "MAP_PRIVATE and MAP_SHARED are mutually exclusive",
        ))
        .boxed());
    }

    let mut access = 0u32;
    if is_private {
        access |= FILE_MAP_COPY;
    } else {
        if prot.0 & 0x1 != 0 {
            access |= FILE_MAP_READ;
        }
        if prot.0 & 0x2 != 0 {
            access |= FILE_MAP_WRITE;
        }
    }
    if prot.0 & 0x4 != 0 {
        access |= FILE_MAP_EXECUTE;
    }

    if access == 0 {
        access = FILE_MAP_READ;
    }

    Ok(access)
}

/// Validate mapping length against NativeSlice limits.
fn validate_mapping_length(length: FileSize) -> RuntimeResult<usize> {
    if length.0 > u64::from(u32::MAX) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "length",
            "mapping length exceeds NativeSlice limits",
        ))
        .boxed());
    }

    Ok(length.0 as usize)
}

/// Create a file-backed memory mapping.
pub(crate) unsafe fn destack_fs_mmap_file(
    context: &RuntimeCallContext,
    out: *mut NativeSlice<u8>,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    prot: MmapProt,
    flags: MmapFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate inputs
    if flags.0 & 0x20 != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "MAP_ANON is invalid for file mappings",
        ))
        .boxed());
    }
    if flags.0 & 0x10 != 0 {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.fs.mmapFile MAP_FIXED",
        ))
        .boxed());
    }

    // resolve the file handle
    let handle = file_handle(context, handle)?;
    let length = validate_mapping_length(length)?;

    // configure protections and access
    let protection = map_protection(prot);
    let access = map_view_access(prot, flags)?;
    if offset.0 < 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "offset",
            "offset must be non-negative",
        ))
        .boxed());
    }
    let offset = offset.0 as u64;
    let max_size = offset.checked_add(length as u64).ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "length",
            "mapping end overflows",
        ))
        .boxed()
    })?;
    let max_size_high = (max_size >> 32) as u32;
    let max_size_low = (max_size & 0xffff_ffff) as u32;

    // create the mapping handle
    let mapping = unsafe {
        CreateFileMappingW(
            handle,
            null_mut(),
            protection,
            max_size_high,
            max_size_low,
            null_mut(),
        )
    };
    if mapping == 0 {
        return Err(last_os_error("CreateFileMappingW", None));
    }

    // map the view
    let offset_high = (offset >> 32) as u32;
    let offset_low = (offset & 0xffff_ffff) as u32;
    let view = unsafe { MapViewOfFile(mapping, access, offset_high, offset_low, length) };
    if view.Value.is_null() {
        unsafe {
            CloseHandle(mapping);
        }
        return Err(last_os_error("MapViewOfFile", None));
    }

    // record the mapping
    let ptr = view.Value as *mut u8;
    insert_mapping(ptr, mapping, false);

    unsafe {
        *out = NativeSlice {
            data: ptr,
            len: length as u32,
        };
    }

    Ok(())
}

/// Create an anonymous memory mapping.
pub(crate) unsafe fn destack_fs_mmap_anonymous(
    _context: &RuntimeCallContext,
    out: *mut NativeSlice<u8>,
    length: FileSize,
    prot: MmapProt,
    flags: MmapFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    if flags.0 & 0x10 != 0 {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.fs.mmapAnonymous MAP_FIXED",
        ))
        .boxed());
    }

    // validate inputs
    let length = validate_mapping_length(length)?;
    let protection = map_protection(prot);

    // allocate the mapping
    let mapping = unsafe { VirtualAlloc(null_mut(), length, MEM_COMMIT | MEM_RESERVE, protection) };
    if mapping.is_null() {
        return Err(last_os_error("VirtualAlloc", None));
    }

    // record the mapping
    insert_mapping(mapping as *mut u8, 0, true);

    unsafe {
        *out = NativeSlice {
            data: mapping as *mut u8,
            len: length as u32,
        };
    }

    Ok(())
}

/// Unmap a memory region.
pub(crate) unsafe fn destack_fs_munmap(
    _context: &RuntimeCallContext,
    mapping: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let slice = unsafe { mapping.as_slice()? };
    if slice.is_empty() {
        return Ok(());
    }

    let entry = remove_mapping(mapping.data);
    if let Some(entry) = entry {
        if entry.is_anon {
            let rc = unsafe { VirtualFree(mapping.data as *mut _, 0, MEM_RELEASE) };
            if rc == 0 {
                return Err(last_os_error("VirtualFree", None));
            }
        } else {
            let view = MEMORY_MAPPED_VIEW_ADDRESS {
                Value: mapping.data as *mut _,
            };
            let rc = unsafe { UnmapViewOfFile(view) };
            if rc == 0 {
                return Err(last_os_error("UnmapViewOfFile", None));
            }
            unsafe {
                CloseHandle(entry.mapping);
            }
        }
        return Ok(());
    }

    let view = MEMORY_MAPPED_VIEW_ADDRESS {
        Value: mapping.data as *mut _,
    };
    let rc = unsafe { UnmapViewOfFile(view) };
    if rc == 0 {
        return Err(last_os_error("UnmapViewOfFile", None));
    }

    Ok(())
}

/// Change memory protection for a mapping.
pub(crate) unsafe fn destack_fs_mprotect(
    _context: &RuntimeCallContext,
    mapping: NativeSlice<u8>,
    prot: MmapProt,
) -> RuntimeResult<()> {
    let slice = unsafe { mapping.as_slice()? };
    if slice.is_empty() {
        return Ok(());
    }

    let mut old = 0u32;
    let rc = unsafe {
        VirtualProtect(
            mapping.data as *mut _,
            mapping.len as usize,
            map_protection(prot),
            &mut old,
        )
    };
    if rc == 0 {
        return Err(last_os_error("VirtualProtect", None));
    }

    Ok(())
}

/// Flush a mapping to storage.
pub(crate) unsafe fn destack_fs_msync(
    _context: &RuntimeCallContext,
    mapping: NativeSlice<u8>,
    _flags: MmapSyncFlags,
) -> RuntimeResult<()> {
    let slice = unsafe { mapping.as_slice()? };
    if slice.is_empty() {
        return Ok(());
    }

    let rc = unsafe { FlushViewOfFile(mapping.data as *const _, mapping.len as usize) };
    if rc == 0 {
        return Err(last_os_error("FlushViewOfFile", None));
    }

    Ok(())
}

/// Advise the kernel about access patterns.
pub(crate) unsafe fn destack_fs_madvise(
    _context: &RuntimeCallContext,
    mapping: NativeSlice<u8>,
    advice: MmapAdvice,
) -> RuntimeResult<()> {
    let slice = unsafe { mapping.as_slice()? };
    if slice.is_empty() {
        return Ok(());
    }

    match advice {
        MmapAdvice::WillNeed => {
            let range = WIN32_MEMORY_RANGE_ENTRY {
                VirtualAddress: mapping.data as *mut _,
                NumberOfBytes: mapping.len as usize,
            };
            let rc = unsafe { PrefetchVirtualMemory(GetCurrentProcess(), 1, &range, 0) };
            if rc == 0 {
                return Err(last_os_error("PrefetchVirtualMemory", None));
            }
        }
        MmapAdvice::DontNeed => {
            let rc = unsafe { DiscardVirtualMemory(mapping.data as *mut _, mapping.len as usize) };
            if rc == 0 {
                return Err(last_os_error("DiscardVirtualMemory", None));
            }
        }
        _ => {}
    }

    Ok(())
}
