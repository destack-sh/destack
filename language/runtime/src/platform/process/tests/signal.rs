use std::time::Duration;

use super::{
    assert_platform_error_code_with_privileged_policy, is_would_block, with_harness_context,
};
use crate::diagnostic::RuntimeError;
#[cfg(target_os = "linux")]
use crate::diagnostic::RuntimeResult;
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::process::{Signal, SignalFdFlags, SignalMaskHow};
#[cfg(target_os = "linux")]
use crate::platform::resource::{ResourceId, SignalFdHandle};

/// Poll attempts used by nonblocking signal tests.
const SIGNAL_POLL_ATTEMPTS: usize = 200;
/// Poll sleep interval in milliseconds used by nonblocking signal tests.
const SIGNAL_POLL_SLEEP_MS: u64 = 10;

/// Block one signal and receive it through signal_try_wait polling.
#[cfg(unix)]
#[test]
fn test_process_signal_mask_and_try_wait_roundtrip() {
    let signal = Signal(libc::SIGUSR1 as u32);

    with_harness_context(|mut context| {
        let original = context.destack_process_signal_mask_read()?;
        let original = context.signal_list_from_value(original)?;

        let test_result = (|| {
            context.destack_process_signal_mask_update(
                SignalMaskHow::Block,
                context.signal_slice_value(&[signal])?,
            )?;

            let raise_result = unsafe { libc::raise(signal.0 as libc::c_int) };
            assert_eq!(raise_result, 0);

            let mut received = None;
            for _ in 0..SIGNAL_POLL_ATTEMPTS {
                match context
                    .destack_process_signal_try_wait(context.signal_slice_value(&[signal])?)
                {
                    Ok(event) => {
                        let event = event.into_inner();
                        received = Some(event);
                        break;
                    }
                    Err(error) if is_would_block(&error) => {
                        std::thread::sleep(Duration::from_millis(SIGNAL_POLL_SLEEP_MS));
                    }
                    Err(error) => return Err(error),
                }
            }

            // signal_try_wait should eventually return the raised signal
            let received = received.ok_or_else(|| {
                RuntimeError::from(PlatformError::io(
                    "signal did not become pending within timeout",
                ))
                .boxed()
            })?;
            assert_eq!(received.signal, signal);

            Ok(())
        })();

        context.destack_process_signal_mask_update(
            SignalMaskHow::Set,
            context.signal_slice_value(&original)?,
        )?;
        test_result
    });
}

/// Subscribe to one signal and receive it through signal_try_receive polling.
#[cfg(unix)]
#[test]
fn test_process_signal_subscribe_try_receive_roundtrip() {
    let signal = Signal(libc::SIGUSR2 as u32);

    with_harness_context(|mut context| {
        let original = context.destack_process_signal_mask_read()?;
        let original = context.signal_list_from_value(original)?;

        let test_result = (|| {
            context.destack_process_signal_mask_update(
                SignalMaskHow::Block,
                context.signal_slice_value(&[signal])?,
            )?;
            let handle = context.destack_process_signal_subscribe(signal)?;

            let raise_result = unsafe { libc::raise(signal.0 as libc::c_int) };
            assert_eq!(raise_result, 0);

            let mut received = None;
            for _ in 0..SIGNAL_POLL_ATTEMPTS {
                match context.destack_process_signal_try_receive(handle) {
                    Ok(event) => {
                        let event = event.into_inner();
                        received = Some(event);
                        break;
                    }
                    Err(error) if is_would_block(&error) => {
                        std::thread::sleep(Duration::from_millis(SIGNAL_POLL_SLEEP_MS));
                    }
                    Err(error) => return Err(error),
                }
            }

            // signal_try_receive should eventually return the raised signal
            let received = received.ok_or_else(|| {
                RuntimeError::from(PlatformError::io(
                    "signal did not become receivable within timeout",
                ))
                .boxed()
            })?;
            assert_eq!(received.signal, signal);

            context.destack_process_signal_unsubscribe(handle)?;
            Ok(())
        })();

        context.destack_process_signal_mask_update(
            SignalMaskHow::Set,
            context.signal_slice_value(&original)?,
        )?;
        test_result
    });
}

