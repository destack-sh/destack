#![allow(dead_code)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::NativeStringRef;
use crate::platform::resource::{
    BarrierHandle, CondVarHandle, MutexHandle, RwLockHandle, ThreadHandle, ThreadLocalKey,
    ThreadSemaphoreHandle,
};
use crate::platform::thread::{ThreadOptionsVm, host as host_thread};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Call one native binding with one output pointer and return the produced value.
fn call_out<T>(call: impl FnOnce(*mut T) -> RuntimeResult<()>) -> RuntimeResult<T> {
    let mut out = std::mem::MaybeUninit::<T>::uninit();
    call(out.as_mut_ptr())?;
    Ok(unsafe { out.assume_init() })
}

/// Convert one VM string handle into one call-context native string reference.
fn string_ref_from_vm(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: vm::StringHandle,
) -> RuntimeResult<NativeStringRef> {
    let value = context
        .string_ref(value)
        .map_err(|error| RuntimeError::from(error).boxed())?;

    Ok(runtime.store_string(value.as_str()))
}

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
pub(crate) fn destack_thread_local_create(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<ThreadLocalKey> {
    call_out(|out| unsafe { host_thread::destack_thread_local_create(runtime, out) })
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
pub(crate) fn destack_thread_local_delete(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    key: ThreadLocalKey,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_local_delete(runtime, key) }
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
pub(crate) fn destack_thread_local_get(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    key: ThreadLocalKey,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { host_thread::destack_thread_local_get(runtime, out, key) })
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
pub(crate) fn destack_thread_local_set(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    key: ThreadLocalKey,
    argument_value: u64,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_local_set(runtime, key, argument_value) }
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
pub(crate) fn destack_thread_get_affinity(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: ThreadHandle,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { host_thread::destack_thread_get_affinity(runtime, out, handle) })
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
pub(crate) fn destack_thread_get_priority(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: ThreadHandle,
) -> RuntimeResult<i32> {
    call_out(|out| unsafe { host_thread::destack_thread_get_priority(runtime, out, handle) })
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
pub(crate) fn destack_thread_set_affinity(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: ThreadHandle,
    mask: u64,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_set_affinity(runtime, handle, mask) }
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
pub(crate) fn destack_thread_set_priority(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: ThreadHandle,
    priority: i32,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_set_priority(runtime, handle, priority) }
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
pub(crate) fn destack_thread_detach(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: ThreadHandle,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_detach(runtime, handle) }
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
pub(crate) fn destack_thread_join(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: ThreadHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_thread::destack_thread_join(runtime, out, handle) })
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
pub(crate) fn destack_thread_spawn(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    entry: vm::StringHandle,
    argument: u64,
    options: ThreadOptionsVm,
) -> RuntimeResult<ThreadHandle> {
    let entry = string_ref_from_vm(runtime, context, entry)?;
    call_out(|out| unsafe {
        host_thread::destack_thread_spawn(runtime, out, entry, argument, options)
    })
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
pub(crate) fn destack_thread_address_wait(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    expected: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_address_wait(runtime, address, expected, timeoutns) }
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
pub(crate) fn destack_thread_address_wake_all(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_address_wake_all(runtime, address) }
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
pub(crate) fn destack_thread_address_wake_one(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_address_wake_one(runtime, address) }
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
pub(crate) fn destack_thread_barrier_create(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    participants: u32,
    flags: u32,
) -> RuntimeResult<BarrierHandle> {
    call_out(|out| unsafe {
        host_thread::destack_thread_barrier_create(runtime, out, participants, flags)
    })
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
pub(crate) fn destack_thread_barrier_wait(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: BarrierHandle,
    timeoutns: u64,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe {
        host_thread::destack_thread_barrier_wait(runtime, out, handle, timeoutns)
    })
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
pub(crate) fn destack_thread_cond_var_create(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    flags: u32,
) -> RuntimeResult<CondVarHandle> {
    call_out(|out| unsafe { host_thread::destack_thread_cond_var_create(runtime, out, flags) })
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
pub(crate) fn destack_thread_cond_var_notify_all(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    condvar: CondVarHandle,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_cond_var_notify_all(runtime, condvar) }
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
pub(crate) fn destack_thread_cond_var_notify_one(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    condvar: CondVarHandle,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_cond_var_notify_one(runtime, condvar) }
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
pub(crate) fn destack_thread_cond_var_wait(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    condvar: CondVarHandle,
    mutex: MutexHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_cond_var_wait(runtime, condvar, mutex, timeoutns) }
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
pub(crate) fn destack_thread_mutex_create(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    flags: u32,
) -> RuntimeResult<MutexHandle> {
    call_out(|out| unsafe { host_thread::destack_thread_mutex_create(runtime, out, flags) })
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
pub(crate) fn destack_thread_mutex_lock(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: MutexHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_mutex_lock(runtime, handle, timeoutns) }
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
pub(crate) fn destack_thread_mutex_unlock(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: MutexHandle,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_mutex_unlock(runtime, handle) }
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
pub(crate) fn destack_thread_rwlock_create(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    flags: u32,
) -> RuntimeResult<RwLockHandle> {
    call_out(|out| unsafe { host_thread::destack_thread_rwlock_create(runtime, out, flags) })
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
pub(crate) fn destack_thread_rwlock_read_lock(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: RwLockHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_rwlock_read_lock(runtime, handle, timeoutns) }
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
pub(crate) fn destack_thread_rwlock_unlock(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: RwLockHandle,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_rwlock_unlock(runtime, handle) }
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
pub(crate) fn destack_thread_rwlock_write_lock(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: RwLockHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_rwlock_write_lock(runtime, handle, timeoutns) }
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
pub(crate) fn destack_thread_semaphore_create(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    initial: u32,
    maximum: u32,
    flags: u32,
) -> RuntimeResult<ThreadSemaphoreHandle> {
    call_out(|out| unsafe {
        host_thread::destack_thread_semaphore_create(runtime, out, initial, maximum, flags)
    })
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
pub(crate) fn destack_thread_semaphore_post(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: ThreadSemaphoreHandle,
    count: u32,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_semaphore_post(runtime, handle, count) }
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
pub(crate) fn destack_thread_semaphore_wait(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: ThreadSemaphoreHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_semaphore_wait(runtime, handle, timeoutns) }
}
