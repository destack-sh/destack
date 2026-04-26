use std::sync::Arc;

use {destack_native as native, destack_vm as vm};

/// Immutable execution image for one engine backend.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Image {
    /// Immutable VM execution image.
    Vm(Arc<vm::snapshot::IsolateImage>),
    /// Immutable native execution image.
    Native(native::Image),
}

impl PartialEq for Image {
    fn eq(&self, other: &Self) -> bool {
        image_bytes(self) == image_bytes(other)
    }
}

impl Eq for Image {}

/// Serialize one engine image for exact equality checks.
fn image_bytes(image: &Image) -> Vec<u8> {
    postcard::to_allocvec(image)
        .unwrap_or_else(|error| panic!("engine image should serialize: {error}"))
}
