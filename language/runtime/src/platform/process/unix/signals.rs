#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
use crate::platform::process::core as core_process;
use crate::platform::{NativeArray, PlatformError, PlatformErrorCode};

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
    let mut current_mask = unsafe { std::mem::zeroed::<libc::sigset_t>() };
    let read_result =
        unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, std::ptr::null(), &mut current_mask) };
    if read_result != 0 {
        return Err(signal_error_from_code(read_result, "pthread_sigmask(read)"));
    }

    let mut signals = Vec::new();
    for signal_number in 1..=MAX_SIGNAL_SCAN {
        let is_member = unsafe { libc::sigismember(&current_mask, signal_number as libc::c_int) };
        if is_member == 1 {
            signals.push(Signal(signal_number));
        }
    }

    unsafe {
        *out = binding.store_array(signals);
    }

    Ok(())
}

/// Update the current thread signal mask.
pub(crate) unsafe fn destack_process_signal_mask_update(
    _binding: &BindingCallContext,
    how: SignalMaskHow,
    signals: NativeSlice<Signal>,
) -> RuntimeResult<()> {
    let signals = unsafe { signals.as_slice()? };
    let operation = match how {
        SignalMaskHow::Set => libc::SIG_SETMASK,
        SignalMaskHow::Block => libc::SIG_BLOCK,
        SignalMaskHow::Unblock => libc::SIG_UNBLOCK,
    };

    let signal_set = signal_set_from_slice(signals)?;
    let update_result =
        unsafe { libc::pthread_sigmask(operation, &signal_set, std::ptr::null_mut()) };
    if update_result != 0 {
        return Err(signal_error_from_code(
            update_result,
            "pthread_sigmask(update)",
        ));
    }

    Ok(())
}

/// Receive the next signal event from a subscription.
pub(crate) unsafe fn destack_process_signal_receive(
    binding: &BindingCallContext,
    out: *mut SignalEvent,
    handle: resource::SignalHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let signals = core_process::resolve_signal_subscription(binding, handle)?;
    let event = process_signal_wait(&signals)?;
    unsafe {
        *out = event;
    }

    Ok(())
}

/// Subscribe to one signal value.
pub(crate) unsafe fn destack_process_signal_subscribe(
    binding: &BindingCallContext,
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
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));

    unsafe {
        *out = resource::SignalHandle(resource_id);
    }

    Ok(())
}

/// Poll one signal event without blocking.
pub(crate) unsafe fn destack_process_signal_try_receive(
    binding: &BindingCallContext,
    out: *mut SignalEvent,
    handle: resource::SignalHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let signals = core_process::resolve_signal_subscription(binding, handle)?;
    let event = process_signal_try_wait(&signals)?;
    unsafe {
        *out = event;
    }

    Ok(())
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
    let _ = core_process::resolve_signal_subscription(binding, handle)?;

    let removed = binding.worker().resources.remove_and_finalize(
        &binding.world(),
        handle.0,
        Some(binding.engine()),
    );
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
    if pid == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "pid",
            "process id must be greater than zero",
        ))
        .boxed());
    }
    if pid > libc::pid_t::MAX as u32 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "pid",
            "process id exceeds host pid range",
        ))
        .boxed());
    }
    if signal > libc::c_int::MAX as u32 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "signal",
            "signal value exceeds host range",
        ))
        .boxed());
    }

    let result = unsafe { libc::kill(pid as libc::pid_t, signal as libc::c_int) };
    if result < 0 {
        let errno = std::io::Error::last_os_error()
            .raw_os_error()
            .unwrap_or(libc::EINVAL);
        let code = if errno == libc::EINTR {
            PlatformErrorCode::IoInterrupted
        } else if errno == libc::EAGAIN || errno == libc::EWOULDBLOCK {
            PlatformErrorCode::IoWouldBlock
        } else {
            PlatformErrorCode::Process
        };

        if code == PlatformErrorCode::IoInterrupted || code == PlatformErrorCode::IoWouldBlock {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(code),
                None,
                Some(errno),
                Some("kill".to_string()),
                None,
                format!("failed to signal process {pid}"),
            ))
            .boxed());
        }

        return Err(RuntimeError::from(PlatformError::process_with(
            Some(code),
            Some(errno.to_string()),
            None,
            None,
            Some("kill".to_string()),
            format!("failed to signal process {pid}"),
        ))
        .boxed());
    }

    Ok(())
}

/// Wait for one signal from the provided set.
pub(super) fn process_signal_wait(signals: &[Signal]) -> RuntimeResult<SignalEvent> {
    ensure_signals_blocked(signals)?;
    let signal_set = signal_set_from_slice(signals)?;

    let mut signal_value: libc::c_int = 0;
    let wait_result = unsafe { libc::sigwait(&signal_set, &mut signal_value) };
    if wait_result != 0 {
        return Err(signal_error_from_code(wait_result, "sigwait"));
    }

    Ok(SignalEvent {
        signal: Signal(signal_value as u32),
        pid: ProcessId(0),
    })
}

