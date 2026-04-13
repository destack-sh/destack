use serde::{Deserialize, Serialize};

use crate::{Expression, LocalNodeId, Node, NodeType};

/// The position of one decorator relative to its owner.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum DecoratorPosition {
    /// Decorator inside the node without a following owner slot.
    BlockInfix,
    /// Decorator preceding the node on previous lines.
    BlockPrefix,
    /// Decorator after the node on a following line.
    BlockPostfix,
    /// Decorator before the node on the same line.
    LinePrefix,
    /// Decorator after the node on the same line.
    LinePostfix,
    /// Decorators after the node on the same line with nothing after them.
    LinePostfixBoundary,
}

/// A decorator attached to a syntax node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Decorator {
    /// The decorator expression.
    pub expression: LocalNodeId<Expression>,
    /// The decorator position relative to its owner.
    pub position: DecoratorPosition,
}

impl Node for Decorator {
    const TYPE: NodeType = NodeType::Decorator;
}
