use super::core::require_out;
use crate::diagnostic::RuntimeResult;
use crate::platform::io::{PollBackend, PollEvent, PollInterest, core as core_io};
use crate::platform::{NativeArray, resource};
use crate::runtime::BindingCallContext;

/// Close a poll instance.
///
/// Close one readiness poller and release host resources.
/// Registered targets are detached as part of host poller teardown.
///
/// # Platform
/// Unix and Windows.
/// Uses host poller close semantics for the selected backend.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.poll`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_poll_close(
    binding: &BindingCallContext,
    handle: resource::PollHandle,
) -> RuntimeResult<()> {
    core_io::poll_close(binding, handle)
}

/// Remove one target from a poll instance.
///
/// Deregister one runtime resource from readiness polling.
/// Pending readiness events may still be observed depending on host backend semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses epoll_ctl del, kevent delete, poll table delete, or Windows readiness teardown.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.poll`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_poll_deregister(
    binding: &BindingCallContext,
    handle: resource::PollHandle,
    target: resource::ResourceId,
) -> RuntimeResult<()> {
    core_io::poll_deregister(binding, handle, target)
}

/// Open a poll instance.
///
/// Create one readiness poller with the selected backend.
/// Backend selection is validated against host capability support.
///
/// # Platform
/// Unix and Windows.
/// Uses epoll, kqueue, poll, or the Windows readiness backend depending on backend.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.poll`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_poll_open(
    binding: &BindingCallContext,
    out: *mut resource::PollHandle,
    backend: PollBackend,
) -> RuntimeResult<()> {
    require_out(out)?;

    let value = core_io::poll_open(binding, backend)?;

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Register one target with a poll instance.
///
/// Register one runtime resource and interest mask with the poll backend.
/// Resource ownership remains unchanged and key values are returned in wait events.
///
/// # Platform
/// Unix and Windows.
/// Uses epoll_ctl add, kevent add, poll table add, or Windows readiness association.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.poll`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_poll_register(
    binding: &BindingCallContext,
    handle: resource::PollHandle,
    target: resource::ResourceId,
    key: u64,
    interest: PollInterest,
) -> RuntimeResult<()> {
    core_io::poll_register(binding, handle, target, key, interest)
}

/// Update one target in a poll instance.
///
/// Replace the registered interest mask and key for one poll target.
/// Target identity remains stable while readiness subscription changes.
///
/// # Platform
/// Unix and Windows.
/// Uses epoll_ctl mod, kevent update, poll table update, or Windows readiness metadata update.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.poll`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_poll_update(
    binding: &BindingCallContext,
    handle: resource::PollHandle,
    target: resource::ResourceId,
    key: u64,
    interest: PollInterest,
) -> RuntimeResult<()> {
    core_io::poll_update(binding, handle, target, key, interest)
}

/// Wait for poll events.
///
/// Wait for readiness events and return one batch of normalized poll events.
/// Timeout units are nanoseconds and follow host backend wait semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses epoll_wait, kevent wait, poll wait, or Windows readiness wait operations.
///
/// # Errors
/// Returns invalidArgument, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.poll`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_poll_wait(
    binding: &BindingCallContext,
    out: *mut NativeArray<PollEvent>,
    handle: resource::PollHandle,
    timeoutns: u64,
    maxevents: u32,
) -> RuntimeResult<()> {
    require_out(out)?;

    let events = core_io::poll_wait(binding, handle, timeoutns, maxevents)?;
    let events = binding.store_array(events);

    unsafe {
        out.write(events);
    }

    Ok(())
}