/// Poll for one signal from the provided set without blocking.
pub(super) fn process_signal_try_wait(signals: &[Signal]) -> RuntimeResult<SignalEvent> {
    ensure_signals_blocked(signals)?;

    let mut pending_mask = unsafe { std::mem::zeroed::<libc::sigset_t>() };
    let pending_result = unsafe { libc::sigpending(&mut pending_mask) };
    if pending_result != 0 {
        let errno = std::io::Error::last_os_error()
            .raw_os_error()
            .unwrap_or(libc::EINVAL);
        return Err(signal_errno_error(errno, "sigpending"));
    }

    let mut pending_signal = None;
    for signal in signals {
        let is_member = unsafe { libc::sigismember(&pending_mask, signal.0 as libc::c_int) };
        if is_member == 1 {
            pending_signal = Some(*signal);
            break;
        }
    }

    let Some(signal) = pending_signal else {
        return Err(RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoWouldBlock),
            None,
            Some(libc::EWOULDBLOCK),
            Some("sigpending".to_string()),
            None,
            "no requested signal is pending",
        ))
        .boxed());
    };

    let single_set = signal_set_from_slice(&[signal])?;
    let mut consumed_value: libc::c_int = 0;
    let consume_result = unsafe { libc::sigwait(&single_set, &mut consumed_value) };
    if consume_result != 0 {
        return Err(signal_error_from_code(consume_result, "sigwait"));
    }

    Ok(SignalEvent {
        signal: Signal(consumed_value as u32),
        pid: ProcessId(0),
    })
}

/// Maximum signal number scanned when materializing a signal mask.
const MAX_SIGNAL_SCAN: u32 = 128;

/// Ensure all requested signals are blocked in the current thread mask.
pub(super) fn ensure_signals_blocked(signals: &[Signal]) -> RuntimeResult<()> {
    let mut current_mask = unsafe { std::mem::zeroed::<libc::sigset_t>() };
    let read_result =
        unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, std::ptr::null(), &mut current_mask) };
    if read_result != 0 {
        return Err(signal_error_from_code(read_result, "pthread_sigmask(read)"));
    }

    for signal in signals {
        if signal.0 == 0 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "signals",
                "signal id 0 is invalid",
            ))
            .boxed());
        }
        if signal.0 > libc::c_int::MAX as u32 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "signals",
                "signal id exceeds host range",
            ))
            .boxed());
        }

        let is_member = unsafe { libc::sigismember(&current_mask, signal.0 as libc::c_int) };
        if is_member < 0 {
            let errno = std::io::Error::last_os_error()
                .raw_os_error()
                .unwrap_or(libc::EINVAL);
            return Err(signal_errno_error(errno, "sigismember"));
        }
        if is_member == 0 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "signals",
                format!("signal {} must be blocked before waiting", signal.0),
            ))
            .boxed());
        }
    }

    Ok(())
}

/// Build a host signal set from runtime signal numbers.
fn signal_set_from_slice(signals: &[Signal]) -> RuntimeResult<libc::sigset_t> {
    let mut signal_set = unsafe { std::mem::zeroed::<libc::sigset_t>() };
    let empty_result = unsafe { libc::sigemptyset(&mut signal_set) };
    if empty_result != 0 {
        let errno = std::io::Error::last_os_error()
            .raw_os_error()
            .unwrap_or(libc::EINVAL);
        return Err(signal_errno_error(errno, "sigemptyset"));
    }

    for signal in signals {
        if signal.0 == 0 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "signals",
                "signal id 0 is invalid",
            ))
            .boxed());
        }

        let add_result = unsafe { libc::sigaddset(&mut signal_set, signal.0 as libc::c_int) };
        if add_result != 0 {
            let errno = std::io::Error::last_os_error()
                .raw_os_error()
                .unwrap_or(libc::EINVAL);
            return Err(signal_errno_error(errno, "sigaddset"));
        }
    }

    Ok(signal_set)
}

/// Build an error from a pthread style error code.
fn signal_error_from_code(code: i32, syscall: &str) -> Box<RuntimeError> {
    let mapped = match code {
        libc::EACCES | libc::EPERM => PlatformErrorCode::ProcessPermissionDenied,
        libc::EINTR => PlatformErrorCode::IoInterrupted,
        value if value == libc::EAGAIN || value == libc::EWOULDBLOCK => {
            PlatformErrorCode::IoWouldBlock
        }
        libc::EINVAL => PlatformErrorCode::InvalidArgumentValue,
        _ => PlatformErrorCode::Process,
    };

    if mapped == PlatformErrorCode::InvalidArgumentValue {
        return RuntimeError::from(PlatformError::invalid_argument_value(
            "signals",
            format!("{syscall} failed with EINVAL"),
        ))
        .boxed();
    }

    if mapped == PlatformErrorCode::IoInterrupted || mapped == PlatformErrorCode::IoWouldBlock {
        return RuntimeError::from(PlatformError::io_with(
            Some(mapped),
            None,
            Some(code),
            Some(syscall.to_string()),
            None,
            format!("{syscall} failed with errno {code}"),
        ))
        .boxed();
    }

    RuntimeError::from(PlatformError::process_with(
        Some(mapped),
        Some(code.to_string()),
        None,
        None,
        Some(syscall.to_string()),
        format!("{syscall} failed with errno {code}"),
    ))
    .boxed()
}

/// Build an error from a syscall errno value.
fn signal_errno_error(errno: i32, syscall: &str) -> Box<RuntimeError> {
    signal_error_from_code(errno, syscall)
}
