use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{
    Block, Condition, Expression, LocalNodeId, LocalNodeIdAny, Node, NodeFold, NodeType, Pattern,
};

/// One arm of a match expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub enum MatchArm {
    /// An arm with an expression body.
    Expression {
        /// The selected pattern.
        pattern: LocalNodeId<Pattern>,
        /// The optional match guard.
        guard: Option<Condition>,
        /// The expression body.
        body: LocalNodeId<Expression>,
    },
    /// An arm with a block body.
    Block {
        /// The selected pattern.
        pattern: LocalNodeId<Pattern>,
        /// The optional match guard.
        guard: Option<Condition>,
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
    pub fn guard(&self) -> Option<&Condition> {
        match self {
            Self::Expression { guard, .. } | Self::Block { guard, .. } => guard.as_ref(),
        }
    }

    /// Return the arm body.
    pub fn body(&self) -> LocalNodeIdAny {
        match self {
            Self::Expression { body, .. } => body.into_any(),
            Self::Block { body, .. } => body.into_any(),
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub struct SwitchCase {
    /// The case selector.
    pub selector: SwitchSelector,
    /// The statement body.
    pub body: LocalNodeId<Block>,
}

impl Node for SwitchCase {
    const TYPE: NodeType = NodeType::SwitchCase;
}
