use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::Storage::FileSystem::{ReadFile, WriteFile};
use windows_sys::Win32::System::Pipes::CreatePipe;

use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::ipc::PipePair;
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::core::{pipe_handle, register_pipe_handle, validate_handle};

/// Open one unnamed pipe pair.
const PIPE_OPEN_OPERATION: &str = "destack.ipc.pipe.open";
/// Read bytes from one pipe endpoint.
const PIPE_READ_OPERATION: &str = "destack.ipc.pipe.read";
/// Write bytes to one pipe endpoint.
const PIPE_WRITE_OPERATION: &str = "destack.ipc.pipe.write";

/// Close one pipe endpoint.
///
/// Close one endpoint of a pipe pair.
/// Pending readers and writers observe host close semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses close(2) on Unix and CloseHandle on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.pipe`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_ipc_pipe_close(
    binding: &BindingCallContext,
    handle: resource::PipeHandle,
) -> RuntimeResult<()> {
    let removed = binding.worker().resources.remove_and_finalize(
        &binding.world(),
        handle.0,
        Some(binding.engine()),
    );
    if !removed {
        return Err(core_platform::invalid_argument(
            "handle",
            "destack.ipc.pipe.close expected one valid pipe handle",
        ));
    }

    Ok(())
}

/// Create one unnamed pipe pair.
///
/// Create one local pipe with read and write endpoints.
/// Endpoint inheritance and blocking mode follow host pipe semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses pipe2 or pipe on Unix and CreatePipe on Windows.
///
/// # Errors
/// Returns ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `ipc.pipe`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_ipc_pipe_open(
    binding: &BindingCallContext,
    out: *mut PipePair,
    flags: u32,
) -> RuntimeResult<()> {
    // validate output and flag payload
    core_platform::ensure_out(out, "out")?;
    core_platform::ensure_zero_flags(flags, "flags")?;

    // create one windows anonymous pipe pair
    let mut read_handle: HANDLE = 0;
    let mut write_handle: HANDLE = 0;
    let status =
        unsafe { CreatePipe(&mut read_handle, &mut write_handle, std::ptr::null_mut(), 0) };
    if status == 0 {
        return Err(core_platform::io_error("CreatePipe"));
    }

    // validate both endpoint handles before registration
    if let Err(error) = validate_handle(read_handle, "out.read", PIPE_OPEN_OPERATION) {
        unsafe {
            CloseHandle(read_handle);
            CloseHandle(write_handle);
        }
        return Err(error);
    }
    if let Err(error) = validate_handle(write_handle, "out.write", PIPE_OPEN_OPERATION) {
        unsafe {
            CloseHandle(read_handle);
            CloseHandle(write_handle);
        }
        return Err(error);
    }

    // register both endpoints in the runtime resource table
    let read = register_pipe_handle(binding, read_handle);
    let write = register_pipe_handle(binding, write_handle);
    let pair = PipePair { read, write };

    // write pair output
    unsafe {
        out.write(pair);
    }

    Ok(())
}

/// Read bytes from a pipe endpoint.
///
/// Read bytes into caller-provided memory from one pipe endpoint.
/// Partial reads are preserved exactly as reported by the host.
///
/// # Platform
/// Unix and Windows.
/// Uses read(2) on Unix and ReadFile on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `ipc.pipe`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_ipc_pipe_read(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::PipeHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate output argument and resolve pipe handle
    core_platform::ensure_out(out, "out")?;
    let handle = pipe_handle(binding, handle, PIPE_READ_OPERATION)?;

    // decode caller buffer and validate host length range
    let bytes = unsafe { buffer.as_mut_slice()? };
    let length = u32::try_from(bytes.len()).map_err(|_| {
        core_platform::invalid_argument("buffer", "buffer length exceeds windows u32 range")
    })?;

    // issue one windows read call
    let mut bytes_read = 0u32;
    let status = unsafe {
        ReadFile(
            handle,
            bytes.as_mut_ptr(),
            length,
            &mut bytes_read,
            std::ptr::null_mut(),
        )
    };
    if status == 0 {
        return Err(core_platform::io_error("ReadFile"));
    }

    // write read-byte count
    unsafe {
        out.write(u64::from(bytes_read));
    }

    Ok(())
}

/// Write bytes to a pipe endpoint.
///
/// Write bytes from caller-provided memory to one pipe endpoint.
/// Partial writes are preserved exactly as reported by the host.
///
/// # Platform
/// Unix and Windows.
/// Uses write(2) on Unix and WriteFile on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `ipc.pipe`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_ipc_pipe_write(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::PipeHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate output argument and resolve pipe handle
    core_platform::ensure_out(out, "out")?;
    let handle = pipe_handle(binding, handle, PIPE_WRITE_OPERATION)?;

    // decode caller buffer and validate host length range
    let bytes = unsafe { buffer.as_slice()? };
    let length = u32::try_from(bytes.len()).map_err(|_| {
        core_platform::invalid_argument("buffer", "buffer length exceeds windows u32 range")
    })?;

    // issue one windows write call
    let mut bytes_written = 0u32;
    let status = unsafe {
        WriteFile(
            handle,
            bytes.as_ptr(),
            length,
            &mut bytes_written,
            std::ptr::null_mut(),
        )
    };
    if status == 0 {
        return Err(core_platform::io_error("WriteFile"));
    }

    // write written-byte count
    unsafe {
        out.write(u64::from(bytes_written));
    }

    Ok(())
}
