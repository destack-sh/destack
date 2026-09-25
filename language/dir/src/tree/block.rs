use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{Expression, LocalNodeId, Node, NodeFold, NodeType};

/// The structural form of a block.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum BlockForm {
    /// Explicit blocks with { ... }
    Explicit,
    /// Explicit `do { ... }` expression blocks.
    Do,
    /// Implicit blocks like in file modules.
    Implicit,
}

/// How a block is interpreted.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub struct Block {
    /// The block context.
    pub context: BlockContext,
    /// The structural form of the block.
    pub form: BlockForm,
    /// The leading expressions whose values are discarded.
    pub leading_expressions: Vec<LocalNodeId<Expression>>,
    /// The optional tail expression whose value becomes the block value.
    pub tail_expression: Option<LocalNodeId<Expression>>,
}

impl Node for Block {
    const TYPE: NodeType = NodeType::Block;
}

impl Block {
    /// Return whether the block has explicit brace delimiters.
    pub const fn is_explicit(&self) -> bool {
        self.form.is_explicit()
    }

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

    /// Return the expression when this block contains exactly one.
    pub fn only_expression(&self) -> Option<LocalNodeId<Expression>> {
        if self.leading_expressions.is_empty() {
            return self.tail_expression;
        }

        if self.leading_expressions.len() == 1 && self.tail_expression.is_none() {
            return self.leading_expressions.first().copied();
        }

        None
    }

    /// Return the last expression in source order.
    pub fn last_expression(&self) -> Option<LocalNodeId<Expression>> {
        self.tail_expression
            .or_else(|| self.leading_expressions.last().copied())
    }

    /// Return the expression that produces this block's value.
    pub fn value_expression(&self) -> Option<LocalNodeId<Expression>> {
        match self.context {
            BlockContext::Expression => self.tail_expression,
            BlockContext::Statement => None,
        }
    }

    /// Iterate the block expressions in source order.
    pub fn iter_expressions(&self) -> impl Iterator<Item = LocalNodeId<Expression>> + '_ {
        self.leading_expressions
            .iter()
            .copied()
            .chain(self.tail_expression)
    }
}

impl BlockForm {
    /// Return whether this form has explicit brace delimiters.
    pub const fn is_explicit(self) -> bool {
        matches!(self, Self::Explicit | Self::Do)
    }
}
