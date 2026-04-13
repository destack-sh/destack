use serde::{Deserialize, Serialize};

use crate::{Expression, LocalNodeId, Node, NodeType, Pattern, TypeExpression};

/// A single variable declarator within a let/const/var statement.
/// Each declarator has its own pattern, optional type, and optional initializer.
///
/// Examples:
/// ```
/// x           // just a binding
/// x: int32    // binding with type
/// x = 1       // binding with value
/// x: int32 = 1  // binding with type and value
/// (a, b) = tuple  // destructuring pattern
/// ```
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
