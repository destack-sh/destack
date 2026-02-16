#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::thread::{bindings_generated as bindings, core as core_thread};
use crate::platform::{NativeStringRef, PlatformError};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::resource;
use crate::platform::thread::ThreadOptions;
/// Wait on one memory address value.
///
/// Wait while the target memory word matches the expected value.
/// Address wait semantics follow host futex or WaitOnAddress primitives.
///
/// # Platform
/// Unix and Windows.
/// Uses futex wait on Linux and WaitOnAddress on Windows.
///
/// # Errors
/// Returns invalidArgument, ioTimedOut, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.wait`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_address_wait(
    context: &RuntimeCallContext,
    address: u64,
    expected: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (address, expected, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.addressWait",
    ))
    .boxed())
}

/// Wake all waiters on a memory address.
///
/// Wake all waiters blocked on the target memory address.
/// Wake ordering follows host wait-address primitive behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses futex wake on Linux and WakeByAddressAll on Windows.
///
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.wait`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_address_wake_all(
    context: &RuntimeCallContext,
    _address: u64,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.addressWakeAll",
    ))
    .boxed())
}

/// Wake one waiter on a memory address.
///
/// Wake one waiter blocked on the target memory address.
/// Wake ordering follows host wait-address primitive behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses futex wake on Linux and WakeByAddressSingle on Windows.
///
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.wait`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_address_wake_one(
    context: &RuntimeCallContext,
    _address: u64,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.addressWakeOne",
    ))
    .boxed())
}

/// Create one thread barrier.
///
/// Create one reusable barrier for a fixed participant count.
/// Participant-count semantics follow host barrier primitives.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread barriers on Unix and runtime-host barrier emulation on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.sync`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_barrier_create(
    context: &RuntimeCallContext,
    out: *mut resource::BarrierHandle,
    participants: u32,
    flags: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, participants, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.barrierCreate",
    ))
    .boxed())
}

/// Wait for barrier rendezvous.
///
/// Block until all participants reach one barrier phase.
/// Return value marks whether the caller became phase leader.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread barriers on Unix and runtime-host barrier emulation on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioTimedOut, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.sync`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_barrier_wait(
    context: &RuntimeCallContext,
    out: *mut bool,
    handle: resource::BarrierHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.barrierWait",
    ))
    .boxed())
}

/// Create one condition variable.
///
/// Create one condition variable for wait-notify synchronization.
/// Condition variable association with mutexes is validated on wait calls.
///
/// # Platform
/// Unix, Windows, and Wasi.
/// Uses pthread condition variables on Unix and condition variable APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.sync`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_cond_var_create(
    context: &RuntimeCallContext,
    out: *mut resource::CondVarHandle,
    flags: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.condVarCreate",
    ))
    .boxed())
}

/// Notify all condition-variable waiters.
///
/// Wake all waiters blocked on a condition variable.
/// Wake ordering and runnable scheduling follow host synchronization semantics.
///
/// # Platform
/// Unix, Windows, and Wasi.
/// Uses host condition-variable broadcast primitives.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.sync`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_cond_var_notify_all(
    context: &RuntimeCallContext,
    _condvar: resource::CondVarHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.condVarNotifyAll",
    ))
    .boxed())
}

/// Notify one condition-variable waiter.
///
/// Wake one waiter blocked on a condition variable.
/// Waiter selection order follows host synchronization semantics.
///
/// # Platform
/// Unix, Windows, and Wasi.
/// Uses host condition-variable notify primitives.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.sync`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_cond_var_notify_one(
    context: &RuntimeCallContext,
    _condvar: resource::CondVarHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.condVarNotifyOne",
    ))
    .boxed())
}

/// Wait on one condition variable.
///
/// Atomically release one mutex and wait for one condition-variable notification.
/// Mutex is reacquired before returning from wait according to host semantics.
///
/// # Platform
/// Unix, Windows, and Wasi.
/// Uses host condition-variable wait primitives.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioTimedOut, notSupported.
///
/// # Security
/// Requires `thread.sync`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_cond_var_wait(
    context: &RuntimeCallContext,
    condvar: resource::CondVarHandle,
    mutex: resource::MutexHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (condvar, mutex, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.condVarWait",
    ))
    .boxed())
}

/// Create one mutex.
///
/// Create one host mutex with runtime-selected attributes.
/// Mutex ownership and recursion behavior follow host primitive configuration.
///
/// # Platform
/// Unix, Windows, and Wasi.
/// Uses pthread mutexes on Unix, SRW or critical section primitives on Windows, and wasi mutex support where available.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.sync`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_mutex_create(
    context: &RuntimeCallContext,
    out: *mut resource::MutexHandle,
    flags: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.mutexCreate",
    ))
    .boxed())
}

