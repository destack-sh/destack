use crate::{Expression, LocalNodeId, Node, NodeType, Pattern, Type};

/// A Declarator is an individual variable declaration within a let/const/var statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Declarator {
    /// A binding declarator: pattern = value.
    Binding {
        pattern: LocalNodeId<Pattern>,
        ty: Option<LocalNodeId<Type>>,
        value: Option<LocalNodeId<Expression>>,
    },
}

impl Node for Declarator {
    const TYPE: NodeType = NodeType::Declarator;
}
