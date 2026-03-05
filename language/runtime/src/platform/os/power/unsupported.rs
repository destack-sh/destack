use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::os::PowerState;
use crate::runtime::BindingCallContext;

use super::core::OS_POWER_STATE_OPERATION;

/// Read one host power-state value from unsupported backends.
pub(super) fn read_power_state(binding: &BindingCallContext) -> RuntimeResult<PowerState> {
    Err(core_platform::not_supported(OS_POWER_STATE_OPERATION))
}
