use destack_source::AdaptImage;
use serde::{Deserialize, Serialize};

use crate::{Expression, LocalNodeId, LocalScopeId, Node, NodeType};

/// How a block is defined.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum BlockFormat {
    /// Explicit blocks with { ... }
    Explicit,
    /// Implicit blocks like in file modules.
    Implicit,
}

/// How a block is interpreted.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, AdaptImage)]
pub enum BlockContext {
    /// The block is expression-position and may keep a value tail.
    Expression,
    /// The block is statement-position and does not keep a value tail.
    Statement,
}

/// A block of expressions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub struct Block {
    /// The block context.
    pub context: BlockContext,
    /// The format of the block.
    pub format: BlockFormat,
    /// The leading expressions whose values are discarded.
    pub leading_expressions: Vec<LocalNodeId<Expression>>,
    /// The optional tail expression whose value becomes the block value.
    pub tail_expression: Option<LocalNodeId<Expression>>,
    /// The scope of the block.
    pub scope: LocalScopeId,
}

impl Node for Block {
    const TYPE: NodeType = NodeType::Block;
}

impl Block {
    /// Return whether the block has no expressions at all.
    pub fn is_empty(&self) -> bool {
        self.leading_expressions.is_empty() && self.tail_expression.is_none()
    }

    /// Return the total number of expressions in the block.
    pub fn len(&self) -> usize {
        self.leading_expressions.len() + usize::from(self.tail_expression.is_some())
    }

    /// Return the first expression in source order.
    pub fn first_expression(&self) -> Option<LocalNodeId<Expression>> {
        self.leading_expressions
            .first()
            .copied()
            .or(self.tail_expression)
    }

    /// Return the last expression in source order.
    pub fn last_expression(&self) -> Option<LocalNodeId<Expression>> {
        self.tail_expression
            .or_else(|| self.leading_expressions.last().copied())
    }

    /// Return whether one expression belongs to this block.
    pub fn contains_expression(&self, expression_id: LocalNodeId<Expression>) -> bool {
        self.leading_expressions.contains(&expression_id)
            || self.tail_expression == Some(expression_id)
    }

    /// Iterate the block expressions in source order.
    pub fn iter_expressions(&self) -> impl Iterator<Item = LocalNodeId<Expression>> + '_ {
        self.leading_expressions
            .iter()
            .copied()
            .chain(self.tail_expression)
    }
}
