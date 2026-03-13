use serde::{Deserialize, Serialize};

use crate::runtime::bindings::BindingAffinity;

/// Stable identifier for one execution context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExecutionContextId(pub u64);

impl ExecutionContextId {
    /// Build one identifier from an opaque hash payload.
    pub const fn from_hash(hash: u64) -> Self {
        Self(hash)
    }
}

/// Runtime execution context for one binding call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Stable execution context identifier.
    pub id: ExecutionContextId,
    /// Whether the current execution context is the process main context.
    pub is_process_main: bool,
}

impl ExecutionContext {
    /// Build one execution context payload.
    pub const fn new(id: ExecutionContextId, is_process_main: bool) -> Self {
        Self {
            id,
            is_process_main,
        }
    }
}

/// Return the stable metadata name for one binding-affinity class.
pub const fn binding_affinity_name(affinity: BindingAffinity) -> &'static str {
    match affinity {
        BindingAffinity::Any => "any",
        BindingAffinity::EventLoop => "eventLoop",
        BindingAffinity::Owner => "owner",
        BindingAffinity::ProcessMain => "processMain",
    }
}
