use std::sync::Arc;
use std::sync::atomic::AtomicU32;
#[cfg(any(target_os = "linux", target_os = "android", windows))]
use std::sync::atomic::Ordering;
#[cfg(any(target_os = "linux", target_os = "android", windows))]
use std::thread;
#[cfg(any(target_os = "linux", target_os = "android", windows))]
use std::time::{Duration, Instant};

#[cfg(any(target_os = "linux", target_os = "android", windows))]
use super::{
    assert_platform_error_code, atomic_word_address, recv_wait_result, spawn_wait_thread,
    spawn_wake_thread, with_harness_context, with_harnesses,
};
#[cfg(not(any(target_os = "linux", target_os = "android", windows)))]
use super::{assert_platform_error_code, atomic_word_address, with_harness_context};

use crate::platform::diagnostic::PlatformErrorCode;

#[cfg(any(target_os = "linux", target_os = "android", windows))]
const WAITER_READY_TIMEOUT: Duration = Duration::from_secs(1);
#[cfg(any(target_os = "linux", target_os = "android", windows))]
const WAITER_SETTLE_DELAY: Duration = Duration::from_millis(30);
#[cfg(any(target_os = "linux", target_os = "android", windows))]
const WAKE_RESULT_TIMEOUT: Duration = Duration::from_millis(250);
#[cfg(any(target_os = "linux", target_os = "android", windows))]
const NO_EXTRA_WAKE_TIMEOUT: Duration = Duration::from_millis(100);
#[cfg(any(target_os = "linux", target_os = "android", windows))]
const WAIT_BINDING_TIMEOUT: Duration = Duration::from_secs(2);
#[cfg(any(target_os = "linux", target_os = "android", windows))]
const ROUNDTRIP_WAKE_DELAY: Duration = Duration::from_millis(20);

/// Wait until the helper waiters have reached the blocking point.
#[cfg(any(target_os = "linux", target_os = "android", windows))]
fn wait_for_ready_waiters(ready_count: &AtomicU32, expected_count: u32) {
    let deadline = Instant::now() + WAITER_READY_TIMEOUT;

    loop {
        if ready_count.load(Ordering::Acquire) >= expected_count {
            return;
        }

        assert!(
            Instant::now() < deadline,
            "thread wait helpers did not become ready in time"
        );

        thread::sleep(Duration::from_millis(5));
    }
}

/// Reject one zero address for wait-address primitives.
#[cfg(any(unix, windows))]
#[test]
fn test_thread_wait_rejects_zero_address() {
    with_harness_context(|mut context| {
        let wait_result = context.destack_thread_address_wait(0, 0, 0);
        assert_platform_error_code(wait_result, PlatformErrorCode::InvalidArgumentValue)?;

        let wake_one_result = context.destack_thread_address_wake_one(0);
        assert_platform_error_code(wake_one_result, PlatformErrorCode::InvalidArgumentValue)?;

        let wake_all_result = context.destack_thread_address_wake_all(0);
        assert_platform_error_code(wake_all_result, PlatformErrorCode::InvalidArgumentValue)?;

        Ok(())
    });
}

/// Wake one waiter without releasing every waiter blocked on the same address.
#[cfg(any(target_os = "linux", target_os = "android", windows))]
#[test]
fn test_thread_wait_wake_one_wakes_single_waiter() {
    with_harnesses(|harness| {
        let kind = harness.kind();
        let word = Arc::new(AtomicU32::new(7));
        let address = atomic_word_address(&word);
        let ready_count = Arc::new(AtomicU32::new(0));
        let (sender, receiver) = std::sync::mpsc::channel();

        // block two waiters on the same address
        let waiter_one = spawn_wait_thread(
            kind,
            Arc::clone(&ready_count),
            address,
            7,
            WAIT_BINDING_TIMEOUT,
            sender.clone(),
        );
        let waiter_two = spawn_wait_thread(
            kind,
            Arc::clone(&ready_count),
            address,
            7,
            WAIT_BINDING_TIMEOUT,
            sender,
        );

        // wait until both helpers are about to block, then give the host primitive time to park them
        wait_for_ready_waiters(&ready_count, 2);
        thread::sleep(WAITER_SETTLE_DELAY);

        // wake only one waiter first
        harness.run(|mut context| context.destack_thread_address_wake_one(address));

        let first_result = recv_wait_result(&receiver, WAKE_RESULT_TIMEOUT)
            .expect("wakeOne should release one waiter");
        first_result.expect("wakeOne waiter should succeed");

        let second_result = recv_wait_result(&receiver, NO_EXTRA_WAKE_TIMEOUT);
        assert!(
            second_result.is_none(),
            "wakeOne should not release every waiter"
        );

        // release the remaining waiter and verify that it completes too
        harness.run(|mut context| context.destack_thread_address_wake_all(address));

        let remaining_result = recv_wait_result(&receiver, WAKE_RESULT_TIMEOUT)
            .expect("wakeAll should release the remaining waiter");
        remaining_result.expect("remaining waiter should succeed");

        waiter_one
            .join()
            .expect("first wait helper thread should join cleanly");
        waiter_two
            .join()
            .expect("second wait helper thread should join cleanly");
    });
}

