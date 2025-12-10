//! Declarator node for variable declarations.

use crate::{Expression, LocalNodeId, Node, NodeType, Pattern};

// nocheckin: make declarators across language simple structs (not enums)

/// A Declarator represents a single variable binding in a let/const/var declaration.
/// For example, in `let a: T1 = v1, b: T2 = v2`, there are two declarators.
#[derive(Debug, Clone, PartialEq)]
pub enum Declarator {
    /// Binding declarator (like `x = 1` or `{ a, b } = obj`).
    Binding {
        /// The pattern to bind (can be a simple identifier or destructuring pattern).
        pattern: LocalNodeId<Pattern>,
        /// Optional type annotation.
        ty: Option<LocalNodeId<Expression>>,
        /// Optional value expression.
        value: Option<LocalNodeId<Expression>>,
    },
}

impl Node for Declarator {
    const TYPE: NodeType = NodeType::Declarator;
}
