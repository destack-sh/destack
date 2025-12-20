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

/// A MatchSelector determines which case is selected in a match/switch expression.
///
/// For match expressions, this is a pattern with an optional guard.
/// For switch expressions, this can also be `Default` (the `default:` case).
#[derive(Debug, Clone, PartialEq)]
pub enum MatchSelector {
    /// A pattern with an optional guard (e.g., `x if x > 0`).
    Pattern {
        pattern: LocalNodeId<Pattern>,
        guard: Option<LocalNodeId<Expression>>,
    },
    /// The default case in a switch statement (`default:`).
    Default,
}

/// A MatchCase is a match case inside a Match expression.
#[derive(Debug, Clone, PartialEq)]
pub enum MatchCase {
    /// A match case with an expression body.
    Expression {
        selector: MatchSelector,
        body: LocalNodeId<Expression>,
        scope: LocalScopeId,
    },
    /// A match case with a block body.
    Block {
        selector: MatchSelector,
        body: LocalNodeId<Block>,
        scope: LocalScopeId,
    },
}

impl Node for MatchCase {
    const TYPE: NodeType = NodeType::MatchCase;
}
