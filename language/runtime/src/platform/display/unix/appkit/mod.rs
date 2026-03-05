mod event;
mod monitor;
mod window;

use crate::platform::display::DisplayBackendCapabilityFlags;
use crate::runtime::BindingCallContext;

pub(super) use event::*;
pub(super) use monitor::*;
pub(super) use window::*;

/// Return backend descriptor availability and capability flags for appkit.
pub(crate) fn backend_descriptor_state(
    context: &BindingCallContext,
) -> (bool, DisplayBackendCapabilityFlags) {
    let _ = context;
    (false, DisplayBackendCapabilityFlags(0))
}
