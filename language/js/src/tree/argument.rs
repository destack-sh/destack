use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{Expression, LocalNodeId, Node, NodeType, Pattern, StringId};

/// Runtime modifiers of one class member.
#[derive(Debug, Copy, Clone, PartialEq, Default, Serialize, Deserialize, Reflect)]
pub struct MemberModifier {
    /// Whether the member belongs to the class itself.
    pub is_static: bool,
    /// Whether the field uses JavaScript auto-accessor semantics.
    pub is_accessor: bool,
}

/// Named or positional parameter to some construct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Parameter {
    /// Named parameter (like `x: int32` or `Validate: boolean = true`).
    Named {
        name: StringId,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Pattern parameter (like `_` or `{ x }` or `{ x, ..rest }: MyType = Foo`).
    Pattern {
        pattern: LocalNodeId<Pattern>,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Variadic parameter with a named binding (like `...args: int32[]`).
    VariadicNamed { name: StringId },
    /// Variadic parameter with a pattern binding (like `...[a, b]`).
    VariadicPattern { pattern: LocalNodeId<Pattern> },
}

impl Node for Parameter {
    const TYPE: NodeType = NodeType::Parameter;
}

/// Positional argument to some construct.
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
