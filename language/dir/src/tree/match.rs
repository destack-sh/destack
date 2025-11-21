use crate::{Block, Expression, LocalNodeId, LocalScopeId, Node, NodeType, Pattern};

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
        pattern: LocalNodeId<Pattern>,
        body: LocalNodeId<Expression>,
        guard: Option<LocalNodeId<Expression>>,
        scope: LocalScopeId,
    },
    /// A match case with a block body.
    Block {
        pattern: LocalNodeId<Pattern>,
        body: LocalNodeId<Block>,
        guard: Option<LocalNodeId<Expression>>,
        scope: LocalScopeId,
    },
}

impl Node for MatchCase {
    const TYPE: NodeType = NodeType::MatchCase;

    fn is_resolved(&self) -> bool {
        true
    }
}
