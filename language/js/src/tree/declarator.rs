use crate::{Expression, LocalNodeId, Node, NodeType, Pattern};

use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// One variable declarator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Declarator {
    /// The bound pattern.
    pub pattern: LocalNodeId<Pattern>,
    /// The initial value when one exists.
    pub value: Option<LocalNodeId<Expression>>,
}

impl Node for Declarator {
    const TYPE: NodeType = NodeType::Declarator;
}
