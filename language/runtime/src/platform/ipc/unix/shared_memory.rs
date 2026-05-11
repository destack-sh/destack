use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeStringRef;
use crate::platform::ipc::SharedMemoryMapping;
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

#[cfg(not(target_os = "android"))]
use super::core::posix_name;
#[cfg(not(target_os = "android"))]
use super::core::register_shared_memory_descriptor;
use super::core::{io_error, shared_memory_descriptor};

/// Create one named shared-memory object.
const SHARED_MEMORY_CREATE_OPERATION: &str = "destack.ipc.sharedMemory.create";
/// Open one named shared-memory object.
const SHARED_MEMORY_OPEN_OPERATION: &str = "destack.ipc.sharedMemory.open";
/// Map one shared-memory range.
const SHARED_MEMORY_MAP_OPERATION: &str = "destack.ipc.sharedMemory.map";
/// Unmap one shared-memory range.
const SHARED_MEMORY_UNMAP_OPERATION: &str = "destack.ipc.sharedMemory.unmap";

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

    #[cfg(not(target_os = "android"))]
    {
        // decode and normalize the shared-memory name
        let name = posix_name(name, "name")?;

        // create one new named shared-memory descriptor
        let descriptor = unsafe {
            libc::shm_open(
                name.as_ptr(),
                libc::O_CREAT | libc::O_EXCL | libc::O_RDWR,
                0o600,
            )
        };
        if descriptor < 0 {
            return Err(io_error(
                SHARED_MEMORY_CREATE_OPERATION,
                "shm_open",
                "failed to create shared-memory object",
            ));
        }

        // size the backing object before exposing the handle
        let size = i64::try_from(size).map_err(|_| {
            core_platform::invalid_argument("size", "size exceeds host off_t range")
        })?;
        let truncate_status = unsafe { libc::ftruncate(descriptor, size) };
        if truncate_status != 0 {
            unsafe {
                libc::close(descriptor);
            }
            return Err(io_error(
                SHARED_MEMORY_CREATE_OPERATION,
                "ftruncate",
                "failed to resize shared-memory object",
            ));
        }

        // register descriptor and write handle output
        let handle = register_shared_memory_descriptor(binding, descriptor);
        unsafe {
            out.write(handle);
        }

        Ok(())
    }

    #[cfg(target_os = "android")]
    {
        let _ = (binding, name);
        Err(core_platform::not_supported(SHARED_MEMORY_CREATE_OPERATION))
    }
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

    // resolve one shared-memory descriptor
    let descriptor = shared_memory_descriptor(binding, handle, SHARED_MEMORY_MAP_OPERATION)?;

    // decode length and offset into host ranges
    let length = core_platform::u64_to_usize(length, "length")?;
    let offset = i64::try_from(offset).map_err(|_| {
        core_platform::invalid_argument("offset", "offset exceeds host off_t range")
    })?;

    // map one shared view into process virtual memory
    let mapped = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            length,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            descriptor,
            offset,
        )
    };
    if mapped == libc::MAP_FAILED {
        return Err(io_error(
            SHARED_MEMORY_MAP_OPERATION,
            "mmap",
            "failed to map shared-memory object",
        ));
    }

    // write mapping descriptor output
    let mapped_address = core_platform::usize_to_u64(mapped as usize, "out.address")?;
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

    #[cfg(not(target_os = "android"))]
    {
        // decode and normalize the shared-memory name
        let name = posix_name(name, "name")?;

        // open one existing shared-memory descriptor
        let descriptor = unsafe { libc::shm_open(name.as_ptr(), libc::O_RDWR, 0o600) };
        if descriptor < 0 {
            return Err(io_error(
                SHARED_MEMORY_OPEN_OPERATION,
                "shm_open",
                "failed to open shared-memory object",
            ));
        }

        // register descriptor and write handle output
        let handle = register_shared_memory_descriptor(binding, descriptor);
        unsafe {
            out.write(handle);
        }

        Ok(())
    }

    #[cfg(target_os = "android")]
    {
        let _ = (binding, name);
        Err(core_platform::not_supported(SHARED_MEMORY_OPEN_OPERATION))
    }
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

    // issue one host unmap call
    let address = core_platform::u64_to_usize(address, "address")?;
    let pointer = address as *mut libc::c_void;
    let length = core_platform::u64_to_usize(length, "length")?;
    let rc = unsafe { libc::munmap(pointer, length) };
    if rc != 0 {
        return Err(io_error(
            SHARED_MEMORY_UNMAP_OPERATION,
            "munmap",
            "failed to unmap shared-memory range",
        ));
    }

    Ok(())
}
