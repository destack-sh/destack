use destack_heap as heap;
use serde::{Deserialize, Serialize};

use crate::runtime::engine::Entry;

use super::{Mutation, RuntimeId};

/// One world input recorded in authoritative trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum Input {
    /// One world tick input.
    Tick,
    /// One runtime removal input.
    RemoveRuntime {
        /// Runtime identifier to remove.
        runtime_id: RuntimeId,
    },
    /// One runtime entrypoint input.
    RunEntrypoint {
        /// Runtime identifier that owns the entrypoint execution.
        runtime_id: RuntimeId,
        /// Replayable entrypoint reference.
        entry: Entry,
        /// Invocation arguments.
        args: Vec<heap::Value>,
    },
    /// One world mutation input.
    Mutation(Mutation),
}

impl Input {
    /// Return the stable input name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Tick => "world.tick",
            Self::RemoveRuntime { .. } => "runtime.remove",
            Self::RunEntrypoint { .. } => "runtime.run_entrypoint",
            Self::Mutation(mutation) => mutation.name(),
        }
    }
}
