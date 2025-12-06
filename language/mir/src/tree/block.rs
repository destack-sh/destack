use smallvec::{SmallVec, smallvec};

use crate::{Instruction, LocalNodeId, Node, NodeType, TypedValue, Value};

/// A basic block is a sequence of instructions with:
/// - A single entry point (can have parameters for SSA)
/// - A single exit point (the terminator)
/// - No control flow within the block
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// SSA parameters passed from predecessor blocks.
    /// Replaces traditional phi nodes with a cleaner model.
    pub parameters: Vec<TypedValue>,
    /// Instructions in execution order.
    pub instructions: Vec<LocalNodeId<Instruction>>,
    /// How control flow leaves this block.
    pub terminator: Terminator,
}

impl Node for Block {
    const TYPE: NodeType = NodeType::Block;
}

impl Block {
    /// Create a new empty block.
    pub fn new() -> Self {
        Self {
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminator: Terminator::Unreachable,
        }
    }

    /// Create a block with parameters.
    pub fn with_parameters(parameters: Vec<TypedValue>) -> Self {
        Self {
            parameters,
            instructions: Vec::new(),
            terminator: Terminator::Unreachable,
        }
    }
}

impl Default for Block {
    fn default() -> Self {
        Self::new()
    }
}

/// Block terminator - how control flow leaves a block.
#[derive(Debug, Clone, PartialEq)]
pub enum Terminator {
    /// Return from the function.
    Return {
        /// The value to return, or None for void functions.
        value: Option<Value>,
    },

    /// Unconditional jump to another block.
    Jump {
        /// The block to jump to.
        target: LocalNodeId<Block>,
        /// Arguments to pass to the target block's parameters.
        arguments: Vec<Value>,
    },

    /// Conditional branch (if-then-else).
    Branch {
        /// The boolean condition to test.
        condition: Value,
        /// The block to jump to if condition is true.
        then_target: LocalNodeId<Block>,
        /// Arguments for the then block's parameters.
        then_arguments: Vec<Value>,
        /// The block to jump to if condition is false.
        else_target: LocalNodeId<Block>,
        /// Arguments for the else block's parameters.
        else_arguments: Vec<Value>,
    },

    /// Switch on an integer value (multi-way branch).
    Switch {
        /// The integer value to switch on.
        value: Value,
        /// The block to jump to if no case matches.
        default: LocalNodeId<Block>,
        /// Arguments for the default block's parameters.
        default_arguments: Vec<Value>,
        /// The cases to match against.
        cases: Vec<SwitchCase>,
    },

    /// Unreachable code (triggers undefined behavior if executed).
    Unreachable,
}

impl Terminator {
    /// Get all successor block ids.
    pub fn successors(&self) -> SmallVec<[LocalNodeId<Block>; 2]> {
        match self {
            Terminator::Return { .. } => smallvec![],
            Terminator::Jump { target, .. } => smallvec![*target],
            Terminator::Branch {
                then_target,
                else_target,
                ..
            } => smallvec![*then_target, *else_target],
            Terminator::Switch { default, cases, .. } => {
                let mut successors = smallvec![*default];
                successors.extend(cases.iter().map(|c| c.target));
                successors
            }
            Terminator::Unreachable => smallvec![],
        }
    }

    /// Get all values used by this terminator.
    pub fn uses(&self) -> SmallVec<[Value; 4]> {
        match self {
            Terminator::Return { value } => value.iter().copied().collect(),
            Terminator::Jump { arguments, .. } => arguments.iter().copied().collect(),
            Terminator::Branch {
                condition,
                then_arguments,
                else_arguments,
                ..
            } => {
                let mut uses = smallvec![*condition];
                uses.extend(then_arguments.iter().copied());
                uses.extend(else_arguments.iter().copied());
                uses
            }
            Terminator::Switch {
                value,
                default_arguments,
                cases,
                ..
            } => {
                let mut uses = smallvec![*value];
                uses.extend(default_arguments.iter().copied());
                for case in cases {
                    uses.extend(case.arguments.iter().copied());
                }
                uses
            }
            Terminator::Unreachable => smallvec![],
        }
    }
}

/// A case in a switch terminator.
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    /// The integer constant to match against.
    pub value: i64,
    /// The block to jump to if this case matches.
    pub target: LocalNodeId<Block>,
    /// Arguments for the target block's parameters.
    pub arguments: Vec<Value>,
}
