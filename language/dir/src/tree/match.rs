use crate::{Block, Expression, Node, NodeId, NodeType, Pattern, ScopeId};

/// A MatchSource is where the match was lowered from.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MatchSource {
    /// Match expression (regular match with cases).
    Match,
    /// Explicit try expression or block (`try { ... }` with optional catch).
    Try,
    /// Maybe unary expression (postfix `?`).
    Maybe,
    /// Must unary expression (postfix `!`).
    Must,
}

/// A MatchCase is a match case inside a Match expression.
/// MatchCases can be any Pattern and can have an optional `if` guard.
#[derive(Debug, Clone, PartialEq)]
pub enum MatchCase {
    /// A match case with an expression body.
    Expression {
        pattern: NodeId<Pattern>,
        body: NodeId<Expression>,
        guard: Option<NodeId<Expression>>,
        scope: ScopeId,
    },
    /// A match case with a block body.
    Block {
        pattern: NodeId<Pattern>,
        body: NodeId<Block>,
        guard: Option<NodeId<Expression>>,
        scope: ScopeId,
    },
}

impl Node for MatchCase {
    const TYPE: NodeType = NodeType::MatchCase;

    fn is_resolved(&self) -> bool {
        true
    }
}
