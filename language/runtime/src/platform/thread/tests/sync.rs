use super::{
    assert_platform_error_code, assert_platform_error_codes, result_or_skip_not_supported,
    wait_forever_timeout, with_harness_context,
};

use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{
    BarrierHandle, CondVarHandle, MutexHandle, ResourceId, RwLockHandle, ThreadSemaphoreHandle,
};

/// Create, lock, and unlock one mutex handle.
#[cfg(any(unix, windows))]
#[test]
fn test_thread_mutex_roundtrip() {
    with_harness_context(|mut context| {
        let mutex = context.destack_thread_mutex_create(0)?;

        context.destack_thread_mutex_lock(mutex, wait_forever_timeout())?;
        context.destack_thread_mutex_unlock(mutex)?;

        Ok(())
    });
}

/// Create one read-write lock and exercise read and write lock paths.
#[cfg(any(unix, windows))]
#[test]
fn test_thread_rwlock_roundtrip() {
    with_harness_context(|mut context| {
        let rwlock = context.destack_thread_rwlock_create(0)?;

        let read_lock_result = context.destack_thread_rwlock_read_lock(rwlock, 0);
        match read_lock_result {
            Ok(()) => {
                context.destack_thread_rwlock_unlock(rwlock)?;
            }
            Err(error) => {
                assert_platform_error_codes::<()>(
                    Err(error),
                    &[
                        PlatformErrorCode::IoWouldBlock,
                        PlatformErrorCode::NotSupported,
                    ],
                )?;
            }
        }

        let write_lock_result = context.destack_thread_rwlock_write_lock(rwlock, 0);
        match write_lock_result {
            Ok(()) => {
                context.destack_thread_rwlock_unlock(rwlock)?;
            }
            Err(error) => {
                assert_platform_error_codes::<()>(
                    Err(error),
                    &[
                        PlatformErrorCode::IoWouldBlock,
                        PlatformErrorCode::NotSupported,
                    ],
                )?;
            }
        }

        Ok(())
    });
}

/// Create one condition variable and validate wait and notify operations.
#[cfg(any(unix, windows))]
#[test]
fn test_thread_cond_var_roundtrip() {
    with_harness_context(|mut context| {
        let mutex = context.destack_thread_mutex_create(0)?;
        let condvar = context.destack_thread_cond_var_create(0)?;

        context.destack_thread_mutex_lock(mutex, 0)?;

        let wait_result = context.destack_thread_cond_var_wait(condvar, mutex, 0);
        assert_platform_error_code(wait_result, PlatformErrorCode::IoWouldBlock)?;

        context.destack_thread_mutex_unlock(mutex)?;

        context.destack_thread_cond_var_notify_one(condvar)?;
        context.destack_thread_cond_var_notify_all(condvar)?;

        Ok(())
    });
}

/// Create one semaphore and validate post and wait operations.
#[cfg(any(unix, windows))]
#[test]
fn test_thread_semaphore_roundtrip() {
    with_harness_context(|mut context| {
        let Some(semaphore) =
            result_or_skip_not_supported(context.destack_thread_semaphore_create(0, 4, 0))?
        else {
            return Ok(());
        };

        let wait_result = context.destack_thread_semaphore_wait(semaphore, 0);
        assert_platform_error_code(wait_result, PlatformErrorCode::IoWouldBlock)?;

        context.destack_thread_semaphore_post(semaphore, 1)?;
        context.destack_thread_semaphore_wait(semaphore, 0)?;

        Ok(())
    });
}

/// Create one single-participant barrier and observe leader status.
#[cfg(any(unix, windows))]
#[test]
fn test_thread_barrier_roundtrip() {
    with_harness_context(|mut context| {
        let Some(barrier) =
            result_or_skip_not_supported(context.destack_thread_barrier_create(1, 0))?
        else {
            return Ok(());
        };

        let leader = context.destack_thread_barrier_wait(barrier, 0)?;

        // single participant barrier waits should always elect one leader
        assert!(leader);

        Ok(())
    });
}

/// Wait and wake on one address primitive.
#[cfg(any(unix, windows))]
#[test]
fn test_thread_address_wait_wake_roundtrip() {
    with_harness_context(|mut context| {
        let mut word = 7_u32;
        let address = (&mut word as *mut u32) as u64;

        let wait_result = context.destack_thread_address_wait(address, word, 0);
        assert_platform_error_codes(
            wait_result,
            &[
                PlatformErrorCode::IoWouldBlock,
                PlatformErrorCode::NotSupported,
            ],
        )?;

        let wake_one_result = context.destack_thread_address_wake_one(address);
        if result_or_skip_not_supported(wake_one_result)?.is_none() {
            return Ok(());
        }

        let wake_all_result = context.destack_thread_address_wake_all(address);
        let _ = result_or_skip_not_supported(wake_all_result)?;

        Ok(())
    });
}

/// Reject synchronization operations for unknown synchronization handles.
#[cfg(any(unix, windows))]
#[test]
fn test_thread_sync_rejects_unknown_handles() {
    with_harness_context(|mut context| {
        let unknown_mutex = MutexHandle(ResourceId(0));
        let unknown_rwlock = RwLockHandle(ResourceId(0));
        let unknown_condvar = CondVarHandle(ResourceId(0));
        let unknown_semaphore = ThreadSemaphoreHandle(ResourceId(0));
        let unknown_barrier = BarrierHandle(ResourceId(0));

        let mutex_lock_result = context.destack_thread_mutex_lock(unknown_mutex, 0);
        assert_platform_error_code(mutex_lock_result, PlatformErrorCode::InvalidArgumentValue)?;

        let mutex_unlock_result = context.destack_thread_mutex_unlock(unknown_mutex);
        assert_platform_error_code(mutex_unlock_result, PlatformErrorCode::InvalidArgumentValue)?;

        let read_lock_result = context.destack_thread_rwlock_read_lock(unknown_rwlock, 0);
        assert_platform_error_code(read_lock_result, PlatformErrorCode::InvalidArgumentValue)?;

        let write_lock_result = context.destack_thread_rwlock_write_lock(unknown_rwlock, 0);
        assert_platform_error_code(write_lock_result, PlatformErrorCode::InvalidArgumentValue)?;

        let rwlock_unlock_result = context.destack_thread_rwlock_unlock(unknown_rwlock);
        assert_platform_error_code(
            rwlock_unlock_result,
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        let condvar_wait_result =
            context.destack_thread_cond_var_wait(unknown_condvar, unknown_mutex, 0);
        assert_platform_error_code(condvar_wait_result, PlatformErrorCode::InvalidArgumentValue)?;

        let condvar_notify_one_result = context.destack_thread_cond_var_notify_one(unknown_condvar);
        assert_platform_error_code(
            condvar_notify_one_result,
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        let condvar_notify_all_result = context.destack_thread_cond_var_notify_all(unknown_condvar);
        assert_platform_error_code(
            condvar_notify_all_result,
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        let semaphore_wait_result = context.destack_thread_semaphore_wait(unknown_semaphore, 0);
        assert_platform_error_code(
            semaphore_wait_result,
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        let semaphore_post_result = context.destack_thread_semaphore_post(unknown_semaphore, 1);
        assert_platform_error_code(
            semaphore_post_result,
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        let barrier_support =
            result_or_skip_not_supported(context.destack_thread_barrier_create(1, 0))?;
        if barrier_support.is_some() {
            let barrier_wait_result = context.destack_thread_barrier_wait(unknown_barrier, 0);
            assert_platform_error_codes(
                barrier_wait_result,
                &[PlatformErrorCode::InvalidArgumentValue],
            )?;
        }

        Ok(())
    });
}
