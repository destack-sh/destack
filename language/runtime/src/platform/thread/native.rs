#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::thread::bindings_generated as bindings;
use crate::platform::{NativeStringRef, PlatformError};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::resource;
use crate::platform::thread::ThreadOptions;

/// Stub for destack.thread.local.create.
pub unsafe fn destack_thread_local_create(
    context: &RuntimeCallContext,
    out: *mut resource::ThreadLocalKey,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_LOCAL_CREATE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.thread.local.create")).boxed())
}

/// Stub for destack.thread.local.delete.
pub unsafe fn destack_thread_local_delete(
    context: &RuntimeCallContext,
    key: resource::ThreadLocalKey,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_LOCAL_DELETE)?;
    let _ = key;

    Err(RuntimeError::from(PlatformError::not_supported("destack.thread.local.delete")).boxed())
}

/// Stub for destack.thread.local.get.
pub unsafe fn destack_thread_local_get(
    context: &RuntimeCallContext,
    out: *mut u64,
    key: resource::ThreadLocalKey,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_LOCAL_GET)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, key);

    Err(RuntimeError::from(PlatformError::not_supported("destack.thread.local.get")).boxed())
}

/// Stub for destack.thread.local.set.
pub unsafe fn destack_thread_local_set(
    context: &RuntimeCallContext,
    key: resource::ThreadLocalKey,
    value: u64,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_LOCAL_SET)?;
    let _ = (key, value);

    Err(RuntimeError::from(PlatformError::not_supported("destack.thread.local.set")).boxed())
}

/// Stub for destack.thread.priority.getAffinity.
pub unsafe fn destack_thread_get_affinity(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: resource::ThreadHandle,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_PRIORITY_GET_AFFINITY)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.priority.getAffinity",
    ))
    .boxed())
}

/// Stub for destack.thread.priority.getPriority.
pub unsafe fn destack_thread_get_priority(
    context: &RuntimeCallContext,
    out: *mut i32,
    handle: resource::ThreadHandle,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_PRIORITY_GET_PRIORITY)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.priority.getPriority",
    ))
    .boxed())
}

/// Stub for destack.thread.priority.setAffinity.
pub unsafe fn destack_thread_set_affinity(
    context: &RuntimeCallContext,
    handle: resource::ThreadHandle,
    mask: u64,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_PRIORITY_SET_AFFINITY)?;
    let _ = (handle, mask);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.priority.setAffinity",
    ))
    .boxed())
}

/// Stub for destack.thread.priority.setPriority.
pub unsafe fn destack_thread_set_priority(
    context: &RuntimeCallContext,
    handle: resource::ThreadHandle,
    priority: i32,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_PRIORITY_SET_PRIORITY)?;
    let _ = (handle, priority);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.priority.setPriority",
    ))
    .boxed())
}

/// Stub for destack.thread.spawn.detach.
pub unsafe fn destack_thread_detach(
    context: &RuntimeCallContext,
    handle: resource::ThreadHandle,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_SPAWN_DETACH)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.thread.spawn.detach")).boxed())
}

/// Stub for destack.thread.spawn.join.
pub unsafe fn destack_thread_join(
    context: &RuntimeCallContext,
    out: *mut u32,
    handle: resource::ThreadHandle,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_SPAWN_JOIN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.thread.spawn.join")).boxed())
}

/// Stub for destack.thread.spawn.spawn.
pub unsafe fn destack_thread_spawn(
    context: &RuntimeCallContext,
    out: *mut resource::ThreadHandle,
    entry: NativeStringRef,
    argument: u64,
    options: ThreadOptions,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_SPAWN_SPAWN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, entry, argument, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.thread.spawn.spawn")).boxed())
}

