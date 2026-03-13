#![allow(clippy::missing_safety_doc)]

use std::time::Duration;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::thread::core as core_thread;
use crate::runtime::BindingCallContext;
use windows_sys::Win32::Foundation::{ERROR_TIMEOUT, GetLastError};
use windows_sys::Win32::System::Threading::{
    INFINITE, WaitOnAddress, WakeByAddressAll, WakeByAddressSingle,
};

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
    let code = unsafe { GetLastError() as i32 };
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
pub(crate) unsafe fn destack_thread_address_wake_all(
    _binding: &BindingCallContext,
    address: u64,
) -> RuntimeResult<()> {
    let address_ptr = core_thread::checked_u32_word_pointer(address, "address")?;
    unsafe {
        WakeByAddressAll(address_ptr as *const std::ffi::c_void);
    }
    Ok(())
}

/// Wake one waiter on a memory address.
pub(crate) unsafe fn destack_thread_address_wake_one(
    _binding: &BindingCallContext,
    address: u64,
) -> RuntimeResult<()> {
    let address_ptr = core_thread::checked_u32_word_pointer(address, "address")?;
    unsafe {
        WakeByAddressSingle(address_ptr as *const std::ffi::c_void);
    }
    Ok(())
}
