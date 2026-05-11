use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::System::Memory::{
    CreateFileMappingW, FILE_MAP_ALL_ACCESS, MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile,
    OpenFileMappingW, PAGE_READWRITE, UnmapViewOfFile,
};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeStringRef;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::ipc::SharedMemoryMapping;
use crate::platform::{PlatformError, core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::core::{register_shared_memory_handle, shared_memory_handle, wide_name};

/// Create one named shared-memory object.
const SHARED_MEMORY_CREATE_OPERATION: &str = "destack.ipc.sharedMemory.create";
/// Map one shared-memory range.
const SHARED_MEMORY_MAP_OPERATION: &str = "destack.ipc.sharedMemory.map";

/// Build one already-exists runtime error.
fn already_exists(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoAlreadyExists),
        None,
        Some(ERROR_ALREADY_EXISTS as i32),
        Some(operation.to_string()),
        None,
        "shared-memory object already exists".to_string(),
    ))
    .boxed()
}

/// Close one shared memory object handle.
pub(crate) unsafe fn destack_ipc_shared_memory_close(
    binding: &BindingCallContext,
    handle: resource::SharedMemoryHandle,
) -> RuntimeResult<()> {
    let removed = binding.worker().resources.remove_and_finalize(
        binding.world(),
        handle.0,
        Some(binding.engine()),
    );
    if !removed {
        return Err(core_platform::invalid_argument(
            "handle",
            "destack.ipc.sharedMemory.close expected one valid shared-memory handle",
        ));
    }

    Ok(())
}

/// Create one named shared memory object.
pub(crate) unsafe fn destack_ipc_shared_memory_create(
    binding: &BindingCallContext,
    out: *mut resource::SharedMemoryHandle,
    name: NativeStringRef,
    size: u64,
    flags: u32,
) -> RuntimeResult<()> {
    // validate output and input flags
    core_platform::ensure_out(out, "out")?;
    core_platform::ensure_zero_flags(flags, "flags")?;
    if size == 0 {
        return Err(core_platform::invalid_argument(
            "size",
            "size must be greater than zero",
        ));
    }

    // decode one UTF-16 mapping name
    let name = wide_name(name, "name")?;

    // create one paging-file backed mapping object
    let size_high = (size >> 32) as u32;
    let size_low = size as u32;
    let mapping = unsafe {
        CreateFileMappingW(
            INVALID_HANDLE_VALUE,
            std::ptr::null_mut(),
            PAGE_READWRITE,
            size_high,
            size_low,
            name.as_ptr(),
        )
    };
    if mapping == 0 || mapping == INVALID_HANDLE_VALUE {
        return Err(core_platform::io_error("CreateFileMappingW"));
    }

    // reject create requests that collided with an existing mapping
    let create_error = unsafe { GetLastError() };
    if create_error == ERROR_ALREADY_EXISTS {
        unsafe {
            CloseHandle(mapping);
        }
        return Err(already_exists(SHARED_MEMORY_CREATE_OPERATION));
    }

    // register mapping handle and write output
    let handle = register_shared_memory_handle(binding, mapping);
    unsafe {
        out.write(handle);
    }

    Ok(())
}

/// Map one shared memory range.
pub(crate) unsafe fn destack_ipc_shared_memory_map(
    binding: &BindingCallContext,
    out: *mut SharedMemoryMapping,
    handle: resource::SharedMemoryHandle,
    offset: u64,
    length: u64,
    flags: u32,
) -> RuntimeResult<()> {
    // validate output and map options
    core_platform::ensure_out(out, "out")?;
    core_platform::ensure_zero_flags(flags, "flags")?;
    if length == 0 {
        return Err(core_platform::invalid_argument(
            "length",
            "length must be greater than zero",
        ));
    }

    // resolve one mapping-object handle
    let mapping = shared_memory_handle(binding, handle, SHARED_MEMORY_MAP_OPERATION)?;

    // decode map offset and length into host ranges
    let offset_high = (offset >> 32) as u32;
    let offset_low = offset as u32;
    let length = core_platform::u64_to_usize(length, "length")?;

    // map one shared view into process virtual memory
    let address = unsafe {
        MapViewOfFile(
            mapping,
            FILE_MAP_ALL_ACCESS,
            offset_high,
            offset_low,
            length,
        )
    };
    if address.Value.is_null() {
        return Err(core_platform::io_error("MapViewOfFile"));
    }

    // write mapping descriptor output
    let mapped_address = core_platform::usize_to_u64(address.Value as usize, "out.address")?;
    let mapping_length = core_platform::usize_to_u64(length, "out.length")?;
    let mapping = SharedMemoryMapping {
        address: mapped_address,
        length: mapping_length,
    };
    unsafe {
        out.write(mapping);
    }

    Ok(())
}

/// Open one named shared memory object.
pub(crate) unsafe fn destack_ipc_shared_memory_open(
    binding: &BindingCallContext,
    out: *mut resource::SharedMemoryHandle,
    name: NativeStringRef,
    flags: u32,
) -> RuntimeResult<()> {
    // validate output and input flags
    core_platform::ensure_out(out, "out")?;
    core_platform::ensure_zero_flags(flags, "flags")?;

    // decode one UTF-16 mapping name
    let name = wide_name(name, "name")?;

    // open one existing named mapping object
    let mapping = unsafe { OpenFileMappingW(FILE_MAP_ALL_ACCESS, 0, name.as_ptr()) };
    if mapping == 0 || mapping == INVALID_HANDLE_VALUE {
        return Err(core_platform::io_error("OpenFileMappingW"));
    }

    // register mapping handle and write output
    let handle = register_shared_memory_handle(binding, mapping);
    unsafe {
        out.write(handle);
    }

    Ok(())
}

/// Unmap one shared memory range.
pub(crate) unsafe fn destack_ipc_shared_memory_unmap(
    _binding: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    // validate unmap address and length inputs
    if address == 0 {
        return Err(core_platform::invalid_argument(
            "address",
            "address must be non-zero",
        ));
    }
    if length == 0 {
        return Err(core_platform::invalid_argument(
            "length",
            "length must be greater than zero",
        ));
    }

    // unmap one mapped view address
    let address = core_platform::u64_to_usize(address, "address")?;
    let view = MEMORY_MAPPED_VIEW_ADDRESS {
        Value: address as *mut core::ffi::c_void,
    };
    let status = unsafe { UnmapViewOfFile(view) };
    if status == 0 {
        return Err(core_platform::io_error("UnmapViewOfFile"));
    }

    Ok(())
}
