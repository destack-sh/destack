use crate::diagnostic::RuntimeResult;
use crate::platform::os::PowerState;
use crate::runtime::BindingCallContext;

use super::core::{OS_POWER_STATE_OPERATION, not_supported};

/// Read one host power-state value from unsupported backends.
pub(super) fn read_power_state(_context: &BindingCallContext) -> RuntimeResult<PowerState> {
    Err(not_supported(OS_POWER_STATE_OPERATION))
}
