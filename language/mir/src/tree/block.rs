use destack_core::StringId;
use serde::{Deserialize, Serialize};

use crate::{BlockParameter, Instruction, LocalNodeId, Node, NodeType, Terminator};

/// A basic block is a sequence of instructions with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    /// Optional explicit block label.
    pub name: Option<StringId>,
    /// SSA parameters passed from predecessor blocks.
    pub parameters: Vec<BlockParameter>,
    /// Instructions in execution order.
    pub instructions: Vec<LocalNodeId<Instruction>>,
    /// How control flow leaves this block.
    pub terminator: LocalNodeId<Terminator>,
}

impl Node for Block {
    const TYPE: NodeType = NodeType::Block;
}

impl Block {
    /// Create a new empty block.
    pub fn new(terminator: LocalNodeId<Terminator>) -> Self {
        Self {
            name: None,
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminator,
        }
    }

    /// Create a block with parameters.
    pub fn with_parameters(
        parameters: Vec<BlockParameter>,
        terminator: LocalNodeId<Terminator>,
    ) -> Self {
        Self {
            name: None,
            parameters,
            instructions: Vec::new(),
            terminator,
        }
    }
}
