use destack_core::{Capture, CaptureMode};
use serde::{Deserialize, Serialize};

use crate::diagnostic::RuntimeError;

/// Worker-owned host module state.
#[derive(Debug, Default)]
pub(crate) struct HostState;

/// Materialized host module state image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct HostStateImage;

impl Capture for HostState {
    type Image = HostStateImage;
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = ();

    /// Capture one host-state image.
    fn capture_image(
        &mut self,
        _mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        Ok(HostStateImage)
    }

    /// Restore one host-state image.
    fn restore_image(
        &mut self,
        _image: &Self::Image,
        _context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl HostState {
    /// Fork this host state for one child worker.
    pub(crate) fn fork(&mut self) -> Result<Self, Box<RuntimeError>> {
        Ok(Self)
    }
}