/// Wake all waiters blocked on the same address.
#[cfg(any(target_os = "linux", target_os = "android", windows))]
#[test]
fn test_thread_wait_wake_all_wakes_every_waiter() {
    with_harnesses(|harness| {
        let kind = harness.kind();
        let word = Arc::new(AtomicU32::new(11));
        let address = atomic_word_address(&word);
        let ready_count = Arc::new(AtomicU32::new(0));
        let (sender, receiver) = std::sync::mpsc::channel();

        // block two waiters on the same address
        let waiter_one = spawn_wait_thread(
            kind,
            Arc::clone(&ready_count),
            address,
            11,
            WAIT_BINDING_TIMEOUT,
            sender.clone(),
        );
        let waiter_two = spawn_wait_thread(
            kind,
            Arc::clone(&ready_count),
            address,
            11,
            WAIT_BINDING_TIMEOUT,
            sender,
        );

        // wait until both helpers are about to block, then give the host primitive time to park them
        wait_for_ready_waiters(&ready_count, 2);
        thread::sleep(WAITER_SETTLE_DELAY);

        // release both waiters and require both completions
        harness.run(|mut context| context.destack_thread_address_wake_all(address));

        let first_result = recv_wait_result(&receiver, WAKE_RESULT_TIMEOUT)
            .expect("wakeAll should release the first waiter");
        first_result.expect("first wakeAll waiter should succeed");

        let second_result = recv_wait_result(&receiver, WAKE_RESULT_TIMEOUT)
            .expect("wakeAll should release the second waiter");
        second_result.expect("second wakeAll waiter should succeed");

        let extra_result = recv_wait_result(&receiver, NO_EXTRA_WAKE_TIMEOUT);
        assert!(
            extra_result.is_none(),
            "wakeAll should only release the blocked waiters"
        );

        waiter_one
            .join()
            .expect("first wait helper thread should join cleanly");
        waiter_two
            .join()
            .expect("second wait helper thread should join cleanly");
    });
}

/// Wait and wake one address primitive with a real cross-thread notifier.
#[cfg(any(target_os = "linux", target_os = "android", windows))]
#[test]
fn test_thread_wait_wake_roundtrip() {
    with_harness_context(|mut context| {
        let word = Arc::new(AtomicU32::new(7));
        let address = atomic_word_address(&word);
        let wake_thread = spawn_wake_thread(Arc::clone(&word), 9, ROUNDTRIP_WAKE_DELAY, false);

        context.destack_thread_address_wait(address, 7, 1_000_000_000)?;
        wake_thread
            .join()
            .expect("wake helper thread should succeed");
        assert_eq!(word.load(Ordering::Acquire), 9);

        Ok(())
    });
}

/// Reject address waits when the observed value no longer matches.
#[cfg(any(target_os = "linux", target_os = "android", windows))]
#[test]
fn test_thread_wait_rejects_mismatched_value() {
    with_harness_context(|mut context| {
        let mut word = 7_u32;
        let address = (&mut word as *mut u32) as u64;

        let wait_result = context.destack_thread_address_wait(address, 8, 0);
        assert_platform_error_code(wait_result, PlatformErrorCode::IoWouldBlock)?;

        Ok(())
    });
}

/// Time out one address wait when the value stays unchanged.
#[cfg(any(target_os = "linux", target_os = "android", windows))]
#[test]
fn test_thread_wait_times_out_without_wake() {
    with_harness_context(|mut context| {
        let word = Arc::new(AtomicU32::new(7));
        let address = atomic_word_address(&word);

        let wait_result = context.destack_thread_address_wait(address, 7, 5_000_000);
        assert_platform_error_code(wait_result, PlatformErrorCode::IoTimedOut)?;

        Ok(())
    });
}

/// Report unsupported wait-address behavior on Unix targets without a host primitive.
#[cfg(all(unix, not(any(target_os = "linux", target_os = "android"))))]
#[test]
fn test_thread_wait_reports_not_supported_on_fallback_unix() {
    with_harness_context(|mut context| {
        let word = Arc::new(AtomicU32::new(7));
        let address = atomic_word_address(&word);

        let wait_result = context.destack_thread_address_wait(address, 7, 5_000_000);
        assert_platform_error_code(wait_result, PlatformErrorCode::NotSupported)?;

        let wake_one_result = context.destack_thread_address_wake_one(address);
        assert_platform_error_code(wake_one_result, PlatformErrorCode::NotSupported)?;

        let wake_all_result = context.destack_thread_address_wake_all(address);
        assert_platform_error_code(wake_all_result, PlatformErrorCode::NotSupported)?;

        Ok(())
    });
}
