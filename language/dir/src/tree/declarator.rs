use serde::{Deserialize, Serialize};

use crate::{Expression, LocalNodeId, Node, NodeType, Pattern, TypeExpression};

/// A Declarator represents a single variable binding in a let/const/var declaration.
/// For example, in `let a: T1 = v1, b: T2 = v2`, there are two declarators.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Declarator {
    /// The pattern to bind (can be a simple identifier or destructuring pattern).
    pub pattern: LocalNodeId<Pattern>,
    /// Optional type annotation.
    pub ty: Option<LocalNodeId<TypeExpression>>,
    /// Optional value expression.
    pub value: Option<LocalNodeId<Expression>>,
}

impl Node for Declarator {
    const TYPE: NodeType = NodeType::Declarator;
}
