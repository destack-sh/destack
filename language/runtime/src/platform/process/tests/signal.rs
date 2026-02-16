use std::time::Duration;

use super::{is_would_block, with_harness_context};
use crate::diagnostic::RuntimeError;
use crate::platform::process::{Signal, SignalFdFlags, SignalMaskHow};

#[cfg(unix)]
#[test]
fn test_process_signal_mask_and_try_wait_roundtrip() {
    let signal = Signal(libc::SIGUSR1 as u32);

    with_harness_context(|mut context| {
        let original = context.signal_mask_read()?;

        let test_result = (|| {
            context.signal_mask_update(SignalMaskHow::Block, &[signal])?;

            let raise_result = unsafe { libc::raise(signal.0 as libc::c_int) };
            assert_eq!(raise_result, 0);

            let mut received = None;
            for _ in 0..20 {
                match context.signal_try_wait(&[signal]) {
                    Ok(event) => {
                        received = Some(event);
                        break;
                    }
                    Err(error) if is_would_block(&error) => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => return Err(error),
                }
            }

            let received = received.ok_or_else(|| {
                RuntimeError::from(crate::platform::PlatformError::io(
                    "signal did not become pending within timeout",
                ))
                .boxed()
            })?;
            assert_eq!(received.signal, signal);

            Ok(())
        })();

        context.signal_mask_update(SignalMaskHow::Set, &original)?;
        test_result
    });
}

#[cfg(unix)]
#[test]
fn test_process_signal_subscribe_try_receive_roundtrip() {
    let signal = Signal(libc::SIGUSR2 as u32);

    with_harness_context(|mut context| {
        let original = context.signal_mask_read()?;

        let test_result = (|| {
            context.signal_mask_update(SignalMaskHow::Block, &[signal])?;
            let handle = context.signal_subscribe(signal)?;

            let raise_result = unsafe { libc::raise(signal.0 as libc::c_int) };
            assert_eq!(raise_result, 0);

            let mut received = None;
            for _ in 0..20 {
                match context.signal_try_receive(handle) {
                    Ok(event) => {
                        received = Some(event);
                        break;
                    }
                    Err(error) if is_would_block(&error) => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => return Err(error),
                }
            }

            let received = received.ok_or_else(|| {
                RuntimeError::from(crate::platform::PlatformError::io(
                    "signal did not become receivable within timeout",
                ))
                .boxed()
            })?;
            assert_eq!(received.signal, signal);

            context.signal_unsubscribe(handle)?;
            Ok(())
        })();

        context.signal_mask_update(SignalMaskHow::Set, &original)?;
        test_result
    });
}

#[cfg(unix)]
#[test]
fn test_process_signal_fd_try_read_roundtrip() {
    let signal = Signal(libc::SIGUSR1 as u32);

    with_harness_context(|mut context| {
        let original = context.signal_mask_read()?;

        let test_result = (|| {
            context.signal_mask_update(SignalMaskHow::Block, &[signal])?;
            let handle = context.signal_fd_open(&[signal], SignalFdFlags(0))?;

            let raise_result = unsafe { libc::raise(signal.0 as libc::c_int) };
            assert_eq!(raise_result, 0);

            let mut received = None;
            for _ in 0..20 {
                match context.signal_fd_try_read(handle) {
                    Ok(event) => {
                        received = Some(event);
                        break;
                    }
                    Err(error) if is_would_block(&error) => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => return Err(error),
                }
            }

            let received = received.ok_or_else(|| {
                RuntimeError::from(crate::platform::PlatformError::io(
                    "signal fd did not become readable within timeout",
                ))
                .boxed()
            })?;
            assert_eq!(received.signal, signal);

            context.signal_fd_close(handle)?;
            Ok(())
        })();

        context.signal_mask_update(SignalMaskHow::Set, &original)?;
        test_result
    });
}
