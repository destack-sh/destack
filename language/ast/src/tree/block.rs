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
    pub format: BlockFormat,
    pub expressions: Vec<LocalNodeId<Expression>>,
}

impl Node for Block {
    const TYPE: NodeType = NodeType::Block;
}
