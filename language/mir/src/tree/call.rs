use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{Block, DispatchSlot, Instruction, LocalNodeId, TypeId};

/// Stable identifier for one callsite inside a function body.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum CallSite {
    /// Callsite stored as an instruction.
    Instruction(LocalNodeId<Instruction>),
    /// Callsite stored as a block terminator.
    Terminator(LocalNodeId<Block>),
}

/// Dispatch kind for a call instruction or terminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum CallDispatchKind {
    /// Direct function call.
    Direct,
    /// Virtual call through an object dispatch slot.
    Virtual {
        /// The dispatch slot for the method.
        slot: DispatchSlot,
    },
    /// Dynamic call through an erased dispatch table slot.
    Dynamic {
        /// The dispatch slot for the method.
        slot: DispatchSlot,
    },
    /// Indirect call through a function pointer.
    Indirect,
}

/// Shared payload for one call-like instruction or terminator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Call<A> {
    /// The call arguments.
    pub arguments: A,
    /// The signature type for the callee.
    pub signature: TypeId,
}

impl<A> Call<A> {
    /// Create one call payload.
    pub fn new(arguments: A, signature: TypeId) -> Self {
        Self {
            arguments,
            signature,
        }
    }
}