/// Read one blocked signal through signal_fd_try_read polling.
#[cfg(target_os = "linux")]
#[test]
fn test_process_signal_fd_try_read_roundtrip() {
    let signal = Signal(libc::SIGUSR1 as u32);

    with_harness_context(|mut context| {
        let original = context.destack_process_signal_mask_read()?;
        let original = context.signal_list_from_value(original)?;

        let test_result = (|| {
            context.destack_process_signal_mask_update(
                SignalMaskHow::Block,
                context.signal_slice_value(&[signal])?,
            )?;
            let handle = context.destack_process_signal_fd_open(
                context.signal_slice_value(&[signal])?,
                SignalFdFlags(0),
            )?;

            let raise_result = unsafe { libc::raise(signal.0 as libc::c_int) };
            assert_eq!(raise_result, 0);

            let mut received = None;
            for _ in 0..SIGNAL_POLL_ATTEMPTS {
                match context.destack_process_signal_fd_try_read(handle) {
                    Ok(event) => {
                        let event = event.into_inner();
                        received = Some(event);
                        break;
                    }
                    Err(error) if is_would_block(&error) => {
                        std::thread::sleep(Duration::from_millis(SIGNAL_POLL_SLEEP_MS));
                    }
                    Err(error) => return Err(error),
                }
            }

            // signal_fd_try_read should eventually return the raised signal
            let received = received.ok_or_else(|| {
                RuntimeError::from(PlatformError::io(
                    "signal fd did not become readable within timeout",
                ))
                .boxed()
            })?;
            assert_eq!(received.signal, signal);

            context.destack_process_signal_fd_close(handle)?;
            Ok(())
        })();

        context.destack_process_signal_mask_update(
            SignalMaskHow::Set,
            context.signal_slice_value(&original)?,
        )?;
        test_result
    });
}

/// Receive blocked signals through subscription receive and signal_wait paths.
#[cfg(unix)]
#[test]
fn test_process_signal_receive_and_wait_roundtrip() {
    let signal = Signal(libc::SIGUSR1 as u32);

    with_harness_context(|mut context| {
        let original = context.destack_process_signal_mask_read()?;
        let original = context.signal_list_from_value(original)?;

        let test_result = (|| {
            context.destack_process_signal_mask_update(
                SignalMaskHow::Block,
                context.signal_slice_value(&[signal])?,
            )?;

            let subscription = context.destack_process_signal_subscribe(signal)?;

            let first_raise = unsafe { libc::raise(signal.0 as libc::c_int) };
            assert_eq!(first_raise, 0);

            // both receive and wait paths should return the raised signal
            let received = context.destack_process_signal_receive(subscription)?;
            let received = received.into_inner();
            assert_eq!(received.signal, signal);

            context.destack_process_signal_unsubscribe(subscription)?;

            let second_raise = unsafe { libc::raise(signal.0 as libc::c_int) };
            assert_eq!(second_raise, 0);

            let waited =
                context.destack_process_signal_wait(context.signal_slice_value(&[signal])?)?;
            let waited = waited.into_inner();
            assert_eq!(waited.signal, signal);

            Ok(())
        })();

        context.destack_process_signal_mask_update(
            SignalMaskHow::Set,
            context.signal_slice_value(&original)?,
        )?;
        test_result
    });
}

/// Read a blocked signal through signal_fd_read after updating signal_fd mask.
#[cfg(target_os = "linux")]
#[test]
fn test_process_signal_fd_read_and_set_mask_roundtrip() {
    let signal = Signal(libc::SIGUSR2 as u32);

    with_harness_context(|mut context| {
        let original = context.destack_process_signal_mask_read()?;
        let original = context.signal_list_from_value(original)?;

        let test_result = (|| {
            context.destack_process_signal_mask_update(
                SignalMaskHow::Block,
                context.signal_slice_value(&[signal])?,
            )?;
            let handle = context.destack_process_signal_fd_open(
                context.signal_slice_value(&[signal])?,
                SignalFdFlags(0),
            )?;

            context.destack_process_signal_fd_set_mask(
                handle,
                context.signal_slice_value(&[signal])?,
            )?;

            let raise_result = unsafe { libc::raise(signal.0 as libc::c_int) };
            assert_eq!(raise_result, 0);

            let received = context.destack_process_signal_fd_read(handle)?;
            let received = received.into_inner();
            assert_eq!(received.signal, signal);

            context.destack_process_signal_fd_close(handle)?;
            Ok(())
        })();

        context.destack_process_signal_mask_update(
            SignalMaskHow::Set,
            context.signal_slice_value(&original)?,
        )?;
        test_result
    });
}

/// Reject signal_wait calls that use an invalid signal value.
#[cfg(unix)]
#[test]
fn test_process_signal_wait_rejects_invalid_signal_value() {
    with_harness_context(|mut context| {
        assert_platform_error_code_with_privileged_policy(
            context.destack_process_signal_wait(context.signal_slice_value(&[Signal(0)])?),
            PlatformErrorCode::InvalidArgumentValue,
        )
    });
}

