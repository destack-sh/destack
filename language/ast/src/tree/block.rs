use serde::{Deserialize, Serialize};

use crate::{Expression, LocalNodeId, Node, NodeType};

/// How a block is defined.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockFormat {
    /// Explicit blocks with { ... }
    Explicit,
    /// Implicit blocks like in file modules.
    Implicit,
}

/// How a block is interpreted.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockContext {
    /// The block is expression-position and may keep a value tail.
    Expression,
    /// The block is statement-position and does not keep a value tail.
    Statement,
}

/// A Block is a block of statements.
///
/// Examples:
/// ```
/// {
///     x = 1
///     y = 2
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    /// The block context.
    pub context: BlockContext,
    /// The format of the block.
    pub format: BlockFormat,
    /// The expressions in the block.
    pub expressions: Vec<LocalNodeId<Expression>>,
}

impl Node for Block {
    const TYPE: NodeType = NodeType::Block;
}
