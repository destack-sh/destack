use std::sync::Arc;

use serde::{Deserialize, Serialize};
use {destack_native as native, destack_vm as vm};

/// Immutable execution image for one engine backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Image {
    /// Immutable VM execution image.
    Vm(Arc<vm::IsolateImage>),
    /// Immutable native execution image.
    Native(native::Image),
}

impl PartialEq for Image {
    fn eq(&self, other: &Self) -> bool {
        let left = image_bytes(self);
        let right = image_bytes(other);

        left.is_ok() && left == right
    }
}

impl Eq for Image {}

/// Serialize one engine image for exact equality checks.
fn image_bytes(image: &Image) -> Result<Vec<u8>, postcard::Error> {
    postcard::to_allocvec(image)
}
