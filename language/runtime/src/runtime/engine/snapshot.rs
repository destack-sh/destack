use std::sync::Arc;

use destack_vm as vm;
use serde::{Deserialize, Serialize};

/// Immutable execution image for one engine backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineImage {
    /// Immutable VM execution image.
    Vm(Arc<vm::snapshot::IsolateImage>),
}
