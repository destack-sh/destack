#![allow(unused_variables)]

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::os::PowerState;
use crate::runtime::BindingCallContext;

use crate::platform::os::power::core::{OS_POWER_STATE_OPERATION, OS_POWER_SUSPEND_OPERATION};

/// Read one host power-state value from unsupported backends.
pub(super) fn read_power_state(binding: &BindingCallContext) -> RuntimeResult<PowerState> {
    Err(core_platform::not_supported(OS_POWER_STATE_OPERATION))
}

/// Request one host suspend transition on unsupported backends.
pub(super) fn request_suspend(_binding: &BindingCallContext) -> RuntimeResult<()> {
    Err(core_platform::not_supported(OS_POWER_SUSPEND_OPERATION))
}
