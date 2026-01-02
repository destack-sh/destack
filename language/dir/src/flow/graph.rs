use indexmap::IndexMap;

use crate::{Expression, LocalNodeId, LocalNodeIdAny};

/// Identify a flow graph block.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FlowBlockId(
    /// Store the block index.
    pub u32,
);

/// Describe a control flow graph for a single expression body.
#[derive(Debug, Clone)]
pub struct FlowGraph {
    /// Identify the entry block for the graph.
    pub entry_block: FlowBlockId,
    /// Identify the exit block for the graph.
    pub exit_block: FlowBlockId,
    /// Store all blocks in the graph.
    pub blocks: Vec<FlowBlock>,
    /// Map nodes to their containing block.
    pub block_by_node: IndexMap<LocalNodeIdAny, FlowBlockId>,
}

/// Represent a single basic block in the control flow graph.
#[derive(Debug, Clone)]
pub struct FlowBlock {
    /// Identify the block.
    pub id: FlowBlockId,
    /// Store the nodes in the block.
    pub nodes: Vec<LocalNodeIdAny>,
    /// Store incoming edges for the block.
    pub predecessors: Vec<FlowEdge>,
    /// Store outgoing edges for the block.
    pub successors: Vec<FlowEdge>,
    /// Mark the block as terminal when it ends control flow.
    pub is_terminal: bool,
}

/// Represent an edge between blocks.
#[derive(Debug, Clone)]
pub struct FlowEdge {
    /// Identify the edge target block.
    pub target: FlowBlockId,
    /// Describe the edge kind.
    pub kind: FlowEdgeKind,
    /// Store an optional guard expression for the edge.
    pub guard: Option<LocalNodeId<Expression>>,
}

/// Describe the kind of control flow edge between blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowEdgeKind {
    /// Unconditional branch.
    Unconditional,
    /// True branch from a guard.
    True,
    /// False branch from a guard.
    False,
    /// Match / switch case branch.
    Case,
    /// Guard branch.
    Guard,
}
