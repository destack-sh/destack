#![allow(clippy::missing_safety_doc)]

use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::ptr;
use std::time::Duration;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::{
    BarrierHandle, CondVarHandle, MutexHandle, ResourceKind, RwLockHandle, ThreadSemaphoreHandle,
};
use crate::platform::thread::{core as core_thread, resource as resource_thread};
use crate::platform::{PlatformError, core as core_platform};
use windows_sys::Win32::Foundation::{ERROR_TIMEOUT, ERROR_TOO_MANY_POSTS, GetLastError};
use windows_sys::Win32::System::Threading::{
    AcquireSRWLockExclusive, AcquireSRWLockShared, CreateSemaphoreW, EnterCriticalSection,
    EnterSynchronizationBarrier, INFINITE, InitializeConditionVariable, InitializeCriticalSection,
    InitializeSRWLock, InitializeSynchronizationBarrier, LeaveCriticalSection,
    ReleaseSRWLockExclusive, ReleaseSRWLockShared, ReleaseSemaphore, SleepConditionVariableCS,
    TryAcquireSRWLockExclusive, TryAcquireSRWLockShared, TryEnterCriticalSection,
    WaitForSingleObject, WaitOnAddress, WakeAllConditionVariable, WakeByAddressAll,
    WakeByAddressSingle, WakeConditionVariable,
};

use crate::runtime::BindingCallContext;

