use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{Block, LocalNodeId};

/// Successor selected by one MIR terminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Successor {
    /// The target of an unconditional jump.
    Jump,
    /// The return continuation of a call terminator.
    CallReturn,
    /// The unwind continuation of a call terminator.
    CallUnwind,
    /// The then target of a branch terminator.
    BranchThen,
    /// The else target of a branch terminator.
    BranchElse,
    /// The success target of a check terminator.
    CheckSuccess,
    /// The failure target of a check terminator.
    CheckFailure,
    /// The success target of a fallible terminator.
    TrySuccess,
    /// The failure target of a fallible terminator.
    TryFailure,
    /// One switch case target.
    SwitchCase {
        /// The matched case value.
        value: i128,
    },
    /// The default target of a switch terminator.
    SwitchDefault,
    /// The resume target of a yield terminator.
    YieldResume,
    /// The unwind target of a yield terminator.
    YieldUnwind,
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
