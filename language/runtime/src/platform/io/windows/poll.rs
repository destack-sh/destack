use super::core::require_out;
use crate::diagnostic::RuntimeResult;
use crate::platform::io::{PollBackend, PollEvent, PollInterest, core as core_io};
use crate::platform::{NativeArray, resource};
use crate::runtime::BindingCallContext;

/// Close a poll instance.
pub(crate) unsafe fn destack_io_poll_close(
    binding: &BindingCallContext,
    handle: resource::PollHandle,
) -> RuntimeResult<()> {
    core_io::poll_close(binding, handle)
}

/// Remove one target from a poll instance.
pub(crate) unsafe fn destack_io_poll_deregister(
    binding: &BindingCallContext,
    handle: resource::PollHandle,
    target: resource::ResourceId,
) -> RuntimeResult<()> {
    core_io::poll_deregister(binding, handle, target)
}

/// Open a poll instance.
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
