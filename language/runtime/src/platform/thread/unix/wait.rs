#![allow(clippy::missing_safety_doc)]

#[cfg(any(target_os = "linux", target_os = "android"))]
use std::ptr;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::time::Instant;

use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(not(any(target_os = "linux", target_os = "android")))]
use crate::platform::PlatformError;
#[cfg(any(target_os = "linux", target_os = "android"))]
use crate::platform::core as core_platform;
use crate::platform::thread::core as core_thread;
use crate::runtime::BindingCallContext;

#[cfg(any(target_os = "linux", target_os = "android"))]
const FUTEX_WAIT_PRIVATE_OPERATION: libc::c_int = 128;
#[cfg(any(target_os = "linux", target_os = "android"))]
const FUTEX_WAKE_PRIVATE_OPERATION: libc::c_int = 129;

/// Return the current unix errno value.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn last_errno() -> i32 {
    std::io::Error::last_os_error()
        .raw_os_error()
        .unwrap_or(libc::EIO)
}

/// Convert one relative timeout into one libc timespec.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn relative_timespec(timeout: std::time::Duration) -> RuntimeResult<libc::timespec> {
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

/// Wait on one memory address value.
pub(crate) unsafe fn destack_thread_address_wait(
    _binding: &BindingCallContext,
    address: u64,
    expected: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // validate the target wait word shape before dispatching to host support
    let address_ptr = core_thread::checked_u32_word_pointer(address, "address")?;

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // resolve the target futex pointer and timeout plan
        let timeout = core_thread::timeout_from_ns(timeoutns);
        let deadline = timeout.and_then(|duration| Instant::now().checked_add(duration));

        // wait until the value changes or the host wakes this futex
        loop {
            let timeout_value = if let Some(total_timeout) = timeout {
                let remaining = match deadline {
                    Some(deadline) => deadline.saturating_duration_since(Instant::now()),
                    None => total_timeout,
                };
                Some(relative_timespec(remaining)?)
            } else {
                None
            };
            let timeout_ptr = timeout_value
                .as_ref()
                .map_or(ptr::null(), |value| value as *const libc::timespec);

            let rc = libc::syscall(
                libc::SYS_futex,
                address_ptr,
                FUTEX_WAIT_PRIVATE_OPERATION,
                expected as libc::c_int,
                timeout_ptr,
                ptr::null::<libc::c_void>(),
                0_usize,
            );
            if rc == 0 {
                return Ok(());
            }

            let errno = last_errno();
            if errno == libc::EINTR {
                if let Some(deadline) = deadline
                    && Instant::now() >= deadline
                {
                    return Err(core_thread::io_timed_out_error(
                        "addressWait",
                        "failed to wait on address: timed out waiting for wake",
                    ));
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
        let _ = (address_ptr, expected, timeoutns);

        // FUGU #Incomplete: implement a principled host substrate for non-Linux Unix address wait or keep it unsupported
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.thread.wait.addressWait",
        ))
        .boxed())
    }
}

/// Wake all waiters on a memory address.
pub(crate) unsafe fn destack_thread_address_wake_all(
    _binding: &BindingCallContext,
    address: u64,
) -> RuntimeResult<()> {
    // validate the target wake word shape before dispatching to host support
    let address_ptr = core_thread::checked_u32_word_pointer(address, "address")?;

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // wake all waiters blocked on this futex word
        let rc = libc::syscall(
            libc::SYS_futex,
            address_ptr,
            FUTEX_WAKE_PRIVATE_OPERATION,
            i32::MAX,
            ptr::null::<libc::timespec>(),
            ptr::null::<libc::c_void>(),
            0_usize,
        );
        if rc < 0 {
            return Err(core_platform::io_error_with_errno(
                "futex",
                last_errno(),
                None,
            ));
        }

        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = address_ptr;

        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.thread.wait.addressWakeAll",
        ))
        .boxed())
    }
}

/// Wake one waiter on a memory address.
pub(crate) unsafe fn destack_thread_address_wake_one(
    _binding: &BindingCallContext,
    address: u64,
) -> RuntimeResult<()> {
    // validate the target wake word shape before dispatching to host support
    let address_ptr = core_thread::checked_u32_word_pointer(address, "address")?;

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // wake one waiter blocked on this futex word
        let rc = libc::syscall(
            libc::SYS_futex,
            address_ptr,
            FUTEX_WAKE_PRIVATE_OPERATION,
            1_i32,
            ptr::null::<libc::timespec>(),
            ptr::null::<libc::c_void>(),
            0_usize,
        );
        if rc < 0 {
            return Err(core_platform::io_error_with_errno(
                "futex",
                last_errno(),
                None,
            ));
        }

        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = address_ptr;

        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.thread.wait.addressWakeOne",
        ))
        .boxed())
    }
}
