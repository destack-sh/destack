use crate::diagnostic::RuntimeResult;
use crate::platform::diagnostic::PlatformError;
use crate::platform::os::PowerState;
use crate::platform::os::power::core::{OS_POWER_STATE_OPERATION, OS_POWER_SUSPEND_OPERATION};
use crate::runtime::BindingCallContext;

/// Report power-state reads as unsupported on parked Unix hosts.
pub(crate) fn read_power_state(_binding: &BindingCallContext) -> RuntimeResult<PowerState> {
    Err(PlatformError::not_supported(OS_POWER_STATE_OPERATION).into())
}

/// Report suspend requests as unsupported on parked Unix hosts.
pub(crate) fn request_suspend(_binding: &BindingCallContext) -> RuntimeResult<()> {
    Err(PlatformError::not_supported(OS_POWER_SUSPEND_OPERATION).into())
}
