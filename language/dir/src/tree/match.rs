use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{Block, Expression, LocalNodeId, Node, NodeType, Pattern};

/// One arm of a match expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum MatchArm {
    /// An arm with an expression body.
    Expression {
        /// The selected pattern.
        pattern: LocalNodeId<Pattern>,
        /// The optional match guard.
        guard: Option<LocalNodeId<Expression>>,
        /// The expression body.
        body: LocalNodeId<Expression>,
    },
    /// An arm with a block body.
    Block {
        /// The selected pattern.
        pattern: LocalNodeId<Pattern>,
        /// The optional match guard.
        guard: Option<LocalNodeId<Expression>>,
        /// The block body.
        body: LocalNodeId<Block>,
    },
}

impl MatchArm {
    /// Return the arm pattern.
    pub fn pattern(&self) -> LocalNodeId<Pattern> {
        match self {
            Self::Expression { pattern, .. } | Self::Block { pattern, .. } => *pattern,
        }
    }

    /// Return the optional arm guard.
    pub fn guard(&self) -> Option<LocalNodeId<Expression>> {
        match self {
            Self::Expression { guard, .. } | Self::Block { guard, .. } => *guard,
        }
    }
}

impl Node for MatchArm {
    const TYPE: NodeType = NodeType::MatchArm;
}

/// One switch case selector.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum SwitchSelector {
    /// A case selected by an expression value.
    Case(LocalNodeId<Expression>),
    /// The default case.
    Default,
}

/// One case of a switch statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SwitchCase {
    /// The case selector.
    pub selector: SwitchSelector,
    /// The statement body.
    pub body: LocalNodeId<Block>,
}

impl Node for SwitchCase {
    const TYPE: NodeType = NodeType::SwitchCase;
}
