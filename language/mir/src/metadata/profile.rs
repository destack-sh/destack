use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use destack_core::StringId;

use crate::{Block, CallSite, Function, LocalNodeId};

/// Edge kind for control flow profile data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeKind {
    /// Unconditional jump.
    Jump,
    /// Continuation of a call terminator.
    Call,
    /// Unwind edge from a panicking call terminator.
    CallUnwind,
    /// Branch to the then target.
    BranchThen,
    /// Branch to the else target.
    BranchElse,
    /// Check success edge.
    CheckSuccess,
    /// Check failure edge.
    CheckFailure,
    /// Fallible allocation success edge.
    AllocationSuccess,
    /// Fallible allocation failure edge.
    AllocationFailure,
    /// Switch case edge.
    SwitchCase { value: i128 },
    /// Switch default edge.
    SwitchDefault,
    /// Coroutine resume edge.
    YieldResume,
}

/// Key identifying a control flow edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeKey {
    /// The source block.
    pub source: LocalNodeId<Block>,
    /// The edge kind.
    pub kind: EdgeKind,
    /// The target block.
    pub target: LocalNodeId<Block>,
}

impl EdgeKey {
    /// Create a new edge key.
    pub fn new(source: LocalNodeId<Block>, kind: EdgeKind, target: LocalNodeId<Block>) -> Self {
        Self {
            source,
            kind,
            target,
        }
    }
}

/// Profile data for a callsite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallSiteProfile {
    /// Total executions at this callsite.
    pub total_count: u64,
    /// Known target distribution for indirect calls.
    pub targets: Vec<CallTargetProfile>,
    /// Count attributed to unknown targets.
    pub unknown_count: u64,
}

/// Profile data for a call target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallTargetProfile {
    /// Target identifier.
    pub target: CallTarget,
    /// Execution count for the target.
    pub count: u64,
}

/// Target identifier for indirect call profiles.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CallTarget {
    /// A known MIR function.
    Function(LocalNodeId<Function>),
    /// A symbol not present in this MIR module.
    Symbol(StringId),
}

/// Profile-guided optimization data for a MIR module.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    /// Per-function entry counts.
    pub functions: HashMap<LocalNodeId<Function>, u64>,
    /// Per-block execution counts.
    pub blocks: HashMap<LocalNodeId<Block>, u64>,
    /// Per-edge execution counts.
    pub edges: HashMap<EdgeKey, u64>,
    /// Per-callsite profiles.
    pub callsites: HashMap<CallSite, CallSiteProfile>,
}

impl Profile {
    /// Create an empty profile.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return true when no profile data is present.
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
            && self.blocks.is_empty()
            && self.edges.is_empty()
            && self.callsites.is_empty()
    }

    /// Look up a function entry count.
    pub fn function_count(&self, function: LocalNodeId<Function>) -> Option<u64> {
        self.functions.get(&function).copied()
    }

    /// Look up a block execution count.
    pub fn block_count(&self, block: LocalNodeId<Block>) -> Option<u64> {
        self.blocks.get(&block).copied()
    }

    /// Look up an edge execution count.
    pub fn edge_count(&self, edge: &EdgeKey) -> Option<u64> {
        self.edges.get(edge).copied()
    }

    /// Look up a callsite profile.
    pub fn callsite_profile(&self, callsite: CallSite) -> Option<&CallSiteProfile> {
        self.callsites.get(&callsite)
    }
}
