use std::sync::Arc;

use destack_vm as vm;
use serde::{Deserialize, Serialize};

/// Immutable execution image for one engine backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineImage {
    /// Immutable VM execution image.
    Vm(Arc<vm::snapshot::IsolateImage>),
}

impl PartialEq for EngineImage {
    fn eq(&self, other: &Self) -> bool {
        engine_image_bytes(self) == engine_image_bytes(other)
    }
}

impl Eq for EngineImage {}

/// Serialize one engine image for exact equality checks.
fn engine_image_bytes(image: &EngineImage) -> Vec<u8> {
    postcard::to_allocvec(image)
        .unwrap_or_else(|error| panic!("engine image should serialize: {error}"))
}
