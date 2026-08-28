use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{Expression, Identifier, LocalNodeId, Node, NodeType, Pattern};

/// One JavaScript function parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Parameter {
    /// Named parameter like `value` or `validate = true`.
    Named {
        name: Identifier,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Pattern parameter like `{ value, ...rest } = source`.
    Pattern {
        pattern: LocalNodeId<Pattern>,
        default: Option<LocalNodeId<Expression>>,
    },
}

impl Node for Parameter {
    const TYPE: NodeType = NodeType::Parameter;
}

/// One element in an array literal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ArrayElement {
    /// One positional array element.
    Expression { value: LocalNodeId<Expression> },
    /// One spread array element.
    Spread { value: LocalNodeId<Expression> },
    /// One elided array slot.
    Elision,
}

impl Node for ArrayElement {
    const TYPE: NodeType = NodeType::ArrayElement;
}

/// One JavaScript call argument.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Argument {
    /// Positional argument like `1` or `foo()`.
    Positional { value: LocalNodeId<Expression> },
    /// Spread argument like `...values`.
    Spread { value: LocalNodeId<Expression> },
}

impl Node for Argument {
    const TYPE: NodeType = NodeType::Argument;
}