/// Lock one mutex.
///
/// Acquire one mutex, waiting until ownership is available.
/// Wait ordering and fairness follow host synchronization semantics.
///
/// # Platform
/// Unix, Windows, and Wasi.
/// Uses host mutex wait primitives.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioTimedOut, notSupported.
///
/// # Security
/// Requires `thread.sync`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_mutex_lock(
    context: &RuntimeCallContext,
    handle: resource::MutexHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.mutexLock",
    ))
    .boxed())
}

/// Unlock one mutex.
///
/// Release ownership of one mutex.
/// Wakeup behavior for waiters follows host synchronization semantics.
///
/// # Platform
/// Unix, Windows, and Wasi.
/// Uses host mutex unlock primitives.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `thread.sync`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_mutex_unlock(
    context: &RuntimeCallContext,
    _handle: resource::MutexHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.mutexUnlock",
    ))
    .boxed())
}

/// Create one read-write lock.
///
/// Create one read-write lock for shared and exclusive access control.
/// Reader and writer preference is host-primitive defined.
///
/// # Platform
/// Unix, Windows, and Wasi.
/// Uses pthread rwlock on Unix and SRW lock abstractions on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.sync`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_rwlock_create(
    context: &RuntimeCallContext,
    out: *mut resource::RwLockHandle,
    flags: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.rwlockCreate",
    ))
    .boxed())
}

/// Lock one read-write lock for read access.
///
/// Acquire shared read access for one read-write lock.
/// Read acquisition ordering follows host synchronization semantics.
///
/// # Platform
/// Unix, Windows, and Wasi.
/// Uses host rwlock read-lock primitives.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioTimedOut, notSupported.
///
/// # Security
/// Requires `thread.sync`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_rwlock_read_lock(
    context: &RuntimeCallContext,
    handle: resource::RwLockHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.rwlockReadLock",
    ))
    .boxed())
}

/// Unlock one read-write lock.
///
/// Release one read or write ownership slot on a read-write lock.
/// Wakeup behavior for waiters follows host synchronization semantics.
///
/// # Platform
/// Unix, Windows, and Wasi.
/// Uses host rwlock unlock primitives.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `thread.sync`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_rwlock_unlock(
    context: &RuntimeCallContext,
    _handle: resource::RwLockHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.rwlockUnlock",
    ))
    .boxed())
}

/// Lock one read-write lock for write access.
///
/// Acquire exclusive write access for one read-write lock.
/// Write acquisition ordering follows host synchronization semantics.
///
/// # Platform
/// Unix, Windows, and Wasi.
/// Uses host rwlock write-lock primitives.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioTimedOut, notSupported.
///
/// # Security
/// Requires `thread.sync`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_rwlock_write_lock(
    context: &RuntimeCallContext,
    handle: resource::RwLockHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.rwlockWriteLock",
    ))
    .boxed())
}

/// Create one thread-scoped semaphore.
///
/// Create one semaphore for in-process thread synchronization.
/// Semaphore bounds and fairness follow host primitive semantics.
///
/// # Platform
/// Unix, Windows, and Wasi.
/// Uses semaphores or equivalent host synchronization primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.sync`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_semaphore_create(
    context: &RuntimeCallContext,
    out: *mut resource::ThreadSemaphoreHandle,
    initial: u32,
    maximum: u32,
    flags: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, initial, maximum, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.semaphoreCreate",
    ))
    .boxed())
}

/// Post one semaphore count for thread synchronization.
///
/// Increment one semaphore by count and wake eligible waiters.
/// Wake behavior follows host semaphore primitives.
///
/// # Platform
/// Unix, Windows, and Wasi.
/// Uses host semaphore post primitives.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.sync`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_semaphore_post(
    context: &RuntimeCallContext,
    handle: resource::ThreadSemaphoreHandle,
    count: u32,
) -> RuntimeResult<()> {
    let _ = (handle, count);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.semaphorePost",
    ))
    .boxed())
}

/// Wait one semaphore count for thread synchronization.
///
/// Decrement one semaphore count, waiting up to the timeout when needed.
/// Wake ordering follows host scheduler behavior.
///
/// # Platform
/// Unix, Windows, and Wasi.
/// Uses host semaphore wait primitives.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioTimedOut, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.sync`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_semaphore_wait(
    context: &RuntimeCallContext,
    handle: resource::ThreadSemaphoreHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.semaphoreWait",
    ))
    .boxed())
}