/// Convert one optional timeout duration into a Win32 millisecond timeout.
fn timeout_to_wait_milliseconds(timeout: Option<Duration>) -> RuntimeResult<u32> {
    let Some(timeout) = timeout else {
        return Ok(INFINITE);
    };

    let milliseconds = timeout.as_millis();
    if milliseconds > u32::MAX as u128 {
        return Ok(u32::MAX - 1);
    }

    Ok(milliseconds as u32)
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
    _binding: &BindingCallContext,
    address: u64,
    expected: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // wait on the host address until wake, mismatch, or timeout
    let address_ptr = core_thread::checked_u32_word_pointer(address, "address")?;
    let timeout = core_thread::timeout_from_ns(timeoutns);
    let timeout_milliseconds = timeout_to_wait_milliseconds(timeout)?;
    let rc = unsafe {
        WaitOnAddress(
            address_ptr as *const std::ffi::c_void,
            &expected as *const u32 as *const std::ffi::c_void,
            std::mem::size_of::<u32>(),
            timeout_milliseconds,
        )
    };
    if rc != 0 {
        return Ok(());
    }

    // map wait completion semantics to binding results
    let code = unsafe { GetLastError() } as i32;
    if code as u32 == ERROR_TIMEOUT {
        if timeoutns == 0 {
            return Err(core_thread::io_would_block_error(
                "addressWait",
                "failed to wait on address: no wake observed",
            ));
        }

        return Err(core_thread::io_timed_out_error(
            "addressWait",
            "failed to wait on address: timed out waiting for wake",
        ));
    }

    Err(core_platform::io_error_with_code("WaitOnAddress", code))
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
    _binding: &BindingCallContext,
    address: u64,
) -> RuntimeResult<()> {
    // wake all blocked waiters
    let address_ptr = core_thread::checked_u32_word_pointer(address, "address")?;
    unsafe {
        WakeByAddressAll(address_ptr as *const std::ffi::c_void);
    }

    Ok(())
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
    _binding: &BindingCallContext,
    address: u64,
) -> RuntimeResult<()> {
    // wake one blocked waiter
    let address_ptr = core_thread::checked_u32_word_pointer(address, "address")?;
    unsafe {
        WakeByAddressSingle(address_ptr as *const std::ffi::c_void);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut BarrierHandle,
    participants: u32,
    flags: u32,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate supported barrier flags
    if flags != 0 {
        return Err(core_thread::unsupported_flags_error("flags", flags));
    }

    // validate participant count
    if participants == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "participants",
            "participants must be greater than zero",
        ))
        .boxed());
    }

    // initialize one synchronization barrier
    let mut barrier =
        MaybeUninit::<windows_sys::Win32::System::Threading::SYNCHRONIZATION_BARRIER>::zeroed();
    let rc =
        unsafe { InitializeSynchronizationBarrier(barrier.as_mut_ptr(), participants as i32, -1) };
    if rc == 0 {
        return Err(core_platform::io_error("InitializeSynchronizationBarrier"));
    }

    // store one barrier resource
    let resource_id = core_thread::insert_thread_resource(
        binding,
        ResourceKind::Barrier,
        "thread.barrier",
        resource_thread::BarrierResource {
            barrier: UnsafeCell::new(unsafe { barrier.assume_init() }),
            participants,
        },
    );
    let handle = BarrierHandle(resource_id);

    // write the output handle
    unsafe {
        *out = handle;
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut bool,
    handle: BarrierHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the barrier resource
    let barrier = core_thread::resolve_thread_resource::<resource_thread::BarrierResource>(
        binding,
        handle.0,
        "handle",
        "barrier handle",
    )?;

    // reject unsupported finite barrier waits
    if timeoutns != core_thread::WAIT_FOREVER {
        if timeoutns == 0 && barrier.participants > 1 {
            return Err(core_thread::io_would_block_error(
                "barrierWait",
                "failed to wait barrier: timeout is zero and participants are still pending",
            ));
        }

        if !(timeoutns == 0 && barrier.participants == 1) {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.thread.sync.barrierWait",
            ))
            .boxed());
        }
    }

    // wait for one barrier rendezvous
    let leader = unsafe { EnterSynchronizationBarrier(barrier.barrier.get(), 0) != 0 };

    // write the output leader marker
    unsafe {
        *out = leader;
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut CondVarHandle,
    flags: u32,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate supported condition-variable flags
    if flags != 0 {
        return Err(core_thread::unsupported_flags_error("flags", flags));
    }

    // initialize one condition variable
    let mut condvar =
        MaybeUninit::<windows_sys::Win32::System::Threading::CONDITION_VARIABLE>::zeroed();
    unsafe {
        InitializeConditionVariable(condvar.as_mut_ptr());
    }

    // store one condition-variable resource
    let resource_id = core_thread::insert_thread_resource(
        binding,
        ResourceKind::CondVar,
        "thread.condvar",
        resource_thread::CondVarResource {
            condvar: UnsafeCell::new(unsafe { condvar.assume_init() }),
        },
    );
    let handle = CondVarHandle(resource_id);

    // write the output handle
    unsafe {
        *out = handle;
    }

    Ok(())
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
    binding: &BindingCallContext,
    condvar: CondVarHandle,
) -> RuntimeResult<()> {
    // resolve the condition-variable resource
    let condvar = core_thread::resolve_thread_resource::<resource_thread::CondVarResource>(
        binding,
        condvar.0,
        "condvar",
        "condition variable handle",
    )?;

    // notify all blocked waiters
    unsafe {
        WakeAllConditionVariable(condvar.condvar.get());
    }

    Ok(())
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
    binding: &BindingCallContext,
    condvar: CondVarHandle,
) -> RuntimeResult<()> {
    // resolve the condition-variable resource
    let condvar = core_thread::resolve_thread_resource::<resource_thread::CondVarResource>(
        binding,
        condvar.0,
        "condvar",
        "condition variable handle",
    )?;

    // notify one blocked waiter
    unsafe {
        WakeConditionVariable(condvar.condvar.get());
    }

    Ok(())
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
    binding: &BindingCallContext,
    condvar: CondVarHandle,
    mutex: MutexHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // resolve synchronization resources
    let condvar = core_thread::resolve_thread_resource::<resource_thread::CondVarResource>(
        binding,
        condvar.0,
        "condvar",
        "condition variable handle",
    )?;
    let mutex = core_thread::resolve_thread_resource::<resource_thread::MutexResource>(
        binding,
        mutex.0,
        "mutex",
        "mutex handle",
    )?;

    // release the mutex and wait for one wake sequence
    let timeout = core_thread::timeout_from_ns(timeoutns);
    let timeout_milliseconds = timeout_to_wait_milliseconds(timeout)?;
    let rc = unsafe {
        SleepConditionVariableCS(
            condvar.condvar.get(),
            mutex.critical_section.get(),
            timeout_milliseconds,
        )
    };
    if rc != 0 {
        return Ok(());
    }

    // map wait completion semantics to binding results
    let code = unsafe { GetLastError() } as i32;
    if code as u32 == ERROR_TIMEOUT {
        if timeoutns == 0 {
            return Err(core_thread::io_would_block_error(
                "condVarWait",
                "failed to wait condition variable: no wake observed",
            ));
        }

        return Err(core_thread::io_timed_out_error(
            "condVarWait",
            "failed to wait condition variable: timed out waiting for wake",
        ));
    }

    Err(core_platform::io_error_with_code(
        "SleepConditionVariableCS",
        code,
    ))
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
    binding: &BindingCallContext,
    out: *mut MutexHandle,
    flags: u32,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate supported mutex flags
    if flags != 0 {
        return Err(core_thread::unsupported_flags_error("flags", flags));
    }

    // initialize one critical section
    let mut critical_section =
        MaybeUninit::<windows_sys::Win32::System::Threading::CRITICAL_SECTION>::uninit();
    unsafe {
        InitializeCriticalSection(critical_section.as_mut_ptr());
    }

    // store one mutex resource
    let resource_id = core_thread::insert_thread_resource(
        binding,
        ResourceKind::Mutex,
        "thread.mutex",
        resource_thread::MutexResource {
            critical_section: UnsafeCell::new(unsafe { critical_section.assume_init() }),
        },
    );
    let handle = MutexHandle(resource_id);

    // write the output handle
    unsafe {
        *out = handle;
    }

    Ok(())
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
    binding: &BindingCallContext,
    handle: MutexHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // resolve the mutex resource
    let mutex = core_thread::resolve_thread_resource::<resource_thread::MutexResource>(
        binding,
        handle.0,
        "handle",
        "mutex handle",
    )?;

    let timeout = core_thread::timeout_from_ns(timeoutns);
    if timeoutns == 0 {
        let rc = unsafe { TryEnterCriticalSection(mutex.critical_section.get()) };
        if rc != 0 {
            return Ok(());
        }

        return Err(core_thread::io_would_block_error(
            "mutexLock",
            "failed to lock mutex: mutex is currently held by another owner",
        ));
    }

    if timeout.is_some() {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.thread.sync.mutexLock",
        ))
        .boxed());
    }

    unsafe {
        EnterCriticalSection(mutex.critical_section.get());
    }

    Ok(())
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
    binding: &BindingCallContext,
    handle: MutexHandle,
) -> RuntimeResult<()> {
    // resolve the mutex resource
    let mutex = core_thread::resolve_thread_resource::<resource_thread::MutexResource>(
        binding,
        handle.0,
        "handle",
        "mutex handle",
    )?;

    // release one critical section lock depth
    unsafe {
        LeaveCriticalSection(mutex.critical_section.get());
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut RwLockHandle,
    flags: u32,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate supported read-write lock flags
    if flags != 0 {
        return Err(core_thread::unsupported_flags_error("flags", flags));
    }

    // initialize one srw lock
    let mut rwlock = MaybeUninit::<windows_sys::Win32::System::Threading::SRWLOCK>::zeroed();
    unsafe {
        InitializeSRWLock(rwlock.as_mut_ptr());
    }

    // store one read-write lock resource
    let resource_id = core_thread::insert_thread_resource(
        binding,
        ResourceKind::RwLock,
        "thread.rwlock",
        resource_thread::RwLockResource {
            rwlock: UnsafeCell::new(unsafe { rwlock.assume_init() }),
            ownership: parking_lot::Mutex::new(resource_thread::WindowsRwLockOwnership::default()),
        },
    );
    let handle = RwLockHandle(resource_id);

    // write the output handle
    unsafe {
        *out = handle;
    }

    Ok(())
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
    binding: &BindingCallContext,
    handle: RwLockHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // resolve the read-write lock resource
    let rwlock = core_thread::resolve_thread_resource::<resource_thread::RwLockResource>(
        binding,
        handle.0,
        "handle",
        "rwlock handle",
    )?;

    // reject lock modes that would deadlock this thread
    let owner = core_thread::current_thread_owner_id();
    {
        let ownership = rwlock.ownership.lock();
        if ownership.writer == Some(owner) {
            return Err(core_thread::thread_deadlock_error(
                "failed to lock rwlock for read: current thread already owns the write lock",
            ));
        }
    }

    // acquire one shared lock slot
    let timeout = core_thread::timeout_from_ns(timeoutns);
    if timeoutns == 0 {
        let rc = unsafe { TryAcquireSRWLockShared(rwlock.rwlock.get()) };
        if rc == 0 {
            return Err(core_thread::io_would_block_error(
                "rwlockReadLock",
                "failed to lock rwlock for read: lock is currently unavailable",
            ));
        }
    } else if timeout.is_some() {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.thread.sync.rwlockReadLock",
        ))
        .boxed());
    } else {
        unsafe {
            AcquireSRWLockShared(rwlock.rwlock.get());
        }
    }

    // record one shared ownership slot for this thread
    {
        let mut ownership = rwlock.ownership.lock();
        let readers = ownership.readers.entry(owner).or_insert(0);
        *readers = readers.saturating_add(1);
    }

    Ok(())
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
    binding: &BindingCallContext,
    handle: RwLockHandle,
) -> RuntimeResult<()> {
    // resolve the read-write lock resource
    let rwlock = core_thread::resolve_thread_resource::<resource_thread::RwLockResource>(
        binding,
        handle.0,
        "handle",
        "rwlock handle",
    )?;

    // resolve the lock ownership for this thread
    let owner = core_thread::current_thread_owner_id();
    let mut is_writer = false;
    {
        let mut ownership = rwlock.ownership.lock();
        if ownership.writer == Some(owner) {
            ownership.writer = None;
            is_writer = true;
        } else {
            let Some(read_count) = ownership.readers.get_mut(&owner) else {
                return Err(core_thread::io_permission_denied_error(
                    "rwlockUnlock",
                    "failed to unlock rwlock: current thread does not hold this lock",
                ));
            };

            *read_count = read_count.saturating_sub(1);
            if *read_count == 0 {
                ownership.readers.remove(&owner);
            }
        }
    }

    // release one write or read slot
    if is_writer {
        unsafe {
            ReleaseSRWLockExclusive(rwlock.rwlock.get());
        }
    } else {
        unsafe {
            ReleaseSRWLockShared(rwlock.rwlock.get());
        }
    }

    Ok(())
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
    binding: &BindingCallContext,
    handle: RwLockHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // resolve the read-write lock resource
    let rwlock = core_thread::resolve_thread_resource::<resource_thread::RwLockResource>(
        binding,
        handle.0,
        "handle",
        "rwlock handle",
    )?;

    // reject lock modes that would deadlock this thread
    let owner = core_thread::current_thread_owner_id();
    {
        let ownership = rwlock.ownership.lock();
        if ownership.writer == Some(owner) {
            return Err(core_thread::thread_deadlock_error(
                "failed to lock rwlock for write: current thread already owns the write lock",
            ));
        }

        if ownership.readers.contains_key(&owner) {
            return Err(core_thread::thread_deadlock_error(
                "failed to lock rwlock for write: current thread already owns one read lock",
            ));
        }
    }

    // acquire one exclusive lock slot
    let timeout = core_thread::timeout_from_ns(timeoutns);
    if timeoutns == 0 {
        let rc = unsafe { TryAcquireSRWLockExclusive(rwlock.rwlock.get()) };
        if rc == 0 {
            return Err(core_thread::io_would_block_error(
                "rwlockWriteLock",
                "failed to lock rwlock for write: lock is currently unavailable",
            ));
        }
    } else if timeout.is_some() {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.thread.sync.rwlockWriteLock",
        ))
        .boxed());
    } else {
        unsafe {
            AcquireSRWLockExclusive(rwlock.rwlock.get());
        }
    }

    // record one exclusive ownership slot
    {
        let mut ownership = rwlock.ownership.lock();
        ownership.writer = Some(owner);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut ThreadSemaphoreHandle,
    initial: u32,
    maximum: u32,
    flags: u32,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate supported semaphore flags
    if flags != 0 {
        return Err(core_thread::unsupported_flags_error("flags", flags));
    }

    // validate semaphore bounds
    if maximum == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "maximum",
            "maximum must be greater than zero",
        ))
        .boxed());
    }
    if initial > maximum {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "initial",
            "initial must be less than or equal to maximum",
        ))
        .boxed());
    }

    // initialize one host semaphore handle
    let semaphore = unsafe {
        CreateSemaphoreW(
            ptr::null::<windows_sys::Win32::Security::SECURITY_ATTRIBUTES>(),
            initial as i32,
            maximum as i32,
            ptr::null(),
        )
    };
    if semaphore == 0 {
        return Err(core_platform::io_error("CreateSemaphoreW"));
    }

    // store one semaphore resource
    let resource_id = core_thread::insert_thread_resource(
        binding,
        ResourceKind::ThreadSemaphore,
        "thread.semaphore",
        resource_thread::SemaphoreResource { semaphore },
    );
    let handle = ThreadSemaphoreHandle(resource_id);

    // write the output handle
    unsafe {
        *out = handle;
    }

    Ok(())
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
    binding: &BindingCallContext,
    handle: ThreadSemaphoreHandle,
    count: u32,
) -> RuntimeResult<()> {
    // resolve the semaphore resource
    let semaphore = core_thread::resolve_thread_resource::<resource_thread::SemaphoreResource>(
        binding,
        handle.0,
        "handle",
        "thread semaphore handle",
    )?;

    // release one or more permits
    if count == 0 {
        return Ok(());
    }

    let release_count = i32::try_from(count).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "count",
            "count exceeds host semaphore release range",
        ))
        .boxed()
    })?;

    let rc = unsafe { ReleaseSemaphore(semaphore.semaphore, release_count, ptr::null_mut()) };
    if rc != 0 {
        return Ok(());
    }

    let code = unsafe { GetLastError() };
    if code == ERROR_TOO_MANY_POSTS {
        return Err(core_thread::io_would_block_error(
            "semaphorePost",
            "failed to post semaphore: count would exceed maximum",
        ));
    }

    Err(core_platform::io_error_with_code(
        "ReleaseSemaphore",
        code as i32,
    ))
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
    binding: &BindingCallContext,
    handle: ThreadSemaphoreHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // resolve the semaphore resource
    let semaphore = core_thread::resolve_thread_resource::<resource_thread::SemaphoreResource>(
        binding,
        handle.0,
        "handle",
        "thread semaphore handle",
    )?;

    // wait for one permit with timeout semantics
    let timeout = core_thread::timeout_from_ns(timeoutns);
    let timeout_milliseconds = timeout_to_wait_milliseconds(timeout)?;
    let status = unsafe { WaitForSingleObject(semaphore.semaphore, timeout_milliseconds) };
    match core_platform::decode_wait_for_single_object_status(status, "WaitForSingleObject")? {
        core_platform::WaitStatus::Signaled => Ok(()),
        core_platform::WaitStatus::TimedOut => {
            if timeoutns == 0 {
                return Err(core_thread::io_would_block_error(
                    "semaphoreWait",
                    "failed to wait semaphore: no permits are currently available",
                ));
            }

            Err(core_thread::io_timed_out_error(
                "semaphoreWait",
                "failed to wait semaphore: timed out waiting for one permit",
            ))
        }
        core_platform::WaitStatus::Abandoned => Err(core_platform::io_error("WaitForSingleObject")),
    }
}
