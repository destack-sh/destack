use std::collections::HashMap;
use std::ptr::null_mut;
use std::sync::Arc;

use parking_lot::Mutex;

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_CALL_NOT_IMPLEMENTED, ERROR_NOT_SUPPORTED, HANDLE,
};
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
use crate::platform::abi::NativeSlice;
use crate::platform::fs::core::{DecodedMmapFlags, decode_mmap_flags, validate_mapping_length};
use crate::platform::fs::{
    FileHandle, FileOffset, FileSize, MmapAdvice, MmapFlags, MmapProt, MmapSyncFlags,
};
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;

/// Stored metadata for a Windows mapping.
#[derive(Debug, Clone, Copy)]
struct MappingEntry {
    /// Mapping handle for file-backed mappings.
    mapping: HANDLE,
    /// Whether the mapping is anonymous.
    is_anon: bool,
}

/// Runtime-owned mutable state for windows mmap lanes.
#[derive(Debug, Default)]
pub(crate) struct WindowsMmapRuntimeState {
    /// Mapping metadata table keyed by base address.
    mapping_table: Mutex<HashMap<usize, MappingEntry>>,
}

/// Return runtime-owned windows mmap mutable state.
fn windows_mmap_runtime_state(binding: &BindingCallContext) -> Arc<WindowsMmapRuntimeState> {
    binding
        .worker()
        .platform_state
        .fs
        .windows_mmap_runtime_state(WindowsMmapRuntimeState::default)
}

/// Insert a mapping entry for a pointer.
fn insert_mapping(
    runtime_state: &WindowsMmapRuntimeState,
    ptr: *mut u8,
    mapping: HANDLE,
    is_anon: bool,
) {
    let mut table = runtime_state.mapping_table.lock();
    table.insert(ptr as usize, MappingEntry { mapping, is_anon });
}

/// Remove a mapping entry for a pointer.
fn remove_mapping(runtime_state: &WindowsMmapRuntimeState, ptr: *mut u8) -> Option<MappingEntry> {
    let mut table = runtime_state.mapping_table.lock();
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
fn map_view_access(prot: MmapProt, flags: DecodedMmapFlags) -> RuntimeResult<u32> {
    let mut access = 0u32;
    if !flags.is_shared {
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

    Ok(access)
}

/// Create a file-backed memory mapping.
pub(crate) unsafe fn destack_fs_mmap_file(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    prot: MmapProt,
    flags: MmapFlags,
) -> RuntimeResult<()> {
    let runtime_state = windows_mmap_runtime_state(binding);

    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate inputs
    let mmap_flags = decode_mmap_flags(flags, false)?;
    if mmap_flags.is_fixed {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.fs.mmapFile MAP_FIXED",
        ))
        .boxed());
    }

    // resolve the file handle
    let handle = file_handle(binding, handle)?;
    let length = validate_mapping_length(length)?;

    // configure protections and access
    let protection = map_protection(prot);
    let access = map_view_access(prot, mmap_flags)?;
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
    insert_mapping(&runtime_state, ptr, mapping, false);

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
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    length: FileSize,
    prot: MmapProt,
    flags: MmapFlags,
) -> RuntimeResult<()> {
    let runtime_state = windows_mmap_runtime_state(binding);

    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let mmap_flags = decode_mmap_flags(flags, true)?;
    if mmap_flags.is_fixed {
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
    insert_mapping(&runtime_state, mapping as *mut u8, 0, true);

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
    binding: &BindingCallContext,
    mapping: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let runtime_state = windows_mmap_runtime_state(binding);

    let slice = unsafe { mapping.as_slice()? };
    if slice.is_empty() {
        return Ok(());
    }

    let entry = remove_mapping(&runtime_state, mapping.data);
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
    _binding: &BindingCallContext,
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
    binding: &BindingCallContext,
    mapping: NativeSlice<u8>,
    _flags: MmapSyncFlags,
) -> RuntimeResult<()> {
    let runtime_state = windows_mmap_runtime_state(binding);

    let slice = unsafe { mapping.as_slice()? };
    if slice.is_empty() {
        return Ok(());
    }

    // anonymous mappings do not have backing storage to flush
    let entry = {
        let table = runtime_state.mapping_table.lock();
        table.get(&(mapping.data as usize)).copied()
    };
    if matches!(entry, Some(entry) if entry.is_anon) {
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
    _binding: &BindingCallContext,
    mapping: NativeSlice<u8>,
    advice: MmapAdvice,
) -> RuntimeResult<()> {
    // operation tags
    const MADVISE_WILL_NEED_OPERATION: &str = "destack.fs.mmap.madvise.willNeed";
    const MADVISE_DONT_NEED_OPERATION: &str = "destack.fs.mmap.madvise.dontNeed";

    // classify Win32 status values that map to notSupported
    let is_not_supported_status =
        |status: u32| status == ERROR_NOT_SUPPORTED || status == ERROR_CALL_NOT_IMPLEMENTED;

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
                let code = core_platform::last_error_code() as u32;
                if code == 0 || is_not_supported_status(code) {
                    return Err(core_platform::not_supported(MADVISE_WILL_NEED_OPERATION));
                }

                return Err(core_platform::io_error_with_code(
                    "PrefetchVirtualMemory",
                    code as i32,
                ));
            }
        }
        MmapAdvice::DontNeed => {
            let status =
                unsafe { DiscardVirtualMemory(mapping.data as *mut _, mapping.len as usize) };
            if status != 0 {
                if is_not_supported_status(status) {
                    return Err(core_platform::not_supported(MADVISE_DONT_NEED_OPERATION));
                }

                return Err(core_platform::io_error_with_code(
                    "DiscardVirtualMemory",
                    status as i32,
                ));
            }
        }
        _ => {}
    }

    Ok(())
}
