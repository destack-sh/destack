#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::thread::bindings_generated as bindings;
use crate::platform::{NativeStringRef, PlatformError};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::resource;
use crate::platform::thread::ThreadOptions;

/// Create one thread-local key.
///
/// Allocate one runtime thread-local storage key.
/// Key lifetime is explicit and must be released with delete.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread TLS keys on Unix and TlsAlloc on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.local`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_local_create(
    context: &RuntimeCallContext,
    out: *mut resource::ThreadLocalKey,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    Err(RuntimeError::from(PlatformError::not_supported("destack.thread.local.create")).boxed())
}

/// Delete one thread-local key.
///
/// Release one thread-local key and associated host resources.
/// Existing per-thread values become invalid after deletion.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread TLS key deletion on Unix and TlsFree on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.local`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_local_delete(
    context: &RuntimeCallContext,
    _key: resource::ThreadLocalKey,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.thread.local.delete")).boxed())
}

/// Read one thread-local value.
///
/// Read one machine-word value from one thread-local key.
/// Value interpretation is caller-defined and ABI-dependent.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread TLS storage on Unix and TlsGetValue on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.local`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_local_get(
    context: &RuntimeCallContext,
    out: *mut u64,
    key: resource::ThreadLocalKey,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, key);

    Err(RuntimeError::from(PlatformError::not_supported("destack.thread.local.get")).boxed())
}

/// Store one thread-local value.
///
/// Write one machine-word value into one thread-local key.
/// Value interpretation is caller-defined and ABI-dependent.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread TLS storage on Unix and TlsSetValue on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.local`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_local_set(
    context: &RuntimeCallContext,
    key: resource::ThreadLocalKey,
    argument_value: u64,
) -> RuntimeResult<()> {
    let _ = (key, argument_value);

    Err(RuntimeError::from(PlatformError::not_supported("destack.thread.local.set")).boxed())
}

/// Read thread affinity mask.
///
/// Read one thread CPU affinity mask.
/// Affinity mask width and normalization are host-architecture dependent.
///
/// # Platform
/// Unix and Windows.
/// Uses sched affinity APIs on Unix and GetThreadGroupAffinity on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `thread.priority`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_get_affinity(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: resource::ThreadHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.priority.getAffinity",
    ))
    .boxed())
}

/// Read thread priority.
///
/// Read one thread priority value from host scheduler state.
/// Priority value normalization is runtime-defined per host.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread scheduling APIs on Unix and GetThreadPriority on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `thread.priority`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_get_priority(
    context: &RuntimeCallContext,
    out: *mut i32,
    handle: resource::ThreadHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.priority.getPriority",
    ))
    .boxed())
}

/// Set thread affinity mask.
///
/// Bind one thread to a CPU affinity mask.
/// Affinity mask semantics are host scheduler-defined.
///
/// # Platform
/// Unix and Windows.
/// Uses sched affinity APIs on Unix and SetThreadAffinityMask on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `thread.priority`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_set_affinity(
    context: &RuntimeCallContext,
    handle: resource::ThreadHandle,
    mask: u64,
) -> RuntimeResult<()> {
    let _ = (handle, mask);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.priority.setAffinity",
    ))
    .boxed())
}

/// Set thread priority.
///
/// Set one thread priority value using host scheduler controls.
/// Priority range and interpretation are host-specific.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread scheduling APIs on Unix and SetThreadPriority on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `thread.priority`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_set_priority(
    context: &RuntimeCallContext,
    handle: resource::ThreadHandle,
    priority: i32,
) -> RuntimeResult<()> {
    let _ = (handle, priority);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.priority.setPriority",
    ))
    .boxed())
}

/// Detach one host thread.
///
/// Detach one thread from join tracking.
/// Detached thread lifecycle and cleanup are host-managed.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread_detach on Unix and handle-release semantics on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.spawn`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_detach(
    context: &RuntimeCallContext,
    _handle: resource::ThreadHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.thread.spawn.detach")).boxed())
}

/// Join one host thread.
///
/// Wait for one joinable thread to exit and return its exit code.
/// Join behavior follows host thread lifecycle rules.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread_join on Unix and WaitForSingleObject plus exit code on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.spawn`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_join(
    context: &RuntimeCallContext,
    out: *mut u32,
    handle: resource::ThreadHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.thread.spawn.join")).boxed())
}

/// Spawn one host thread.
///
/// Spawn one host thread that enters a runtime-provided entry symbol.
/// Entry dispatch and argument passing are runtime ABI contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread_create on Unix and CreateThread on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.spawn`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_spawn(
    context: &RuntimeCallContext,
    out: *mut resource::ThreadHandle,
    entry: NativeStringRef,
    argument: u64,
    options: ThreadOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, entry, argument, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.thread.spawn.start")).boxed())
}

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
/// Unix and Windows.
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
/// Unix and Windows.
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
/// Unix and Windows.
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
/// Unix and Windows.
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
/// Unix and Windows.
/// Uses pthread mutexes on Unix and SRW or critical section primitives on Windows.
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
/// Unix and Windows.
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
/// Unix and Windows.
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
/// Unix and Windows.
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
/// Unix and Windows.
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
/// Unix and Windows.
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
/// Unix and Windows.
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
/// Unix and Windows.
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
/// Unix and Windows.
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
/// Unix and Windows.
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
