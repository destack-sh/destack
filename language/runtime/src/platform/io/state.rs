use std::collections::HashMap;

use destack_core::{Capture, CaptureMode};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::diagnostic::RuntimeError;
use crate::platform::ResourceId;

/// Event attachment key for one token.
type EventAttachmentKey = ResourceId;
/// Event attachment targets keyed by poll resource id.
type EventAttachmentTargets = HashMap<ResourceId, u64>;
/// Event attachment registry keyed by token.
type EventAttachmentRegistry = HashMap<EventAttachmentKey, EventAttachmentTargets>;

/// Worker-owned I/O module state.
#[derive(Debug, Default)]
pub(crate) struct PlatformIoState {
    /// Attachment routes from event tokens into poll targets.
    event_attachments: Mutex<EventAttachmentRegistry>,
}

impl PlatformIoState {
    /// Return the mutable event-attachment registry.
    pub(crate) fn event_attachments(&self) -> &Mutex<EventAttachmentRegistry> {
        &self.event_attachments
    }
}

/// Materialized I/O platform-state image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PlatformIoImage {
    /// Event attachment routes from event tokens into poll targets.
    event_attachments: EventAttachmentRegistry,
}

impl Capture for PlatformIoState {
    type Image = PlatformIoImage;
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = ();

    /// Capture one I/O platform-state image.
    fn capture_image(
        &mut self,
        _mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        Ok(PlatformIoImage {
            event_attachments: self.event_attachments.lock().clone(),
        })
    }

    /// Restore one I/O platform-state image.
    fn restore_image(
        &mut self,
        image: &Self::Image,
        _context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        *self.event_attachments.lock() = image.event_attachments.clone();

        Ok(())
    }
}
