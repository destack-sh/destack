use super::core::require_out;
use crate::diagnostic::RuntimeResult;
use crate::platform::io::{EventToken, core as io_core};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

/// Attach an event token to a poll target key.
pub(crate) unsafe fn destack_io_event_attach(
    binding: &BindingCallContext,
    token: EventToken,
    target: resource::ResourceId,
    key: u64,
) -> RuntimeResult<()> {
    io_core::event_attach(binding, token, target, key)
}

/// Close a user-event token.
pub(crate) unsafe fn destack_io_event_close(
    binding: &BindingCallContext,
    token: EventToken,
) -> RuntimeResult<()> {
    io_core::event_close(binding, token)
}

/// Create a user-event token.
pub(crate) unsafe fn destack_io_event_open(
    binding: &BindingCallContext,
    out: *mut EventToken,
    initial: u64,
) -> RuntimeResult<()> {
    require_out(out)?;
    let value = io_core::event_open(binding, initial)?;

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Signal a user-event token.
pub(crate) unsafe fn destack_io_event_signal(
    binding: &BindingCallContext,
    token: EventToken,
    argument_value: u64,
) -> RuntimeResult<()> {
    io_core::event_signal(binding, token, argument_value)
}
