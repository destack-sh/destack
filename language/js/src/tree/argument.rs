use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{Expression, Identifier, LocalNodeId, Node, NodeType, Pattern};

/// Runtime modifiers of one class member.
#[derive(Debug, Copy, Clone, PartialEq, Default, Serialize, Deserialize, Reflect)]
pub struct MemberModifier {
    /// Whether the member belongs to the class itself.
    pub is_static: bool,
    /// Whether the field uses JavaScript auto-accessor semantics.
    pub is_accessor: bool,
}

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
    /// Variadic parameter with a named binding (like `...args: int32[]`).
    VariadicNamed { name: Identifier },
    /// Variadic parameter with a pattern binding (like `...[a, b]`).
    VariadicPattern { pattern: LocalNodeId<Pattern> },
}

impl Node for Parameter {
    const TYPE: NodeType = NodeType::Parameter;
}

/// One JavaScript call argument.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Argument {
    /// Positional argument (like `1` or `foo()`).
    Positional { value: LocalNodeId<Expression> },
    /// Spread argument (like `...args`).
    Spread { value: LocalNodeId<Expression> },
}

impl Node for Argument {
    const TYPE: NodeType = NodeType::Argument;
}
