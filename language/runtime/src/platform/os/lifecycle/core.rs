use crate::diagnostic::RuntimeResult;
use crate::platform::os::{LifecycleEventValue, LifecycleState, state};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

/// Read the current lifecycle state from runtime-owned OS state.
pub(crate) fn state(binding: &BindingCallContext) -> RuntimeResult<LifecycleState> {
    state::lifecycle_state(binding)
}

/// Open one lifecycle event stream.
pub(crate) fn open(binding: &BindingCallContext) -> RuntimeResult<resource::LifecycleEventHandle> {
    state::lifecycle_open(binding)
}

/// Close one lifecycle event stream.
pub(crate) fn close(
    binding: &BindingCallContext,
    handle: resource::LifecycleEventHandle,
) -> RuntimeResult<()> {
    state::lifecycle_close(binding, handle)
}

/// Wait for one lifecycle event.
pub(crate) fn read(
    binding: &BindingCallContext,
    handle: resource::LifecycleEventHandle,
    timeout_ns: u64,
) -> RuntimeResult<LifecycleEventValue> {
    state::lifecycle_read(binding, handle, timeout_ns)
}

/// Poll one lifecycle event without blocking.
pub(crate) fn try_read(
    binding: &BindingCallContext,
    handle: resource::LifecycleEventHandle,
) -> RuntimeResult<LifecycleEventValue> {
    state::lifecycle_try_read(binding, handle)
}
