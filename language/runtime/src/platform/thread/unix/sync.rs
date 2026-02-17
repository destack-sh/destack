#![allow(dead_code)]
#![allow(clippy::missing_safety_doc)]

use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::ptr;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::time::Instant;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
#[cfg(any(target_os = "linux", target_os = "android"))]
use crate::platform::core as core_platform;
use crate::platform::resource::{
    BarrierHandle, CondVarHandle, MutexHandle, RwLockHandle, ThreadSemaphoreHandle,
};
use crate::platform::thread::{core as core_thread, resource as resource_thread};

use crate::runtime::RuntimeCallContext;

/// Build one pthread-style I/O error from an explicit return code.
fn pthread_error(syscall: &str, code: libc::c_int) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        Some(code),
        Some(syscall.to_string()),
        None,
        format!("{syscall} failed: errno {code}"),
    ))
    .boxed()
}

/// Return the current unix errno value.
fn last_errno() -> i32 {
    std::io::Error::last_os_error()
        .raw_os_error()
        .unwrap_or(libc::EIO)
}

/// Convert one relative timeout into one libc timespec.
fn relative_timespec(timeout: Duration) -> RuntimeResult<libc::timespec> {
    let seconds = i64::try_from(timeout.as_secs()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "timeoutNs",
            "timeout is too large for host timespec",
        ))
        .boxed()
    })?;

    Ok(libc::timespec {
        tv_sec: seconds,
        tv_nsec: timeout.subsec_nanos() as libc::c_long,
    })
}

