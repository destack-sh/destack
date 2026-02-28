use windows_sys::Win32::Foundation::{WAIT_ABANDONED, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows_sys::Win32::System::Threading::{
    CreateSemaphoreW, ReleaseSemaphore, WaitForSingleObject,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::{NativeStringRef, resource};
use crate::runtime::BindingCallContext;

use super::core::{
    ensure_out, ensure_zero_flags, invalid_argument, io_error, not_supported,
    register_semaphore_handle, semaphore_handle, timed_out, timeout_to_wait_milliseconds,
    wide_name,
};

/// Increment one semaphore count.
const SEMAPHORE_POST_OPERATION: &str = "destack.ipc.sync.semaphorePost";
/// Wait one semaphore count.
const SEMAPHORE_WAIT_OPERATION: &str = "destack.ipc.sync.semaphoreWait";
/// Wait one futex word.
const FUTEX_WAIT_OPERATION: &str = "destack.ipc.sync.futexWait";
/// Wake futex waiters.
const FUTEX_WAKE_OPERATION: &str = "destack.ipc.sync.futexWake";

/// Wait on one shared-memory futex word.
pub(crate) unsafe fn destack_ipc_futex_wait(
    _context: &BindingCallContext,
    sharedmemory: resource::SharedMemoryHandle,
    offset: u64,
    expected: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (sharedmemory, offset, expected, timeoutns);

    Err(not_supported(FUTEX_WAIT_OPERATION))
}

/// Wake futex waiters for one shared-memory word.
pub(crate) unsafe fn destack_ipc_futex_wake(
    _context: &BindingCallContext,
    out: *mut u32,
    sharedmemory: resource::SharedMemoryHandle,
    offset: u64,
    count: u32,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    let _ = (sharedmemory, offset, count);

    Err(not_supported(FUTEX_WAKE_OPERATION))
}

/// Create one named semaphore.
pub(crate) unsafe fn destack_ipc_semaphore_create(
    context: &BindingCallContext,
    out: *mut resource::SemaphoreHandle,
    name: NativeStringRef,
    initial: u32,
    flags: u32,
) -> RuntimeResult<()> {
    // validate output and input flags
    ensure_out(out, "out")?;
    ensure_zero_flags(flags, "flags")?;

    // decode one UTF-16 semaphore name
    let name = wide_name(name, "name")?;

    // decode semaphore counts into windows host ranges
    let initial_count = i32::try_from(initial)
        .map_err(|_| invalid_argument("initial", "initial count exceeds windows i32 range"))?;
    let maximum_count = i32::MAX;

    // create one named semaphore object
    let semaphore = unsafe {
        CreateSemaphoreW(
            std::ptr::null_mut(),
            initial_count,
            maximum_count,
            name.as_ptr(),
        )
    };
    if semaphore == 0 {
        return Err(io_error("CreateSemaphoreW"));
    }

    // register semaphore handle and write output
    let handle = register_semaphore_handle(context, semaphore);
    unsafe {
        out.write(handle);
    }

    Ok(())
}

/// Increment one semaphore count.
pub(crate) unsafe fn destack_ipc_semaphore_post(
    context: &BindingCallContext,
    handle: resource::SemaphoreHandle,
    count: u32,
) -> RuntimeResult<()> {
    // resolve one semaphore handle and decode count range
    let semaphore = semaphore_handle(context, handle, SEMAPHORE_POST_OPERATION)?;
    let release_count = i32::try_from(count)
        .map_err(|_| invalid_argument("count", "count exceeds windows i32 range"))?;

    // release one or more semaphore permits
    let status = unsafe { ReleaseSemaphore(semaphore, release_count, std::ptr::null_mut()) };
    if status == 0 {
        return Err(io_error("ReleaseSemaphore"));
    }

    Ok(())
}

/// Wait one semaphore count.
pub(crate) unsafe fn destack_ipc_semaphore_wait(
    context: &BindingCallContext,
    handle: resource::SemaphoreHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // resolve one semaphore handle and decode timeout
    let semaphore = semaphore_handle(context, handle, SEMAPHORE_WAIT_OPERATION)?;
    let timeout_milliseconds = timeout_to_wait_milliseconds(timeoutns);

    // wait on one semaphore permit
    let status = unsafe { WaitForSingleObject(semaphore, timeout_milliseconds) };
    if status == WAIT_OBJECT_0 {
        return Ok(());
    }
    if status == WAIT_TIMEOUT {
        return Err(timed_out(
            SEMAPHORE_WAIT_OPERATION,
            "failed to wait semaphore: timed out",
        ));
    }
    if status == WAIT_ABANDONED {
        return Err(io_error("WaitForSingleObject"));
    }
    if status == WAIT_FAILED {
        return Err(io_error("WaitForSingleObject"));
    }

    Err(io_error("WaitForSingleObject"))
}
