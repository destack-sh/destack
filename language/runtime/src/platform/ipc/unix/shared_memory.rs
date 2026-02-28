use crate::diagnostic::RuntimeResult;
use crate::platform::ipc::SharedMemoryMapping;
use crate::platform::{NativeStringRef, resource};
use crate::runtime::BindingCallContext;

use super::core::{
    ensure_out, ensure_zero_flags, invalid_argument, io_error, posix_name,
    register_shared_memory_descriptor, shared_memory_descriptor,
};

/// Create one named shared-memory object.
const SHARED_MEMORY_CREATE_OPERATION: &str = "destack.ipc.sharedMemory.create";
/// Open one named shared-memory object.
const SHARED_MEMORY_OPEN_OPERATION: &str = "destack.ipc.sharedMemory.open";
/// Map one shared-memory range.
const SHARED_MEMORY_MAP_OPERATION: &str = "destack.ipc.sharedMemory.map";
/// Unmap one shared-memory range.
const SHARED_MEMORY_UNMAP_OPERATION: &str = "destack.ipc.sharedMemory.unmap";

/// Close one shared memory object handle.
///
/// Close one shared memory handle without unmapping process mappings.
/// Mapping lifetime remains independent until explicit unmap calls.
///
/// # Platform
/// Unix and Windows.
/// Uses close(2) on Unix and CloseHandle on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.shared.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_ipc_shared_memory_close(
    context: &BindingCallContext,
    handle: resource::SharedMemoryHandle,
) -> RuntimeResult<()> {
    let removed = context.runtime().resources.remove_and_finalize(handle.0);
    if !removed {
        return Err(invalid_argument(
            "handle",
            "destack.ipc.sharedMemory.close expected one valid shared-memory handle",
        ));
    }

    Ok(())
}

/// Create one named shared memory object.
///
/// Create one shared memory object with explicit size and creation flags.
/// Name namespace and visibility follow host object manager semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses shm_open or memfd-style APIs on Unix and file mapping objects on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioAlreadyExists, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.shared.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_ipc_shared_memory_create(
    context: &BindingCallContext,
    out: *mut resource::SharedMemoryHandle,
    name: NativeStringRef,
    size: u64,
    flags: u32,
) -> RuntimeResult<()> {
    // validate output and input flags
    ensure_out(out, "out")?;
    ensure_zero_flags(flags, "flags")?;
    if size == 0 {
        return Err(invalid_argument("size", "size must be greater than zero"));
    }

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
    let size = i64::try_from(size)
        .map_err(|_| invalid_argument("size", "size exceeds host off_t range"))?;
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
    let handle = register_shared_memory_descriptor(context, descriptor);
    unsafe {
        out.write(handle);
    }

    Ok(())
}

/// Map one shared memory range.
///
/// Map one region of a shared memory object into the current process address space.
/// Mapping protection and coherence follow host virtual-memory semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses mmap family on Unix and MapViewOfFile on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.shared.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_ipc_shared_memory_map(
    context: &BindingCallContext,
    out: *mut SharedMemoryMapping,
    handle: resource::SharedMemoryHandle,
    offset: u64,
    length: u64,
    flags: u32,
) -> RuntimeResult<()> {
    // validate output and map options
    ensure_out(out, "out")?;
    ensure_zero_flags(flags, "flags")?;
    if length == 0 {
        return Err(invalid_argument(
            "length",
            "length must be greater than zero",
        ));
    }

    // resolve one shared-memory descriptor
    let descriptor = shared_memory_descriptor(context, handle, SHARED_MEMORY_MAP_OPERATION)?;

    // decode length and offset into host ranges
    let length = usize::try_from(length)
        .map_err(|_| invalid_argument("length", "length exceeds host usize range"))?;
    let offset = i64::try_from(offset)
        .map_err(|_| invalid_argument("offset", "offset exceeds host off_t range"))?;

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
    let mapped_address = u64::try_from(mapped as usize)
        .map_err(|_| invalid_argument("out.address", "mapped address exceeds u64 range"))?;
    let mapping_length = u64::try_from(length)
        .map_err(|_| invalid_argument("out.length", "length exceeds u64 range"))?;
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
///
/// Open one existing shared memory object by name.
/// Access rights and visibility follow host object manager semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses shm_open-style APIs on Unix and OpenFileMapping on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.shared.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_ipc_shared_memory_open(
    context: &BindingCallContext,
    out: *mut resource::SharedMemoryHandle,
    name: NativeStringRef,
    flags: u32,
) -> RuntimeResult<()> {
    // validate output and input flags
    ensure_out(out, "out")?;
    ensure_zero_flags(flags, "flags")?;

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
    let handle = register_shared_memory_descriptor(context, descriptor);
    unsafe {
        out.write(handle);
    }

    Ok(())
}

/// Unmap one shared memory range.
///
/// Unmap one previously mapped memory range from the process address space.
/// Unmap operation does not destroy the underlying shared memory object.
///
/// # Platform
/// Unix and Windows.
/// Uses munmap on Unix and UnmapViewOfFile on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.shared.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_ipc_shared_memory_unmap(
    _context: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    // validate unmap address and length inputs
    if address == 0 {
        return Err(invalid_argument("address", "address must be non-zero"));
    }
    if length == 0 {
        return Err(invalid_argument(
            "length",
            "length must be greater than zero",
        ));
    }

    // issue one host unmap call
    let address = usize::try_from(address)
        .map_err(|_| invalid_argument("address", "address exceeds host usize range"))?;
    let pointer = address as *mut libc::c_void;
    let length = usize::try_from(length)
        .map_err(|_| invalid_argument("length", "length exceeds host usize range"))?;
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
