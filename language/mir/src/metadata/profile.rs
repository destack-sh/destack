use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use destack_core::StringId;

use crate::{Block, CallSite, Function, LocalNodeId};

/// Execution count from profile data.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Count(
    /// The raw count value.
    pub u64,
);

impl Count {
    /// Create one execution count.
    #[inline]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw count value.
    #[inline]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Successor selected by one MIR terminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    SwitchCase { value: i128 },
    /// The default target of a switch terminator.
    SwitchDefault,
    /// The resume target of a yield terminator.
    YieldResume,
    /// The unwind target of a yield terminator.
    YieldUnwind,
}

/// Control flow edge selected by a terminator successor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Edge {
    /// The source block.
    pub source: LocalNodeId<Block>,
    /// The successor field selected from the source terminator.
    pub successor: Successor,
    /// The target block.
    pub target: LocalNodeId<Block>,
}

impl Edge {
    /// Create one control flow edge.
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

/// Profile data for a callsite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallSiteProfile {
    /// Total executions at this callsite.
    pub total_count: Count,
    /// Known target distribution for indirect calls.
    pub targets: Vec<CallTargetProfile>,
    /// Count attributed to unknown targets.
    pub unknown_count: Count,
}

/// Profile data for a call target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallTargetProfile {
    /// Target identifier.
    pub target: CallTarget,
    /// Execution count for the target.
    pub count: Count,
}

/// Target identifier for indirect call profiles.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CallTarget {
    /// A known MIR function.
    Function(LocalNodeId<Function>),
    /// A symbol not present in this MIR module.
    Symbol(StringId),
}

/// Identifier for one emitted profile counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProfileCounterId(
    /// The zero-based profile counter index.
    pub u32,
);

/// Profile-guided optimization data for a MIR module.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    /// Per-function entry counts.
    pub functions: HashMap<LocalNodeId<Function>, Count>,
    /// Per-block execution counts.
    pub blocks: HashMap<LocalNodeId<Block>, Count>,
    /// Per-edge execution counts.
    pub edges: HashMap<Edge, Count>,
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
    pub fn function_count(&self, function: LocalNodeId<Function>) -> Option<Count> {
        self.functions.get(&function).copied()
    }

    /// Look up a block execution count.
    pub fn block_count(&self, block: LocalNodeId<Block>) -> Option<Count> {
        self.blocks.get(&block).copied()
    }

    /// Look up an edge execution count.
    pub fn edge_count(&self, edge: &Edge) -> Option<Count> {
        self.edges.get(edge).copied()
    }

    /// Look up a callsite profile.
    pub fn callsite_profile(&self, callsite: CallSite) -> Option<&CallSiteProfile> {
        self.callsites.get(&callsite)
    }
}
