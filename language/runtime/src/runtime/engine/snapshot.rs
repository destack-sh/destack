use std::sync::Arc;

use destack_vm as vm;
use serde::{Deserialize, Serialize};

/// Immutable execution image for one engine backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineImage {
    /// Immutable VM execution image.
    Vm(Arc<vm::snapshot::IsolateImage>),
}

/// Serialized execution snapshot for one engine backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineSnapshot {
    /// Serialized VM execution snapshot.
    Vm(vm::snapshot::IsolateSnapshot),
}
