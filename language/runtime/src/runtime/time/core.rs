use crate::diagnostic::RuntimeError;
use crate::platform::PlatformError;
use crate::runtime::{BindingCallContext, RuntimeHookState};
use destack_workspace::TimeMode;

/// Return one invalid-pointer error.
pub(crate) fn null_pointer_error(field: &str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::null_pointer(field)).boxed()
}

/// Return one unsupported runtime host time operation error.
#[cfg(all(unix, not(any(target_os = "linux", target_os = "android"))))]
pub(crate) fn unsupported_host_operation_error(operation: &str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(operation)).boxed()
}

/// Return true when the runtime clock is virtualized.
pub(crate) fn is_virtual_clock(context: &BindingCallContext) -> bool {
    context.runtime().time.mode() == TimeMode::Virtual
}

/// Return one runtime-backed wall clock sample.
pub(crate) fn runtime_wall_nanos(context: &BindingCallContext) -> u64 {
    context
        .hooks()
        .on_time_read(RuntimeHookState::from_engine(Some(context.engine())));
    context.runtime().time.wall_nanos()
}

/// Return one runtime-backed monotonic clock sample.
pub(crate) fn runtime_mono_nanos(context: &BindingCallContext) -> u64 {
    context
        .hooks()
        .on_time_read(RuntimeHookState::from_engine(Some(context.engine())));
    context.runtime().time.mono_nanos()
}

/// Sleep one runtime-backed duration.
pub(crate) fn runtime_sleep_nanos(context: &BindingCallContext, duration: u64) {
    context.runtime().time.sleep_nanos(duration);
}

/// Sleep until one runtime-backed wall deadline.
pub(crate) fn runtime_sleep_until_wall_nanos(context: &BindingCallContext, deadline: u64) {
    context.runtime().time.sleep_until_nanos(deadline);
}

/// Sleep until one runtime-backed monotonic deadline.
pub(crate) fn runtime_sleep_until_mono_nanos(context: &BindingCallContext, deadline: u64) {
    let now = runtime_mono_nanos(context);
    if deadline <= now {
        return;
    }

    let delta = deadline.saturating_sub(now);
    runtime_sleep_nanos(context, delta);
}
