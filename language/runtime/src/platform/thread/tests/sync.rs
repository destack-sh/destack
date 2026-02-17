use super::{
    assert_platform_error_code, assert_platform_error_codes, wait_forever_timeout,
    with_harness_context,
};

use crate::platform::diagnostic::PlatformErrorCode;

/// Create, lock, and unlock one mutex handle.
#[cfg(any(unix, windows))]
#[test]
fn test_thread_mutex_roundtrip() {
    with_harness_context(|mut context| {
        let mutex = context.mutex_create(0)?;

        context.mutex_lock(mutex, wait_forever_timeout())?;
        context.mutex_unlock(mutex)?;

        Ok(())
    });
}

/// Create one read-write lock and exercise read and write lock paths.
#[cfg(any(unix, windows))]
#[test]
fn test_thread_rwlock_roundtrip() {
    with_harness_context(|mut context| {
        let rwlock = context.rwlock_create(0)?;

        let read_lock_result = context.rwlock_read_lock(rwlock, 0);
        match read_lock_result {
            Ok(()) => {
                context.rwlock_unlock(rwlock)?;
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

        let write_lock_result = context.rwlock_write_lock(rwlock, 0);
        match write_lock_result {
            Ok(()) => {
                context.rwlock_unlock(rwlock)?;
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
        let mutex = context.mutex_create(0)?;
        let condvar = context.cond_var_create(0)?;

        context.mutex_lock(mutex, 0)?;

        let wait_result = context.cond_var_wait(condvar, mutex, 0);
        assert_platform_error_code(wait_result, PlatformErrorCode::IoWouldBlock)?;

        context.mutex_unlock(mutex)?;

        context.cond_var_notify_one(condvar)?;
        context.cond_var_notify_all(condvar)?;

        Ok(())
    });
}

/// Create one semaphore and validate post and wait operations.
#[cfg(any(unix, windows))]
#[test]
fn test_thread_semaphore_roundtrip() {
    with_harness_context(|mut context| {
        let semaphore = match context.semaphore_create(0, 4, 0) {
            Ok(semaphore) => semaphore,
            Err(error) => {
                assert_platform_error_code::<crate::platform::resource::ThreadSemaphoreHandle>(
                    Err(error),
                    PlatformErrorCode::NotSupported,
                )?;
                return Ok(());
            }
        };

        let wait_result = context.semaphore_wait(semaphore, 0);
        assert_platform_error_code(wait_result, PlatformErrorCode::IoWouldBlock)?;

        context.semaphore_post(semaphore, 1)?;
        context.semaphore_wait(semaphore, 0)?;

        Ok(())
    });
}

/// Create one single-participant barrier and observe leader status.
#[cfg(any(unix, windows))]
#[test]
fn test_thread_barrier_roundtrip() {
    with_harness_context(|mut context| {
        let barrier = match context.barrier_create(1, 0) {
            Ok(barrier) => barrier,
            Err(error) => {
                assert_platform_error_code::<crate::platform::resource::BarrierHandle>(
                    Err(error),
                    PlatformErrorCode::NotSupported,
                )?;
                return Ok(());
            }
        };

        let leader = context.barrier_wait(barrier, 0)?;

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

        let wait_result = context.address_wait(address, word, 0);
        assert_platform_error_codes(
            wait_result,
            &[
                PlatformErrorCode::IoWouldBlock,
                PlatformErrorCode::NotSupported,
            ],
        )?;

        let wake_one_result = context.address_wake_one(address);
        if let Err(error) = wake_one_result {
            assert_platform_error_code::<()>(Err(error), PlatformErrorCode::NotSupported)?;
            return Ok(());
        }

        let wake_all_result = context.address_wake_all(address);
        if let Err(error) = wake_all_result {
            assert_platform_error_code::<()>(Err(error), PlatformErrorCode::NotSupported)?;
        }

        Ok(())
    });
}
