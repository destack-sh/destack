use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{Block, LocalNodeId};

/// Successor selected by one MIR terminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Successor {
    /// The target of an unconditional jump.
    Jump,
    /// The then target of a branch terminator.
    BranchThen,
    /// The else target of a branch terminator.
    BranchElse,
    /// The success target of a check terminator.
    CheckSuccess,
    /// The failure target of a check terminator.
    CheckFailure,
    /// One switch case target.
    SwitchCase {
        /// The matched case value.
        value: i128,
    },
    /// The default target of a switch terminator.
    SwitchDefault,
    /// The normal continuation of an invoke.
    InvokeNormal,
    /// The unwind continuation of an invoke.
    InvokeUnwind,
    /// The success target of a fallible allocation.
    NewSuccess,
    /// The failure target of a fallible allocation.
    NewFailure,
}

/// Control-flow edge selected by a terminator successor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Edge {
    /// The source block.
    pub source: LocalNodeId<Block>,
    /// The successor field selected from the source terminator.
    pub successor: Successor,
    /// The target block.
    pub target: LocalNodeId<Block>,
}

impl Edge {
    /// Create one control-flow edge.
    pub fn new(
        source: LocalNodeId<Block>,
        successor: Successor,
        target: LocalNodeId<Block>,
    ) -> Self {
        Self {
            source,
            successor,
            target,
        }
    }
}
