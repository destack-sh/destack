use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use destack_core::StringId;

use crate::{Block, Function, Instruction, LocalNodeId};

/// Source of profile data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProfileSource {
    /// Exact counts from instrumentation.
    Instrumentation,
    /// Sampled counts with a known sampling period.
    Sampled { period: u64 },
    /// Synthetic data produced by the compiler.
    Synthetic,
}

/// Confidence level for a profile count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ProfileConfidence {
    /// Count is exact.
    #[default]
    Precise,
    /// Count is estimated (sampled or inferred).
    Estimated,
    /// Count is synthetic or heuristic.
    Synthetic,
}

/// A profiling count with confidence metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ProfileCount {
    /// Raw execution count.
    pub value: u64,
    /// Confidence in the count.
    pub confidence: ProfileConfidence,
}

impl ProfileCount {
    /// Create a new profile count.
    pub fn new(value: u64, confidence: ProfileConfidence) -> Self {
        Self { value, confidence }
    }

    /// True when the count is zero.
    pub fn is_zero(self) -> bool {
        self.value == 0
    }
}

/// Profile data for a function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionProfile {
    /// Entry count for the function.
    pub entry_count: ProfileCount,
}

/// Profile data for a basic block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockProfile {
    /// Execution count for the block.
    pub execution_count: ProfileCount,
}

/// Edge kind for control flow profile data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeKind {
    /// Unconditional jump.
    Jump,
    /// Continuation of a call terminator.
    Call,
    /// Branch to the then target.
    BranchThen,
    /// Branch to the else target.
    BranchElse,
    /// Check success edge.
    CheckSuccess,
    /// Check failure edge.
    CheckFailure,
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

/// Profile data for a control flow edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeProfile {
    /// Execution count for the edge.
    pub count: ProfileCount,
}

/// Profile data for a callsite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallSiteProfile {
    /// Total executions at this callsite.
    pub total_count: ProfileCount,
    /// Known target distribution for indirect calls.
    pub targets: Vec<CallTargetProfile>,
    /// Count attributed to unknown targets.
    pub unknown_count: ProfileCount,
}

/// Profile data for a call target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallTargetProfile {
    /// Target identifier.
    pub target: CallTarget,
    /// Execution count for the target.
    pub count: ProfileCount,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileTable {
    /// The profile data source.
    pub source: ProfileSource,
    /// Per-function entry counts.
    pub functions: HashMap<LocalNodeId<Function>, FunctionProfile>,
    /// Per-block execution counts.
    pub blocks: HashMap<LocalNodeId<Block>, BlockProfile>,
    /// Per-edge execution counts.
    pub edges: HashMap<EdgeKey, EdgeProfile>,
    /// Per-callsite profiles.
    pub callsites: HashMap<LocalNodeId<Instruction>, CallSiteProfile>,
}

impl ProfileTable {
    /// Create an empty profile table.
    pub fn new(source: ProfileSource) -> Self {
        Self {
            source,
            functions: HashMap::new(),
            blocks: HashMap::new(),
            edges: HashMap::new(),
            callsites: HashMap::new(),
        }
    }

    /// Return true when no profile data is present.
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
            && self.blocks.is_empty()
            && self.edges.is_empty()
            && self.callsites.is_empty()
    }

    /// Look up a function profile.
    pub fn function_profile(&self, function: LocalNodeId<Function>) -> Option<&FunctionProfile> {
        self.functions.get(&function)
    }

    /// Look up a block profile.
    pub fn block_profile(&self, block: LocalNodeId<Block>) -> Option<&BlockProfile> {
        self.blocks.get(&block)
    }

    /// Look up an edge profile.
    pub fn edge_profile(&self, edge: &EdgeKey) -> Option<&EdgeProfile> {
        self.edges.get(edge)
    }

    /// Look up a callsite profile.
    pub fn callsite_profile(&self, callsite: LocalNodeId<Instruction>) -> Option<&CallSiteProfile> {
        self.callsites.get(&callsite)
    }

    /// Look up the count for an edge.
    pub fn edge_count(&self, edge: &EdgeKey) -> Option<ProfileCount> {
        self.edges.get(edge).map(|profile| profile.count)
    }
}

impl Default for ProfileTable {
    fn default() -> Self {
        Self::new(ProfileSource::Instrumentation)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Block, EdgeKey, EdgeKind, EdgeProfile, LocalNodeId, ProfileConfidence, ProfileCount,
        ProfileSource, ProfileTable,
    };

    /// An empty profile table reports no data.
    #[test]
    fn test_profile_table_empty() {
        // create empty profile table
        let table = ProfileTable::new(ProfileSource::Synthetic);

        // verify no data is present
        assert!(table.is_empty());
    }

    /// Edge count lookup returns the stored count.
    #[test]
    fn test_profile_table_edge_lookup() {
        // set up a profile table with a single edge
        let mut table = ProfileTable::new(ProfileSource::Instrumentation);
        let source = LocalNodeId::<Block>::new(1);
        let target = LocalNodeId::<Block>::new(2);
        let edge = EdgeKey::new(source, EdgeKind::BranchThen, target);

        // record edge count
        table.edges.insert(
            edge,
            EdgeProfile {
                count: ProfileCount::new(42, ProfileConfidence::Precise),
            },
        );

        // look up edge count
        let count = table.edge_count(&edge).expect("missing edge count");

        // confirm the stored count is returned
        assert_eq!(count.value, 42);
    }
}
