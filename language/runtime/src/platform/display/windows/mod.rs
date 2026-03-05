mod backend;
mod core;
mod event;
mod monitor;
mod win32;
mod window;

use crate::platform::display::DisplayBackendDescriptor;
use crate::runtime::BindingCallContext;

pub(crate) use event::*;
pub(crate) use monitor::*;
pub(crate) use win32::{DisplayEventRuntimeState, WindowRuntimeState};
pub(crate) use window::*;

/// List windows display backend descriptors for the active host.
pub(super) fn display_backend_descriptors(
    binding: &BindingCallContext,
) -> Vec<DisplayBackendDescriptor> {
    core::backend_descriptors(binding)
}