/// Require blocked signals before allowing signal-fd open and mask updates.
#[cfg(target_os = "linux")]
#[test]
fn test_process_signal_fd_requires_blocked_mask() {
    let signal = Signal(libc::SIGUSR1 as u32);

    with_harness_context(|mut context| {
        let original = context.destack_process_signal_mask_read()?;
        let original = context.signal_list_from_value(original)?;

        let test_result = (|| {
            context.destack_process_signal_mask_update(
                SignalMaskHow::Unblock,
                context.signal_slice_value(&[signal])?,
            )?;

            assert_platform_error_code_with_privileged_policy(
                context.destack_process_signal_fd_open(
                    context.signal_slice_value(&[signal])?,
                    SignalFdFlags(0),
                ),
                PlatformErrorCode::InvalidArgumentValue,
            )?;

            context.destack_process_signal_mask_update(
                SignalMaskHow::Block,
                context.signal_slice_value(&[signal])?,
            )?;
            let handle = context.destack_process_signal_fd_open(
                context.signal_slice_value(&[signal])?,
                SignalFdFlags(0),
            )?;

            context.destack_process_signal_mask_update(
                SignalMaskHow::Unblock,
                context.signal_slice_value(&[signal])?,
            )?;
            assert_platform_error_code_with_privileged_policy(
                context.destack_process_signal_fd_set_mask(
                    handle,
                    context.signal_slice_value(&[signal])?,
                ),
                PlatformErrorCode::InvalidArgumentValue,
            )?;

            context.destack_process_signal_fd_close(handle)?;
            Ok(())
        })();

        context.destack_process_signal_mask_update(
            SignalMaskHow::Set,
            context.signal_slice_value(&original)?,
        )?;
        test_result
    });
}

/// Require blocked signals before allowing signal_wait and signal_try_wait.
#[cfg(unix)]
#[test]
fn test_process_signal_wait_requires_blocked_mask() {
    let signal = Signal(libc::SIGUSR1 as u32);

    with_harness_context(|mut context| {
        let original = context.destack_process_signal_mask_read()?;
        let original = context.signal_list_from_value(original)?;
        let test_result = (|| {
            context.destack_process_signal_mask_update(
                SignalMaskHow::Unblock,
                context.signal_slice_value(&[signal])?,
            )?;
            assert_platform_error_code_with_privileged_policy(
                context.destack_process_signal_wait(context.signal_slice_value(&[signal])?),
                PlatformErrorCode::InvalidArgumentValue,
            )?;
            assert_platform_error_code_with_privileged_policy(
                context.destack_process_signal_try_wait(context.signal_slice_value(&[signal])?),
                PlatformErrorCode::InvalidArgumentValue,
            )?;
            Ok(())
        })();

        context.destack_process_signal_mask_update(
            SignalMaskHow::Set,
            context.signal_slice_value(&original)?,
        )?;
        test_result
    });
}

/// Report specific errors for invalid signal-fd flags and forged handles.
#[cfg(target_os = "linux")]
#[test]
fn test_process_signal_fd_validation_errors_are_specific() {
    with_harness_context(|mut context| {
        // invalid signal-fd arguments should return invalid-argument errors
        assert_platform_error_code_with_privileged_policy(
            context.destack_process_signal_fd_open(
                context.signal_slice_value(&[Signal(libc::SIGUSR1 as u32)])?,
                SignalFdFlags(1),
            ),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        let invalid_handle = SignalFdHandle(ResourceId::local(0));
        assert_platform_error_code_with_privileged_policy(
            context.destack_process_signal_fd_try_read(invalid_handle),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        assert_platform_error_code_with_privileged_policy(
            context.destack_process_signal_fd_read(invalid_handle),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        assert_platform_error_code_with_privileged_policy(
            context.destack_process_signal_fd_set_mask(
                invalid_handle,
                context.signal_slice_value(&[Signal(libc::SIGUSR1 as u32)])?,
            ),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        assert_platform_error_code_with_privileged_policy(
            context.destack_process_signal_fd_close(invalid_handle),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        // forged signal-fd handles must be rejected even when ids overlap
        let original = context.destack_process_signal_mask_read()?;
        let original = context.signal_list_from_value(original)?;
        let signal = Signal(libc::SIGUSR2 as u32);
        let forged_result: RuntimeResult<()> = (|| {
            context.destack_process_signal_mask_update(
                SignalMaskHow::Block,
                context.signal_slice_value(&[signal])?,
            )?;
            let subscription = context.destack_process_signal_subscribe(signal)?;
            let forged_handle = SignalFdHandle(subscription.0);

            assert_platform_error_code_with_privileged_policy(
                context.destack_process_signal_fd_close(forged_handle),
                PlatformErrorCode::InvalidArgumentValue,
            )?;

            context.destack_process_signal_unsubscribe(subscription)?;
            Ok(())
        })();
        context.destack_process_signal_mask_update(
            SignalMaskHow::Set,
            context.signal_slice_value(&original)?,
        )?;
        forged_result?;

        Ok(())
    });
}

/// Park signal-fd support on unix hosts without signalfd semantics.
#[cfg(all(unix, not(target_os = "linux")))]
#[test]
fn test_process_signal_fd_reports_not_supported_without_signalfd() {
    with_harness_context(|mut context| {
        assert_platform_error_code_with_privileged_policy(
            context.destack_process_signal_fd_open(
                context.signal_slice_value(&[Signal(libc::SIGUSR1 as u32)])?,
                SignalFdFlags(0),
            ),
            PlatformErrorCode::NotSupported,
        )?;

        Ok(())
    });
}
