use windows_sys::Win32::System::Threading::{
    CreateSemaphoreW, ReleaseSemaphore, WaitForSingleObject,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::{core as core_platform, resource};
use crate::runtime::{BindingCallContext, NativeStringRef};

use super::core::{
    register_semaphore_handle, semaphore_handle, timed_out, timeout_to_wait_milliseconds, wide_name,
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
///
/// Wait while one futex word matches the expected value.
/// Offset is byte-based within the mapped shared memory object.
///
/// # Platform
/// Unix and Windows.
/// Uses futex wait on Linux and WaitOnAddress-style primitives on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioTimedOut, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.futex`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_ipc_futex_wait(
    binding: &BindingCallContext,
    sharedmemory: resource::SharedMemoryHandle,
    offset: u64,
    expected: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (sharedmemory, offset, expected, timeoutns);

    Err(core_platform::not_supported(FUTEX_WAIT_OPERATION))
}

/// Wake futex waiters for one shared-memory word.
///
/// Wake up to count waiters blocked on one futex word.
/// Wake ordering follows host scheduler semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses futex wake on Linux and WakeByAddress-style primitives on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.futex`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_ipc_futex_wake(
    binding: &BindingCallContext,
    out: *mut u32,
    sharedmemory: resource::SharedMemoryHandle,
    offset: u64,
    count: u32,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;
    let _ = (sharedmemory, offset, count);

    Err(core_platform::not_supported(FUTEX_WAKE_OPERATION))
}

/// Create one named semaphore.
///
/// Create one named interprocess semaphore with an initial count.
/// Name visibility and ownership follow host semaphore namespace rules.
///
/// # Platform
/// Unix and Windows.
/// Uses POSIX semaphores on Unix and named semaphore objects on Windows.
///
/// # Errors
/// Returns invalidArgument, ioAlreadyExists, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.semaphore`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_ipc_semaphore_create(
    binding: &BindingCallContext,
    out: *mut resource::SemaphoreHandle,
    name: NativeStringRef,
    initial: u32,
    flags: u32,
) -> RuntimeResult<()> {
    // validate output and input flags
    core_platform::ensure_out(out, "out")?;
    core_platform::ensure_zero_flags(flags, "flags")?;

    // decode one UTF-16 semaphore name
    let name = wide_name(name, "name")?;

    // decode semaphore counts into windows host ranges
    let initial_count = i32::try_from(initial).map_err(|_| {
        core_platform::invalid_argument("initial", "initial count exceeds windows i32 range")
    })?;
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
        return Err(core_platform::io_error("CreateSemaphoreW"));
    }

    // register semaphore handle and write output
    let handle = register_semaphore_handle(binding, semaphore);
    unsafe {
        out.write(handle);
    }

    Ok(())
}

/// Increment one semaphore count.
///
/// Release one waiting semaphore acquisition by incrementing the count.
/// Wakeup ordering follows host semaphore scheduling behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses sem_post on Unix and ReleaseSemaphore on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.semaphore`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_ipc_semaphore_post(
    binding: &BindingCallContext,
    handle: resource::SemaphoreHandle,
    count: u32,
) -> RuntimeResult<()> {
    // resolve one semaphore handle and decode count range
    let semaphore = semaphore_handle(binding, handle, SEMAPHORE_POST_OPERATION)?;
    let release_count = i32::try_from(count)
        .map_err(|_| core_platform::invalid_argument("count", "count exceeds windows i32 range"))?;

    // release one or more semaphore permits
    let status = unsafe { ReleaseSemaphore(semaphore, release_count, std::ptr::null_mut()) };
    if status == 0 {
        return Err(core_platform::io_error("ReleaseSemaphore"));
    }

    Ok(())
}

/// Wait one semaphore count.
///
/// Decrement one semaphore count, waiting up to the provided timeout.
/// Timeout units are nanoseconds and follow host wait semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses sem_timedwait or sem_wait on Unix and WaitForSingleObject on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioTimedOut, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.semaphore`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_ipc_semaphore_wait(
    binding: &BindingCallContext,
    handle: resource::SemaphoreHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // resolve one semaphore handle and decode timeout
    let semaphore = semaphore_handle(binding, handle, SEMAPHORE_WAIT_OPERATION)?;
    let timeout_milliseconds = timeout_to_wait_milliseconds(timeoutns);

    // wait on one semaphore permit
    let status = unsafe { WaitForSingleObject(semaphore, timeout_milliseconds) };
    match core_platform::decode_wait_for_single_object_status(status, "WaitForSingleObject")? {
        core_platform::WaitStatus::Signaled => Ok(()),
        core_platform::WaitStatus::TimedOut => Err(timed_out(
            SEMAPHORE_WAIT_OPERATION,
            "failed to wait semaphore: timed out",
        )),
        core_platform::WaitStatus::Abandoned => Err(core_platform::io_error("WaitForSingleObject")),
    }
}
