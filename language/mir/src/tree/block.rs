use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{BlockParameter, Instruction, LocalNodeId, Node, NodeType, Terminator};

/// A basic block is a sequence of instructions terminated by one control transfer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Block {
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
            parameters,
            instructions: Vec::new(),
            terminator,
        }
    }
}
