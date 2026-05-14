use serde::{Deserialize, Serialize};

use crate::{Block, Expression, LocalNodeId, Node, NodeType, Pattern};

/// The style of a match expression.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MatchForm {
    /// Regular match expression (like `match <expr> { ... }`).
    Match,
    /// Switch expression with cases (like `switch <expr> { ... }`).
    Switch,
}

/// A MatchSelector determines which case is selected in a match/switch expression.
///
/// For match expressions, this is a pattern with an optional guard.
/// For switch expressions, this can also be `Default` (the `default:` case).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MatchSelector {
    /// A pattern with an optional guard (e.g., `x if x > 0`).
    Pattern {
        pattern: LocalNodeId<Pattern>,
        guard: Option<LocalNodeId<Expression>>,
    },
    /// The default case in a switch statement (`default:`).
    Default,
}

impl MatchSelector {
    /// Return true when this selector is the default arm.
    pub fn is_default(&self) -> bool {
        matches!(self, Self::Default)
    }

    /// Return true when this selector has a guard expression.
    pub fn has_guard(&self) -> bool {
        matches!(self, Self::Pattern { guard: Some(_), .. })
    }

    /// Return the pattern id for pattern selectors.
    pub fn pattern_id(&self) -> Option<LocalNodeId<Pattern>> {
        match self {
            Self::Pattern { pattern, guard: _ } => Some(*pattern),
            Self::Default => None,
        }
    }

    /// Return the guard expression id for pattern selectors.
    pub fn guard_expression_id(&self) -> Option<LocalNodeId<Expression>> {
        match self {
            Self::Pattern {
                pattern: _,
                guard: Some(guard_id),
            } => Some(*guard_id),
            Self::Pattern {
                pattern: _,
                guard: None,
            }
            | Self::Default => None,
        }
    }
}

/// A MatchCase is a match case inside a Match expression.
///
/// Examples:
/// ```
/// 2 => parse_int(2)
/// (x, y) if x > y => {
///     ...
/// }
/// default: { ... }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MatchCase {
    /// A match case with an expression body.
    Expression {
        selector: MatchSelector,
        body: LocalNodeId<Expression>,
    },
    /// A match case with a block body.
    Block {
        selector: MatchSelector,
        body: LocalNodeId<Block>,
    },
}

impl MatchCase {
    /// Return the selector for this match case.
    pub fn selector(&self) -> &MatchSelector {
        match self {
            Self::Expression { selector, body: _ } | Self::Block { selector, body: _ } => selector,
        }
    }
}

impl Node for MatchCase {
    const TYPE: NodeType = NodeType::MatchCase;
}