/// Stub for destack.thread.sync.addressWait.
pub unsafe fn destack_thread_address_wait(
    context: &RuntimeCallContext,
    address: u64,
    expected: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_SYNC_ADDRESS_WAIT)?;
    let _ = (address, expected, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.addressWait",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.addressWakeAll.
pub unsafe fn destack_thread_address_wake_all(
    context: &RuntimeCallContext,
    address: u64,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_SYNC_ADDRESS_WAKE_ALL)?;
    let _ = address;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.addressWakeAll",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.addressWakeOne.
pub unsafe fn destack_thread_address_wake_one(
    context: &RuntimeCallContext,
    address: u64,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_SYNC_ADDRESS_WAKE_ONE)?;
    let _ = address;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.addressWakeOne",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.barrierCreate.
pub unsafe fn destack_thread_barrier_create(
    context: &RuntimeCallContext,
    out: *mut resource::BarrierHandle,
    participants: u32,
    flags: u32,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_SYNC_BARRIER_CREATE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, participants, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.barrierCreate",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.barrierWait.
pub unsafe fn destack_thread_barrier_wait(
    context: &RuntimeCallContext,
    out: *mut bool,
    handle: resource::BarrierHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_SYNC_BARRIER_WAIT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.barrierWait",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.condVarCreate.
pub unsafe fn destack_thread_cond_var_create(
    context: &RuntimeCallContext,
    out: *mut resource::CondVarHandle,
    flags: u32,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_SYNC_COND_VAR_CREATE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.condVarCreate",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.condVarNotifyAll.
pub unsafe fn destack_thread_cond_var_notify_all(
    context: &RuntimeCallContext,
    condvar: resource::CondVarHandle,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_SYNC_COND_VAR_NOTIFY_ALL)?;
    let _ = condvar;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.condVarNotifyAll",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.condVarNotifyOne.
pub unsafe fn destack_thread_cond_var_notify_one(
    context: &RuntimeCallContext,
    condvar: resource::CondVarHandle,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_SYNC_COND_VAR_NOTIFY_ONE)?;
    let _ = condvar;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.condVarNotifyOne",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.condVarWait.
pub unsafe fn destack_thread_cond_var_wait(
    context: &RuntimeCallContext,
    condvar: resource::CondVarHandle,
    mutex: resource::MutexHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_SYNC_COND_VAR_WAIT)?;
    let _ = (condvar, mutex, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.condVarWait",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.mutexCreate.
pub unsafe fn destack_thread_mutex_create(
    context: &RuntimeCallContext,
    out: *mut resource::MutexHandle,
    flags: u32,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_SYNC_MUTEX_CREATE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.mutexCreate",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.mutexLock.
pub unsafe fn destack_thread_mutex_lock(
    context: &RuntimeCallContext,
    handle: resource::MutexHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_SYNC_MUTEX_LOCK)?;
    let _ = (handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.mutexLock",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.mutexUnlock.
pub unsafe fn destack_thread_mutex_unlock(
    context: &RuntimeCallContext,
    handle: resource::MutexHandle,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_SYNC_MUTEX_UNLOCK)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.mutexUnlock",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.rwlockCreate.
pub unsafe fn destack_thread_rwlock_create(
    context: &RuntimeCallContext,
    out: *mut resource::RwLockHandle,
    flags: u32,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_SYNC_RWLOCK_CREATE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.rwlockCreate",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.rwlockReadLock.
pub unsafe fn destack_thread_rwlock_read_lock(
    context: &RuntimeCallContext,
    handle: resource::RwLockHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_SYNC_RWLOCK_READ_LOCK)?;
    let _ = (handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.rwlockReadLock",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.rwlockUnlock.
pub unsafe fn destack_thread_rwlock_unlock(
    context: &RuntimeCallContext,
    handle: resource::RwLockHandle,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_SYNC_RWLOCK_UNLOCK)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.rwlockUnlock",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.rwlockWriteLock.
pub unsafe fn destack_thread_rwlock_write_lock(
    context: &RuntimeCallContext,
    handle: resource::RwLockHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_SYNC_RWLOCK_WRITE_LOCK)?;
    let _ = (handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.rwlockWriteLock",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.semaphoreCreate.
pub unsafe fn destack_thread_semaphore_create(
    context: &RuntimeCallContext,
    out: *mut resource::ThreadSemaphoreHandle,
    initial: u32,
    maximum: u32,
    flags: u32,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_SYNC_SEMAPHORE_CREATE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, initial, maximum, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.semaphoreCreate",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.semaphorePost.
pub unsafe fn destack_thread_semaphore_post(
    context: &RuntimeCallContext,
    handle: resource::ThreadSemaphoreHandle,
    count: u32,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_SYNC_SEMAPHORE_POST)?;
    let _ = (handle, count);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.semaphorePost",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.semaphoreWait.
pub unsafe fn destack_thread_semaphore_wait(
    context: &RuntimeCallContext,
    handle: resource::ThreadSemaphoreHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    context.check_policy(THREAD_SYNC_SEMAPHORE_WAIT)?;
    let _ = (handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.semaphoreWait",
    ))
    .boxed())
}
