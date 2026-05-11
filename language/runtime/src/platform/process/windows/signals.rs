#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
use crate::platform::process::core as core_process;
use crate::platform::{NativeArray, PlatformError, PlatformErrorCode, core as core_platform};

use crate::runtime::BindingCallContext;

use crate::platform::process::{ProcessId, Signal, SignalEvent, SignalMaskHow};
use crate::platform::resource;

/// Send a signal to a target process.
pub(crate) unsafe fn destack_process_kill(
    _binding: &BindingCallContext,
    pid: ProcessId,
    signal: Signal,
) -> RuntimeResult<()> {
    process_kill(pid.0, signal.0)
}

/// Read the current thread signal mask.
pub(crate) unsafe fn destack_process_signal_mask_read(
    binding: &BindingCallContext,
    out: *mut NativeArray<Signal>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalMaskRead",
    ))
    .boxed())
}

/// Update the current thread signal mask.
pub(crate) unsafe fn destack_process_signal_mask_update(
    binding: &BindingCallContext,
    how: SignalMaskHow,
    signals: NativeSlice<Signal>,
) -> RuntimeResult<()> {
    let _ = (binding, how, signals);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalMaskUpdate",
    ))
    .boxed())
}

/// Receive the next signal event from a subscription.
pub(crate) unsafe fn destack_process_signal_receive(
    _binding: &BindingCallContext,
    out: *mut SignalEvent,
    handle: resource::SignalHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalReceive",
    ))
    .boxed())
}

/// Subscribe to one signal value.
pub(crate) unsafe fn destack_process_signal_subscribe(
    _binding: &BindingCallContext,
    out: *mut resource::SignalHandle,
    signal: Signal,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    if signal.0 == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "signal",
            "signal id 0 is invalid",
        ))
        .boxed());
    }

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalSubscribe",
    ))
    .boxed())
}

/// Poll one signal event without blocking.
pub(crate) unsafe fn destack_process_signal_try_receive(
    _binding: &BindingCallContext,
    out: *mut SignalEvent,
    handle: resource::SignalHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalTryReceive",
    ))
    .boxed())
}

/// Poll one signal from a requested set without blocking.
pub(crate) unsafe fn destack_process_signal_try_wait(
    _binding: &BindingCallContext,
    out: *mut SignalEvent,
    signals: NativeSlice<Signal>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let signals = unsafe { signals.as_slice()? };
    let event = process_signal_try_wait(signals)?;
    unsafe {
        *out = event;
    }

    Ok(())
}

/// Remove a signal subscription handle.
pub(crate) unsafe fn destack_process_signal_unsubscribe(
    binding: &BindingCallContext,
    handle: resource::SignalHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalUnsubscribe",
    ))
    .boxed())
}

/// Wait for one signal from a requested set.
pub(crate) unsafe fn destack_process_signal_wait(
    _binding: &BindingCallContext,
    out: *mut SignalEvent,
    signals: NativeSlice<Signal>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let signals = unsafe { signals.as_slice()? };
    let event = process_signal_wait(signals)?;
    unsafe {
        *out = event;
    }

    Ok(())
}

/// Send a signal to the given process.
pub(super) fn process_kill(pid: u32, signal: u32) -> RuntimeResult<()> {
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_INVALID_PARAMETER};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE, TerminateProcess,
    };

    let pid = core_process::process_pid_to_windows_target(pid, "pid")?;
    let access = if signal == 0 {
        PROCESS_QUERY_LIMITED_INFORMATION
    } else {
        PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE
    };

    let process = unsafe { OpenProcess(access, 0, pid) };
    if process == 0 {
        let error = core_platform::last_error_code() as u32;
        let code = if error == ERROR_INVALID_PARAMETER {
            PlatformErrorCode::ProcessNotFound
        } else {
            PlatformErrorCode::ProcessPermissionDenied
        };
        return Err(RuntimeError::from(PlatformError::process_with(
            Some(code),
            Some(error.to_string()),
            None,
            None,
            Some("OpenProcess".to_string()),
            format!("failed to open process {pid}"),
        ))
        .boxed());
    }

    if signal == 0 {
        unsafe {
            CloseHandle(process);
        }
        return Ok(());
    }

    if signal != 1 && signal != 2 && signal != 9 && signal != 15 {
        unsafe {
            CloseHandle(process);
        }
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "signal",
            format!("unsupported signal {signal} on windows"),
        ))
        .boxed());
    }

    let result = unsafe { TerminateProcess(process, 128_u32.saturating_add(signal)) };
    let status = if result == 0 {
        let error = core_platform::last_error_code();
        Err(RuntimeError::from(PlatformError::io(format!(
            "failed to terminate process {pid}: {error}",
        )))
        .boxed())
    } else {
        Ok(())
    };

    unsafe {
        CloseHandle(process);
    }

    status
}

/// Wait for one signal from the provided set.
pub(super) fn process_signal_wait(_signals: &[Signal]) -> RuntimeResult<SignalEvent> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalWait",
    ))
    .boxed())
}

/// Poll for one signal from the provided set without blocking.
pub(super) fn process_signal_try_wait(_signals: &[Signal]) -> RuntimeResult<SignalEvent> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalTryWait",
    ))
    .boxed())
}
