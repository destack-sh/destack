use destack_vm as vm;
use serde::{Deserialize, Serialize};

/// Durable execution snapshot for one engine backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineSnapshot {
    /// Durable VM snapshot payload.
    Vm(vm::snapshot::Snapshot),
}
