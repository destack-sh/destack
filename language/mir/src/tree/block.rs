use destack_core::StringId;
use serde::{Deserialize, Serialize};

use crate::{Instruction, LocalNodeId, Node, NodeType, Parameter, Terminator};

/// A basic block is a sequence of instructions with:
/// - A single entry point (can have parameters for SSA)
/// - A single exit point (the terminator)
/// - No control flow within the block
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    /// Optional explicit block label.
    pub name: Option<StringId>,
    /// SSA parameters passed from predecessor blocks.
    /// Replaces traditional phi nodes with a cleaner model.
    pub parameters: Vec<Parameter>,
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
        parameters: Vec<Parameter>,
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
