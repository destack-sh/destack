use destack_mir::{Block, Function, Instruction, LocalNodeId};
use destack_program::FunctionId;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Anchor for MIR-level error locations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum DiagnosticAnchor {
    /// No specific location.
    None,
    /// Specific function.
    Function(LocalNodeId<Function>),
    /// Specific block within a function.
    Block {
        /// The function containing the block.
        function: LocalNodeId<Function>,
        /// The anchored block.
        block: LocalNodeId<Block>,
    },
    /// Specific instruction within a block.
    Instruction {
        /// The function containing the instruction.
        function: LocalNodeId<Function>,
        /// The block containing the instruction.
        block: LocalNodeId<Block>,
        /// The anchored instruction.
        instruction: LocalNodeId<Instruction>,
    },
}

/// One frame in a diagnostic call stack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct StackTraceFrame {
    /// The function being executed.
    pub function: FunctionId,
    /// The executable block index being executed.
    pub block: u32,
    /// Function name, if available.
    pub function_name: Option<String>,
}
