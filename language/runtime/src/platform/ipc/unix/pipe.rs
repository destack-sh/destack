use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::ipc::PipePair;
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::core::{io_error, pipe_descriptor, register_pipe_descriptor};

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

    // create one unix pipe pair
    let mut descriptors = [0; 2];
    let rc = unsafe { libc::pipe(descriptors.as_mut_ptr()) };
    if rc != 0 {
        return Err(io_error(
            PIPE_OPEN_OPERATION,
            "pipe",
            "failed to create pipe",
        ));
    }

    // register both endpoints in the runtime resource table
    let read_handle = register_pipe_descriptor(binding, descriptors[0]);
    let write_handle = register_pipe_descriptor(binding, descriptors[1]);
    let pair = PipePair {
        read: read_handle,
        write: write_handle,
    };

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
    // validate output argument and resolve pipe descriptor
    core_platform::ensure_out(out, "out")?;
    let descriptor = pipe_descriptor(binding, handle, PIPE_READ_OPERATION)?;

    // decode caller buffer and issue one read call
    let bytes = unsafe { buffer.as_mut_slice()? };
    let rc = unsafe {
        libc::read(
            descriptor,
            bytes.as_mut_ptr().cast::<libc::c_void>(),
            bytes.len(),
        )
    };
    if rc < 0 {
        return Err(io_error(
            PIPE_READ_OPERATION,
            "read",
            "failed to read pipe bytes",
        ));
    }

    // write read-byte count
    unsafe {
        out.write(rc as u64);
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
    // validate output argument and resolve pipe descriptor
    core_platform::ensure_out(out, "out")?;
    let descriptor = pipe_descriptor(binding, handle, PIPE_WRITE_OPERATION)?;

    // decode caller buffer and issue one write call
    let bytes = unsafe { buffer.as_slice()? };
    let rc = unsafe {
        libc::write(
            descriptor,
            bytes.as_ptr().cast::<libc::c_void>(),
            bytes.len(),
        )
    };
    if rc < 0 {
        return Err(io_error(
            PIPE_WRITE_OPERATION,
            "write",
            "failed to write pipe bytes",
        ));
    }

    // write written-byte count
    unsafe {
        out.write(rc as u64);
    }

    Ok(())
}
