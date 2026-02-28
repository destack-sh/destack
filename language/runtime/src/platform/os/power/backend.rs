use crate::diagnostic::RuntimeResult;
use crate::platform::os::PowerState;
use crate::runtime::BindingCallContext;

/// Read one host power-state value from the active backend.
pub(super) fn read_power_state(context: &BindingCallContext) -> RuntimeResult<PowerState> {
    // dispatch to the platform backend
    platform_backend::read_power_state(context)
}

#[cfg(unix)]
use super::unix as platform_backend;
#[cfg(not(any(unix, windows)))]
use super::unsupported as platform_backend;
#[cfg(windows)]
use super::windows as platform_backend;
