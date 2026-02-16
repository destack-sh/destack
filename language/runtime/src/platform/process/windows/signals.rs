#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::{bindings_generated as bindings, core as core_process};
use crate::platform::{
    NativeArray, NativeSlice, NativeStringRef, NativeStringSlice, PlatformError,
};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::process::{
    ExecAtFlags, GroupId, ProcessCpuSet, ProcessFdAction, ProcessFdActionKind, ProcessFdFlags,
    ProcessFdSignalFlags, ProcessGroupIds, ProcessId, ProcessLimit, ProcessLimitResource,
    ProcessNamespaceKind, ProcessSchedulerConfig, ProcessSchedulerPolicy, ProcessSpawnOptions,
    ProcessStdio, ProcessStdioKind, ProcessUnshareFlags, ProcessUserIds, ProcessWaitFlags,
    ProcessWaitKind, ProcessWaitStatus, Signal, SignalEvent, SignalFdFlags, SignalMaskHow,
    SyscallFilterFlags, UserId,
};
use crate::platform::{fs, resource};

/// Resolve a signal subscription handle into its signal set.
fn resolve_signal_subscription(
    context: &RuntimeCallContext,
    handle: resource::SignalHandle,
) -> RuntimeResult<Vec<Signal>> {
    let resolved = context.runtime().resources.with_entry(handle.0, |entry| {
        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<core_process::SignalSubscription>())
            .map(|subscription| subscription.signals.clone())
    });

    resolved.flatten().ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown signal subscription handle",
        ))
        .boxed()
    })
}
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
    context: &RuntimeCallContext,
    pid: ProcessId,
    signal: Signal,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SIGNALS_KILL)?;
    core_process::process_kill(pid.0, signal.0)
}

/// Read the current thread signal mask.
///
/// Return the active signal mask as an explicit signal set.
/// Mask semantics follow host thread-signal rules.
///
/// # Platform
/// Unix and Windows.
/// Uses sigprocmask or pthread_sigmask on Unix and runtime policy emulation on Windows.
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
    context: &RuntimeCallContext,
    out: *mut NativeArray<Signal>,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SIGNALS_SIGNAL_MASK_READ)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let signals = core_process::process_signal_mask_read()?;
    unsafe {
        *out = context.store_array(signals);
    }

    Ok(())
}

/// Update the current thread signal mask.
///
/// Apply one set, block, or unblock operation to the active signal mask.
/// Mask transitions are atomic under host signal APIs.
///
/// # Platform
/// Unix and Windows.
/// Uses sigprocmask or pthread_sigmask on Unix and runtime policy emulation on Windows.
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
    context: &RuntimeCallContext,
    how: SignalMaskHow,
    signals: NativeSlice<Signal>,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SIGNALS_SIGNAL_MASK_UPDATE)?;
    let signals = unsafe { signals.as_slice()? };
    core_process::process_signal_mask_update(how, signals)
}

/// Receive the next signal event from a subscription.
///
/// Wait for the next queued signal event for the subscription.
/// Delivery ordering and batching follow runtime and host signal queue semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses host signal waiting primitives and runtime queues.
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
    context: &RuntimeCallContext,
    out: *mut SignalEvent,
    handle: resource::SignalHandle,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SIGNALS_SIGNAL_RECEIVE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let signals = resolve_signal_subscription(context, handle)?;
    let event = core_process::process_signal_wait(&signals)?;
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
/// Uses host signal registration primitives and runtime subscription state.
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
    context: &RuntimeCallContext,
    out: *mut resource::SignalHandle,
    signal: Signal,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SIGNALS_SIGNAL_SUBSCRIBE)?;
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

    let entry = crate::platform::resource::ResourceEntry::new(
        crate::platform::resource::ResourceKind::Unknown,
    )
    .with_label("process.signal.subscription")
    .with_payload(core_process::SignalSubscription {
        signals: vec![signal],
    });
    let resource_id = context.runtime().resources.insert(entry);

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
/// Uses nonblocking host and runtime signal queue polling.
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
    context: &RuntimeCallContext,
    out: *mut SignalEvent,
    handle: resource::SignalHandle,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SIGNALS_SIGNAL_TRY_RECEIVE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let signals = resolve_signal_subscription(context, handle)?;
    let event = core_process::process_signal_try_wait(&signals)?;
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
/// Uses nonblocking signal wait primitives and runtime queue polling.
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
    context: &RuntimeCallContext,
    out: *mut SignalEvent,
    signals: NativeSlice<Signal>,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SIGNALS_SIGNAL_TRY_WAIT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let signals = unsafe { signals.as_slice()? };
    let event = core_process::process_signal_try_wait(signals)?;
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
/// Uses runtime unregistration with host signal integration.
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
    context: &RuntimeCallContext,
    handle: resource::SignalHandle,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SIGNALS_SIGNAL_UNSUBSCRIBE)?;
    let removed = context.runtime().resources.remove_and_finalize(handle.0);
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
/// Uses sigwait-style primitives on Unix and runtime signal wait integration on Windows.
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
    context: &RuntimeCallContext,
    out: *mut SignalEvent,
    signals: NativeSlice<Signal>,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SIGNALS_SIGNAL_WAIT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let signals = unsafe { signals.as_slice()? };
    let event = core_process::process_signal_wait(signals)?;
    unsafe {
        *out = event;
    }

    Ok(())
}