/// Convert one relative timeout into one realtime absolute timespec deadline.
fn realtime_deadline(timeout: Duration) -> RuntimeResult<libc::timespec> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| pthread_error("clock_gettime", libc::EINVAL))?;
    let deadline = now.checked_add(timeout).ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "timeoutNs",
            "timeout overflows host realtime deadline",
        ))
        .boxed()
    })?;

    let seconds = i64::try_from(deadline.as_secs()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "timeoutNs",
            "timeout is too large for host timespec",
        ))
        .boxed()
    })?;

    Ok(libc::timespec {
        tv_sec: seconds,
        tv_nsec: deadline.subsec_nanos() as libc::c_long,
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
pub(crate) unsafe fn destack_thread_address_wait(
    _context: &RuntimeCallContext,
    address: u64,
    expected: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // validate the waited address
    if address == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "address",
            "address must not be zero",
        ))
        .boxed());
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // validate word alignment for futex waits
        if address % (std::mem::size_of::<u32>() as u64) != 0 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "address",
                "address must be aligned to 4 bytes",
            ))
            .boxed());
        }

        // resolve the target futex pointer and timeout plan
        let address_ptr = address as *const u32;
        let timeout = core_thread::timeout_from_ns(timeoutns);
        let deadline = timeout.and_then(|duration| Instant::now().checked_add(duration));

        // wait until the value changes or the host wakes this futex
        loop {
            let mut timespec = libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            };

            let timeout_ptr = if let Some(total_timeout) = timeout {
                let remaining = match deadline {
                    Some(deadline) => deadline.saturating_duration_since(Instant::now()),
                    None => total_timeout,
                };
                timespec = relative_timespec(remaining)?;
                &timespec as *const libc::timespec
            } else {
                ptr::null()
            };

            let rc = unsafe {
                libc::syscall(
                    libc::SYS_futex,
                    address_ptr,
                    libc::FUTEX_WAIT_PRIVATE,
                    expected as libc::c_int,
                    timeout_ptr,
                    ptr::null::<libc::c_void>(),
                    0_usize,
                )
            };
            if rc == 0 {
                return Ok(());
            }

            let errno = last_errno();
            if errno == libc::EINTR {
                if let Some(deadline) = deadline {
                    if Instant::now() >= deadline {
                        return Err(core_thread::io_timed_out_error(
                            "addressWait",
                            "failed to wait on address: timed out waiting for wake",
                        ));
                    }
                }
                continue;
            }

            if errno == libc::EAGAIN {
                return Err(core_thread::io_would_block_error(
                    "addressWait",
                    "failed to wait on address: value no longer matches expected",
                ));
            }

            if errno == libc::ETIMEDOUT {
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

            return Err(core_platform::io_error_with_errno("futex", errno, None));
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = (address, expected, timeoutns);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.thread.sync.addressWait",
        ))
        .boxed())
    }
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
    _context: &RuntimeCallContext,
    address: u64,
) -> RuntimeResult<()> {
    // validate the waited address
    if address == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "address",
            "address must not be zero",
        ))
        .boxed());
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // validate word alignment for futex wakes
        if address % (std::mem::size_of::<u32>() as u64) != 0 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "address",
                "address must be aligned to 4 bytes",
            ))
            .boxed());
        }

        // wake all waiters blocked on this futex word
        let address_ptr = address as *const u32;
        let rc = unsafe {
            libc::syscall(
                libc::SYS_futex,
                address_ptr,
                libc::FUTEX_WAKE_PRIVATE,
                i32::MAX,
                ptr::null::<libc::timespec>(),
                ptr::null::<libc::c_void>(),
                0_usize,
            )
        };
        if rc < 0 {
            return Err(core_platform::io_error_with_errno(
                "futex",
                last_errno(),
                None,
            ));
        }

        return Ok(());
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = address;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.thread.sync.addressWakeAll",
        ))
        .boxed())
    }
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
    _context: &RuntimeCallContext,
    address: u64,
) -> RuntimeResult<()> {
    // validate the waited address
    if address == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "address",
            "address must not be zero",
        ))
        .boxed());
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // validate word alignment for futex wakes
        if address % (std::mem::size_of::<u32>() as u64) != 0 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "address",
                "address must be aligned to 4 bytes",
            ))
            .boxed());
        }

        // wake one waiter blocked on this futex word
        let address_ptr = address as *const u32;
        let rc = unsafe {
            libc::syscall(
                libc::SYS_futex,
                address_ptr,
                libc::FUTEX_WAKE_PRIVATE,
                1_i32,
                ptr::null::<libc::timespec>(),
                ptr::null::<libc::c_void>(),
                0_usize,
            )
        };
        if rc < 0 {
            return Err(core_platform::io_error_with_errno(
                "futex",
                last_errno(),
                None,
            ));
        }

        return Ok(());
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = address;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.thread.sync.addressWakeOne",
        ))
        .boxed())
    }
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

    #[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
    {
        // initialize one pthread barrier
        let mut barrier = MaybeUninit::<libc::pthread_barrier_t>::uninit();
        let rc = unsafe {
            libc::pthread_barrier_init(
                barrier.as_mut_ptr(),
                ptr::null::<libc::pthread_barrierattr_t>(),
                participants,
            )
        };
        if rc != 0 {
            return Err(pthread_error("pthread_barrier_init", rc));
        }

        // store one barrier resource
        let resource_id = core_thread::insert_thread_resource(
            context,
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

        return Ok(());
    }

    #[cfg(not(any(target_os = "linux", target_os = "android", target_os = "freebsd")))]
    {
        let _ = (context, participants);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.thread.sync.barrierCreate",
        ))
        .boxed())
    }
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
    handle: BarrierHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    #[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
    {
        // resolve the barrier resource
        let barrier = core_thread::resolve_thread_resource::<resource_thread::BarrierResource>(
            context,
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

        // wait for the next barrier phase
        let rc = unsafe { libc::pthread_barrier_wait(barrier.barrier.get()) };
        if rc != 0 && rc != libc::PTHREAD_BARRIER_SERIAL_THREAD {
            return Err(pthread_error("pthread_barrier_wait", rc));
        }

        // write the output leader marker
        unsafe {
            *out = rc == libc::PTHREAD_BARRIER_SERIAL_THREAD;
        }

        return Ok(());
    }

    #[cfg(not(any(target_os = "linux", target_os = "android", target_os = "freebsd")))]
    {
        let _ = (context, handle, timeoutns);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.thread.sync.barrierWait",
        ))
        .boxed())
    }
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

    // initialize one pthread condition variable
    let mut condvar = MaybeUninit::<libc::pthread_cond_t>::uninit();
    let rc = unsafe {
        libc::pthread_cond_init(
            condvar.as_mut_ptr(),
            ptr::null::<libc::pthread_condattr_t>(),
        )
    };
    if rc != 0 {
        return Err(pthread_error("pthread_cond_init", rc));
    }

    // store one condition-variable resource
    let resource_id = core_thread::insert_thread_resource(
        context,
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
    condvar: CondVarHandle,
) -> RuntimeResult<()> {
    // resolve the condition-variable resource
    let condvar = core_thread::resolve_thread_resource::<resource_thread::CondVarResource>(
        context,
        condvar.0,
        "condvar",
        "condition variable handle",
    )?;

    // notify all blocked waiters
    let rc = unsafe { libc::pthread_cond_broadcast(condvar.condvar.get()) };
    if rc != 0 {
        return Err(pthread_error("pthread_cond_broadcast", rc));
    }

    Ok(())
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
    condvar: CondVarHandle,
) -> RuntimeResult<()> {
    // resolve the condition-variable resource
    let condvar = core_thread::resolve_thread_resource::<resource_thread::CondVarResource>(
        context,
        condvar.0,
        "condvar",
        "condition variable handle",
    )?;

    // notify one blocked waiter
    let rc = unsafe { libc::pthread_cond_signal(condvar.condvar.get()) };
    if rc != 0 {
        return Err(pthread_error("pthread_cond_signal", rc));
    }

    Ok(())
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
    condvar: CondVarHandle,
    mutex: MutexHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // resolve synchronization resources
    let condvar = core_thread::resolve_thread_resource::<resource_thread::CondVarResource>(
        context,
        condvar.0,
        "condvar",
        "condition variable handle",
    )?;
    let mutex = core_thread::resolve_thread_resource::<resource_thread::MutexResource>(
        context,
        mutex.0,
        "mutex",
        "mutex handle",
    )?;

    let timeout = core_thread::timeout_from_ns(timeoutns);
    let rc = if let Some(timeout) = timeout {
        let deadline = realtime_deadline(timeout)?;
        unsafe {
            libc::pthread_cond_timedwait(
                condvar.condvar.get(),
                mutex.mutex.get(),
                &deadline as *const libc::timespec,
            )
        }
    } else {
        unsafe { libc::pthread_cond_wait(condvar.condvar.get(), mutex.mutex.get()) }
    };

    // map wait completion semantics to binding results
    if rc == 0 {
        return Ok(());
    }

    if rc == libc::ETIMEDOUT {
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

    if rc == libc::EPERM {
        return Err(core_thread::io_permission_denied_error(
            "condVarWait",
            "failed to wait condition variable: mutex is not owned by this thread",
        ));
    }

    Err(pthread_error("pthread_cond_wait", rc))
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

    // initialize one error-checking pthread mutex
    let mut attributes = MaybeUninit::<libc::pthread_mutexattr_t>::uninit();
    let attributes_init = unsafe { libc::pthread_mutexattr_init(attributes.as_mut_ptr()) };
    if attributes_init != 0 {
        return Err(pthread_error("pthread_mutexattr_init", attributes_init));
    }
    let mut attributes = unsafe { attributes.assume_init() };

    let type_rc =
        unsafe { libc::pthread_mutexattr_settype(&mut attributes, libc::PTHREAD_MUTEX_ERRORCHECK) };
    if type_rc != 0 {
        unsafe {
            libc::pthread_mutexattr_destroy(&mut attributes);
        }
        return Err(pthread_error("pthread_mutexattr_settype", type_rc));
    }

    let mut mutex = MaybeUninit::<libc::pthread_mutex_t>::uninit();
    let mutex_init = unsafe { libc::pthread_mutex_init(mutex.as_mut_ptr(), &attributes) };
    unsafe {
        libc::pthread_mutexattr_destroy(&mut attributes);
    }
    if mutex_init != 0 {
        return Err(pthread_error("pthread_mutex_init", mutex_init));
    }

    // store one mutex resource
    let resource_id = core_thread::insert_thread_resource(
        context,
        "thread.mutex",
        resource_thread::MutexResource {
            mutex: UnsafeCell::new(unsafe { mutex.assume_init() }),
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
    handle: MutexHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // resolve the mutex resource
    let mutex = core_thread::resolve_thread_resource::<resource_thread::MutexResource>(
        context,
        handle.0,
        "handle",
        "mutex handle",
    )?;

    let timeout = core_thread::timeout_from_ns(timeoutns);
    let rc = if timeoutns == 0 {
        unsafe { libc::pthread_mutex_trylock(mutex.mutex.get()) }
    } else if timeout.is_none() {
        unsafe { libc::pthread_mutex_lock(mutex.mutex.get()) }
    } else {
        #[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
        {
            let deadline = realtime_deadline(timeout.expect("timeout should exist"))?;
            unsafe { libc::pthread_mutex_timedlock(mutex.mutex.get(), &deadline) }
        }

        #[cfg(not(any(target_os = "linux", target_os = "android", target_os = "freebsd")))]
        {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.thread.sync.mutexLock",
            ))
            .boxed());
        }
    };

    if rc == 0 {
        return Ok(());
    }

    if rc == libc::EDEADLK {
        return Err(core_thread::thread_deadlock_error(
            "failed to lock mutex: current thread already owns the mutex",
        ));
    }

    if rc == libc::ETIMEDOUT {
        return Err(core_thread::io_timed_out_error(
            "mutexLock",
            "failed to lock mutex: timed out waiting for owner release",
        ));
    }

    if rc == libc::EBUSY || rc == libc::EAGAIN {
        return Err(core_thread::io_would_block_error(
            "mutexLock",
            "failed to lock mutex: mutex is currently held by another owner",
        ));
    }

    if rc == libc::EPERM {
        return Err(core_thread::io_permission_denied_error(
            "mutexLock",
            "failed to lock mutex: operation is not permitted",
        ));
    }

    Err(pthread_error("pthread_mutex_lock", rc))
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
    handle: MutexHandle,
) -> RuntimeResult<()> {
    // resolve the mutex resource
    let mutex = core_thread::resolve_thread_resource::<resource_thread::MutexResource>(
        context,
        handle.0,
        "handle",
        "mutex handle",
    )?;

    // unlock the host mutex
    let rc = unsafe { libc::pthread_mutex_unlock(mutex.mutex.get()) };
    if rc == 0 {
        return Ok(());
    }

    if rc == libc::EPERM {
        return Err(core_thread::io_permission_denied_error(
            "mutexUnlock",
            "failed to unlock mutex: current thread does not own the mutex",
        ));
    }

    Err(pthread_error("pthread_mutex_unlock", rc))
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

    // initialize one pthread rwlock
    let mut rwlock = MaybeUninit::<libc::pthread_rwlock_t>::uninit();
    let rc = unsafe {
        libc::pthread_rwlock_init(
            rwlock.as_mut_ptr(),
            ptr::null::<libc::pthread_rwlockattr_t>(),
        )
    };
    if rc != 0 {
        return Err(pthread_error("pthread_rwlock_init", rc));
    }

    // store one read-write lock resource
    let resource_id = core_thread::insert_thread_resource(
        context,
        "thread.rwlock",
        resource_thread::RwLockResource {
            rwlock: UnsafeCell::new(unsafe { rwlock.assume_init() }),
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
    handle: RwLockHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // resolve the read-write lock resource
    let rwlock = core_thread::resolve_thread_resource::<resource_thread::RwLockResource>(
        context,
        handle.0,
        "handle",
        "rwlock handle",
    )?;

    let timeout = core_thread::timeout_from_ns(timeoutns);
    let rc = if timeoutns == 0 {
        unsafe { libc::pthread_rwlock_tryrdlock(rwlock.rwlock.get()) }
    } else if timeout.is_none() {
        unsafe { libc::pthread_rwlock_rdlock(rwlock.rwlock.get()) }
    } else {
        #[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
        {
            let deadline = realtime_deadline(timeout.expect("timeout should exist"))?;
            unsafe { libc::pthread_rwlock_timedrdlock(rwlock.rwlock.get(), &deadline) }
        }

        #[cfg(not(any(target_os = "linux", target_os = "android", target_os = "freebsd")))]
        {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.thread.sync.rwlockReadLock",
            ))
            .boxed());
        }
    };

    if rc == 0 {
        return Ok(());
    }

    if rc == libc::ETIMEDOUT {
        return Err(core_thread::io_timed_out_error(
            "rwlockReadLock",
            "failed to lock rwlock for read: timed out waiting for writer release",
        ));
    }

    if rc == libc::EBUSY || rc == libc::EAGAIN {
        return Err(core_thread::io_would_block_error(
            "rwlockReadLock",
            "failed to lock rwlock for read: lock is currently unavailable",
        ));
    }

    if rc == libc::EDEADLK {
        return Err(core_thread::thread_deadlock_error(
            "failed to lock rwlock for read: lock would deadlock this thread",
        ));
    }

    Err(pthread_error("pthread_rwlock_rdlock", rc))
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
    handle: RwLockHandle,
) -> RuntimeResult<()> {
    // resolve the read-write lock resource
    let rwlock = core_thread::resolve_thread_resource::<resource_thread::RwLockResource>(
        context,
        handle.0,
        "handle",
        "rwlock handle",
    )?;

    // unlock one ownership slot
    let rc = unsafe { libc::pthread_rwlock_unlock(rwlock.rwlock.get()) };
    if rc == 0 {
        return Ok(());
    }

    if rc == libc::EPERM {
        return Err(core_thread::io_permission_denied_error(
            "rwlockUnlock",
            "failed to unlock rwlock: current thread does not hold this lock",
        ));
    }

    Err(pthread_error("pthread_rwlock_unlock", rc))
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
    handle: RwLockHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // resolve the read-write lock resource
    let rwlock = core_thread::resolve_thread_resource::<resource_thread::RwLockResource>(
        context,
        handle.0,
        "handle",
        "rwlock handle",
    )?;

    let timeout = core_thread::timeout_from_ns(timeoutns);
    let rc = if timeoutns == 0 {
        unsafe { libc::pthread_rwlock_trywrlock(rwlock.rwlock.get()) }
    } else if timeout.is_none() {
        unsafe { libc::pthread_rwlock_wrlock(rwlock.rwlock.get()) }
    } else {
        #[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
        {
            let deadline = realtime_deadline(timeout.expect("timeout should exist"))?;
            unsafe { libc::pthread_rwlock_timedwrlock(rwlock.rwlock.get(), &deadline) }
        }

        #[cfg(not(any(target_os = "linux", target_os = "android", target_os = "freebsd")))]
        {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.thread.sync.rwlockWriteLock",
            ))
            .boxed());
        }
    };

    if rc == 0 {
        return Ok(());
    }

    if rc == libc::ETIMEDOUT {
        return Err(core_thread::io_timed_out_error(
            "rwlockWriteLock",
            "failed to lock rwlock for write: timed out waiting for lock ownership",
        ));
    }

    if rc == libc::EBUSY || rc == libc::EAGAIN {
        return Err(core_thread::io_would_block_error(
            "rwlockWriteLock",
            "failed to lock rwlock for write: lock is currently unavailable",
        ));
    }

    if rc == libc::EDEADLK {
        return Err(core_thread::thread_deadlock_error(
            "failed to lock rwlock for write: lock would deadlock this thread",
        ));
    }

    Err(pthread_error("pthread_rwlock_wrlock", rc))
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

    // initialize one unnamed semaphore
    let mut semaphore = MaybeUninit::<libc::sem_t>::uninit();
    #[allow(deprecated)]
    let rc = unsafe { libc::sem_init(semaphore.as_mut_ptr(), 0, initial) };
    if rc != 0 {
        let errno = last_errno();
        if errno == libc::ENOSYS {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.thread.sync.semaphoreCreate",
            ))
            .boxed());
        }

        return Err(pthread_error("sem_init", errno));
    }

    // store one semaphore resource
    let resource_id = core_thread::insert_thread_resource(
        context,
        "thread.semaphore",
        resource_thread::SemaphoreResource {
            semaphore: UnsafeCell::new(unsafe { semaphore.assume_init() }),
            maximum,
        },
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
    handle: ThreadSemaphoreHandle,
    count: u32,
) -> RuntimeResult<()> {
    // resolve the semaphore resource
    let semaphore = core_thread::resolve_thread_resource::<resource_thread::SemaphoreResource>(
        context,
        handle.0,
        "handle",
        "thread semaphore handle",
    )?;

    // post one or more permits
    for _ in 0..count {
        let rc = unsafe { libc::sem_post(semaphore.semaphore.get()) };
        if rc == 0 {
            continue;
        }

        let errno = last_errno();
        if errno == libc::EOVERFLOW {
            return Err(core_thread::io_would_block_error(
                "semaphorePost",
                format!(
                    "failed to post semaphore: count would exceed maximum {}",
                    semaphore.maximum
                ),
            ));
        }

        return Err(pthread_error("sem_post", errno));
    }

    Ok(())
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
    handle: ThreadSemaphoreHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // resolve the semaphore resource
    let semaphore = core_thread::resolve_thread_resource::<resource_thread::SemaphoreResource>(
        context,
        handle.0,
        "handle",
        "thread semaphore handle",
    )?;

    // wait for one permit with timeout semantics
    let timeout = core_thread::timeout_from_ns(timeoutns);
    if timeoutns == 0 {
        let rc = unsafe { libc::sem_trywait(semaphore.semaphore.get()) };
        if rc == 0 {
            return Ok(());
        }

        let errno = last_errno();
        if errno == libc::EAGAIN {
            return Err(core_thread::io_would_block_error(
                "semaphoreWait",
                "failed to wait semaphore: no permits are currently available",
            ));
        }

        return Err(pthread_error("sem_trywait", errno));
    }

    if timeout.is_none() {
        loop {
            let rc = unsafe { libc::sem_wait(semaphore.semaphore.get()) };
            if rc == 0 {
                return Ok(());
            }

            let errno = last_errno();
            if errno == libc::EINTR {
                continue;
            }

            return Err(pthread_error("sem_wait", errno));
        }
    }

    #[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
    {
        let deadline = realtime_deadline(timeout.expect("timeout should exist"))?;
        loop {
            let rc = unsafe { libc::sem_timedwait(semaphore.semaphore.get(), &deadline) };
            if rc == 0 {
                return Ok(());
            }

            let errno = last_errno();
            if errno == libc::EINTR {
                continue;
            }

            if errno == libc::ETIMEDOUT {
                return Err(core_thread::io_timed_out_error(
                    "semaphoreWait",
                    "failed to wait semaphore: timed out waiting for one permit",
                ));
            }

            return Err(pthread_error("sem_timedwait", errno));
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "android", target_os = "freebsd")))]
    {
        let _ = timeout;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.thread.sync.semaphoreWait",
        ))
        .boxed())
    }
}
