#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::{bindings_generated as bindings, core as core_process};
use crate::platform::{NativeArray, PlatformError, PlatformErrorCode, core as core_platform};
use crate::runtime::{NativeSlice, NativeStringRef, NativeStringSlice};

use crate::runtime::BindingCallContext;
use bindings::*;

use crate::platform::process::{
    ExecAtFlags, GroupId, ProcessCpuSet, ProcessFdAction, ProcessFdFlags, ProcessFdSignalFlags,
    ProcessGroupIds, ProcessId, ProcessLimit, ProcessLimitResource, ProcessNamespaceKind,
    ProcessSchedulerConfig, ProcessSchedulerPolicy, ProcessSpawnOptions, ProcessStdio,
    ProcessUnshareFlags, ProcessUserIds, ProcessWaitFlags, ProcessWaitStatus, Signal, SignalEvent,
    SignalFdFlags, SignalMaskHow, SyscallFilterFlags, UserId,
};
use crate::platform::{fs, resource};

/// Send a signal to a target process.
///
/// Deliver one signal value to the target process according to host signal semantics.
/// Delivery guarantees and supported signal numbers are host-defined.
///
/// # Platform
/// Unix and Windows.
/// Uses kill(2) on Unix and terminate or control-event APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.signal.send`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_kill(
    _context: &BindingCallContext,
    pid: ProcessId,
    signal: Signal,
) -> RuntimeResult<()> {
    process_kill(pid.0, signal.0)
}

/// Read the current thread signal mask.
///
/// Return the active signal mask as an explicit signal set.
/// Mask semantics follow host thread-signal rules.
///
/// # Platform
/// Unix and Windows.
/// Uses sigprocmask or pthread_sigmask on Unix and host-equivalent APIs where available.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_signal_mask_read(
    context: &BindingCallContext,
    out: *mut NativeArray<Signal>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signalMaskRead",
    ))
    .boxed())
}

/// Update the current thread signal mask.
///
/// Apply one set, block, or unblock operation to the active signal mask.
/// Mask transitions are atomic under host signal APIs.
///
/// # Platform
/// Unix and Windows.
/// Uses sigprocmask or pthread_sigmask on Unix and host-equivalent APIs where available.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_signal_mask_update(
    context: &BindingCallContext,
    how: SignalMaskHow,
    signals: NativeSlice<Signal>,
) -> RuntimeResult<()> {
    let _ = (context, how, signals);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signalMaskUpdate",
    ))
    .boxed())
}

/// Receive the next signal event from a subscription.
///
/// Wait for the next queued signal event for the subscription.
/// Delivery ordering and batching follow runtime and host signal queue semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses host signal delivery queues and wait primitives.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_signal_receive(
    context: &BindingCallContext,
    out: *mut SignalEvent,
    handle: resource::SignalHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let signals = core_process::resolve_signal_subscription(context, handle)?;
    let event = process_signal_wait(&signals)?;
    unsafe {
        *out = event;
    }

    Ok(())
}

/// Subscribe to one signal value.
///
/// Register one runtime subscription handle for signal delivery.
/// Subscription mode and coalescing behavior follow runtime and host integration rules.
///
/// # Platform
/// Unix and Windows.
/// Uses host signal subscription state and queue integration.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_signal_subscribe(
    context: &BindingCallContext,
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

    let entry = resource::ResourceEntry::new(resource::ResourceKind::Signal)
        .with_label("process.signal.subscription")
        .with_payload(core_process::SignalSubscription {
            signals: vec![signal],
        });
    let resource_id = context
        .runtime()
        .resources
        .insert(entry, Some(context.engine()));

    unsafe {
        *out = resource::SignalHandle(resource_id);
    }

    Ok(())
}

/// Poll one signal event without blocking.
///
/// Read a queued signal event when available and return immediately otherwise.
/// Empty queue behavior is reported through host-specific not-ready errors.
///
/// # Platform
/// Unix and Windows.
/// Uses host signal queue polling with nonblocking probes.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_signal_try_receive(
    context: &BindingCallContext,
    out: *mut SignalEvent,
    handle: resource::SignalHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let signals = core_process::resolve_signal_subscription(context, handle)?;
    let event = process_signal_try_wait(&signals)?;
    unsafe {
        *out = event;
    }

    Ok(())
}

/// Poll one signal from a requested set without blocking.
///
/// Check whether one requested signal is pending and return immediately.
/// Empty readiness is reported through host-specific not-ready errors.
///
/// # Platform
/// Unix and Windows.
/// Uses nonblocking signal wait primitives and host-equivalent polling APIs where available.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_signal_try_wait(
    _context: &BindingCallContext,
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
///
/// Unregister one signal subscription from runtime delivery.
/// Pending events may still be readable depending on host queueing behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses host signal subscription teardown semantics.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_signal_unsubscribe(
    context: &BindingCallContext,
    handle: resource::SignalHandle,
) -> RuntimeResult<()> {
    let _ = core_process::resolve_signal_subscription(context, handle)?;

    let removed = context
        .runtime()
        .resources
        .remove_and_finalize(handle.0, Some(context.engine()));
    if !removed {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown signal subscription handle",
        ))
        .boxed());
    }

    Ok(())
}

/// Wait for one signal from a requested set.
///
/// Block until one of the requested signals is observed and returned.
/// Selection and wakeup semantics follow host signal wait behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses sigwait-style primitives on Unix and host-equivalent wait APIs where available.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_signal_wait(
    _context: &BindingCallContext,
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
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.signalWait")).boxed())
}

/// Poll for one signal from the provided set without blocking.
pub(super) fn process_signal_try_wait(_signals: &[Signal]) -> RuntimeResult<SignalEvent> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signalTryWait",
    ))
    .boxed())
}
