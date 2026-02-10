use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::thread::ThreadOptionsVm;
use crate::platform::{PlatformError, resource};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.thread.local.create.
pub(super) fn destack_thread_local_create(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<resource::ThreadLocalKey> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.local.create is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.local.delete.
pub(super) fn destack_thread_local_delete(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    key: resource::ThreadLocalKey,
) -> RuntimeResult<()> {
    let _ = key;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.local.delete is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.local.get.
pub(super) fn destack_thread_local_get(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    key: resource::ThreadLocalKey,
) -> RuntimeResult<u64> {
    let _ = key;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.local.get is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.local.set.
pub(super) fn destack_thread_local_set(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    key: resource::ThreadLocalKey,
    value: u64,
) -> RuntimeResult<()> {
    let _ = (key, value);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.local.set is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.priority.getAffinity.
pub(super) fn destack_thread_get_affinity(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::ThreadHandle,
) -> RuntimeResult<u64> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.priority.getAffinity is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.priority.getPriority.
pub(super) fn destack_thread_get_priority(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::ThreadHandle,
) -> RuntimeResult<i32> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.priority.getPriority is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.priority.setAffinity.
pub(super) fn destack_thread_set_affinity(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::ThreadHandle,
    mask: u64,
) -> RuntimeResult<()> {
    let _ = (handle, mask);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.priority.setAffinity is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.priority.setPriority.
pub(super) fn destack_thread_set_priority(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::ThreadHandle,
    priority: i32,
) -> RuntimeResult<()> {
    let _ = (handle, priority);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.priority.setPriority is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.spawn.detach.
pub(super) fn destack_thread_detach(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::ThreadHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.spawn.detach is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.spawn.join.
pub(super) fn destack_thread_join(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::ThreadHandle,
) -> RuntimeResult<u32> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.spawn.join is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.spawn.spawn.
pub(super) fn destack_thread_spawn(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    entry: vm::StringHandle,
    argument: u64,
    options: ThreadOptionsVm,
) -> RuntimeResult<resource::ThreadHandle> {
    let _ = (entry, argument, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.spawn.spawn is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.addressWait.
pub(super) fn destack_thread_address_wait(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    address: u64,
    expected: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (address, expected, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.addressWait is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.addressWakeAll.
pub(super) fn destack_thread_address_wake_all(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    address: u64,
) -> RuntimeResult<()> {
    let _ = address;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.addressWakeAll is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.addressWakeOne.
pub(super) fn destack_thread_address_wake_one(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    address: u64,
) -> RuntimeResult<()> {
    let _ = address;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.addressWakeOne is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.barrierCreate.
pub(super) fn destack_thread_barrier_create(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    participants: u32,
    flags: u32,
) -> RuntimeResult<resource::BarrierHandle> {
    let _ = (participants, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.barrierCreate is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.barrierWait.
pub(super) fn destack_thread_barrier_wait(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::BarrierHandle,
    timeoutns: u64,
) -> RuntimeResult<bool> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.barrierWait is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.condVarCreate.
pub(super) fn destack_thread_cond_var_create(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    flags: u32,
) -> RuntimeResult<resource::CondVarHandle> {
    let _ = flags;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.condVarCreate is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.condVarNotifyAll.
pub(super) fn destack_thread_cond_var_notify_all(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    condvar: resource::CondVarHandle,
) -> RuntimeResult<()> {
    let _ = condvar;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.condVarNotifyAll is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.condVarNotifyOne.
pub(super) fn destack_thread_cond_var_notify_one(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    condvar: resource::CondVarHandle,
) -> RuntimeResult<()> {
    let _ = condvar;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.condVarNotifyOne is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.condVarWait.
pub(super) fn destack_thread_cond_var_wait(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    condvar: resource::CondVarHandle,
    mutex: resource::MutexHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (condvar, mutex, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.condVarWait is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.mutexCreate.
pub(super) fn destack_thread_mutex_create(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    flags: u32,
) -> RuntimeResult<resource::MutexHandle> {
    let _ = flags;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.mutexCreate is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.mutexLock.
pub(super) fn destack_thread_mutex_lock(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::MutexHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.mutexLock is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.mutexUnlock.
pub(super) fn destack_thread_mutex_unlock(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::MutexHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.mutexUnlock is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.rwlockCreate.
pub(super) fn destack_thread_rwlock_create(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    flags: u32,
) -> RuntimeResult<resource::RwLockHandle> {
    let _ = flags;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.rwlockCreate is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.rwlockReadLock.
pub(super) fn destack_thread_rwlock_read_lock(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::RwLockHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.rwlockReadLock is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.rwlockUnlock.
pub(super) fn destack_thread_rwlock_unlock(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::RwLockHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.rwlockUnlock is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.rwlockWriteLock.
pub(super) fn destack_thread_rwlock_write_lock(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::RwLockHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.rwlockWriteLock is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.semaphoreCreate.
pub(super) fn destack_thread_semaphore_create(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    initial: u32,
    maximum: u32,
    flags: u32,
) -> RuntimeResult<resource::ThreadSemaphoreHandle> {
    let _ = (initial, maximum, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.semaphoreCreate is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.semaphorePost.
pub(super) fn destack_thread_semaphore_post(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::ThreadSemaphoreHandle,
    count: u32,
) -> RuntimeResult<()> {
    let _ = (handle, count);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.semaphorePost is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.thread.sync.semaphoreWait.
pub(super) fn destack_thread_semaphore_wait(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::ThreadSemaphoreHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.sync.semaphoreWait is not available in the VM yet",
    ))
    .boxed())
}
